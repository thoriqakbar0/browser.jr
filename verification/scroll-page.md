# Verification: scroll the page and reveal an element

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/scroll-page.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SCROLL-01 | P1 | package | Page scrolling commits bounded offsets ([Begin running](../interaction/scroll-page.md#begin-running)). | Serve a document wider and taller than 1280 by 720. | Scroll down and right. | The reply contains the committed positive offsets. | partial: package test passed, 2026-08-31 |
| SCROLL-02 | P1 | package | Every direction clamps safely ([While running](../interaction/scroll-page.md#while-running)). | Start from nonzero offsets. | Scroll up, down, left, and right beyond each limit. | Offsets stay between zero and the document maximum. | partial: package test passed, 2026-08-31 |
| SCROLL-03 | P1 | package | Normal boxes become viewport-relative ([Edge cases](../interaction/scroll-page.md#edge-cases)). | Serve one supported normal-flow button. | Read its box, scroll, and read again. | The box subtracts the committed offsets. | partial: package test passed, 2026-08-31 |
| SCROLL-04 | P1 | package | Fixed boxes stay anchored ([Edge cases](../interaction/scroll-page.md#edge-cases)). | Serve one fixed button. | Read its box before and after page scrolling. | Its coordinates do not change. | partial: package test passed, 2026-08-31 |
| SCROLL-05 | P1 | package | Reference element scrolling reveals the nearest supported box ([While running](../interaction/scroll-page.md#while-running)). | Put a supported button below and right of the viewport. | Capture its reference and scroll it into view. | The returned offsets reveal the box and preserve its reference. | partial: package test passed, 2026-08-31 |
| SCROLL-06 | P1 | package | Locator element scrolling uses strict current resolution ([Invoke](../interaction/scroll-page.md#invoke)). | Serve fixed, normal, and hidden targets. | Scroll through CSS locators. | One supported match returns its identity. Hidden evidence blocks. | partial: package test passed, 2026-08-31 |
| SCROLL-07 | P1 | package | Navigation replacement resets offsets ([Edge cases](../interaction/scroll-page.md#edge-cases)). | Scroll one page away from zero. | Reload it and read the same locator box. | The fresh page starts at zero offsets. | partial: package test passed, 2026-08-31 |
| SCROLL-08 | P1 | pipe | Session page scrolling defaults to 300 pixels ([Invoke](../interaction/scroll-page.md#invoke)). | Serve a tall page. | Send `scroll down`. | Output reports `y=300` and `moved=true`. | partial: parser and compiled-process tests passed, 2026-08-31 |
| SCROLL-09 | P1 | pipe | Reference and selector element commands share state ([Finish](../interaction/scroll-page.md#finish)). | Capture a lower button and keep a fixed selector. | Send `scrollintoview @e1`, then `scrollinto #fixed`. | Output identifies each target and reports committed offsets. | partial: parser and compiled-process tests passed, 2026-08-31 |
| SCROLL-10 | P1 | pipe | Semantic locator scrolling resolves current roles ([Invoke](../interaction/scroll-page.md#invoke)). | Serve a named button. | Send `find role button scroll --name Target --exact`. | Output reports the resolved role, name, element, and offsets. | partial: parser and compiled-process tests passed, 2026-08-31 |
| SCROLL-11 | P2 | package | Missing pages reject scrolling ([Exit immediately](../interaction/scroll-page.md#exit-immediately)). | Start a new session without a page. | Submit `ScrollPage`. | The request returns `SessionError::NoPage`. | partial: package test passed, 2026-08-31 |
| SCROLL-12 | P2 | package | Failed element scrolling preserves usable state ([While running](../interaction/scroll-page.md#while-running)). | Capture visible and hidden references. | Reject the hidden target, then read the visible reference. | The read succeeds and offsets stay committed. | partial: package test passed, 2026-08-31 |
| SCROLL-13 | P1 | package | Pointer actions reveal supported target boxes after checks ([While running](../interaction/scroll-page.md#while-running)). | Put click, check, hover, and disabled targets below a short viewport. | Run each supported action and one rejected action. | Successful changed actions reveal their boxes. The rejection preserves offsets. | partial: package and compiled-process tests passed, 2026-08-31 |

Not checkable yet:

- Nested scroll containers do not exist.
- Fractional and smooth scrolling do not exist.
- A caller option to disable action auto-scroll does not exist.
