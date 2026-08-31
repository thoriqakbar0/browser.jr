# Verification: read focused state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-focused.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FOCUSED-01 | P1 | package | A new document gives element targets an unfocused state ([The simple case](../inspection/read-focused.md#the-simple-case)). | Serve two controls inside an explicit body. | Open, capture, and read the first reference. | The element returns false; the body locator returns true. | partial: package test passed, 2026-08-31 |
| FOCUSED-02 | P1 | package | One focus change updates both target results ([Begin running](../inspection/read-focused.md#begin-running)). | Use the FOCUSED-01 page. | Focus the second control by exact role, then read both references. | The first returns false and the second returns true. | partial: package test passed, 2026-08-31 |
| FOCUSED-03 | P1 | package | A Tab boundary restores body focus ([While running](../inspection/read-focused.md#while-running)). | Focus the last sequential target. | Press Tab, then read the target and body. | The target returns false and the body returns true. | partial: package and compiled-process tests passed, 2026-08-31 |
| FOCUSED-04 | P1 | package | Structural targets have a defined false state ([Begin running](../inspection/read-focused.md#begin-running)). | Serve a plain div beside focused controls. | Focus one control, then read the div through CSS. | The div returns false without an unsupported error. | partial: package and compiled-process tests passed, 2026-08-31 |
| FOCUSED-05 | P1 | package | Focused reads keep standard resolution failures ([Exit immediately](../inspection/read-focused.md#exit-immediately)). | Start without a page and retain one old reference after recapture. | Read by locator before open, then read the old reference. | Return `NoPage`, then `StaleElementReference`. | partial: package test passed, 2026-08-31 |
| FOCUSED-06 | P1 | pipe | Session outputs separate refs, selectors, and semantic locators ([Finish](../inspection/read-focused.md#finish)). | Serve labeled controls and a structural target. | Read through refs, CSS, body, and exact role. | Each path returns its documented Boolean form and preserves references. | partial: parser and compiled-process tests passed, 2026-08-31 |
| FOCUSED-07 | P2 | package | Focused state agrees with browser active-element evidence ([Interactions with other systems](../inspection/read-focused.md#interactions-with-other-systems)). | Serve two controls. | Compare initial, focused, and post-Tab results. | browser.jr and Chromium agree on body and element ownership. | partial: package and controlled Chromium tests passed, 2026-08-31 |

Not checkable yet:

- Retrying assertions do not exist.
- Blur and browser focus events do not exist.
- Shadow-tree and iframe active-element ownership do not exist.
