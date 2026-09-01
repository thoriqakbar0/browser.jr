# Rust browser and network research landscape

**Evidence date:** 2026-09-01  
**Scope:** Rust browser/network projects and systems research that can inform browser.jr. This is a research note, not an implementation decision.

## Executive conclusion

browser.jr should reuse the mature Rust protocol stack, but keep its security and reproducibility policy in a separate, narrow fetch layer. The strongest near-term building blocks are `url` for WHATWG URL parsing, `hyper`/`hyper-util` for HTTP mechanics, `rustls` for TLS, and Hickory DNS when browser.jr needs resolver control rather than the operating system resolver. `reqwest` is useful as a reference and may be acceptable for a first client, but its convenience defaults and connector abstraction do not by themselves provide a complete SSRF boundary. ([url](https://github.com/servo/rust-url), [hyper](https://github.com/hyperium/hyper), [rustls](https://github.com/rustls/rustls), [Hickory DNS](https://github.com/hickory-dns/hickory-dns), [reqwest IP-limiting discussion](https://github.com/seanmonstar/reqwest/issues/1515))

Servo is the best Rust reference for browser-grade Fetch behavior, not a small dependency to transplant wholesale. Its networking component implements Fetch concepts such as CORS, redirects, cancellation, cookies, HTTP state, and resource timing across several coupled modules. ([Servo fetch methods](https://github.com/servo/servo/blob/master/components/net/fetch/methods.rs), [Servo HTTP loader](https://github.com/servo/servo/blob/master/components/net/http_loader.rs), [Servo resource thread](https://github.com/servo/servo/blob/master/components/net/resource_thread.rs), [Servo connector](https://github.com/servo/servo/blob/master/components/net/connector.rs))

The core security lesson from capability systems, browser isolation, and SSRF research is consistent: authority should be granted at the point where a connection is made, not inferred once from the original URL. DNS answers, every redirect target, and the actual peer address must remain inside the same policy boundary. ([WASI security model](https://wasi.dev/security), [Chromium Site Isolation](https://www.chromium.org/developers/design-documents/site-isolation/), [Gazelle](https://www.usenix.org/event/sec09/tech/full_papers/wang.pdf), [Preventing SSRF Attacks](https://dl.acm.org/doi/10.1145/3412841.3442036))

## Proven reusable components

### 1. `url`: canonical URL semantics

The Servo-maintained `url` crate implements the WHATWG URL Standard and exposes parsing, base-relative joining, mutation, origin handling, and serialization. It is a better boundary type than strings or ad hoc URI parsing for browser navigation. ([rust-url repository](https://github.com/servo/rust-url), [WHATWG URL Standard](https://url.spec.whatwg.org/))

**Useful for browser.jr**

- Parse once into `Url`, then make scheme and host policy decisions on structured fields. ([rust-url API documentation](https://docs.rs/url/))
- Resolve redirects and document-relative links with WHATWG joining rules rather than filesystem-like string concatenation. ([WHATWG URL parsing](https://url.spec.whatwg.org/#concept-basic-url-parser))
- Treat serialization as canonical output, while retaining the original user input separately if diagnostics need it. ([WHATWG URL serialization](https://url.spec.whatwg.org/#concept-url-serializer))

**Boundary:** URL parsing is not network authorization. A syntactically valid public-looking hostname can resolve to a private address, and a later redirect can change the destination. The reqwest IP-limiting issue describes exactly this missing connector-time policy hook. ([reqwest issue #1515](https://github.com/seanmonstar/reqwest/issues/1515))

### 2. `hyper` and `hyper-util`: HTTP mechanism below policy

Hyper describes itself as a relatively low-level HTTP library and building block. Its split between protocol machinery and connectors is a good fit when browser.jr must own destination checks, timeouts, limits, redirects, and evidence collection. ([hyper repository](https://github.com/hyperium/hyper), [hyper client guide](https://hyper.rs/guides/1/client/basic/))

Hyper 1.x deliberately leaves higher-level behavior to callers and companion crates; `hyper-util` supplies legacy client and connector utilities. This makes the pair more work than reqwest, but also gives browser.jr a clearer place to place a policy-enforcing connector. ([hyper 1.0 announcement](https://hyper.rs/blog/2023/11/15/hyper-1-0/), [hyper-util repository](https://github.com/hyperium/hyper-util))

**Useful for browser.jr**

- Put byte, header, body, connection, and time budgets around each protocol phase instead of relying on one whole-request timeout. Hyper exposes bodies as streams and does not require whole-body buffering. ([Hyper body module](https://docs.rs/hyper/latest/hyper/body/))
- Inject a connector that accepts only pre-authorized socket addresses, then verify the connected peer address before sending HTTP bytes. Hyper's client connection APIs are transport-generic. ([Hyper client connection API](https://docs.rs/hyper/latest/hyper/client/conn/))
- Keep redirect handling above Hyper so every hop re-enters URL, DNS, address, and budget policy. Hyper is not a redirect-following browser client by itself. ([hyper client guide](https://hyper.rs/guides/1/client/basic/))

### 3. `reqwest`: strong convenience layer, incomplete security boundary

Reqwest supplies asynchronous and blocking clients, TLS, proxies, cookies, decompression, customizable redirects, DNS overrides, and timeouts. Its current client is built from Hyper-family components. ([reqwest documentation](https://docs.rs/reqwest/), [reqwest async client source](https://github.com/seanmonstar/reqwest/blob/master/src/async_impl/client.rs))

Reqwest is reusable if browser.jr can fully constrain its connector path. It has custom redirect policy and resolver hooks, but the open IP-limiting design discussion documents why hostname validation plus redirect validation still does not automatically validate every address selected at connection time. ([redirect policy](https://docs.rs/reqwest/latest/reqwest/redirect/struct.Policy.html), [DNS resolve trait](https://docs.rs/reqwest/latest/reqwest/dns/trait.Resolve.html), [reqwest issue #1515](https://github.com/seanmonstar/reqwest/issues/1515))

Reqwest also enables system proxies by default unless proxy behavior is explicitly disabled; for a deterministic and least-authority fetcher, browser.jr should not inherit ambient proxy configuration. ([reqwest `no_proxy` source](https://docs.rs/reqwest/latest/src/reqwest/async_impl/client.rs.html))

**Assessment:** proven for general HTTP; suitable for a prototype only if tests prove that all resolution, redirect, proxy, and connection paths pass through browser.jr policy. Hyper is the safer long-term base when the connection boundary itself must be owned.

### 4. Hickory DNS: explicit resolver control

Hickory provides separate protocol, client, resolver, recursive resolver, and server crates. Its resolver can replace operating-system resolution, and the project supports DNSSEC plus DNS-over-TLS/HTTPS features. ([Hickory README](https://github.com/hickory-dns/hickory-dns/blob/main/README.md), [hickory-resolver docs](https://docs.rs/hickory-resolver/))

**Useful for browser.jr**

- Obtain the full resolved address set before connection selection, classify every address, and reject a name if policy requires all candidates to be public. ([hickory-resolver lookup API](https://docs.rs/hickory-resolver/latest/hickory_resolver/struct.Resolver.html))
- Inject a resolver configuration for tests instead of depending on `/etc/resolv.conf`, search domains, or host-specific cache state. ([ResolverConfig](https://docs.rs/hickory-resolver/latest/hickory_resolver/config/struct.ResolverConfig.html))
- Record DNS results and TTL-related metadata as inputs to replay. Hickory exposes DNS lookup records rather than only one opaque socket destination. ([Lookup](https://docs.rs/hickory-resolver/latest/hickory_resolver/lookup/struct.Lookup.html))

**Boundary:** DNSSEC authenticates DNS data; it does not decide whether an authenticated address is allowed. Hickory's bundled-root and local-policy choices also need an explicit product decision. ([Hickory DNSSEC status](https://github.com/hickory-dns/hickory-dns/blob/main/README.md#dnssec-status), [system root-key discussion](https://github.com/hickory-dns/hickory-dns/issues/2855))

### 5. `rustls`: memory-safe TLS with explicit configuration

Rustls is a Rust TLS library implementing TLS 1.2 and 1.3. Its public configuration separates root trust, client configuration, protocol versions, and cryptographic provider choices. ([rustls repository](https://github.com/rustls/rustls), [rustls manual](https://docs.rs/rustls/latest/rustls/manual/))

**Useful for browser.jr**

- Use explicit root stores and avoid silently changing trust sources between machines. ([rustls `RootCertStore`](https://docs.rs/rustls/latest/rustls/struct.RootCertStore.html))
- Record negotiated TLS version, ALPN, certificate failure class, and server name as evidence without exposing secrets. ([rustls client connection](https://docs.rs/rustls/latest/rustls/client/struct.ClientConnection.html))
- Keep certificate verification enabled. Custom certificate verifiers are a dangerous escape hatch and are gated behind rustls's `dangerous_configuration`/danger APIs. ([rustls client danger API](https://docs.rs/rustls/latest/rustls/client/danger/index.html))

**Boundary:** Rustls protects the TLS channel after destination selection. It does not enforce URL, DNS, redirect, private-network, or response-size policy.

## Servo as a browser-network reference

Servo's network stack is valuable because it maps Rust code to the Fetch Standard instead of treating navigation as “HTTP GET plus redirects.” Its fetch implementation explicitly follows Fetch algorithms and connects them to CORS cache state, request population, cancellation, response handling, and timing. ([Servo fetch methods](https://github.com/servo/servo/blob/master/components/net/fetch/methods.rs), [Fetch Standard](https://fetch.spec.whatwg.org/))

Servo separates resource-thread coordination, HTTP loading, connector construction, cookie/HSTS/cache state, and Fetch algorithms. That separation is evidence that browser networking quickly becomes shared browser state rather than one stateless helper call. ([resource thread](https://github.com/servo/servo/blob/master/components/net/resource_thread.rs), [HTTP loader](https://github.com/servo/servo/blob/master/components/net/http_loader.rs), [connector](https://github.com/servo/servo/blob/master/components/net/connector.rs))

**Reuse directly:** individual Web Platform Test cases, Fetch algorithm structure, header/redirect edge cases, cancellation patterns, and separation between request policy and transport. Servo runs the shared WPT corpus stored in its repository. ([Servo WPT directory](https://github.com/servo/servo/tree/master/tests/wpt), [web-platform-tests repository](https://github.com/web-platform-tests/wpt))

**Do not transplant yet:** Servo's entire `net` component. Its internal types and channels are coupled to a full browser engine and browser state. browser.jr's current static, bounded loader can learn from it while preserving a smaller trusted computing base. ([Servo component structure](https://book.servo.org/contributing/crate-dependencies.html), [Servo engine embedding API](https://github.com/servo/servo/blob/master/components/servo/lib.rs))

## Sandboxed and capability-based fetch design

WASI 0.2 models capabilities as explicit component imports; networking, clocks, randomness, and filesystem access are separate authority surfaces supplied by the host. This is a strong model for browser.jr even without compiling the fetcher to WebAssembly. ([WASI security](https://wasi.dev/security))

The `wasi:sockets` design uses a network capability at bind/connect time; a newly created socket cannot communicate until it is associated with an authorized network. ([WASI sockets interface](https://wa.dev/wasi%3Asockets))

A browser.jr analogue would pass a `NetworkPermit` or policy object into the connector rather than letting arbitrary inner code call global DNS or `TcpStream::connect`. The permit should cover schemes, ports, resolved address classes, redirect count, total time, and byte ceilings. This is an architectural inference from the WASI capability boundary, not an existing Rust crate API. ([WASI security](https://wasi.dev/security), [WASI sockets interface](https://wa.dev/wasi%3Asockets))

For stronger isolation, a helper process can receive a narrow request description and return a bounded response through IPC while having no filesystem credentials and a restricted network namespace. Chromium's Site Isolation and Gazelle support the general lesson that process boundaries are useful only when the privileged broker exclusively controls resource access. ([Chromium Site Isolation design](https://www.chromium.org/developers/design-documents/site-isolation/), [Gazelle paper](https://www.usenix.org/event/sec09/tech/full_papers/wang.pdf))

**Assessment:** capability-shaped Rust APIs are immediately practical. WASI component execution or an OS-sandboxed helper is promising defense in depth, but experimental for browser.jr until cost, DNS/TLS integration, platform parity, cancellation, and error reporting are prototyped.

## Deterministic and reproducible HTTP

A live HTTP request cannot be intrinsically deterministic because DNS answers, routing, server state, certificates, compression, content negotiation, clocks, and redirects can change. Reproducibility therefore requires either controlled fixtures or record/replay of all external inputs. This conclusion is consistent with research on agent replay and archived-web temporal violations. ([Deterministic Replay for AI Agent Systems](https://arxiv.org/abs/2607.16200), [Right HTML, Wrong JSON](https://arxiv.org/abs/2305.01071))

A useful browser.jr trace should record, per hop: normalized request URL and method; policy decision; DNS answers; selected peer; request headers after redaction; status; response headers; body bytes or a content-addressed body reference; redirect decision; TLS metadata; timing; truncation; and terminal error. The proposal is an inference from transport-level record/replay and web-archive findings. ([Deterministic Replay for AI Agent Systems](https://arxiv.org/abs/2607.16200), [Right HTML, Wrong JSON](https://arxiv.org/abs/2305.01071))

Replay should disable outbound networking and fail closed on an unmatched request. The 2026 `agrepl` paper uses an isolated environment with zero outbound access and a request-key matching function; browser.jr should treat this as an experimental research result, not mature dependency guidance. ([Deterministic Replay for AI Agent Systems](https://arxiv.org/abs/2607.16200))

For stable tests today, local HTTP fixtures remain the proven option. WPT's server-side test infrastructure and Servo's WPT use provide a large source of deterministic browser-network cases. ([WPT server documentation](https://web-platform-tests.org/writing-tests/server-features.html), [Servo WPT directory](https://github.com/servo/servo/tree/master/tests/wpt))

## SSRF and destination-policy findings

SSRF defenses must cover more than string allow/deny lists. OWASP recommends allowlisting when the application can enumerate destinations and stresses validation of resolved IP addresses and protection against DNS rebinding. ([OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html))

The AsiaCCS paper *Preventing Server-Side Request Forgery Attacks* analyzed more than 60 vulnerability reports and proposed a reverse-proxy-based defense for internal services, reinforcing the value of a centralized enforcement point rather than scattered caller validation. ([ACM DOI](https://dl.acm.org/doi/10.1145/3412841.3442036))

The reqwest IP-limiting issue captures the Rust-specific TOCTOU problem: checking a hostname before request execution is insufficient when resolution and connection occur later inside the client, and redirects repeat the problem. ([reqwest issue #1515](https://github.com/seanmonstar/reqwest/issues/1515))

**Required invariant for browser.jr:** no connection attempt occurs to an address that the current request policy has not approved. Apply the invariant to the initial URL, each redirect, all DNS candidates, proxy destinations, and the final connected peer. This is a design inference from the cited SSRF guidance and connector gap. ([OWASP cheat sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html), [reqwest issue #1515](https://github.com/seanmonstar/reqwest/issues/1515))

## Academic browser-isolation ideas

Gazelle models the browser as a multi-principal operating system whose browser kernel exclusively controls resources shared across site principals. The reusable principle is to place cross-principal authority in a small broker rather than in content-processing code. ([Gazelle paper](https://www.usenix.org/event/sec09/tech/full_papers/wang.pdf), [Microsoft Research publication page](https://www.microsoft.com/en-us/research/publication/the-multi-principal-os-construction-of-the-gazelle-web-browser/))

Chromium Site Isolation uses sandboxed renderer processes as boundaries between sites and keeps sensitive cross-site resources under browser-process control. Its out-of-process iframe design demonstrates that isolation must account for nested principals, not only top-level pages. ([Site Isolation design](https://www.chromium.org/developers/design-documents/site-isolation/), [OOPIF design](https://www.chromium.org/developers/design-documents/oop-iframes/))

Servo's experience report supports Rust as a viable language for a browser engine with fine-grained parallelism and improved memory safety, while also documenting the engineering costs of a new systems language and browser architecture. ([Servo experience report](https://arxiv.org/abs/1505.07383))

**For browser.jr now:** isolate authority by Rust module/API boundaries and keep fetched bytes untrusted. **Later experiment:** move public-network fetching into a brokered helper process. **Not justified now:** site-per-process rendering, because browser.jr has no JavaScript execution or full multi-origin document runtime to isolate.

## Recommended prototypes, in order

1. **Policy-enforcing connector spike.** Use `hyper`/`hyper-util`, `url`, `rustls`, and a fake resolver. Prove that literal IPs, DNS answers, redirects, proxies, and connected peers all pass through one policy function. ([hyper-util](https://github.com/hyperium/hyper-util), [rust-url](https://github.com/servo/rust-url), [rustls](https://github.com/rustls/rustls), [OWASP SSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html))
2. **Resolver determinism spike.** Put Hickory behind a small trait. Test multiple A/AAAA answers, rebinding-like answer changes, CNAME chains, timeouts, and injected fixed answers. ([Hickory resolver](https://docs.rs/hickory-resolver/), [reqwest issue #1515](https://github.com/seanmonstar/reqwest/issues/1515))
3. **Fetch trace/replay spike.** Record one redirect chain with DNS, TLS metadata, headers, and bounded bodies; replay with networking disabled. Treat the trace format as browser.jr-owned and versioned. ([Deterministic Replay for AI Agent Systems](https://arxiv.org/abs/2607.16200), [WPT server fixtures](https://web-platform-tests.org/writing-tests/server-features.html))
4. **Servo conformance mining.** Port small Fetch/WPT cases for redirect methods, credential/header stripping, cancellation, malformed responses, and CORS only when browser.jr claims the related behavior. ([Servo Fetch implementation](https://github.com/servo/servo/blob/master/components/net/fetch/methods.rs), [Fetch tests in WPT](https://github.com/web-platform-tests/wpt/tree/master/fetch))
5. **Helper-process experiment.** Only after the in-process policy is correct, compare an OS-sandboxed fetch helper or WASI component for binary size, startup, throughput, platform support, cancellation, DNS/TLS configurability, and diagnostic fidelity. ([WASI security](https://wasi.dev/security), [Chromium Site Isolation](https://www.chromium.org/developers/design-documents/site-isolation/))

## Reuse versus experiment matrix

| Item | Classification | Why |
|---|---|---|
| `url` | Proven reusable | Browser-standard parsing and joining; policy still belongs above it. ([source](https://github.com/servo/rust-url)) |
| `hyper` + `hyper-util` | Proven reusable | Low-level HTTP and connector seams support a browser.jr-owned enforcement point. ([source](https://github.com/hyperium/hyper)) |
| `rustls` | Proven reusable | Mature TLS mechanism with explicit configuration; not a destination policy. ([source](https://github.com/rustls/rustls)) |
| Hickory resolver | Proven reusable, optional | Useful when browser.jr needs explicit DNS inputs, protocols, or test injection. ([source](https://github.com/hickory-dns/hickory-dns)) |
| `reqwest` | Proven general client; conditional fit | Strong convenience APIs, but a complete connector-time IP policy requires proof or customization. ([source](https://github.com/seanmonstar/reqwest/issues/1515)) |
| Servo `net` code | Proven reference, poor direct transplant | Browser-grade semantics but tightly coupled to the full engine. ([source](https://github.com/servo/servo/tree/master/components/net)) |
| WPT Fetch cases | Proven reusable tests | Shared web-platform conformance corpus. ([source](https://github.com/web-platform-tests/wpt/tree/master/fetch)) |
| Capability-shaped connector API | Near-term design experiment | Directly applies least authority without requiring WASM. ([source](https://wasi.dev/security)) |
| WASI fetch component | Experimental | Attractive authority boundary; integration and platform costs need measurement. ([source](https://wa.dev/wasi%3Asockets)) |
| Sandboxed helper process | Experimental defense in depth | Stronger fault/authority boundary, with IPC and platform complexity. ([source](https://www.chromium.org/developers/design-documents/site-isolation/)) |
| HTTP record/replay | Experimental product feature; proven testing pattern | Determinism requires capturing external inputs and disabling live egress during replay. ([source](https://arxiv.org/abs/2607.16200)) |

## Bottom line

The smallest defensible stack is `url` + a browser.jr-owned destination policy + Hyper connector control + rustls, with Hickory added when resolver observability or determinism is required. Servo and WPT should supply semantics and tests. Reqwest should not be treated as the security boundary without a concrete connector-level proof. Sandboxing and record/replay are worthwhile follow-on prototypes, but correctness of the in-process authorization path comes first. ([rust-url](https://github.com/servo/rust-url), [hyper](https://github.com/hyperium/hyper), [rustls](https://github.com/rustls/rustls), [Hickory](https://github.com/hickory-dns/hickory-dns), [Servo Fetch](https://github.com/servo/servo/blob/master/components/net/fetch/methods.rs), [reqwest IP policy gap](https://github.com/seanmonstar/reqwest/issues/1515))
