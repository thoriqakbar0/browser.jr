# Verification: read hovered state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-hovered.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HOVERED-01 | P1 | package | Every target starts false ([Edge cases](../inspection/read-hovered.md#edge-cases)). | Serve one visible button. | Read by locator before hover. | The read returns false. | partial: package test passed, 2026-08-31 |
| HOVERED-02 | P1 | pipe | Reference reads track the current pointer target ([The simple case](../inspection/read-hovered.md#the-simple-case)). | Serve two buttons and capture. | Hover the first, then read both. | First is true. Second is false. | partial: package and compiled-process tests passed, 2026-08-31 |
| HOVERED-03 | P1 | pipe | Direct selectors read structural targets ([Invoke](../inspection/read-hovered.md#invoke)). | Serve one structural card. | Hover and read through CSS. | The read returns true. | partial: package, parser, and compiled-process tests passed, 2026-08-31 |
| HOVERED-04 | P1 | pipe | Semantic locators report identity and state ([Finish](../inspection/read-hovered.md#finish)). | Serve one disabled named button. | Hover, then find its hovered state by role. | Output names the target and returns true. | partial: parser and compiled-process tests passed, 2026-08-31 |
| HOVERED-05 | P1 | package | Failed hover preserves the prior read ([Edge cases](../inspection/read-hovered.md#edge-cases)). | Serve visible and hidden buttons. | Hover visible, reject hidden, then read visible. | Visible remains true. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- CSS pseudo-class ancestor matching does not exist.
- Machine-readable session responses do not exist.
