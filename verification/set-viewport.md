# Verification: set the viewport size

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/set-viewport.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| VIEWPORT-01 | P1 | package | The session starts at 1280 by 720 CSS pixels ([Summary](../interaction/set-viewport.md#summary)). | Create a new session. | Submit `GetViewportSize`. | The typed result contains 1280 and 720. | partial: package test passed, 2026-08-31 |
| VIEWPORT-02 | P1 | package | Viewport size can be set before open ([Begin running](../interaction/set-viewport.md#begin-running)). | Create a session without a page. | Set 640 by 480, then open a page. | The page uses the configured width and height. | partial: package test passed, 2026-08-31 |
| VIEWPORT-03 | P1 | package | Current page geometry reflows ([Begin running](../interaction/set-viewport.md#begin-running)). | Serve an explicit-height body. | Read at 640 width, resize to 800, and read again. | Supported body width changes from 624 to 784. | partial: package test passed, 2026-08-31 |
| VIEWPORT-04 | P1 | package | Live control and focus state survive reflow ([While running](../interaction/set-viewport.md#while-running)). | Serve one text input. | Fill and focus it, resize, then read through the old reference. | The changed value and focus remain current. | partial: package test passed, 2026-08-31 |
| VIEWPORT-05 | P1 | package | Resize clamps obsolete scroll offsets ([While running](../interaction/set-viewport.md#while-running)). | Scroll a 900-pixel page in a 640 by 480 viewport. | Resize to 800 by 600. | Vertical scroll clamps from 428 to 308 and reports movement. | partial: package test passed, 2026-08-31 |
| VIEWPORT-06 | P1 | package | Equal resize is idempotent ([While running](../interaction/set-viewport.md#while-running)). | Set 800 by 600 twice. | Compare both replies. | The second reports `resized=false` and `moved=false`. | partial: package test passed, 2026-08-31 |
| VIEWPORT-07 | P1 | package | Invalid dimensions preserve the current viewport ([Exit immediately](../interaction/set-viewport.md#exit-immediately)). | Configure 800 by 600. | Set zero width, then read the size. | The error includes the invalid values. The size stays 800 by 600. | partial: package test passed, 2026-08-31 |
| VIEWPORT-08 | P1 | pipe | Session set and get report committed dimensions ([Finish](../interaction/set-viewport.md#finish)). | Start session mode. | Get, set before open, resize after open, and get again. | Output reports default, configured, and resized dimensions. | partial: parser and compiled-process tests passed, 2026-08-31 |
| VIEWPORT-09 | P1 | pipe | Current references survive session resize ([While running](../interaction/set-viewport.md#while-running)). | Capture and fill one input. | Resize, then read value and focus through its reference. | Both reads report the state from before resize. | partial: compiled-process test passed, 2026-08-31 |
| VIEWPORT-10 | P2 | pipe | Invalid session syntax is rejected locally ([Exit immediately](../interaction/set-viewport.md#exit-immediately)). | Start session mode. | Parse zero, missing, non-numeric, and extra values. | Each command is invalid before page work. | partial: parser test passed, 2026-08-31 |

Not checkable yet:

- Screen size and device emulation do not exist.
- Stylesheet media queries do not exist.
- Resize events and JavaScript observation do not exist.
- Remote wire dimension limits do not exist.
