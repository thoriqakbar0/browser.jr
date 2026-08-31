# Verification: read visible state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-visible.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| VISIBLE-01 | P1 | pipe | Supported default boxes return true ([The simple case](../inspection/read-visible.md#the-simple-case)). | Serve a native button and text-bearing link. | Open, snapshot, and read both states. | Both reads return true. | partial: unit and package tests passed, 2026-08-31 |
| VISIBLE-02 | P1 | pipe | Definite hidden states return false ([While running](../inspection/read-visible.md#while-running)). | Serve hidden and ancestor-hidden buttons. | Read each current reference. | The hidden attribute and ancestor `display:none` both return false. | partial: unit, package, and compiled-process tests passed, 2026-08-31 |
| VISIBLE-03 | P1 | pipe | Inherited visibility supports a visible override ([While running](../inspection/read-visible.md#while-running)). | Serve two buttons under `visibility:hidden`; override one child. | Read both references. | The inherited target returns false. The override returns true. | partial: unit test passed, 2026-08-31 |
| VISIBLE-04 | P1 | pipe | Empty non-replaced elements return false ([Edge cases](../inspection/read-visible.md#edge-cases)). | Serve an empty `div` with an interactive role and name. | Snapshot and read its visible state. | The read returns false. | partial: unit and package tests passed, 2026-08-31 |
| VISIBLE-05 | P1 | pipe | Missing box evidence reports unsupported ([Finish](../inspection/read-visible.md#finish)). | Serve a button with inline width and an embedded stylesheet page. | Read visibility, then reuse the current reference. | Visibility reports unsupported. Another read still accepts the reference. | partial: unit and package tests passed, 2026-08-31 |
| VISIBLE-06 | P2 | pipe | Session mode reports stable Boolean output ([Finish](../inspection/read-visible.md#finish)). | Serve visible and hidden buttons. | Send `is visible` for each reference. | Output reports each reference and an unquoted Boolean. | partial: compiled-process test passed, 2026-08-31 |
| VISIBLE-07 | P1 | pipe | Direct selectors inspect non-interactive visibility without a snapshot ([Finish](../inspection/read-visible.md#finish)). | Serve a hidden non-interactive element. | Read visible state through CSS. | The result is false. | partial: package, compiled-process, and controlled agent-browser comparison passed, 2026-08-31 |

Not checkable yet:

- Stylesheet cascade and complete vertical geometry do not exist.
- Waiting and retrying visibility assertions do not exist.
- Stability, pointer targeting, and viewport intersection do not exist.
