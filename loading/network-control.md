# Control network loading

## Summary

browser.jr loads public HTTP and HTTPS pages. Explicit `localhost` and loopback-IP targets remain available for local development. Other private and non-routable targets are blocked.

## The simple case

The caller opens a public URL. browser.jr resolves the host, rejects unsafe address results, disables environment proxies, and reads a bounded HTML response.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> validating
    validating --> rejected : invalid URL or blocked literal
    validating --> resolving : permitted URL
    resolving --> rejected : private or non-routable answer
    resolving --> requesting : permitted answers
    requesting --> redirecting : redirect below limit
    redirecting --> resolving
    requesting --> finished : HTML response
    requesting --> failed : timeout or response failure
```

### Invoke

The caller supplies an HTTP or HTTPS URL without credentials.

### Exit immediately

Unsupported schemes, credentials, and blocked literal addresses fail before a request.

### Begin running

The loader resolves the target through its own resolver and does not use an environment proxy.

### While running

Every resolved address must be public unless the URL explicitly names `localhost` or a loopback IP. The request has a 15-second limit, a one MiB body limit, and a five-redirect limit. Redirect destinations pass through the same resolver policy.

### Finish

A successful open installs the final response URL. Failures preserve the previously installed page.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | No network-policy flag exists. | Nothing changes. |
| Project configuration | No network configuration exists. | Nothing reloads. |
| Target matrix | One URL is loaded. | Redirects may replace the target URL. |
| Output channel | Load failures use the command's normal error channel. | The channel stays fixed. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The process exits. | The operating system stops the request. |
| Ctrl+C again before the evaluation stops | The process already exits. | No second-stage handler exists. |
| The process receives SIGTERM | The process exits. | The operating system stops the process. |
| The terminal closes | The process may exit. | Output may fail. |
| stdin or stdout closes | One-shot loading is unchanged. | A session may close. |
| The network fails or times out | No page is installed. | The load fails. |
| The inspected page changes | No effect. | The fetched response may differ. |
| Another load targets the same page | Both may start. | Sessions remain separate. |
| The process exits outright | No page is installed. | Partial data is discarded. |

## Interactions with other systems

**Configuration precedence.** No network override exists.

**Output and exit status.** Invalid targets are input failures. Runtime network failures are unavailable failures.

**Resource limits.** Responses are limited to one MiB, five redirects, and 15 seconds per load operation.

**Network and storage.** Proxies are disabled. DNS results are checked before their socket addresses are passed to the connector. No page data is persisted by the loader.

**Rendering compatibility.** Only HTML and XHTML response media types are accepted when a content type is present.

**Isolation.** Public hosts cannot resolve to a mixture containing private or non-routable addresses.

**Accessibility inspection.** Network policy does not change the supported static semantic subset.

## Edge cases

- Credentials are rejected.
- IPv4-mapped IPv6 addresses use the IPv4 policy.
- Carrier-grade NAT, link-local, multicast, documentation, benchmarking, and reserved address ranges are blocked.
- Explicit loopback URLs remain supported for local fixtures.
- Redirects receive the same DNS and address checks as the initial request.

## Open questions and verification

- Decide whether private-network access needs an explicit opt-in mode.
- Verify redirect failures and final-URL history through compiled-process checks.
- Verify TLS certificate failures through a deterministic local fixture.

Drafted from the Rust implementation and focused tests on 2026-09-01.
