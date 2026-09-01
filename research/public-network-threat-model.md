# Public-network fetch threat model

**Evidence date:** 2026-09-01  
**Scope:** Security and standards requirements for `browser.jr` when it fetches an arbitrary user-supplied HTTP or HTTPS URL. This is a research note, not a claim that every requirement is implemented.

## Security objective

Treat the URL, every DNS answer, every redirect, every HTTP field, and every response byte as attacker-controlled. A successful public fetch must not give the requester a network path, identity, credential, or resource-exhaustion primitive that they would not have without `browser.jr`.

The core SSRF risk is that a server-side fetcher can be induced to contact internal services or cloud metadata endpoints. OWASP recommends allowlisting schemes, validating both A and AAAA answers, and applying the same validation to every resolved address; it also calls out DNS pinning/rebinding as a bypass of validation that is separated from use. [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)

## Assets and trust boundaries

Assets at risk are:

- services reachable only from the host, container, cluster, VPC, VPN, or developer LAN;
- cloud workload identity and instance data;
- ambient proxy credentials, cookies, authorization fields, client certificates, and local trust roots;
- host CPU, memory, file descriptors, DNS capacity, connection slots, and wall-clock time;
- the integrity of the final URL and the evidence returned to the caller.

The first trust boundary is the URL parser. RFC 3986 makes `userinfo`, `host`, and `port` distinct authority subcomponents and warns that misleading userinfo can look like a trusted host. It also defines percent encoding and IPv6 literal syntax. One component-aware parser must therefore own both validation and request construction; string prefixes, substring tests, reparsing by a second library, and validation of a differently normalized string are not security boundaries. [RFC 3986, sections 3.2, 3.2.1, 3.2.2, 6, and 7.6](https://www.rfc-editor.org/rfc/rfc3986.html)

The second trust boundary is name resolution and connection establishment. DNS rebinding is specifically a time-of-check/time-of-use attack when an approved resolution is discarded and a later resolution selects the connected address. The peer-reviewed DNS-rebinding work demonstrates that changing a name's address can cross network boundaries despite same-origin-style hostname checks. [Jackson et al., “Protecting Browsers from DNS Rebinding Attacks,” ACM CCS 2007](https://crypto.stanford.edu/dns/dns-rebinding.pdf) A 2024 USENIX Security study of SSRF defenses separately classifies parser confusion and DNS rebinding as practical defense bypasses. [Pellegrino et al., “A Study of SSRF-Defenses in PHP Applications”](https://trouge.net/papers/sec24_SSRF.pdf)

The third trust boundary is HTTP processing. Redirect targets are new target URIs, response framing can be length-delimited, chunked, or connection-delimited, and `Content-Encoding` is decoded to obtain representation data. Limits that inspect only the initial URL, declared `Content-Length`, or compressed bytes are incomplete. [RFC 9110, sections 8.1, 8.4, and 15.4](https://www.rfc-editor.org/rfc/rfc9110.html) [RFC 9112, sections 6.2 and 6.3](https://www.rfc-editor.org/rfc/rfc9112.html)

## Threats

### SSRF and special-purpose addresses

An attacker can supply a literal address, a name resolving to a non-public address, or an address written in an alternate form. The authoritative classification inputs are IANA's live IPv4 and IPv6 Special-Purpose Address Registries, established by RFC 6890 and clarified by RFC 8190; ad hoc lists and language-library predicates can lag registry changes. [IANA IPv4 Special-Purpose Address Registry](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml) [IANA IPv6 Special-Purpose Address Registry](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml) [RFC 6890](https://www.rfc-editor.org/rfc/rfc6890.html) [RFC 8190](https://www.rfc-editor.org/rfc/rfc8190.html)

IPv4-mapped IPv6 addresses must be reduced to their embedded IPv4 address before classification. IPv6 scoped/zone identifiers are interface-specific and must not be accepted in a public URL policy. [RFC 4291, section 2.5.5](https://www.rfc-editor.org/rfc/rfc4291.html) [RFC 6874](https://www.rfc-editor.org/rfc/rfc6874.html)

### Cloud metadata and platform-local services

AWS EC2 metadata can expose temporary IAM role credentials. IMDSv2 adds a token-oriented PUT-then-GET flow and configurable hop limits, but those platform controls are defense in depth, not permission for a general fetcher to contact link-local space. [AWS EC2 instance metadata categories](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html) [AWS IMDSv2 operation](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/configuring-instance-metadata-service.html)

Google Compute Engine metadata requires `Metadata-Flavor: Google`; Azure IMDS is hosted at the well-known non-routable address `169.254.169.254` and requires its metadata request conventions. A fetcher that lets users control methods or headers can satisfy protections that block a simple GET, so link-local denial must be independent of method and header restrictions. [Google Cloud metadata query documentation](https://cloud.google.com/compute/docs/metadata/querying-metadata) [Azure Instance Metadata Service](https://learn.microsoft.com/en-us/azure/virtual-machines/instance-metadata-service)

### DNS rebinding, mixed answers, and failover

A hostile name can return a public answer during validation and a private answer during connection, or return public and private A/AAAA answers together and rely on client address selection or fallback. OWASP's guidance is to retrieve all A and AAAA answers and reject unsafe results. [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)

Validation must bind the exact approved socket addresses to the connector. Resolving once for validation and allowing the HTTP stack, proxy, or later retry to resolve again leaves a TOCTOU gap. The connected peer address must also be checked before HTTP bytes are sent, because retries, connection racing, proxying, and implementation mistakes can otherwise escape the approved set. This binding requirement is the direct engineering consequence of the rebinding results and OWASP's all-answer validation rule. [Jackson et al.](https://crypto.stanford.edu/dns/dns-rebinding.pdf) [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)

### Redirect chains

HTTP defines 3xx responses and the `Location` field as redirection to another URI. Each hop can change scheme, host, port, and resolved addresses. Automatic redirect handling is unsafe unless the security policy is invoked before each next request. [RFC 9110, sections 10.2.2 and 15.4](https://www.rfc-editor.org/rfc/rfc9110.html)

An HTTPS-to-HTTP redirect removes transport confidentiality and server authentication. RFC 9325 requires TLS 1.2 support, recommends TLS 1.3 support, and says TLS 1.0 and 1.1 must not be negotiated; certificate and hostname validation remain mandatory. A public fetcher should reject an HTTPS-to-HTTP redirect by default rather than silently downgrade. [RFC 9325, sections 3.1, 4, and 6](https://www.rfc-editor.org/rfc/rfc9325.html)

### URL parser confusion

Dangerous cases include userinfo (`trusted.example@127.0.0.1`), percent-encoded delimiters, backslash-versus-slash disagreement, malformed bracketed IPv6, IPv6 zone identifiers, embedded NUL/control characters, non-decimal or abbreviated IPv4 forms, Unicode/IDNA disagreement, and a validator and transport that use different parsers or normalization rules. RFC 3986 supplies the generic syntax but also notes that some schemes use syntax outside its generic parser; the WHATWG URL Standard defines the browser URL parsing algorithm, including special-scheme host parsing. `browser.jr` must choose and document one URL model, rather than mixing RFC parsing for policy with browser-like parsing for navigation. [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986.html) [WHATWG URL Standard](https://url.spec.whatwg.org/)

The peer-reviewed SSRF-defense study found that parser-confusion defenses fail when validation and request APIs interpret a URL differently. [Pellegrino et al.](https://trouge.net/papers/sec24_SSRF.pdf)

### Proxies and ambient authority

A proxy changes the actual network peer and can perform DNS resolution on behalf of the client. Environment-controlled `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and `NO_PROXY` settings therefore bypass a destination policy unless proxy use is an explicit, separately secured mode. The Rust HTTP clients used by this project expose proxy configuration and automatic environment-proxy behavior, so this must be set deliberately rather than inherited. [ureq proxy API](https://docs.rs/ureq/latest/ureq/struct.Proxy.html) [ureq agent configuration](https://docs.rs/ureq/latest/ureq/config/struct.Config.html)

### Response framing, compression, and resource exhaustion

`Content-Length` is not always present and does not bound a chunked or close-delimited response. It also describes message content before representation decoding, so it does not bound the expanded result of gzip, deflate, Brotli, or Zstandard decoding. [RFC 9112, sections 6.2 and 6.3](https://www.rfc-editor.org/rfc/rfc9112.html) [RFC 9110, sections 8.1 and 8.4](https://www.rfc-editor.org/rfc/rfc9110.html)

Compression bombs are an availability attack; OWASP explicitly identifies ZIP/XML bombs and huge inputs as ways to exhaust server resources. The same control principle applies to automatically decoded HTTP content: enforce separate compressed-wire and decoded-representation ceilings while streaming, plus a bounded expansion ratio or disable automatic decoding. [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)

A single total timeout is necessary but not sufficient for concurrency safety. The fetcher also needs connect, DNS, TLS-handshake, per-read idle, and total wall-clock deadlines, plus concurrency and file-descriptor caps. HTTP permits responses whose end is determined only by connection close, making an idle/read deadline essential. [RFC 9112, section 6.3](https://www.rfc-editor.org/rfc/rfc9112.html)

## Required invariants for browser.jr

These are release-blocking invariants for arbitrary public URL loading.

### 1. One parsed URL object

1. Parse once with the same URL implementation used to construct the request and resolve redirects.
2. Accept only absolute `http` and `https` URLs with a non-empty host.
3. Reject userinfo, fragments for network transmission, control characters, malformed percent encoding, IPv6 zone identifiers, and any host form whose canonical numeric meaning cannot be classified unambiguously.
4. Canonicalize the host once. Apply IDNA processing once for domain names. Parse IP literals into binary `IpAddr` values before policy checks.
5. Do not authorize by string prefix, suffix, regex, substring, or a separately reparsed URL.

Sources: [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986.html), [WHATWG URL Standard](https://url.spec.whatwg.org/), [Pellegrino et al.](https://trouge.net/papers/sec24_SSRF.pdf).

### 2. Public-address policy

1. Deny every address in the current IANA IPv4 or IPv6 Special-Purpose Address Registry by default. A narrowly justified exception must be explicit and tested; do not infer safety only from a language runtime's `is_private` predicate.
2. Deny IPv4-mapped IPv6 according to the embedded IPv4 classification.
3. Deny multicast, unspecified, broadcast, loopback, link-local, private-use/unique-local, benchmarking, documentation, discard-only, protocol-assignment, shared-address, transition, and future/reserved ranges.
4. Treat unclassified, unroutable, parse-failed, and empty resolution results as denial.
5. Keep local development as a separate capability: exact literal `127.0.0.0/8` or `::1` only, explicit opt-in, no redirects from local to public or public to local, and no DNS name such as `localhost` unless its complete answer set is pinned and loopback-only. Production/public mode must not inherit this exception.

Sources: [IANA IPv4 registry](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml), [IANA IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml), [RFC 6890](https://www.rfc-editor.org/rfc/rfc6890.html), [RFC 8190](https://www.rfc-editor.org/rfc/rfc8190.html).

### 3. Resolve, approve, connect, and verify as one operation

1. Resolve both A and AAAA records under one bounded DNS deadline.
2. Reject the hostname if **any** returned address is denied. Never select only the public member of a mixed set.
3. Pass only the approved binary socket addresses to the connector. Do not perform a second DNS lookup during connection, retries, redirects, or TLS setup.
4. Before sending the HTTP request, verify that the actual peer IP equals one of the approved addresses and still passes policy.
5. Bind TLS SNI and certificate hostname verification to the canonical URL hostname, not the chosen IP.
6. Do not retry with an address that was not in the approved set. Bound attempts and the total deadline.
7. Apply equivalent controls at the deployment network layer: egress firewall rules must deny special-purpose, private, link-local, metadata, cluster, and control-plane destinations. Application checks are not the sole boundary.

Sources: [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html), [Jackson et al.](https://crypto.stanford.edu/dns/dns-rebinding.pdf), [RFC 9325](https://www.rfc-editor.org/rfc/rfc9325.html).

### 4. Redirects are fresh requests

1. Disable the HTTP library's opaque automatic redirects unless its callback runs the full policy before connection.
2. Parse and resolve each `Location` against the current effective URL with the one URL implementation.
3. Re-run scheme, credentials, address classification, all-answer validation, connection binding, peer verification, method, header, and resource checks for every hop.
4. Reject loops and cap the chain. Count every 3xx hop, including repeated URLs and cross-origin hops.
5. Reject HTTPS-to-HTTP downgrade by default.
6. Never forward `Authorization`, `Cookie`, `Proxy-Authorization`, origin-specific headers, or caller-controlled metadata-service headers across an origin change. Prefer a stateless fetcher with no cookie jar.
7. Record the complete validated redirect chain and final effective URL in diagnostics without logging credentials or secret query data.

Sources: [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html), [RFC 9325](https://www.rfc-editor.org/rfc/rfc9325.html), [AWS IMDSv2](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/configuring-instance-metadata-service.html), [Google metadata requests](https://cloud.google.com/compute/docs/metadata/querying-metadata), [Azure IMDS](https://learn.microsoft.com/en-us/azure/virtual-machines/instance-metadata-service).

### 5. No ambient proxy or credential behavior

1. Public mode must ignore environment and OS proxy configuration.
2. If proxy support is later required, make it an administrator-configured mode with an allowlisted proxy endpoint, authenticated/encrypted transport, no user override, and enforcement at the proxy. The proxy becomes responsible for DNS and destination filtering; client-side validation alone cannot prove the proxy's upstream peer.
3. Do not load ambient cookies, `.netrc`, browser profiles, client certificates, platform credentials, or custom root CAs. Do not accept arbitrary request methods or headers from the URL submitter.
4. Use GET/HEAD only for page fetches. Never expose CONNECT, PUT, POST, or arbitrary headers through this primitive.

Sources: [ureq proxy API](https://docs.rs/ureq/latest/ureq/struct.Proxy.html), [RFC 9110 method semantics](https://www.rfc-editor.org/rfc/rfc9110.html), [AWS IMDSv2](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/configuring-instance-metadata-service.html), [Google metadata requests](https://cloud.google.com/compute/docs/metadata/querying-metadata).

### 6. TLS cannot become optional

1. Verify the certificate chain and URL hostname. Do not expose an “accept invalid certificates” path to URL submitters.
2. Follow RFC 9325: do not negotiate SSLv2, SSLv3, TLS 1.0, or TLS 1.1; support TLS 1.2 and prefer/support TLS 1.3 with recommended cipher and algorithm constraints.
3. Do not fall back from failed HTTPS to HTTP or retry a TLS failure as plaintext.
4. Keep trust-store changes and client certificate configuration outside user-controlled fetch input.

Source: [RFC 9325](https://www.rfc-editor.org/rfc/rfc9325.html).

### 7. Streaming resource limits

1. Set finite DNS, connect, TLS handshake, response-header, per-read idle, and total wall-clock deadlines. The total deadline spans DNS, every redirect, retries, body read, and decompression; redirects do not reset it.
2. Cap redirect count, DNS answer count, connection attempts, response-header bytes/count, compressed wire bytes, decoded bytes, parser input bytes, and concurrent fetches.
3. Reject an over-limit declared `Content-Length` early, but still count bytes actually read because framing can be chunked or close-delimited.
4. Count decoded bytes after every content-coding layer. Abort as soon as the decoded cap or expansion-ratio cap is crossed. Prefer disabling unsupported or nested content codings.
5. Stream into a bounded buffer. Never call an unbounded `read_to_end`, `text`, or equivalent convenience method before the decoded limit is enforced.
6. On cancellation or any limit failure, close the response/connection, release permits, and return a stable non-sensitive error.

Sources: [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html), [RFC 9112](https://www.rfc-editor.org/rfc/rfc9112.html), [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html).

## Current browser.jr posture and research gaps

A source review on 2026-09-01 found useful foundations in `src/loading.rs`: only HTTP/HTTPS, rejection of userinfo, proxy disabling, a 15-second global timeout, five redirects, a one-MiB body read limit, validation inside a custom resolver, rejection of every address in a mixed answer set, and explicit handling of IPv4-mapped IPv6. These are repository observations, not external claims.

The following need proof or hardening before the public-network loader should be treated as a complete SSRF boundary:

- The handwritten IPv4/IPv6 deny logic is not demonstrably generated from or exhaustively tested against the current IANA registries. In particular, the IPv6 registry is broader than loopback, multicast, unique-local, link-local, documentation, and IPv4-mapped cases.
- The resolver returns approved addresses to the HTTP stack, which is the right shape, but a test must prove no hidden second resolution occurs and that the connected peer belongs to the approved set across retries and address fallback.
- Redirect enforcement currently depends on the client library invoking the custom resolver for every hop. Replace that assumption with a manual redirect loop or a conformance test that proves every hop re-enters all policy checks.
- The body limit's position relative to automatic content decoding must be proven. The required limit is on decoded representation bytes, with a separate wire-byte limit; a one-MiB reader limit is not enough if it is applied before decoding or after an unbounded decoder buffer.
- The global timeout must be shown to include DNS, connection, TLS, all redirects, and decoded body consumption without resetting. Add explicit idle/read and concurrency limits.
- The public mode currently permits hostname `localhost` when the resolver returns loopback. That local exception should be a separate explicit capability, not part of the arbitrary-public-URL policy.
- TLS minimum versions, certificate/hostname verification, downgrade rejection, trust roots, header forwarding, cookies, and ambient credentials need explicit configuration and tests rather than dependency-default assumptions.

## Required adversarial verification matrix

Before release, tests should use a controlled DNS server and controlled HTTP/TLS endpoints to prove these outcomes:

| Case | Required outcome |
| --- | --- |
| Literal IPv4/IPv6 entry from every current IANA special-purpose prefix | rejected before connection |
| IPv4-mapped IPv6 for a denied IPv4 address | rejected |
| Public A plus private A; public AAAA plus private AAAA; mixed A/AAAA | entire target rejected |
| First DNS response public, later response private | no second lookup; private endpoint receives zero connections |
| Connector fallback from approved to unapproved address | unapproved peer receives zero bytes |
| Public URL redirects to private, link-local, metadata, loopback, or local hostname | rejected before next connection |
| HTTPS redirects to HTTP | rejected by default |
| Redirect loop and chain over limit | bounded failure under the original total deadline |
| Userinfo, encoded delimiters, backslashes, integer/octal/hex IPv4, malformed IPv6, zone ID, Unicode host edge cases | one stable parse or rejection; validator and connector agree |
| `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY` set hostilely | ignored in public mode |
| Chunked/close-delimited endless body and slow drip | idle or total timeout; bounded memory |
| False small/missing `Content-Length` | actual bytes enforce the cap |
| Tiny gzip/Brotli/Zstd body with oversized expansion or nested encodings | decoded/ratio cap aborts during streaming |
| Invalid certificate, wrong hostname, TLS 1.0/1.1 only | TLS failure; no plaintext retry |
| Concurrent slow requests above the configured permit count | excess work rejected or queued within a bound |
| AWS/GCP/Azure metadata endpoint by literal, DNS, redirect, mapped IPv6, and proxy | zero metadata connections |

The address corpus should be generated from the downloadable IANA registry data on the test date, with boundary cases immediately below, at the start, at the end, and immediately above every prefix. [IANA IPv4 registry](https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml) [IANA IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml)

## Decision summary

The safe primitive is not “validate a URL, then fetch it.” It is a single bounded state machine that parses once, resolves all addresses, rejects any non-public answer, binds the approved addresses to the connector, verifies the actual peer, manually repeats the full process for each redirect, forbids ambient proxies and credentials, preserves TLS, and enforces limits on time, wire bytes, decoded bytes, and concurrency. DNS rebinding and TOCTOU are prevented only when validation and connection are one indivisible operation, backed by egress filtering.
