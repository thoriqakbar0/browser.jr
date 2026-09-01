# Verification: hover an element

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/hover-element.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HOVER-01 | P1 | package | A current reference can become the pointer target ([The simple case](../interaction/hover-element.md#the-simple-case)). | Serve one visible button. | Capture, hover its reference, then read state. | Hover succeeds. The same reference reads true. | partial: package and compiled-process tests passed, 2026-08-31 |
| HOVER-02 | P1 | pipe | Semantic and direct locators hover without a snapshot ([Invoke](../interaction/hover-element.md#invoke)). | Serve one button and structural card. | Hover by exact role and CSS. | Each strict visible target becomes current. | partial: package, parser, and compiled-process tests passed, 2026-08-31 |
| HOVER-03 | P1 | package | Another hover replaces the current target ([While running](../interaction/hover-element.md#while-running)). | Serve two visible targets. | Hover each target, then read both. | The first reads false. The second reads true. | partial: package test passed, 2026-08-31 |
| HOVER-04 | P1 | package | Hidden targets reject without mutation ([Begin running](../interaction/hover-element.md#begin-running)). | Serve visible and hidden buttons. | Hover visible, then try hidden. | Hidden hover blocks. Visible remains current. | partial: package test passed, 2026-08-31 |
| HOVER-05 | P1 | package | Disabled controls may become pointer targets ([Begin running](../interaction/hover-element.md#begin-running)). | Serve one disabled button. | Hover through a current reference. | Hover succeeds without changing focus or control state. | partial: package and compiled-process tests passed, 2026-08-31 |
| HOVER-06 | P1 | package | Document replacement clears pointer state ([Edge cases](../interaction/hover-element.md#edge-cases)). | Serve the same visible button twice. | Hover, reload, then read by locator. | The new document returns false. | partial: package test passed, 2026-08-31 |
| HOVER-07 | P1 | package | Hover auto-scrolls after visibility checks ([While running](../interaction/hover-element.md#while-running)). | Put one supported structural target below a short viewport. | Hover it through CSS, then read its box. | The complete box is inside the viewport. Pointer state becomes current. | partial: package and compiled-process tests passed, 2026-08-31 |
| HOVER-08 | P1 | pipe | Hover blocks unsupported motion without replacing pointer state ([Begin running](../interaction/hover-element.md#begin-running)). | Give reference and locator targets inline motion declarations. | Hover each target, then inspect current pointer state. | Both paths report the stable check. No target becomes hovered. | partial: package and compiled-process tests passed, 2026-09-01 |
| HOVER-09 | P1 | pipe | A supported fixed blocker rejects hover transactionally ([Begin running](../interaction/hover-element.md#begin-running)). | Cover a supported hover target's action point with an earlier fixed element. | Hover through reference and locator paths, then inspect pointer state. | Both paths name the receives-events check and blocker. Offsets and pointer state remain unchanged. | partial: package and compiled-process tests passed; Playwright 1.62.1 and `agent-browser` 0.32.4 rejected the blocker, 2026-09-01 |
| HOVER-10 | P1 | package | Hover records target transitions and related targets ([While running](../interaction/hover-element.md#while-running)). | Serve two supported buttons. | Hover each button, drain events, then repeat the second hover. | The transition records out, leave, over, enter, and move. The repeat records only pointer and mouse move. | partial: package test passed; Playwright 1.62.1 Chrome matched the target order, 2026-09-01 |

Not checkable yet:

- Complete ancestor pointer dispatch, receives-events hit testing, and motion frame sampling do not exist.
- Pointer coordinates, buttons, pointer IDs, and actual descendant hit targets do not exist.
- Dynamic CSS `:hover` matching does not exist.
- Hover options and auto-waiting do not exist.
