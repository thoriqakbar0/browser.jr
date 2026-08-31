# Verification: focus an element

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/focus-element.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FOCUS-01 | P1 | pipe | A current reference can become page focus ([The simple case](../interaction/focus-element.md#the-simple-case)). | Serve one labeled input. | Open, capture, then focus its reference. | Focus returns the reference and static element identity. | partial: package and compiled-process tests passed, 2026-08-31 |
| FOCUS-02 | P1 | pipe | Semantic and direct locators focus without a snapshot ([Invoke](../interaction/focus-element.md#invoke)). | Serve one labeled input and textarea. | Focus through an exact role locator and CSS selector. | Each strict target becomes current focus. | partial: package and compiled-process tests passed, 2026-08-31 |
| FOCUS-03 | P1 | package | Failed focus preserves the previous target ([Edge cases](../interaction/focus-element.md#edge-cases)). | Serve enabled and disabled inputs. | Focus enabled, reject disabled, then press one character. | The character changes the enabled input. | partial: package test passed, 2026-08-31 |
| FOCUS-04 | P1 | package | Document replacement clears focus ([While running](../interaction/focus-element.md#while-running)). | Serve the same input twice. | Focus it, reload, then press a character. | Press reports no focused element. | partial: package test passed, 2026-08-31 |
| FOCUS-05 | P2 | pipe | Focus reports identity without control values ([Finish](../interaction/focus-element.md#finish)). | Serve an input with a non-empty value. | Focus through role and reference paths. | Output names role, name, element, or reference without the value. | partial: compiled-process test passed, 2026-08-31 |
| FOCUS-06 | P1 | package | Each text control preserves its selection ([While running](../interaction/focus-element.md#while-running)). | Serve two text controls. | Edit the first, edit the second, then refocus the first. | The next key uses the first control's earlier caret. | partial: package test passed, 2026-08-31 |
| FOCUS-07 | P1 | package | Sequential traversal can move focus to the document body ([While running](../interaction/focus-element.md#while-running)). | Serve one natural target. | Press `Tab` twice from the body. | Focus moves to the target, then returns to the body. | partial: package and controlled Chromium tests passed, 2026-08-31 |
| FOCUS-08 | P1 | package | Negative `tabindex` supports direct focus but not sequential order ([Edge cases](../interaction/focus-element.md#edge-cases)). | Serve natural, negative, and positive targets. | Directly focus the negative target, then traverse in both directions. | Traversal leaves toward the adjacent supported document-order target. | partial: package and controlled Chromium tests passed, 2026-08-31 |
| FOCUS-09 | P1 | package | A radio group tracks its selected natural tab stop ([Interactions with other systems](../interaction/focus-element.md#interactions-with-other-systems)). | Serve two named radio groups and one unnamed radio. | Select another first-group member, then traverse. | Traversal leaves that member for the next group's one natural target. | partial: unit, package, compiled-process, and controlled Chromium tests passed, 2026-08-31 |

Not checkable yet:

- Browser focus events do not exist.
- Blur does not exist.
- Explicit radio `tabindex`, stylesheet, shadow DOM, and iframe focus order do not exist.
