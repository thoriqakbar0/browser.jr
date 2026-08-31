# Verification: fill text

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/fill-text.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FILL-01 | P1 | pipe | Fill replaces a supported input value ([The simple case](../interaction/fill-text.md#the-simple-case)). | Serve a labeled text input with an initial value. | Open, snapshot, fill, and snapshot again. | The second snapshot reports the replacement value. | partial: package and compiled-process tests passed, 2026-08-31 |
| FILL-02 | P1 | pipe | One snapshot can drive more than one fill ([While running](../interaction/fill-text.md#while-running)). | Serve a text input and textarea. | Capture once and fill both references. | Both fills succeed before another capture. | partial: package boundary test passed, 2026-08-31 |
| FILL-03 | P1 | pipe | Unsupported fill preserves current references ([Edge cases](../interaction/fill-text.md#edge-cases)). | Serve a button with supported controls. | Try to fill the button, then fill a current text reference. | The button fails. The text fill still reaches action dispatch. | partial: package boundary test passed, 2026-08-31 |
| FILL-04 | P1 | pipe | A newer snapshot stales old fill references ([Finish](../interaction/fill-text.md#finish)). | Serve one text input. | Capture, fill, capture again, then fill with the old reference. | The last request reports a stale reference. | partial: package boundary test passed, 2026-08-31 |
| FILL-05 | P2 | pipe | Session mode accepts spaces without echoing the result value ([Finish](../interaction/fill-text.md#finish)). | Serve one text input. | Send `fill @e1 hello world`, then capture again. | Fill reports 11 characters. The snapshot reports `hello world`. | partial: compiled-process test passed, 2026-08-31 |
| FILL-06 | P1 | pipe | Successful fill records one bubbling input event ([While running](../interaction/fill-text.md#while-running)). | Serve one nested text input. | Fill once, then drain DOM events twice. | The first drain reports one input event and its ancestry. The second drain is empty. | partial: package and compiled-process tests passed, 2026-08-31 |

Not checkable yet:

- `beforeinput`, focus, listener invocation, and native validation do not exist.
- Password handling does not exist.
- Keyboard entry does not exist.
