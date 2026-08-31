# Verification: screenshot capture

Run these checks against a controlled loopback page. Record its source and the browser.jr commit.

## inspection/capture-screenshot.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SCREENSHOT-01 | P1 | pipe | Viewport capture writes valid PNG bytes. | Serve solid background boxes. | Open, size the viewport, then run `screenshot <path.png>`. | The file has PNG signature and viewport dimensions. | partial: compiled-process fixture passed, 2026-08-31 |
| SCREENSHOT-02 | P1 | pipe | Full-page capture uses the document extent. | Serve one box taller than the viewport. | Run `screenshot --full <path.png>`. | Reported height equals the supported document height. | partial: package and compiled-process fixtures passed, 2026-08-31 |
| SCREENSHOT-03 | P1 | pipe | Locator capture scrolls and clips to one strict box. | Place one colored target below the viewport. | Run `screenshot <selector> <path.png>`. | Page scroll changes. Image dimensions and pixels match the target. | partial: package and compiled-process fixtures passed, 2026-08-31 |
| SCREENSHOT-04 | P1 | pipe | Unsupported paint cannot produce a successful file. | Serve visible text inside a supported box. | Request a viewport screenshot. | The command reports unsupported text paint and writes no new file. | partial: package and compiled-process fixtures passed, 2026-08-31 |
| SCREENSHOT-05 | P1 | pipe | Oversized captures fail before allocation. | Prepare a scene above the pixel limit. | Render it through the software rasterizer. | Rendering returns the documented limit error. | partial: unit fixture passed, 2026-08-31 |
| SCREENSHOT-06 | P2 | pipe | Alpha fills use source-over compositing. | Prepare overlapping blue and half-alpha red fills. | Rasterize and inspect RGBA bytes. | The overlap is `[128, 0, 127, 255]`. | partial: unit fixture passed, 2026-08-31 |
| SCREENSHOT-07 | P1 | pipe | Excessive overdraw fails before allocation. | Prepare five full 4096-square fills. | Render through the software rasterizer. | Rendering reports the clipped-paint limit. | partial: unit fixture passed, 2026-08-31 |

Not checkable by hand yet:

- Text, images, native controls, complex effects, and stylesheet paint remain unsupported.
- The planned external renderer helper does not exist.
- TTY cancellation and concurrent same-path writes remain unverified.
