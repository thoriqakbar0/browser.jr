# Verification: click an element

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/click-element.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CLICK-01 | P1 | package | A native button click stores focus ([The simple case](../interaction/click-element.md#the-simple-case)). | Serve one visible `type="button"`. | Capture, click its reference, then read focused state. | The result is `Activated`. Focus is true. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-02 | P1 | pipe | Semantic and direct checkbox clicks toggle current state ([While running](../interaction/click-element.md#while-running)). | Serve one labeled unchecked checkbox. | Click by exact role, read state, then click by CSS. | Results commit true, then false. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-03 | P1 | package | Native clicks preserve current references ([While running](../interaction/click-element.md#while-running)). | Serve one button and checkbox, then capture. | Click both through reference and locator paths. | Earlier references still read focus, checked state, and text. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-04 | P1 | package | Hidden and disabled controls reject clicks ([Begin running](../interaction/click-element.md#begin-running)). | Serve a hidden button and disabled checkbox. | Capture, then click each reference. | Both report unsupported click. Neither changes state. | partial: package test passed, 2026-08-31 |
| CLICK-05 | P1 | package | Supported GET submitters navigate ([Begin running](../interaction/click-element.md#begin-running)). | Serve one default button inside a GET form. | Click through an exact role locator. | The result reports navigation with encoded successful controls. | partial: unit and package tests passed, 2026-08-31 |
| CLICK-06 | P1 | pipe | Native output reports identity without content values ([Finish](../interaction/click-element.md#finish)). | Serve one button and checkbox. | Click the reference, semantic locator, and direct selector. | Output reports focus and checked effects only. | partial: compiled-process test passed, 2026-08-31 |
| CLICK-07 | P1 | package | Same-context link clicks still replace the document ([While running](../interaction/click-element.md#while-running)). | Serve a link and one destination page. | Capture and click the link reference. | The result is `Navigated`. Old references become stale. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-08 | P1 | pipe | Radio clicks select one exclusive group member ([While running](../interaction/click-element.md#while-running)). | Serve one named radio group. | Click another member, then repeat that click. | The target stays true, its peer becomes false, and focus moves to the target. | partial: package, compiled-process, controlled Chromium, and controlled agent-browser evidence passed, 2026-08-31 |
| CLICK-09 | P1 | package | Local clicks auto-scroll after actionability checks ([While running](../interaction/click-element.md#while-running)). | Put one supported button below a short viewport. | Click its current reference, then read its box. | The complete box is inside the viewport. Focus and the reference remain current. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-10 | P1 | package | Native click records preserve source identity across navigation ([While running](../interaction/click-element.md#while-running)). | Serve a button, checkbox, radio, link, and destination page. | Click supported targets, navigate, then drain events. | Each click records `click`. Changed checked controls add `input`, `change`. Navigation retains source metadata. | partial: package and compiled-process tests passed, 2026-08-31 |
| CLICK-11 | P1 | pipe | Click blocks unsupported motion before effects ([Begin running](../interaction/click-element.md#begin-running)). | Give a target an inline animation or transition declaration. | Click through reference and locator paths, then inspect state and events. | Both paths report the stable check. Focus, state, references, and events remain unchanged. | partial: package and compiled-process tests passed; Playwright 1.62.1 timed out across Chromium, Firefox, and WebKit, 2026-09-01 |

Not checkable yet:

- Pointer, mouse, focus, and submit event records do not exist.
- Page-script event delivery does not exist.
- Receives-events checks and motion frame sampling do not exist.
- POST, reset, image submitters, and broader form defaults do not exist.
- Pointer options, auto-waiting, and action timeouts do not exist.
