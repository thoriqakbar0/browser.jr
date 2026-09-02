# Browser network architecture patterns

**Evidence date:** 2026-09-01

## Scope

This note compares implementation patterns in Chromium, Firefox/Necko, WebKit, Servo, Lightpanda, and browser automation protocols. It focuses on URL loading, redirects, DNS, TLS, network isolation, private-network controls, and final-URL reporting. The recommendations target a small Rust agent browser. They do not propose cloning a full browser stack.

## Cross-engine findings

### 1. Keep navigation policy above the HTTP transport

Chromium separates navigation and web-platform policy from its low-level Network Service. A caller creates a `ResourceRequest` and a `URLLoaderClient`; a `URLLoaderFactory` creates a `URLLoader`; and the loader drives a `URLRequest` through cache, connection, and protocol layers. Chromium's Network Service design explicitly keeps higher-level browser features outside the low-level service except for minimal enforcement hooks. Sources: [Chromium: Life of a URLRequest](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md), [Chromium Network Service README](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/README.md).

Firefox uses the same broad seam with different names: Necko exposes asynchronous channels, commonly `nsHttpChannel`, while load context and security metadata live in `LoadInfo`; socket work is handled away from untrusted content, including by the socket process/thread. Sources: [Mozilla: Necko lingo](https://firefox-source-docs.mozilla.org/networking/necko_lingo.html), [Mozilla `nsIChannel.idl`](https://searchfox.org/mozilla-central/source/netwerk/base/nsIChannel.idl), [Mozilla `LoadInfo.cpp`](https://searchfox.org/mozilla-central/source/netwerk/base/LoadInfo.cpp).

WebKit places request execution in the Network Process. `NetworkLoad` mediates between a client and a platform-specific `NetworkDataTask`; it sends redirect decisions back to the client instead of letting the platform transport silently own navigation policy. Sources: [WebKit `NetworkLoad.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkLoad.cpp), [WebKit `NetworkDataTask.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkDataTask.cpp), [WebKit `ResourceLoader.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/loader/ResourceLoader.cpp).

Servo uses a resource manager outside each content pipeline and implements Fetch semantics in its network crate. Its architecture favors actor-like components with isolated state and message boundaries. Sources: [Servo architecture](https://book.servo.org/design-documentation/architecture.html), [Servo `http_loader.rs`](https://github.com/servo/servo/blob/main/components/net/http_loader.rs), [Servo `fetch/methods.rs`](https://github.com/servo/servo/blob/main/components/net/fetch/methods.rs).

**Small-browser pattern:** use three layers:

1. `NavigationController`: user-visible navigation, history, cancellation, committed/final URL.
2. `FetchEngine`: redirect policy, request mutation, limits, address policy, response/body limits.
3. `Transport`: DNS, connect, proxy, TLS, HTTP framing, and byte streaming.

This preserves a narrow transport interface while preventing a library's redirect or DNS defaults from bypassing browser policy.

### 2. Redirects are a state machine, not a transport option

The Fetch Standard models a request with a URL list, a current URL pointing to the last list entry, and a redirect count. HTTP redirect fetch resolves `Location` against the response URL, carries the old fragment when required, limits redirect count, changes methods/bodies for specific statuses, strips sensitive headers on cross-origin transitions, updates referrer policy, appends the new URL, and fetches again. Source: [WHATWG Fetch Standard](https://fetch.spec.whatwg.org/#http-redirect-fetch).

Chromium exposes each redirect through `URLLoaderClient::OnReceiveRedirect` and waits for `FollowRedirect`, allowing the trusted client and throttles to approve or modify the next request. Source: [Chromium `url_loader.mojom`](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/public/mojom/url_loader.mojom).

WebKit's `NetworkLoad::willPerformHTTPRedirection` passes the old request, proposed new request, and redirect response to its client, then resumes with the client's chosen request. Source: [WebKit `NetworkLoad.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkLoad.cpp).

Servo directly implements the Fetch redirect algorithm, maintains `redirect_count`, appends the target to the request URL list, and caps the count at 20 to match Fetch. Source: [Servo `http_loader.rs`](https://github.com/servo/servo/blob/main/components/net/http_loader.rs).

Lightpanda disables libcurl's automatic redirect following, detects a redirect status plus `Location`, resolves the target itself, preserves fragments, applies method/body rules, strips cross-origin credentials, counts hops, and reissues the request through its pipeline. Source: [Lightpanda `HttpClient.zig`](https://github.com/lightpanda-io/browser/blob/main/src/network/HttpClient.zig).

**Small-browser pattern:** disable automatic redirects in the HTTP client. Represent each hop as data:

```text
RedirectHop {
  request_url,
  status,
  location_raw,
  resolved_url,
  method_before,
  method_after,
}
```

Run the same validation pipeline for the initial URL and every redirect target. Keep one navigation ID across hops, but create a new request attempt for each hop.

### 3. Enforce private-network policy on resolved socket addresses

URL-string checks alone do not stop DNS rebinding or hostnames that resolve to loopback, link-local, private, or metadata addresses. The current Local Network Access specification classifies targets using the IP address space discovered for the connection and distinguishes public, local, and loopback address spaces. Sources: [WICG Local Network Access](https://wicg.github.io/local-network-access/), [WICG Private Network Access](https://wicg.github.io/private-network-access/).

Chromium carries address-space and network-isolation metadata with requests and performs checks in the Network Service, which owns host resolution and connection creation. Its `NetworkContext` exposes host resolution and applies network restrictions before network use. Sources: [Chromium: Life of a URLRequest](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md), [Chromium `network_context.cc`](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/network_context.cc), [Chromium `network_context.mojom`](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/public/mojom/network_context.mojom).

WebKit rejects restricted ports and disallowed IP addresses in both WebCore loading and Network Process task creation. Source: [WebKit `ResourceLoader.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/loader/ResourceLoader.cpp), [WebKit `NetworkDataTask.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkDataTask.cpp).

Lightpanda attaches an IP filter to libcurl's socket-opening path. Its filter covers private IPv4/IPv6 groups, IPv4-mapped IPv6, link-local cloud-metadata addresses, custom CIDRs, allowlist exceptions, and fail-closed handling for unknown address families. Sources: [Lightpanda `Network.zig`](https://github.com/lightpanda-io/browser/blob/main/src/network/Network.zig), [Lightpanda `IpFilter.zig`](https://github.com/lightpanda-io/browser/blob/main/src/network/IpFilter.zig).

**Small-browser pattern:**

- Parse and scheme-check the URL before DNS.
- Resolve the host to all candidate addresses.
- Classify every candidate address.
- Refuse the request if any address the connector may use is forbidden, or constrain the connector to an explicitly approved address set.
- Re-run resolution and classification for every redirect and retry.
- Validate the actual peer address after connect when the transport exposes it.
- Normalize IPv4-mapped IPv6 before classification.
- Treat loopback, link-local, unspecified, multicast, private-use, and cloud metadata ranges as separate named policy classes.

Do not implement public/private safety as a preflight `is_private_hostname()` test followed by an unconstrained client request.

### 4. DNS belongs inside the security boundary

Chromium's `URLRequestContext` owns the host resolver alongside the cache and cookie store; connection setup consumes its results. Network isolation metadata is passed into resolution so DNS state can be partitioned. Source: [Chromium: Life of a URLRequest](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md), [Chromium `HostResolver`](https://chromium.googlesource.com/chromium/src/+/HEAD/net/dns/host_resolver.h).

Firefox's Necko owns DNS and supports system DNS and Trusted Recursive Resolver/DoH paths. Firefox documentation identifies DNS, DoH/TRR, the socket process, and NSS as parts of the networking/security stack rather than page code. Sources: [Mozilla networking overview](https://firefox-source-docs.mozilla.org/overview/gecko.html#networking-necko), [Mozilla DNS-over-HTTPS implementation docs](https://firefox-source-docs.mozilla.org/networking/dns/dns-over-https-trr.html), [Mozilla `nsHostResolver.cpp`](https://searchfox.org/mozilla-central/source/netwerk/dns/nsHostResolver.cpp).

**Small-browser pattern:** expose a resolver interface that returns classified addresses and resolution metadata. The fetch engine should pass an isolation key and deadline into it. The connector should consume only the approved result, rather than resolve the hostname again internally.

### 5. TLS policy must be explicit and default-secure

Firefox uses NSS for TLS. Source: [Mozilla networking overview](https://firefox-source-docs.mozilla.org/overview/gecko.html#networking-necko).

Servo builds a rustls client configuration, records handshake/certificate information, and treats successful rustls TLS 1.2+ connections as secure under its chosen cipher policy; certificate-error overrides are explicit state. Source: [Servo `http_loader.rs`](https://github.com/servo/servo/blob/main/components/net/http_loader.rs), [Servo connector module](https://github.com/servo/servo/tree/main/components/net).

Lightpanda initializes TLS-capable libcurl connections, installs certificate configuration, and applies host verification per connection. Its CLI exposes an insecure verification override for testing, which is deliberately named as insecure. Sources: [Lightpanda `Network.zig`](https://github.com/lightpanda-io/browser/blob/main/src/network/Network.zig), [Lightpanda `Certificates.zig`](https://github.com/lightpanda-io/browser/blob/main/src/network/Certificates.zig), [Lightpanda README](https://github.com/lightpanda-io/browser#run-wpt-test-suite).

**Small-browser pattern:** use rustls with WebPKI roots or a documented platform-root adapter. Verify the certificate chain and hostname by default. Keep insecure TLS behind an explicit test-only configuration field, surface it in result metadata, and never silently retry plaintext HTTP after TLS failure.

### 6. Partition reusable network state even in a small design

Chromium groups network state in `NetworkContext`s owned by storage partitions. Requests carry `IsolationInfo`, including a network isolation/anonymization key; caches and other network resources use that key to avoid cross-site sharing. Sources: [Chromium: Life of a URLRequest](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md), [Chromium network isolation key design](https://chromium.googlesource.com/chromium/src/+/HEAD/net/base/network_isolation_key.h), [Chromium network anonymization key](https://chromium.googlesource.com/chromium/src/+/HEAD/net/base/network_anonymization_key.h).

Firefox attaches origin attributes and other security context to principals and `LoadInfo`; private and normal browsing use distinct network state. Sources: [Mozilla `LoadInfo.cpp`](https://searchfox.org/mozilla-central/source/netwerk/base/LoadInfo.cpp), [Mozilla `OriginAttributes.h`](https://searchfox.org/mozilla-central/source/caps/OriginAttributes.h), [Mozilla `nsILoadInfo.idl`](https://searchfox.org/mozilla-central/source/netwerk/base/nsILoadInfo.idl).

WebKit creates per-session network state and uses partitioned cache/storage policies for tracking prevention. Sources: [WebKit `NetworkSession.cpp`](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkSession.cpp), [WebKit Tracking Prevention](https://webkit.org/tracking-prevention/).

**Small-browser pattern:** define a cheap `NetworkPartitionKey` now, even if its first value is only `SessionId`. Key DNS cache, connection pool, cookies, credentials, and HTTP cache by it. Use a distinct ephemeral key for each isolated agent session. This avoids an API-breaking redesign when parallel pages or untrusted tasks arrive.

### 7. Preserve requested, current, and final URLs separately

The Fetch Standard keeps the whole URL list internally, defines current URL as the last entry, and exposes a response URL pointing to the last response URL. Intermediate URLs are not generally exposed to page script because redirect handling is atomic. Source: [WHATWG Fetch Standard: requests and responses](https://fetch.spec.whatwg.org/#requests).

Firefox channels distinguish `originalURI` from the current `URI`, so callers can retain the initial address while redirects update the channel's current address. Source: [Mozilla `nsIChannel.idl`](https://searchfox.org/mozilla-central/source/netwerk/base/nsIChannel.idl).

Playwright exposes redirect relationships on requests, returns the main-resource response from `page.goto`, and exposes the current page URL separately. This lets automation inspect the chain without confusing the user-visible committed URL with the initial input. Sources: [Playwright `Request.redirectedFrom`](https://playwright.dev/docs/api/class-request#request-redirected-from), [Playwright `page.goto`](https://playwright.dev/docs/api/class-page#page-goto), [Playwright `page.url`](https://playwright.dev/docs/api/class-page#page-url).

Chrome DevTools Protocol navigation responses include the frame and loader identity, while frame state carries the current URL; network events use stable request/loader relationships to report redirect responses. Sources: [CDP Page domain](https://chromedevtools.github.io/devtools-protocol/tot/Page/), [CDP Network domain](https://chromedevtools.github.io/devtools-protocol/tot/Network/).

**Small-browser pattern:** return all three concepts:

```text
NavigationResult {
  requested_url,       // normalized initial input
  final_url,           // last successfully committed URL
  redirect_chain,      // ordered hop records
  status,
  response_headers,
  remote_address,
  tls,
}
```

Do not overwrite the requested URL in place. Update page/history state only when the navigation commits. On failure before commit, retain the previous page and return the last attempted URL separately in the error.

## Engine-specific takeaways

| Engine/runtime | Useful pattern for browser.jr | Avoid copying |
| --- | --- | --- |
| Chromium | Privileged request factory; explicit request/client channel; partition key; client-approved redirects. [Source](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md) | Mojo/process complexity, speculative loading, large interceptor stack. |
| Firefox/Necko | Async channel abstraction; `LoadInfo`; original/current URI split; resolver and TLS owned by networking. [Source](https://firefox-source-docs.mozilla.org/networking/necko_lingo.html) | XPCOM surface and process-specific channel variants. |
| WebKit | `NetworkLoad` policy object over platform `NetworkDataTask`; redirect callback; restricted-address checks near task creation. [Source](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/NetworkProcess/NetworkLoad.cpp) | Per-platform backend matrix and UI-process IPC. |
| Servo | Rust actors/channels; Fetch-shaped redirect algorithm; rustls; bounded redirect count. [Source](https://github.com/servo/servo/blob/main/components/net/http_loader.rs) | Full Fetch/CORS/service-worker machinery before browser.jr needs it. |
| Lightpanda | Practical libcurl wrapper that disables auto-redirects and installs a connect-time IP filter. [Source](https://github.com/lightpanda-io/browser/blob/main/src/network/HttpClient.zig) | Large compatibility surface, CDP interception, cache/cookie features not required for initial public loading. |
| Playwright/CDP | Stable navigation identity, redirect chain inspection, and explicit final page URL. [Source](https://playwright.dev/docs/api/class-request#request-redirected-from) | Protocol compatibility details inside the core loader. Put adapters above it. |

## Recommended minimal Rust architecture

```text
Page
└── NavigationController
    ├── NavigationState { id, requested_url, attempts, commit_state }
    └── FetchEngine
        ├── UrlPolicy
        ├── RedirectPolicy
        ├── AddressPolicy
        ├── Limits
        └── NetworkSession
            ├── Resolver
            ├── Connector/TLS
            ├── HTTP client
            └── partitioned pools/caches
```

Suggested core interfaces:

```rust
trait Resolver {
    async fn resolve(
        &self,
        host: &str,
        port: u16,
        partition: &NetworkPartitionKey,
        deadline: Instant,
    ) -> Result<ResolvedHost, ResolveError>;
}

trait Transport {
    async fn send(
        &self,
        request: WireRequest,
        approved: &ResolvedHost,
        partition: &NetworkPartitionKey,
        limits: &NetworkLimits,
    ) -> Result<WireResponse, TransportError>;
}
```

`ResolvedHost` should contain all candidate IPs, their address-space classifications, DNS provenance, and expiry. `WireResponse` should include the actual peer address and TLS summary. The fetch engine should reject a mismatch between the approved set and the peer address.

## Priority order for browser.jr

1. **Own redirects.** Disable client auto-follow and record every hop.
2. **Close DNS-to-connect gaps.** Classify resolved addresses and bind connection attempts to the approved set.
3. **Apply policy on every hop and retry.** A safe initial URL does not make its redirect safe.
4. **Return explicit URL identities.** Keep requested, attempted/current, and final/committed URLs distinct.
5. **Make limits one object.** Redirects, DNS time, connect time, headers, body bytes, decompressed bytes, and total wall time should share one navigation budget.
6. **Add a partition key before caches.** Key future DNS, connection, cookie, and cache state consistently.
7. **Keep TLS strict.** Make test overrides explicit and observable.

## Important limits of this comparison

Chromium, Firefox, and WebKit have browser-process and sandbox boundaries that browser.jr may not need initially. The transferable design is the *logical* privilege boundary: untrusted page or automation input must not choose resolver behavior, peer addresses, certificate exceptions, redirect limits, or partition keys. Sources: [Chromium process model and Site Isolation](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/process_model_and_site_isolation.md), [Servo architecture](https://book.servo.org/design-documentation/architecture.html), [Mozilla Necko lingo](https://firefox-source-docs.mozilla.org/networking/necko_lingo.html).

Private/local network controls are evolving. As of this evidence date, Chromium documentation describes Local Network Access permission behavior, while the WICG Local Network Access specification supersedes terminology from Private Network Access. browser.jr should implement its own documented server-side-agent policy rather than claim exact Chrome UI permission parity. Sources: [Chrome Local Network Access](https://developer.chrome.com/blog/local-network-access), [WICG Local Network Access](https://wicg.github.io/local-network-access/), [Chrome 142 release notes](https://developer.chrome.com/release-notes/142#local-network-access-restrictions).
