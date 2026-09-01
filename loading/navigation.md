# Navigate by clicking a link

## Summary

Package callers use `ClickElement` with a reference from the latest interactive snapshot.

Session-mode callers send `click <ref>` after `snapshot --interactive` in the same process.

This action follows one same-context HTML link.

Native link `Enter` follows the same loading transaction. [`press-key.md`](../interaction/press-key.md) owns its typed effect.

[`interaction/click-element.md`](../interaction/click-element.md) owns native control effects and unsupported click boundaries.

[`interaction/submit-form.md`](../interaction/submit-form.md) owns bounded GET form navigation.

The one-shot CLI has no `click` command. [`browser.jr session`](../automation/ai-session.md) preserves the required state over stdin.

## The simple case

The caller opens a page through `OpenPage`. It captures `CaptureInteractiveSnapshot` and selects one link reference.

The caller submits `ClickElement`. browser.jr resolves the link against the current URL and loads the next document.

The result returns `ClickResult::Navigated` with the clicked reference and opened page summary.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> checking_reference
    checking_reference --> rejected : missing or stale reference
    checking_reference --> checking_action : current reference
    checking_action --> unsupported : action outside implemented subset
    checking_action --> loading : same-context link
    loading --> failed : invalid or unavailable target
    loading --> navigated : valid HTML response
    navigated --> finished
    rejected --> finished
    unsupported --> finished
    failed --> finished
```

### Invoke

The package caller submits `ClickElement` to the session that produced the reference.

The session-mode caller sends the displayed reference to the process that produced it.

### Exit immediately

A missing page, stale reference, or unsupported click returns without network access.

### Begin running

A supported link click resolves its `href` against the current page URL.

The loader then applies the same target, response, and body rules as [`OpenPage`](open-page.md).

### While running

The current page and reference remain usable until the replacement document loads successfully.

Links with `download` are unsupported. Links targeting another browsing context are unsupported.

Supported submitter clicks may enter GET form navigation.

Other native control clicks do not navigate. Explicit ARIA controls have no native default action.

### Finish

Successful navigation installs a new document epoch. It invalidates the previous snapshot references and layout evidence.

It also clears the previous page's stored [focus](../interaction/focus-element.md).

The replacement starts [page scroll](../interaction/scroll-page.md) at zero on both axes.

It keeps the page's configured [viewport size](../interaction/set-viewport.md).

Session mode also clears its displayed-reference set.

`GetPageUrl` and session-mode `get url` then report the resolved navigation target.

Failed navigation preserves the current page and latest snapshot reference.

It also preserves the current focus.

It preserves the current page scroll offsets.

The caller must capture another interactive snapshot before the next reference action.

A [locator action](../inspection/query-elements.md) resolves the replacement document directly.

[History navigation](history-navigation.md) records successful link and supported form navigation.

Successful native link `Enter` navigation records the same history entry.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | The package request contains one typed reference. Session mode accepts `click <ref>`. | The reference cannot change. |
| Project configuration | No navigation configuration exists. | Nothing reloads. |
| Target matrix | The current session page supplies the base URL. | Success replaces that page document. |
| Output channel | The package returns typed values. Session mode writes line-oriented text. | Session mode flushes after each command. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The host process controls cancellation. | No package cancellation contract exists. |
| Ctrl+C again before the evaluation stops | The host process controls termination. | No second-stage package handler exists. |
| The process receives SIGTERM | The host process may exit. | Partial work does not commit a document. |
| The terminal closes | Package behavior does not depend on a terminal. | The host may continue. |
| stdin or stdout closes | Package behavior does not depend on either stream. | The request may continue. |
| The network fails or a request times out | No replacement occurs. | The current page remains installed. |
| The inspected page changes | No script mutation exists. | Another successful open makes the reference stale. |
| Another lint run targets the same page | Another session stays independent. | This session keeps its own page. |
| The process exits outright | In-memory state disappears. | No persistent partial navigation exists. |

## Interactions with other systems

**Configuration precedence.** The current page URL and clicked `href` determine the target.

**Output and exit status.** The package returns `ClickResult` or `SessionError`. Session mode reports results and accumulates its final exit status.

**Resource limits.** The body limit is one MiB. A wall-clock request timeout is not implemented yet.

**Network and storage.** The resolved target must remain within the session network policy. Navigation writes no persistent state.

**Rendering compatibility.** Navigation parses static HTML. JavaScript and browser event dispatch remain unsupported.

**Isolation.** A failed target cannot replace the current page. Separate sessions do not share page state.

**Accessibility inspection.** The click uses a snapshot reference, supported visibility, and static stability evidence.

It does not sample motion frames or check event reception.

## Edge cases

- A fresh snapshot makes references from the previous snapshot stale.
- A successful navigation makes its clicked reference stale.
- A relative `href` resolves against the current page URL.
- A target outside the session network policy fails without replacing the page.
- A supported native button returns `Activated` without navigation.
- A supported native submitter returns `Navigated` after GET form loading.
- A `_blank` target does not silently navigate the current page.
- A download link does not silently navigate the current page.
- A session-mode click requires a reference from that process's latest snapshot.

## Open questions and verification

- Define same-document fragment navigation.
- Define redirects and response commit boundaries.
- Define link event dispatch and JavaScript navigation.
- Define motion frame sampling and complete receives-events hit testing.
- Define whether a later one-shot client uses a daemon, socket, or another retained-session transport.

Drafted from Rust implementation and package boundary tests on 2026-09-01.
