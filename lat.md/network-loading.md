# Network loading

The loading subsystem resolves one approved URL into HTML bytes while keeping network authority, transport work, redirects, and document installation in separate steps.

The product-facing access rules and failure behavior belong to [network control](../loading/network-control.md). This page maps their implementation ownership.

## Authority flow

[[src/loading.rs#NetworkAccess]] enters with the session and reaches [[src/loading.rs#load_html]] with each navigation request. [[src/loading.rs#network_mode_for_url]] classifies the requested URL. [[src/loading.rs#address_is_allowed_for_mode]] applies that authority after resolution.

The same authority remains available during redirect processing. This keeps URL parsing, DNS results, and redirect targets under one request-scoped decision rather than separate adapter checks.

## Fetch state

[[src/loading.rs#FetchEngine]] owns the in-progress fetch state. It carries redirect history and the remaining request budget between transport steps.

[[src/loading.rs#process_network_response]] checks one response and either returns loaded content or produces the next redirect step. The transport returns bytes and response metadata. It does not install browser state.

```text
request URL and NetworkAccess
  -> parse and classify
  -> resolve and approve endpoints
  -> transport request
  -> response or redirect step
  -> LoadedHtml
```

## Session boundary

[[src/loading.rs#LoadedHtml]] crosses back into [[src/session.rs#Session]]. The session builds page evidence and replaces the current document only after the loading call succeeds.

This separation keeps network errors outside page construction. [[session-state]] maps the later document installation, and [[page-pipeline]] starts from the returned HTML.

## Design links

[[decisions#Network authority is explicit and narrow]] records why authority is session-scoped. The exact limits and observable errors remain in [network control](../loading/network-control.md).
