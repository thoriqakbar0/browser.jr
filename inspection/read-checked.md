# Read checked state

## Summary

Package callers submit `GetElementChecked` with a reference or `GetCheckedByLocator` with a locator.

Session-mode callers send `is checked <ref|selector>`. Direct selectors need no snapshot.

Interactive snapshots also report current native checkbox and radio state.

## The simple case

The caller opens a page and captures an interactive snapshot. It selects a checkbox or radio reference.

browser.jr returns the current Boolean state without changing the page or reference.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> resolving_target
    resolving_target --> rejected : stale, missing, or non-unique target
    resolving_target --> checking_state : current reference or strict locator
    checking_state --> unsupported : state unavailable
    checking_state --> reporting : native checkbox or radio
    reporting --> finished
    rejected --> finished
    unsupported --> finished
```

### Invoke

The package request contains one typed reference or locator.

Session mode reads `is checked` and one reference or selector.

### Exit immediately

A stale package reference returns `SessionError::StaleElementReference`.

An unknown session label reports invalid input. Neither path reads another element.

### Begin running

browser.jr resolves a reference through the latest interactive snapshot or a locator through the current document.

State inspection supports native checkbox and radio inputs, including disabled controls.

### While running

The engine copies the current Boolean state. It does not capture a new snapshot.

The read does not dispatch events, run scripts, validate input, or change focus.

### Finish

The package returns `ElementChecked` for references or `LocatorChecked` for locators.

Reference output reports `checked ref=<ref> value=<boolean>`. Direct selectors print the Boolean.

The reference remains current after the read.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | The package and session commands take one reference or locator. | No state-read flags exist. |
| Project configuration | No checked-state inspection configuration exists. | Nothing reloads. |
| Target matrix | The current page and snapshot select one control. | The read does not change it. |
| Output channel | The package returns a typed value. Session mode uses flushed text. | Snapshots expose the same Boolean. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The host or CLI process may exit. | The read has no asynchronous phase. |
| Ctrl+C again before the evaluation stops | The process may already be gone. | No second-stage handler exists. |
| The process receives SIGTERM | The process may exit before the request. | In-memory state disappears. |
| The terminal closes | Package behavior is unchanged. | Session output may fail. |
| stdin or stdout closes | Package behavior is unchanged. | Closed stdin ends session mode. |
| The network fails or times out | The read uses no network. | The page already exists in memory. |
| The inspected page changes | Navigation can stale the reference. | This read does not change state. |
| Another lint run targets the page | It owns another session. | It cannot read this state. |
| The process exits outright | No result returns. | No session state survives. |

## Interactions with other systems

**Configuration precedence.** Current session state is the only source.

**Output and exit status.** Package callers receive a typed result or `SessionError`. Session failures use status two or three.

**Resource limits.** No separate read limit exists.

**Network and storage.** The read uses no network and writes no storage.

**Rendering compatibility.** The result comes from browser.jr's native checked-state model.

**Isolation.** Checked state and references belong to one session and document.

**Accessibility inspection.** Snapshot state stays separate from the accessible name.

## Edge cases

- Unchecked native checkboxes return false.
- Checked native checkboxes return true.
- Native radios return their normalized exclusive group state.
- Disabled native checkboxes and radios remain readable.
- Switches, buttons, links, and textboxes report unsupported state.
- A successful state change affects the next direct read immediately.
- A direct read does not invalidate its reference.
- Direct selectors resolve strictly without a prior snapshot.
- A later snapshot invalidates references from the previous snapshot.

## Open questions and verification

- Define ARIA checked-state inspection independently from native state.
- Define machine-readable response encoding.

Drafted from Rust package and compiled-process tests on 2026-08-31.
