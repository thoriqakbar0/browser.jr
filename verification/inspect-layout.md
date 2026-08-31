# Verification: inspect element geometry

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/inspect-layout.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BOX-01 | P1 | package | Fixed inline geometry returns one complete border box ([The simple case](../inspection/inspect-layout.md#the-simple-case)). | Serve a fixed button with integer pixel edges. | Read its current reference. | The reply contains `x`, `y`, `width`, and `height`. | partial: package test passed, 2026-08-31 |
| BOX-02 | P1 | package | Supported box sizing controls returned dimensions ([While running](../inspection/inspect-layout.md#while-running)). | Serve content-box and border-box targets with longhand edges. | Read through references and locators. | Content-box expands. Border-box keeps its declared dimensions. | partial: package test passed, 2026-08-31 |
| BOX-03 | P1 | pipe | References, CSS, and XPath share one box ([Begin running](../inspection/inspect-layout.md#begin-running)). | Serve one fixed interactive element. | Read by reference, CSS, and XPath. | Every path reports identical coordinates and dimensions. | partial: package and compiled-process tests passed, 2026-08-31 |
| BOX-04 | P1 | pipe | Hidden targets return no box ([While running](../inspection/inspect-layout.md#while-running)). | Serve a hidden fixed element. | Read by reference and CSS. | Package replies contain `None`. Session mode prints `null`. | partial: package and compiled-process tests passed, 2026-08-31 |
| BOX-05 | P1 | package | Incomplete geometry blocks without partial evidence ([Finish](../inspection/inspect-layout.md#finish)). | Serve intrinsic text and a collapsing margin. | Read each box, then reuse a current reference. | Each box reports unsupported. The reference remains current. | partial: package test passed, 2026-08-31 |
| BOX-06 | P2 | pipe | Session output matches the agent command shape ([Finish](../inspection/inspect-layout.md#finish)). | Serve one supported box. | Send `get box` through three target forms. | Each reply prints four stable named lines. | partial: parser and compiled-process tests passed, 2026-08-31 |
| BOX-07 | P1 | package | Missing pages and locators keep standard failures ([Exit immediately](../inspection/inspect-layout.md#exit-immediately)). | Start without a page. | Read through a CSS locator. | The request returns `NoPage`. | partial: package test passed, 2026-08-31 |
| BOX-08 | P1 | package | Normal blocks stack and size auto-height parents ([While running](../inspection/inspect-layout.md#while-running)). | Serve nested static blocks with explicit child heights. | Read the body, parent, and children. | Child edges stack. Parent heights contain supported in-flow children. | partial: package test passed, 2026-08-31 |
| BOX-09 | P1 | package | Out-of-flow and hidden boxes preserve normal placement ([Edge cases](../inspection/inspect-layout.md#edge-cases)). | Mix fixed, hidden, invisible, empty, and normal siblings. | Read every target. | Fixed and non-generated boxes do not shift siblings. Invisible boxes do shift them. | partial: package test passed, 2026-08-31 |
| BOX-10 | P1 | pipe | Explicit normal-flow interactive boxes support references and selectors ([Begin running](../inspection/inspect-layout.md#begin-running)). | Serve a border-box button inside stacked blocks. | Read its reference and a later CSS target. | Both boxes report their normal-flow coordinates. The reference remains usable. | partial: package and compiled-process tests passed, 2026-08-31 |
| BOX-11 | P1 | package | Box reads use current viewport offsets ([While running](../inspection/inspect-layout.md#while-running)). | Serve normal and fixed supported boxes. | Scroll both axes and read both boxes. | Normal coordinates change. Fixed coordinates stay stable. | partial: package test passed, 2026-08-31 |
| BOX-12 | P1 | package | Viewport resize recomputes supported boxes ([Begin running](../inspection/inspect-layout.md#begin-running)). | Serve an explicit-height body. | Read at 640 width, resize to 800, and read again. | The viewport-relative body width changes from 624 to 784. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Stylesheet, transform, intrinsic, margin-collapse, and fractional geometry do not exist.
- Child-frame coordinate conversion does not exist.
- Geometry auto-waiting does not exist.
