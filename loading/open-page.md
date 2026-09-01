# Open a page

## Summary

`browser.jr lint <url>` and `browser.jr snapshot <url> --interactive` load one public HTTP or HTTPS page, or an explicit loopback page. Package callers use `OpenPage`.

Session-mode callers send `open <url>`. They can use `get url` and `get title` to inspect the current page.

## The simple case

The developer supplies a public HTTP or HTTPS URL, or starts a local server and supplies its loopback URL.

browser.jr reads HTML with a one MiB body cap, a 15-second request limit, and at most five redirects. It then starts lint or interactive snapshot work.

A package or session caller may retain that page. URL requests report its installed URL. Title requests report its normalized title.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> parsing
    parsing --> rejected : invalid target
    parsing --> loading : permitted HTTP or HTTPS target
    loading --> processing : valid HTML response
    loading --> blocked : request or response failure
    processing --> finished : result written
    blocked --> finished
    rejected --> finished
```

### Invoke

The CLI or package boundary parses the URL before network access. It rejects credentials, unsupported schemes, and literal private or non-routable targets.

### Exit immediately

Invalid targets exit with status two. No request starts.

### Begin running

The loader starts an HTTP GET. It follows at most five redirects.

### While running

The HTML body limit is one MiB. The request limit is 15 seconds.

### Finish

A 2xx HTML response enters the requested operation. Other responses exit with status three.

A successful retained open makes its URL and title available. Another successful open or navigation replaces both values.

Successful document replacement clears the page's stored [focus](../interaction/focus-element.md).

It also starts [page scroll](../interaction/scroll-page.md) at zero on both axes.

The new page uses the session's current [viewport size](../interaction/set-viewport.md).

Each successful retained open adds an entry to [navigation history](history-navigation.md).

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | Lint accepts a viewport. Snapshot requires interactive mode. Session inspection commands take no flags. | Flags stay fixed. |
| Project configuration | No loading configuration exists. | Nothing reloads. |
| Target matrix | One URL and viewport run. | The target stays fixed. |
| Output channel | CLI results use stdout. Failures use stderr. Package requests return typed values. | Session mode flushes after each command. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The process exits. | The operating system stops the process. |
| Ctrl+C again before the evaluation stops | The process already exits. | No graceful second-stage handler exists. |
| The process receives SIGTERM | The process exits. | The operating system stops the process. |
| The terminal closes | The process may exit. | Output may fail. |
| stdin or stdout closes | Closed stdin has no effect on one-shot commands. | Closed session stdin closes the session. Closed stdout stops complete output. |
| The network fails or a request times out | No result exists. | The run exits with status three. |
| The inspected page changes | No effect. | The response already read may differ. |
| Another lint run targets the same page | Both runs may start. | Sessions remain separate. |
| The process exits outright | No result exists. | Partial output is not a final result. |

## Interactions with other systems

**Configuration precedence.** Explicit `--viewport` overrides the 1280 CSS pixel default.

**Output and exit status.** Load failures use stderr and status three.

**Resource limits.** The body limit is one MiB. The request limit is 15 seconds and the redirect limit is five.

**Network and storage.** The loader permits public HTTP and HTTPS, plus explicit loopback targets. It disables proxies and rejects DNS answers that include private or non-routable addresses. It writes no page data to storage.

**Rendering compatibility.** The loader accepts HTML. The first title element supplies the page title. Title whitespace collapses.

**Isolation.** browser.jr does not execute page scripts in this slice.

**Accessibility inspection.** Interactive snapshot extracts the documented static semantic subset after loading.

## Edge cases

- Missing content type is accepted as HTML.
- Non-HTML content types fail.
- Compressed response bodies fail until bounded decompression exists.
- More than five redirects fail.
- Invalid UTF-8 fails.
- HTTP errors fail.
- URL credentials fail.
- `get url` before a successful retained open reports that no page is open.
- `get title` before a successful retained open reports that no page is open.
- A missing title produces an empty title.
- Later title elements do not replace the first title.
- A failed replacement preserves the previously installed URL and title.
- A failed replacement preserves the previous page focus.
- A failed replacement preserves the previous page scroll offsets.

## Open questions and verification

- Decide whether current limits remain product defaults.

Drafted from the Rust implementation and focused tests on 2026-09-01.
