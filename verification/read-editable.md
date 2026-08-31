# Verification: read editable state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-editable.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| EDITABLE-01 | P1 | pipe | Enabled text controls return true ([The simple case](../inspection/read-editable.md#the-simple-case)). | Serve an input and textarea. | Read each state through a reference and selector. | Every result is true. | partial: package, compiled-process, and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-02 | P1 | pipe | Disabled and read-only text controls return false ([While running](../inspection/read-editable.md#while-running)). | Serve disabled and read-only inputs. | Read each state. | Every result is false. | partial: package, compiled-process, and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-03 | P1 | package | Native input types share one rule ([Edge cases](../inspection/read-editable.md#edge-cases)). | Serve checkbox and button input types. | Read both through strict selectors. | Both enabled inputs return true. | partial: package and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-04 | P1 | package | Enabled selects return true ([While running](../inspection/read-editable.md#while-running)). | Serve one enabled select. | Read its state through CSS. | The result is true. | partial: package and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-05 | P1 | pipe | Contenteditable state inherits through ancestors ([While running](../inspection/read-editable.md#while-running)). | Serve a span inside an editing host. | Read the span through CSS. | The result is true. | partial: package, compiled-process, and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-06 | P1 | package | Unsupported elements do not invent false state ([Edge cases](../inspection/read-editable.md#edge-cases)). | Serve a button and explicit false contenteditable element. | Read each state. | Both report unsupported inspection. | partial: package and controlled Chromium tests passed, 2026-08-31 |
| EDITABLE-07 | P1 | pipe | Editable reads preserve current references ([Finish](../inspection/read-editable.md#finish)). | Serve an input and capture it. | Read editable state, then read text through the same reference. | Both commands use the current reference. | partial: package and compiled-process tests passed, 2026-08-31 |
| EDITABLE-08 | P1 | package | Disabled fieldset evidence blocks uncertain state ([While running](../inspection/read-editable.md#while-running)). | Serve an input inside a disabled fieldset. | Read through CSS. | Inspection blocks with the disabled-fieldset boundary. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Disabled fieldset first-legend behavior does not exist.
- Contenteditable interactive references do not exist.
- ARIA read-only observation does not exist.
