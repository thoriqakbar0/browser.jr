# Read a text-control value

## Summary

Package callers submit `GetElementValue` with a current interactive reference.

Session-mode callers send `get value <ref>` after an interactive snapshot in the same process.

Interactive snapshots also include each supported current value.

## The simple case

The caller opens a page and captures an interactive snapshot. It selects a text-control reference.

The caller requests its value. browser.jr returns the current string without changing page or reference state.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> checking_reference
    checking_reference --> rejected : missing or stale reference
    checking_reference --> checking_value : current reference
    checking_value --> unsupported : value unavailable
    checking_value --> reporting : supported text value
    reporting --> finished
    rejected --> finished
    unsupported --> finished
```

### Invoke

The package request contains one typed reference.

Session mode reads `get value` and one displayed reference.

### Exit immediately

A stale package reference returns `SessionError::StaleElementReference`.

An unknown or stale session label reports invalid input. Neither path reads another element.

### Begin running

browser.jr resolves the reference through the latest interactive snapshot.

Value inspection supports `textarea` and these `input` types: empty, `text`, `email`, `search`, `tel`, and `url`.

Disabled and read-only controls in that subset remain readable.

### While running

The engine clones the current in-memory value. It does not capture a new snapshot.

The read does not dispatch events, run scripts, validate input, or change focus.

### Finish

The package returns `ElementValue` with the reference and current string.

Session mode writes `value ref=<ref> <quoted-value>` and flushes stdout.

The reference remains current after the read.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | The package and session commands take one reference. | No value-read flags exist. |
| Project configuration | No value-inspection configuration exists. | Nothing reloads. |
| Target matrix | The current page and snapshot select one control. | The read does not change the target. |
| Output channel | The package returns a typed value. Session mode uses escaped quoted text. | Session mode flushes after the command. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The host or CLI process may exit. | The read has no asynchronous phase or graceful handler. |
| Ctrl+C again before the evaluation stops | The process may already be gone. | No second-stage handler exists. |
| The process receives SIGTERM | The process may exit before the request. | In-memory state disappears with the process. |
| The terminal closes | Package behavior is unchanged. | Session-mode output may fail. |
| stdin or stdout closes | Package behavior is unchanged. | Closed stdin ends session mode. Closed stdout causes status three. |
| The network fails or a request times out | The read uses no network. | The current page already exists in memory. |
| The inspected page changes | Another snapshot or navigation can stale the reference. | This read does not change the page. |
| Another lint run targets the same page | It owns another session. | It cannot read this session's value. |
| The process exits outright | No result returns. | No session value survives. |

## Interactions with other systems

**Configuration precedence.** The current session value is the only source.

**Output and exit status.** Package callers receive `ElementValue` or `SessionError`. Session mode uses status two or three for failures.

**Resource limits.** No separate read limit exists. The current stored value bounds the returned string.

**Network and storage.** The read uses no network and writes no storage.

**Rendering compatibility.** The result comes from browser.jr's text-control model, not a platform DOM property implementation.

**Isolation.** Values and references belong to one session and document.

**Accessibility inspection.** The current value stays separate from the accessible name.

## Edge cases

- An empty supported value returns an empty string.
- Disabled and read-only supported text controls remain readable.
- Password values remain unavailable.
- Number, checkbox, radio, range, select, button, link, and contenteditable values remain unavailable.
- A successful fill changes the next direct read immediately.
- A value read does not invalidate its reference.
- A later snapshot invalidates references from the previous snapshot.
- Session-mode output escapes quotes, backslashes, control characters, and line breaks.

## Open questions and verification

- Define password reads without exposing secrets.
- Define other form-control value types.
- Define machine-readable response encoding.
- Define value size limits.

Drafted from the Rust implementation and package and compiled-process tests on 2026-08-31.
