# Rust options for safe public-network fetching

**Evidence date:** 2026-09-01  
**Scope:** Production-grade fetching of user-supplied public `http`/`https` URLs for browser.jr. This is library and architecture research, not an implementation specification.

## Recommendation

Use **`reqwest` 0.13 with `rustls`**, an explicit redirect loop, and a small security-owned connector/resolver boundary that returns only approved socket addresses. Parse and join URLs with **`url::Url`**. Disable ambient proxies. Bound every phase and stream the decoded response through an application byte limit.

This is the best long-term choice because `reqwest` exposes stable client controls for redirect policy, proxy disabling, DNS overrides, connect/read/total timeouts, TLS selection, and response streaming while building on `hyper`; its current stable release was 0.13.4 on the evidence date ([crate record](https://crates.io/api/v1/crates/reqwest), [ClientBuilder docs](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html), [redirect policy docs](https://docs.rs/reqwest/0.13.4/reqwest/redirect/struct.Policy.html)).

Do **not** build directly on `hyper` unless browser.jr needs transport behavior that `reqwest` cannot expose. Hyper deliberately provides HTTP primitives rather than a complete ergonomic client, and client users must assemble a runtime, connector, pooling, TLS, redirects, DNS policy, and body handling themselves; stable 1.11.1 remained actively released on 2026-08-28 ([crate record](https://crates.io/api/v1/crates/hyper), [client guide](https://hyper.rs/guides/1/client/basic/), [hyper-util legacy client docs](https://docs.rs/hyper-util/latest/hyper_util/client/legacy/index.html)).

The current **`ureq` 3.4.0 + rustls** implementation is a reasonable synchronous proof-of-concept, but its security-critical custom resolver and connector APIs are under `ureq::unversioned`; ureq explicitly says those APIs do not yet follow semantic versioning ([resolver source/docs](https://docs.rs/ureq/3.4.0/ureq/unversioned/resolver/index.html), [transport source/docs](https://docs.rs/ureq/3.4.0/ureq/unversioned/transport/index.html), [crate record](https://crates.io/api/v1/crates/ureq)). Keeping ureq is defensible for the next small milestone if the redirect loop becomes explicit and the unversioned dependency is accepted, but it is not the strongest durable boundary for a network-security subsystem.

## Required security shape

1. Parse once with `url::Url`, require exactly `http` or `https`, require a host, reject credentials, and derive every redirect with `Url::join`. The `url` crate implements the WHATWG URL Standard, including host parsing, IDNA processing, special-scheme behavior, and relative-reference resolution; stable 2.5.8 was current on the evidence date ([crate record](https://crates.io/api/v1/crates/url), [Url docs](https://docs.rs/url/2.5.8/url/struct.Url.html), [WHATWG URL Standard](https://url.spec.whatwg.org/)). `http::Uri` is an HTTP message URI type, not a browser URL parser or relative-URL implementation ([`http::Uri` docs](https://docs.rs/http/1.5.0/http/uri/struct.Uri.html)).
2. Resolve the hostname inside the connection path, classify **every returned address**, and pass only those exact approved addresses to the socket connector. Preflight DNS validation followed by normal hostname connection is vulnerable to time-of-check/time-of-use changes; OWASP recommends allow/deny validation at the resolved-IP boundary and warns about DNS pinning/rebinding ([OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)).
3. Treat each redirect as a new untrusted URL. Disable automatic redirects or use a hook only to stop and return the candidate; then repeat URL validation, DNS resolution, address classification, method rules, header stripping, and limits. Reqwest supports `Policy::none`, `Policy::limited`, and `Policy::custom`, but its policy hook decides follow/stop/error rather than replacing the full security transaction ([Policy docs](https://docs.rs/reqwest/0.13.4/reqwest/redirect/struct.Policy.html), [Attempt docs](https://docs.rs/reqwest/0.13.4/reqwest/redirect/struct.Attempt.html)).
4. Disable proxies by default. Reqwest otherwise uses system proxy configuration, and `ClientBuilder::no_proxy()` disables all proxy use; ureq's default configuration also discovers proxy settings from the environment, while `.proxy(None)` disables them ([reqwest proxy docs](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.no_proxy), [ureq Config docs](https://docs.rs/ureq/3.4.0/ureq/config/struct.Config.html#method.proxy)). A proxy would move DNS and connection enforcement outside browser.jr's connector unless the proxy itself were a separately trusted policy component.
5. Apply separate connect, read-idle, and whole-operation deadlines, plus redirect count, response-header size, decoded-body byte count, and decompression-ratio/work limits. Reqwest exposes connect, read, and total request timeouts and streams bodies as chunks; it can also transparently decompress content when compression features are enabled ([timeout methods](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.connect_timeout), [read timeout](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.read_timeout), [Response::bytes_stream](https://docs.rs/reqwest/0.13.4/reqwest/struct.Response.html#method.bytes_stream), [compression controls](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.no_gzip)). Ureq's buffered helpers default to a 10 MiB limit and allow an explicit body-reader limit, but the application must still select its own smaller policy ([ureq Body docs](https://docs.rs/ureq/3.4.0/ureq/struct.Body.html)).
6. Preserve the original hostname for the HTTP `Host` authority and TLS SNI/certificate verification while pinning only the socket address. Reqwest's `resolve`/`resolve_to_addrs` methods override DNS resolution for a domain while leaving the request URL domain intact ([resolve docs](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.resolve_to_addrs)). Do not rewrite an HTTPS URL to an IP literal.
7. Disable connection reuse across security-policy changes and be deliberate about pooling. A pool entry is already connected to an approved address, but changes to loopback allowances, tenant policy, proxy policy, or DNS expectations should not silently share it. Reqwest exposes pool idle timeout and maximum idle connections per host ([pool controls](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.pool_max_idle_per_host)); ureq exposes analogous pool controls ([ureq Config docs](https://docs.rs/ureq/3.4.0/ureq/config/struct.Config.html)).

## Library comparison

| Option | Production strengths | Security/testability drawbacks | Verdict for browser.jr |
|---|---|---|---|
| `reqwest` 0.13.4 | Stable high-level API; async and blocking facades; custom redirects; proxy disable; timeout controls; DNS overrides; streaming; rustls/native-tls selection ([features](https://docs.rs/reqwest/0.13.4/reqwest/#optional-features), [builder](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html)). | `resolve_to_addrs` is static configuration, so per-request dynamic policy is awkward; a custom `dns::Resolve` or explicit per-hop client/connector design is needed. Automatic decompression and pooling require explicit policy ([dns module](https://docs.rs/reqwest/0.13.4/reqwest/dns/index.html)). | **Preferred.** Use async internally even if the CLI remains synchronous at its edge. |
| `hyper` 1.11.1 + `hyper-util` | Maximum control over connector, destination metadata, HTTP versions, pooling, and bodies; strong base for a deep network module ([client conn docs](https://docs.rs/hyper/1.11.1/hyper/client/conn/index.html), [Connect trait](https://docs.rs/hyper-util/latest/hyper_util/client/legacy/connect/trait.Connect.html)). | Redirects, cookies, DNS, TLS, decompression, ergonomics, and many policy details are application work. More code creates more audit surface. | Use only if reqwest's resolver seam proves insufficient. |
| `ureq` 3.4.0 | Small synchronous API; pluggable resolver/connector; explicit global timeout, proxy, redirects, header limits, pool limits; bounded body helper ([configuration](https://docs.rs/ureq/3.4.0/ureq/config/struct.ConfigBuilder.html), [Body](https://docs.rs/ureq/3.4.0/ureq/struct.Body.html)). | Resolver/transport extension points are explicitly unversioned; automatic redirect processing offers less natural per-hop orchestration; blocking I/O complicates cancellation and concurrency ([unversioned resolver](https://docs.rs/ureq/3.4.0/ureq/unversioned/resolver/index.html)). | Acceptable short-term; not preferred durable foundation. |
| `isahc` 2.0.1 / `curl` 0.4.50 | libcurl has mature protocol, proxy, DNS override, redirect, timeout, and transfer-limit facilities; Isahc provides a Rust HTTP facade and custom DNS resolver support ([Isahc crate](https://crates.io/api/v1/crates/isahc), [Isahc config docs](https://docs.rs/isahc/2.0.1/isahc/config/index.html), [curl easy options](https://curl.se/libcurl/c/curl_easy_setopt.html), [curl crate](https://crates.io/api/v1/crates/curl)). | Native libcurl/OpenSSL/platform dependency surface is larger and less Rust-native; behavior varies with how libcurl was built. Isahc's abstraction still inherits libcurl lifecycle and callback complexity. | Strong operational alternative when libcurl features are required; otherwise unnecessary weight. |
| `hickory-resolver` 0.26.1 | Pure-Rust async resolver with configurable name servers, DNSSEC-related options, caching, hosts-file support, and pluggable runtime providers ([crate docs](https://docs.rs/hickory-resolver/0.26.1/hickory_resolver/), [crate record](https://crates.io/api/v1/crates/hickory-resolver)). | A separate DNS stack adds configuration, cache, split-DNS, search-domain, and observability decisions. It does not itself enforce “public address only.” | Optional behind a resolver trait; do not require it initially. |
| `trust-dns-resolver` 0.23.2 | Historical predecessor with the same broad resolver role. | The project was renamed from Trust-DNS to Hickory DNS; the Trust-DNS crate record's latest stable release dates to 2023, while Hickory is current ([rename announcement](https://github.com/hickory-dns/hickory-dns/blob/main/README.md), [Trust-DNS crate](https://crates.io/api/v1/crates/trust-dns-resolver), [Hickory crate](https://crates.io/api/v1/crates/hickory-resolver)). | Do not adopt for new code; use Hickory. |

## TLS choice

Prefer **rustls 0.23.x**. Rustls is a modern TLS implementation written in Rust, documents no unsafe code in the main crate, supports TLS 1.2 and 1.3, and requires an explicit cryptography provider and trust-root strategy ([rustls manual](https://docs.rs/rustls/0.23.43/rustls/), [crate record](https://crates.io/api/v1/crates/rustls), [provider docs](https://docs.rs/rustls/0.23.43/rustls/crypto/struct.CryptoProvider.html)). This matches browser.jr's memory-safety goal and gives consistent cross-platform behavior.

`native-tls` is a wrapper over the platform TLS implementation—SChannel on Windows, Secure Transport on macOS, and OpenSSL on other platforms—so it integrates with platform behavior but makes backend details and deployed capabilities platform-dependent ([native-tls docs](https://docs.rs/native-tls/0.2.18/native_tls/), [crate record](https://crates.io/api/v1/crates/native-tls)). Choose it only if operating-system certificate policy, enterprise roots, or platform TLS integration is a stated product requirement. With rustls, decide explicitly between bundled WebPKI roots and native certificate loading; reqwest exposes `rustls-tls-webpki-roots` and `rustls-tls-native-roots` features ([reqwest features](https://docs.rs/reqwest/0.13.4/reqwest/#optional-features)).

Never disable certificate or hostname validation. Reqwest exposes dangerous opt-outs such as `danger_accept_invalid_certs` and `danger_accept_invalid_hostnames`; production configuration must not call them ([ClientBuilder docs](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.danger_accept_invalid_certs)).

## DNS and connection pinning design

Define an application-owned interface such as `PublicEndpointResolver::resolve(url_host, port, policy) -> ApprovedEndpoints`. Its implementation should:

- distinguish domain, IPv4 literal, and IPv6 literal using `url::Host`, not string heuristics ([Host docs](https://docs.rs/url/2.5.8/url/enum.Host.html));
- resolve domain names under a deadline;
- normalize IPv4-mapped IPv6 before classification because the IPv6 address API exposes that representation explicitly ([`Ipv6Addr::to_ipv4_mapped`](https://doc.rust-lang.org/stable/std/net/struct.Ipv6Addr.html#method.to_ipv4_mapped));
- reject the entire hop if no approved endpoint remains; a conservative policy may reject the whole answer set if any prohibited endpoint appears;
- return the exact approved `SocketAddr` values to the connector, avoiding a second lookup;
- retain the URL hostname separately for `Host` and TLS identity;
- expose fake/static implementations for deterministic unit and integration tests.

The public/private classification should be project-owned and table-driven from IANA registries, not only Rust's convenience predicates. IANA maintains authoritative IPv4 and IPv6 special-purpose registries with per-prefix “Globally Reachable” properties ([IPv4 registry](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml), [IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml)). This also makes policy updates and tests reviewable.

Hickory is useful if browser.jr later needs a fully async resolver, controlled nameservers, or deterministic injected resolver configuration. Otherwise, Tokio/system DNS behind the application trait has fewer moving parts. Reqwest's DNS module exposes its resolver abstraction, and Hyper's connector layer can consume already-approved endpoints ([reqwest DNS module](https://docs.rs/reqwest/0.13.4/reqwest/dns/index.html), [hyper-util connect module](https://docs.rs/hyper-util/latest/hyper_util/client/legacy/connect/index.html)).

## Redirect behavior

Prefer an explicit loop with automatic redirects disabled:

1. validate and normalize the current `Url`;
2. resolve and pin approved endpoints;
3. send one request;
4. for a redirect status, parse `Location` as bytes/header text according to the HTTP client contract, join it against the current URL, decrement the hop budget, strip sensitive headers, and restart at step 1;
5. reject HTTPS-to-HTTP downgrade unless product behavior explicitly permits it.

HTTP semantics define redirect status codes and method rewriting; notably 307 and 308 preserve the method, while historical behavior for 301/302 differs and 303 directs retrieval with GET/HEAD semantics ([RFC 9110 §15.4](https://www.rfc-editor.org/rfc/rfc9110.html#name-redirection-3xx), [RFC 9110 §15.4.8](https://www.rfc-editor.org/rfc/rfc9110.html#status.307), [RFC 9110 §15.4.9](https://www.rfc-editor.org/rfc/rfc9110.html#status.308)). Browser.jr currently performs GET page loads, which simplifies this, but explicit handling remains easier to audit and test.

Do not forward `Authorization`, `Proxy-Authorization`, cookies, or other origin-bound credentials across an origin change. RFC 9110 defines the origin tuple and warns intermediaries and clients about forwarding sensitive fields ([origin definition](https://www.rfc-editor.org/rfc/rfc9110.html#name-origin), [field forwarding](https://www.rfc-editor.org/rfc/rfc9110.html#name-forwarding-messages)). Browser.jr currently rejects URL userinfo and does not need authentication, so the safest request builder should omit these headers entirely.

## Limits and failure model

A single “15 second timeout” is insufficient because it does not communicate which phase stalled. Use named limits and errors for DNS, connect, TLS/response headers, read idle, total hop, total navigation, redirect count, header bytes, decoded body bytes, and decompression work. Reqwest's timeout controls cover connection, per-read inactivity, and total request duration, but DNS policy and whole-navigation budgets remain application responsibilities ([ClientBuilder timeout](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.timeout), [connect timeout](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.connect_timeout), [read timeout](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.read_timeout)).

Do not trust `Content-Length` as the body limit: transfer coding, compression, missing lengths, and incorrect peer metadata require counting bytes actually delivered to the HTML decoder. RFC 9112 defines message framing and requires recipients to handle invalid or conflicting length metadata carefully ([RFC 9112 §6](https://www.rfc-editor.org/rfc/rfc9112.html#name-message-body)).

## Testability checklist

The network module should accept injected interfaces for resolver, connector/transport, clock/deadlines, and redirect decisions. Test with local servers plus fake DNS answers; do not depend on public DNS or internet sites. Reqwest permits a preconfigured client and DNS overrides, ureq permits custom resolver/connector parts, Isahc exposes client configuration and DNS resolution hooks, and Hyper permits a custom connector ([reqwest resolve](https://docs.rs/reqwest/0.13.4/reqwest/struct.ClientBuilder.html#method.resolve_to_addrs), [ureq Agent::with_parts](https://docs.rs/ureq/3.4.0/ureq/struct.Agent.html#method.with_parts), [Isahc resolver module](https://docs.rs/isahc/2.0.1/isahc/net/index.html), [Hyper Connect trait](https://docs.rs/hyper-util/latest/hyper_util/client/legacy/connect/trait.Connect.html)).

Minimum adversarial cases:

- literal encodings and URL normalization, userinfo, empty/malformed hosts, IDNA, IPv6 brackets, and non-HTTP schemes;
- DNS answers containing private, loopback, link-local, multicast, documentation, benchmarking, shared-address, unspecified, IPv4-mapped, and mixed public/prohibited addresses, based on IANA's registries ([IPv4 registry](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml), [IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml));
- every redirect hop crossing hostname, port, scheme, public/private classification, and redirect limits;
- DNS success followed by connection only to the injected approved address;
- slow DNS, connect stall, header stall, slow-drip body, oversized headers/body, chunked body, incorrect `Content-Length`, and compressed expansion;
- proxy environment variables present while the client still connects directly;
- pool reuse and concurrent requests under different policies;
- TLS hostname mismatch, untrusted issuer, expired certificate, and HTTPS connection pinned to an IP while verified for the original hostname.

## Migration options

### Lowest-risk short-term

Keep ureq 3.4.0, but disable automatic redirects and implement the per-hop loop; keep `.proxy(None)`; retain the current custom resolver; move IP policy into a separately tested module; replace hand-written reference joining with `url::Url`; and explicitly document that `ureq::unversioned` is an accepted upgrade risk ([ureq redirect configuration](https://docs.rs/ureq/3.4.0/ureq/config/struct.ConfigBuilder.html#method.max_redirects), [unversioned module](https://docs.rs/ureq/3.4.0/ureq/unversioned/index.html), [`Url::join`](https://docs.rs/url/2.5.8/url/struct.Url.html#method.join)).

### Preferred production path

Adopt async reqwest with rustls and `url::Url`. First implement the application resolver/policy trait and explicit redirect state machine independent of reqwest. Then adapt it to a reqwest custom resolver or one-hop client whose connection receives only approved endpoints. This isolates the part most likely to require a later move to direct Hyper while keeping URL policy, limits, errors, and tests stable ([reqwest DNS module](https://docs.rs/reqwest/0.13.4/reqwest/dns/index.html), [hyper client connection API](https://docs.rs/hyper/1.11.1/hyper/client/conn/index.html)).

## Bottom line

- **Choose:** reqwest + rustls + url.
- **Own:** redirect loop, DNS/IP policy, approved-endpoint pinning, limits, errors, and tests.
- **Optionally add:** Hickory behind the resolver trait when controlled async DNS becomes necessary.
- **Avoid for new work:** Trust-DNS lineage crates under the old name.
- **Keep only temporarily:** ureq when synchronous simplicity outweighs reliance on explicitly unversioned resolver/transport APIs.
- **Escalate to:** direct Hyper only after a demonstrated reqwest connector limitation.
