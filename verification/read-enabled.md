# Verification: read enabled state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-enabled.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ENABLED-01 | P1 | pipe | Active native controls return true ([The simple case](../inspection/read-enabled.md#the-simple-case)). | Serve an enabled checkbox and button. | Open, snapshot, and read both states. | Both package reads return true. | partial: package boundary test passed, 2026-08-31 |
| ENABLED-02 | P1 | pipe | Disabled native controls return false ([Begin running](../inspection/read-enabled.md#begin-running)). | Serve a disabled checkbox. | Read enabled state through its reference. | The package and session results return false. | partial: package and compiled-process tests passed, 2026-08-31 |
| ENABLED-03 | P1 | pipe | Explicit roles do not invent native state ([While running](../inspection/read-enabled.md#while-running)). | Serve a `div` with role `switch`. | Read its enabled state. | The package returns `UnsupportedEnabledState`. | partial: package boundary test passed, 2026-08-31 |
| ENABLED-04 | P1 | pipe | Session reads preserve current references ([Finish](../inspection/read-enabled.md#finish)). | Serve one native checkbox. | Read enabled state, then read checked state. | Both commands use the same reference. | partial: compiled-process test passed, 2026-08-31 |

Not checkable yet:

- Disabled fieldset inheritance does not exist.
- ARIA-disabled observation does not exist.
- Full browser actionability does not exist.
