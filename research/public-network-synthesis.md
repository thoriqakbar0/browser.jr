# Public-network solution synthesis

**Evidence date:** 2026-09-01  
**Inputs:** Rust ecosystem, browser architecture, security standards, and Rust/browser research tracks.

## Decision

Build browser.jr's durable public-network loader as a **browser.jr-owned fetch state machine** over:

- [`url::Url`](https://docs.rs/url/2.5.8/url/struct.Url.html) for WHATWG URL parsing and redirect joining,
- [`hyper` and `hyper-util`](https://hyper.rs/guides/1/client/basic/) for HTTP mechanics and connector control,
- [`rustls`](https://github.com/rustls/rustls) for strict TLS,
- a browser.jr-owned resolver trait, initially backed by the operating-system resolver through a controlled adapter,
- optionally [`hickory-resolver`](https://docs.rs/hickory-resolver/) when deterministic DNS, DNS metadata, or async resolver control becomes necessary.

Keep policy, redirect handling, limits, errors, and tests independent of the HTTP library.

The current `ureq` implementation is a useful prototype, not the production security boundary. Its resolver and connector customization lives under an explicitly non-semver `unversioned` API. [`ureq` documents that status directly](https://docs.rs/ureq/3.4.0/ureq/unversioned/resolver/index.html).

## Why this resolves the crate disagreement

The Rust ecosystem track found `reqwest 0.13 + rustls` to be the most convenient production client. It exposes proxy disabling, redirects, timeouts, DNS overrides, and response streaming through a stable high-level API. [`ClientBuilder`](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html) provides those controls.

The security and browser tracks add a stronger requirement: browser.jr must prove that the connected peer is one of the addresses approved during resolution, without a second hidden lookup, including retries and redirects. That requirement favors direct connector ownership. Hyper is intentionally lower-level, but this is the layer where browser.jr needs control. Reqwest remains a valid short-term spike or could be adopted later if a connector-level proof satisfies the same contract.

Therefore:

```text
Convenience-first HTTP client       reqwest
Security-boundary HTTP mechanism    hyper + hyper-util
Current synchronous prototype       ureq
Chosen durable browser.jr path       hyper + hyper-util
```

## Required architecture

```text
Page
`-- NavigationController
    |-- NavigationState
    |   |-- requested_url
    |   |-- current_attempt_url
    |   |-- redirect_chain
    |   `-- committed_url
    `-- FetchEngine
        |-- UrlPolicy
        |-- RedirectPolicy
        |-- AddressPolicy
        |-- NetworkLimits
        `-- NetworkSession
            |-- Resolver
            |-- ApprovedEndpoints
            |-- Connector + rustls
            `-- HTTP transport
```

The browser engines surveyed use the same broad separation: navigation and redirect policy sit above the transport, while the network layer owns connections and protocol mechanics. Chromium documents this split in its [Network Service](https://chromium.googlesource.com/chromium/src/+/HEAD/services/network/README.md) and [URLRequest lifecycle](https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/life-of-a-url-request.md). Servo is the closest Rust reference for Fetch semantics, but its network component is a reference to mine rather than a small module to copy. See Servo's [Fetch implementation](https://github.com/servo/servo/blob/master/components/net/fetch/methods.rs) and [HTTP loader](https://github.com/servo/servo/blob/master/components/net/http_loader.rs).

## Release-blocking invariants

### URL

- Parse once with `url::Url`.
- Accept absolute `http` and `https` only.
- Reject userinfo, ambiguous numeric hosts, IPv6 zone identifiers, and malformed input.
- Remove fragments before transmission.
- Resolve every redirect with `Url::join` and rerun the entire policy.

The [`url` crate](https://github.com/servo/rust-url) implements the [WHATWG URL Standard](https://url.spec.whatwg.org/). `http::Uri` is not a browser URL parser.

### DNS and connection

- Resolve A and AAAA records under one navigation deadline.
- Reject the complete hostname if **any** answer is denied.
- Classify addresses from the current [IANA IPv4](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml) and [IPv6](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml) special-purpose registries.
- Pass only approved binary socket addresses to the connector.
- Do not perform another DNS lookup during connection or retry.
- Verify the actual peer belongs to the approved set before sending the request.
- Keep the canonical hostname for TLS SNI and certificate verification.

This closes the DNS rebinding and validation-to-connection gap described by [OWASP SSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html) and the original [DNS rebinding research](https://crypto.stanford.edu/dns/dns-rebinding.pdf).

### Redirects

- Disable automatic redirects.
- Treat every redirect as a fresh request.
- Reapply URL, DNS, address, TLS, header, and resource policy.
- Reject loops and enforce one shared redirect budget.
- Reject HTTPS-to-HTTP downgrade by default.
- Do not forward origin-bound credentials or headers across origins.
- Preserve the validated redirect chain and final committed URL.

Redirect method semantics come from [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html). Strict TLS and downgrade guidance comes from [RFC 9325](https://www.rfc-editor.org/rfc/rfc9325.html).

### Authority and state

- Disable environment and operating-system proxies.
- Do not load cookies, `.netrc`, browser profiles, client certificates, custom roots, or ambient credentials.
- Public fetch is stateless GET/HEAD only.
- Keep local development as a separate explicit capability. Public mode must not include `localhost` or loopback exceptions.
- Partition future DNS, connection, cookie, and cache state with one network partition key.

### Resource limits

Use one `NetworkLimits` value and one total navigation deadline covering:

```text
DNS
  -> connection attempts
  -> TLS handshake
  -> response headers
  -> every redirect
  -> wire body
  -> decompression
  -> decoded parser input
```

Set distinct caps for DNS answers, connection attempts, redirects, header bytes/count, compressed wire bytes, decoded bytes, decompression ratio/work, per-read idle time, total time, and concurrent fetches. Redirects must not reset the total budget.

## Comparison with the current prototype

```diff
 Current ureq prototype
+  HTTP and HTTPS with rustls
+  proxies disabled
+  one MiB body-reader limit
+  15-second configured timeout
+  five redirects
+  custom resolver filters every returned address
+  rejects mixed public/private answers
+  tracks ureq's final response URL

 Missing before production
+  WHATWG parsing with url::Url
+  manual redirect state machine
+  HTTPS-to-HTTP downgrade rejection
+  public and loopback as separate capabilities
+  IANA-derived exhaustive address policy
+  proof that connection uses only approved addresses
+  actual peer-address verification
+  one deadline spanning all redirects and body decoding
+  separate wire-byte and decoded-byte limits
+  decompression work/ratio limits
+  header, attempt, idle, and concurrency limits
+  explicit TLS-version and trust-root tests
+  deterministic resolver and connector test seams
```

The prototype should remain uncommitted or clearly labeled experimental until these gaps are resolved.

## Implementation sequence

1. **Freeze behavior in tests.** Add adversarial cases for URL ambiguity, all IANA special ranges, mixed DNS answers, rebinding-like answer changes, redirect-to-private, downgrade, timeout accumulation, oversized wire/decoded bodies, and peer mismatch.
2. **Create policy types.** Add `ParsedHttpUrl`, `AddressSpace`, `ApprovedEndpoints`, `NetworkLimits`, `NetworkPartitionKey`, and structured error variants.
3. **Build the redirect state machine.** Make it transport-independent with an injected fake resolver and transport.
4. **Build a Hyper connector spike.** Prove resolve → approve → connect → peer verify uses one address set while TLS validates the hostname.
5. **Add rustls and streaming limits.** Keep decompression disabled until both wire and decoded limits are enforced.
6. **Separate modes.** `PublicOnly` by default; explicit `LoopbackOnly` for local fixtures. Do not permit public-to-local or local-to-public redirects.
7. **Mine conformance tests.** Port only relevant cases from [Web Platform Tests Fetch](https://github.com/web-platform-tests/wpt/tree/master/fetch) and Servo.
8. **Replace the ureq prototype** after the new loader passes the threat-model matrix.
9. **Add deployment defense.** Document egress firewall requirements so application checks are not the only SSRF boundary.

## Deferred work

- Hickory DNS until resolver determinism or DNS metadata is required.
- Connection pooling until partitioning and policy-change behavior are defined.
- Cookies, caching, authentication, proxies, and arbitrary request headers.
- Helper-process or WASI sandboxing until the in-process authorization path is correct.
- Record/replay after the fetch trace format and privacy rules are designed.

## Implementation status

As of 2026-09-01, browser.jr uses the recommended `url` + policy + approved endpoints + Hyper + rustls path. The transport connects to exact approved socket addresses, verifies the peer before the HTTP handshake, preserves the hostname for TLS, owns redirects, rejects downgrade, and streams an uncompressed body through its byte limit.

Browser.jr now separates public and loopback access and reports DNS, connection, TLS, timeout, response, and body failures by phase. Deployment egress controls remain an environment responsibility and must reinforce the application policy.

## Bottom line

The Rust-world solution is not one crate. It is a narrow, browser-owned authorization and navigation state machine with mature crates underneath it:

```text
url + browser.jr policy + approved endpoints + hyper + rustls
```

Research and real browser designs agree on the critical boundary: authorize the destination at connection time, repeat that authorization for every redirect, verify the actual peer, and enforce one bounded navigation budget.
