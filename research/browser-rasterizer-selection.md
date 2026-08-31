# Browser rasterizer selection

Research date: 31 August 2026

Source state: repository `HEAD` commits fetched on the research date. Skia `24a8072e`, rust-skia `5f5c96d6`, WebRender mirror `e1c924eb`, Servo `f62228b1`, Vello `3a9220a1`, tiny-skia `5d475477`, and Blend2D `6dbc2cef`.

## Decision

There is no evidenced "fastest rasterizer in the world." The published tests use different scenes, hardware, backends, compilers, fonts, and correctness limits. They cannot produce a universal ranking.

A rasterizer alone also cannot render the web. browser.jr needs three separate parts:

1. Paint converts styled fragments into ordered drawing commands, clipping, effects, and stacking contexts.
2. Rasterization converts those commands into pixels or GPU work.
3. Compositing combines retained results, applies transforms and opacity, and presents the page.

browser.jr's selected direction is an on-demand screenshot process. Normal inspection stays pixel-free. The main process builds browser.jr-owned paint commands only for an explicit viewport, element, or rectangle capture. A separate helper loads the selected renderer, returns pixels, and can exit to release its memory.

Study **Skia through rust-skia as the first complete screenshot helper**. Start with its CPU backend because occasional screenshots do not need a persistent GPU, shader compilation, or GPU readback. Skia has mature text, image, filter, color, and software-raster support. Its C++ and build cost stays outside the main browser.jr process. [Skia documentation](https://skia.org/docs/), [Skia text overview](https://docs.skia.org/docs/dev/design/text_overview/), and [rust-skia](https://github.com/rust-skia/rust-skia).

Keep **GPUI as a lighter subset experiment**. Its scene and headless renderer may produce early pixels for rectangles, paths, text sprites, images, and shadows. It does not supply browser paint semantics or Skia's breadth. A shared browser.jr paint suite must block unsupported scenes instead of producing incomplete screenshots. [GPUI scene](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/scene.rs) and [GPUI Metal renderer](https://github.com/zed-industries/zed/blob/main/crates/gpui_apple/src/metal_renderer.rs).

Keep **WebRender for a future continuous-rendering requirement**. It remains the closest Rust browser renderer, but retained GPU compositing and responsive scrolling add machinery that occasional screenshot capture does not need. WebRender remains tightly coupled to Servo and Gecko, and its canonical upstream lives inside `mozilla-central`. Servo tracks renderer abstraction as unfinished work. [WebRender repository](https://github.com/servo/webrender) and [Servo renderer alternatives issue](https://github.com/servo/servo/issues/37149).

Keep **Vello as a measured future path**, not today's foundation. Its Rust and `wgpu` design fits browser.jr, but Vello still calls its GPU renderer alpha and lists unfinished filters, allocation, artifacts, and glyph caching. [Vello repository](https://github.com/linebender/vello).

## Candidate comparison

| Candidate | Correct layer and capabilities | Browser evidence | Rust and safety boundary | Performance evidence | Fit now |
| --- | --- | --- | --- | --- | --- |
| WebRender | Browser display lists, GPU rasterization, glyph atlas work, retained tiles, spatial and clip trees, and compositing. It does not build CSS paint commands for browser.jr. | Firefox and Servo use it. Servo's current path already joins layout, display-list construction, and the compositor. | Rust core. GPU APIs, shaders, drivers, fonts, and embedding code remain trusted boundaries. | The source has detailed stage profiling. No current cross-candidate result proves it universally fastest. | Defer until continuous rendering needs retained GPU compositing. |
| Skia via rust-skia | Immediate canvas drawing, recorded `SkPicture` commands, CPU raster, several GPU backends, text shaping, images, filters, and color. browser.jr must still implement web paint and its compositor policy. | Skia powers Chrome, Android, Flutter, and other production software. Its test corpus includes captured workloads. | Safe Rust wrapper over a large C++ implementation and generated bindings. This does not preserve a pure-Rust trusted core. | Skia runs more than 400,000 Perf measurements per commit. Results depend on build and backend. Official docs warn that non-Clang software builds can be much slower. | First complete on-demand screenshot-helper study. |
| GPUI | Retained UI scenes with quads, paths, glyph and image sprites, shadows, surfaces, platform GPU renderers, and headless image output. It does not define browser paint semantics. | Zed uses it for an application interface, not web content. | Rust with platform graphics, font, shader, and driver boundaries. | Production use shows its UI path works, but no comparable browser-scene benchmark exists. | Lightweight subset experiment before a completeness decision. |
| Vello GPU | A GPU compute 2D scene renderer using `wgpu`. It provides shapes, images, gradients, text integration, and layers, not a browser paint or compositor model. | Used by Xilem, not a production browser. Upstream calls it alpha. | Rust narrows the application boundary. `wgpu`, native graphics APIs, shaders, and drivers remain boundaries. | Upstream reports 177 fps for `paris-30k` on an M1 Max at 1600 squared. It calls this a best case and says formal benchmarks are pending. | Future experiment only. |
| Vello CPU | Pure-Rust CPU scene renderer with a related imaging model. It does not provide browser display-list or compositor policy. | No production-browser path found. | Rust implementation, with dependency and platform font boundaries still relevant. | Upstream calls performance competitive, but its linked chart is not time-locked. | Include in the CPU bake-off. API stability is absent. |
| tiny-skia | Minimal CPU fills, strokes, clipping, gradients, patterns, and blending. No text, GPU, resource cache, ICC color, or compositor. | Upstream explicitly says it is not a Skia replacement. | Mostly safe Rust. Upstream limits `unsafe` to checked SIMD intrinsics and `bytemuck::Pod`. | Its historical suite reports slower results than Skia on tested x86 and ARM machines. It used Skia 90 and Rust 1.62. [Benchmark method](https://github.com/linebender/tiny-skia/blob/main/benches/README.md). | Useful small reference or fallback for supported primitives. Insufficient alone. |
| Blend2D | CPU analytic rasterizer, text, images, gradients, blend modes, JIT SIMD pipelines, and experimental command-batched multithreading. No browser compositor. | No production-browser integration found in the official project. | C++ through a C API. Rust needs an unsafe FFI wrapper. Runtime code generation adds a security and deployment boundary. | First-party benchmarks make it a serious CPU challenger, but they do not compare whole browser pipelines. [Blend2D](https://blend2d.com/) and [multithreaded rendering](https://blend2d.com/doc/multithreaded-rendering.html). | Benchmark if CPU screenshots matter. Do not adopt before the bake-off. |

Pathfinder remains useful prior art, but its official repository calls it incomplete and under heavy development. It has weaker current adoption evidence than WebRender, Skia, or Vello. [Pathfinder repository](https://github.com/servo/pathfinder).

## Text, determinism, and safety

Text changes the choice. Skia has shaping and browser font matching support. WebRender rasterizes glyphs into an atlas, but the embedding browser still supplies fonts and shaped runs. tiny-skia cannot draw text. Vello's glyph cache remains unfinished. Blend2D draws glyph runs, but browser.jr still needs shaping, fallback, line breaking, and CSS font matching.

Do not promise identical pixels across operating systems or GPU backends. WebRender uses OSMesa for consistent tests and still warns that font libraries can change output. Skia maintains platform result triage because backends and platforms differ. [WebRender testing note](https://github.com/servo/webrender) and [Skia testing](https://skia.org/docs/dev/testing/).

Memory-safe Rust does not make a renderer dependency fully safe. rust-skia crosses into C++. WebRender and Vello cross into graphics APIs, shaders, drivers, and font libraries. browser.jr should isolate the renderer behind owned scene data, validate sizes and resource identifiers, cap allocations, and avoid renderer pointers in its document or layout model.

## Evaluation plan

Build one browser.jr-owned paint list before selecting a backend. Start with solid rectangles, borders, clips, images, glyph runs, opacity groups, transforms, and stacking order. Give every command a stable fragment identifier. Write a GPUI subset adapter and a Skia CPU adapter first. Add WebRender, Vello CPU, Vello GPU, and Blend2D only after the same correctness fixtures run or a continuous-rendering requirement appears.

Use one captured scene suite:

- small forms and text-heavy documents
- deep clips and stacking contexts
- large images, gradients, shadows, transforms, and opacity
- scrolling and small damaged regions
- cold fonts, warm glyph caches, and repeated frames
- adversarial path counts and allocation limits

Pin the scene, output size, scale factor, fonts, color space, antialiasing mode, compiler, SIMD flags, thread count, backend, device, driver, and cache state. Compare output against per-platform references with an explicit tolerance.

Measure paint-list construction, scene submission, CPU raster, GPU work, upload, compositing, and readback separately. Record p50, p95, and p99 frame time, peak memory, bytes uploaded, damaged pixels, first-frame time, and warm-frame time. A screenshot workflow must include readback cost. An interactive workflow should not.

The first gate is correctness, not speed. Require browser stacking, clipping, transforms, opacity, and text to pass before ranking a screenshot helper. Use GPUI only for the scenes it reports as supported. Select Skia CPU when the complete suite passes within the screenshot budget.

## What remains unknown

- Which first screenshot scenes GPUI can render without weakening browser paint semantics.
- Whether rust-skia's build and C++ boundary fit browser.jr's distribution and trusted-code goals.
- Whether browser.jr needs deterministic software screenshots before it needs interactive GPU presentation.
- Whether Vello reaches browser-required filter, text-cache, and robustness maturity during browser.jr's paint implementation.

These are implementation questions. The proposed adapters and shared scene suite answer them without locking browser.jr to one renderer first.
