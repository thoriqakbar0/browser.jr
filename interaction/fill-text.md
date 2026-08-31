# Fill a text control

## Summary

Package callers submit `FillElement` with a current interactive reference and replacement text.

Session-mode callers send `fill <ref> <text>` after an interactive snapshot in the same process.

[Locator actions](../inspection/query-elements.md) define the snapshot-free fill path.

Fill replaces the complete value. It does not imitate keyboard entry or dispatch browser events.

## The simple case

The caller opens a page and captures an interactive snapshot. It selects a textbox reference such as `@e1`.

The caller fills that reference. browser.jr replaces the stored value and reports success.

Another snapshot reports the replacement value in escaped quoted form.

## The interaction, event by event

```mermaid
stateDiagram-v2
    [*] --> checking_reference
    checking_reference --> rejected : missing or stale reference
    checking_reference --> checking_control : current reference
    checking_control --> unsupported : control is not fillable
    checking_control --> replacing : supported text control
    replacing --> filled
    rejected --> finished
    unsupported --> finished
    filled --> finished
```

### Invoke

The package request contains one typed reference and one string.

Session mode reads the reference and treats the remaining line as its string.

### Exit immediately

A stale package reference returns `SessionError::StaleElementReference`.

An unknown or stale session label reports invalid input. Neither path changes the document.

### Begin running

browser.jr checks the referenced element's fill capability.

[Value inspection](../inspection/read-value.md) defines supported text controls. Disabled and read-only controls expose values but reject fill.

### While running

Fill replaces the stored value once. It keeps the current reference usable.

The current implementation does not dispatch `beforeinput`, `input`, `change`, focus, or keyboard events.

It does not run constraint validation, page scripts, or form submission.

### Finish

The package returns `FillResult` with the reference and replacement value.

Session mode reports the reference and Unicode scalar-value count. It does not repeat the value in that result line.

A later interactive snapshot reports the current value. That new capture makes the earlier reference stale.

## Variants

| Modifier | Set at invocation | Changed while running |
| --- | --- | --- |
| Flags and options | The package takes a string. Session mode takes the rest of one line. | Fill has no flags. |
| Project configuration | No fill configuration exists. | Nothing reloads. |
| Target matrix | The current page and snapshot select one control. | Fill does not navigate or create a page. |
| Output channel | The package returns a typed value. Session mode uses flushed text. | A later snapshot exposes the stored value. |

## Cancel and interrupt

| Event | Before running | While running |
| --- | --- | --- |
| Ctrl+C once | The host or CLI process may exit. | Fill has no asynchronous phase or graceful handler. |
| Ctrl+C again before the evaluation stops | The process may already be gone. | No second-stage handler exists. |
| The process receives SIGTERM | The process may exit before the request. | In-memory state disappears with the process. |
| The terminal closes | Package behavior is unchanged. | Session-mode output may fail. |
| stdin or stdout closes | Package behavior is unchanged. | Closed session stdin ends the process. Closed stdout causes status three. |
| The network fails or a request times out | Fill uses no network. | The stored page already exists in memory. |
| The inspected page changes | Another snapshot or navigation can stale the reference. | Fill itself changes only the supported value. |
| Another lint run targets the same page | It owns another session. | It cannot observe this in-memory value. |
| The process exits outright | No fill occurs. | No filled value survives. |

## Interactions with other systems

**Configuration precedence.** The request string is the only value source.

**Output and exit status.** Package callers receive `FillResult` or `SessionError`. Session mode uses status two or three for failures.

**Resource limits.** No value-length limit exists yet. Session mode limits one value to one input line.

**Network and storage.** Fill uses no network and writes no persistent storage.

**Rendering compatibility.** Fill mutates the engine's text-control model. It does not implement browser editing algorithms.

**Isolation.** Values belong to one session and document. A successful open or navigation replaces them.

**Accessibility inspection.** The accessible name stays separate from the current value.

## Edge cases

- Package strings may contain line breaks. Session-mode strings may not.
- Session mode removes delimiter whitespace before the value.
- Session mode preserves trailing whitespace.
- A trailing delimiter with no text fills an empty string.
- Disabled and read-only controls reject fill.
- Password, number, checkbox, radio, range, select, and contenteditable controls reject fill.
- Explicit ARIA roles do not make a non-editable element fillable.
- A rejected fill preserves the current value and reference.
- Repeated fill replaces the previous value.
- Fill does not invalidate the current reference. A later snapshot does.

## Open questions and verification

- Define password input handling without exposing secrets.
- Define native validation and event order before expanding input types.
- Define maximum value size.
- Add keyboard entry as a separate action because it has different event behavior.

Drafted from the Rust implementation and package and compiled-process tests on 2026-08-31.
