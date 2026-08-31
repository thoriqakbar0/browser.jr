# Verification: type text

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/type-text.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TYPE-01 | P1 | pipe | Type appends through a current reference ([The simple case](../interaction/type-text.md#the-simple-case)). | Serve a text input with value `old`. | Open, capture, type ` plus`, then read the value. | The current reference reports `old plus`. | partial: package and compiled-process tests passed, 2026-08-31 |
| TYPE-02 | P1 | pipe | A direct selector appends without a snapshot ([Invoke](../interaction/type-text.md#invoke)). | Serve a text input with value `old`. | Open, type ` more` through its CSS selector, then read the value. | The input reports `old more`. | partial: package and compiled-process tests passed, 2026-08-31 |
| TYPE-03 | P2 | pipe | Empty appended text is a successful no-op ([While running](../interaction/type-text.md#while-running)). | Serve a textarea with value `draft`. | Type an empty string through a current reference. | The result and later read report `draft`. | partial: package test passed, 2026-08-31 |
| TYPE-04 | P1 | pipe | Rejected type preserves current state ([Edge cases](../interaction/type-text.md#edge-cases)). | Serve read-only text and a button. | Type into both, then read the text value. | Both requests fail. The text value stays unchanged. | partial: package test passed, 2026-08-31 |
| TYPE-05 | P1 | pipe | A newer snapshot stales old type references ([Finish](../interaction/type-text.md#finish)). | Serve one text input. | Capture, type, capture again, then type through the old reference. | The final request reports a stale reference. | partial: package test passed, 2026-08-31 |
| TYPE-06 | P1 | pipe | Session output reports only the appended character count ([Finish](../interaction/type-text.md#finish)). | Serve an input and textarea with initial values. | Type through a reference and direct selector, read both values, then capture. | Outputs report counts. Reads and snapshot report each complete value. | partial: compiled-process test passed, 2026-08-31 |
| TYPE-07 | P2 | package | Type preserves an existing selection ([While running](../interaction/type-text.md#while-running)). | Focus an input with a collapsed caret. | Type appended text, then press one character. | Type appends; press inserts at the preserved caret. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Browser events do not exist.
- Per-character delay does not exist.
- Password handling does not exist.
- Type does not compose the separate bounded key-press action.
