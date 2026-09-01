# browser.jr

A small browser for agents, built in Rust.

browser.jr is a proof of concept. It explores how agents can inspect a page, control it, and verify their work without a full consumer browser.

Playwright and `agent-browser` compatibility are goals. Neither protocol is complete today.

## Try it

Open a public or local web page:

```sh
./browser.jr lint https://example.com
```

Inspect the supported accessibility tree:

```sh
./browser.jr --allow-loopback snapshot http://localhost:3000
```

Project the tree to agent-oriented reference targets:

```sh
./browser.jr --allow-loopback snapshot http://localhost:3000 --interactive
```

Pipe one full snapshot as JSON:

```sh
./browser.jr --json snapshot http://localhost:3000
```

Keep one page alive across commands:

```sh
printf 'open http://localhost:3000\nsnapshot\nget title\nexit\n' \
  | ./browser.jr --allow-loopback session
```

Stream identified JSON results for the same commands:

```sh
printf 'open http://localhost:3000\nget title\nexit\n' \
  | ./browser.jr --allow-loopback --json session
```

Capture its current viewport as a PNG:

```sh
printf 'open http://localhost:3000\nscreenshot page.png\nexit\n' \
  | ./browser.jr --allow-loopback session
```

## What works

- bounded public HTTP and HTTPS loading, with explicit loopback access and private-network blocking
- static HTML parsing with normalized ancestry
- a small inline and embedded CSS cascade with horizontal and static block-flow layout subsets
- role locators with current HTML roles, non-presentational descendant image `alt` text, description, state, level, and accessibility-hidden filters
- text, label, attribute, CSS, and XPath locators
- direct CSS and XPath targets for implemented session actions, reads, and counts
- text fill, append, focused keyboard insertion, held-key modifiers, phase-correct native `Space`, sequential focus, bounded caret editing, native activation, link navigation, and visible-target hover state
- a data-minimized native event transcript for supported text, keyboard, select, pointer, mouse, focus, click, check, and uncheck action phases
- bounded same-context native GET form submission through submitters and implicit text-control `Enter`
- native checkbox and radio checked-state reads, writes, snapshots, and exclusive radio groups
- back and forward navigation through bounded same-context history
- URL, title, HTML, text, value, attribute, checked, editable, enabled, focused, hovered, and visibility reads
- complete viewport-relative boxes for supported fixed and static block geometry
- bounded page scrolling, explicit scroll-into-view, and supported-box auto-scroll for local click, hover, check, and uncheck
- supported static stability checks for click, hover, and changed checked-state actions
- static and fixed action-point blocker checks for supported click, hover, and changed checked-state scenes
- runtime viewport sizing with state-preserving static page reflow
- normalized whole-page static text through package, one-shot CLI, and session mode
- whole-page and CSS-scoped accessibility-tree snapshots with ordered text, native list markers, compact pruning, depth limits, and resolved link URLs
- agent-oriented snapshots with controls, links, headings, navigation landmarks, nested references, and document-wide scoped labels
- one-shot snapshot JSON with structured success and failure envelopes
- line-oriented session JSON with lifecycle events and command sequence identifiers
- two deterministic layout checks
- transactional incremental layout for `x` and `width`
- viewport, full-page, and strict-locator screenshots through session mode
- bounded solid-box paint lists, source-over RGBA compositing, and PNG output
- lazy software-rasterizer activation with image and clipped-paint work limits

## What does not work

- JavaScript
- DOM event delivery to page scripts, complete ancestor dispatch, or complete pointer, focus, and keyboard event metadata
- a complete DOM, CSS cascade, or layout engine
- complete stacking, clipping, transformed geometry, or dynamic receives-events hit testing
- text, image, native-control, stylesheet, clip, effect, or complete browser paint
- a separate screenshot helper process or complete browser compositor
- Playwright or `agent-browser` protocol compatibility
- a browser window

Unsupported evidence blocks a check. browser.jr does not turn missing evidence into a pass.

## Safety

The project targets memory safety. Its Rust source has no `unsafe` blocks today.

The crate does not yet use `#![forbid(unsafe_code)]`. Dependencies keep their own safety boundaries.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Run the cross-engine browser-control benchmark separately:

```sh
cd benchmarks
pnpm install --frozen-lockfile
pnpm browsers:install
pnpm bench
```

## Benchmark snapshot

This benchmark compares browser.jr with six other browser-control paths over the same local two-page fixture:

- browser.jr through one persistent release CLI session
- Chrome, Firefox, and WebKit through Playwright
- Lightpanda through Puppeteer over CDP
- agent-browser through its CLI and daemon, using either Chrome or Lightpanda

Each adapter starts once and reuses one warm browser session. Scenario setup runs before the timer.

The full workflow includes its initial navigation. Browser startup, browser.jr compilation, and correctness checks stay outside the timer.

The harness discards one warmup, then records 10 samples. It sorts them and reports the fifth value as the median.

Every supported sample must return its expected title, snapshot content, or control value. Unsupported scenarios receive no timing.

A clean Apple M3 run produced these full-workflow medians:

| Adapter | Full-workflow median |
| --- | ---: |
| browser.jr | 22.55 ms |
| Chrome | 375.36 ms |
| Firefox | 643.46 ms |
| WebKit | 681.95 ms |
| Lightpanda | 492.45 ms |
| agent-browser with Chrome | 7,366.00 ms |
| agent-browser with Lightpanda | 1,693.90 ms |

The timed workflow opens the fixture, snapshots its controls, fills text, checks a checkbox, and selects `blue`.

It then reads all three values, follows a link, and reads the destination title.

browser.jr ran this workflow within its current static HTML boundary. It did not run the JavaScript evaluation or screenshot scenarios.

These numbers measure browser-control latency, not rendering-engine speed or browser compatibility. agent-browser timings include its CLI and daemon overhead.

[Read the benchmark method](benchmarks/README.md) for every scenario and fairness rule.

[Read the complete result](benchmarks/results/README.md) for p95 values, scenario tables, runtime versions, and comparison limits.

## Project notes

- [`architecture.md`](architecture.md) describes the proposed engine design.
- [`glossary.md`](glossary.md) defines shared terms.
- [`goal.md`](goal.md) explains how to maintain the product description.
- [`verification/`](verification/README.md) holds runtime checklists.
- [`bug-triage.md`](bug-triage.md) records confirmed conflicts.
- [`benchmarks/README.md`](benchmarks/README.md) defines cross-engine correctness and latency checks.

Evidence date: 1 September 2026.

## Coverage

`drafted` means code or a product decision supports the document. `verified` requires a recorded runtime check.

| Document | Status |
| --- | --- |
| `architecture.md` | drafted |
| `glossary.md` | drafted |
| `bug-triage.md` | drafted |
| `verification/README.md` | drafted |
| `verification/design-lint.md` | drafted |
| `verification/capture-snapshot.md` | drafted |
| `verification/capture-screenshot.md` | drafted |
| `verification/query-elements.md` | drafted |
| `verification/navigation.md` | drafted |
| `verification/history-navigation.md` | drafted |
| `verification/ai-session.md` | drafted |
| `verification/fill-text.md` | drafted |
| `verification/type-text.md` | drafted |
| `verification/focus-element.md` | drafted |
| `verification/hover-element.md` | drafted |
| `verification/click-element.md` | drafted |
| `verification/press-key.md` | drafted |
| `verification/submit-form.md` | drafted |
| `verification/read-value.md` | drafted |
| `verification/select-option.md` | drafted |
| `verification/check-state.md` | drafted |
| `verification/read-text.md` | drafted |
| `verification/read-page.md` | drafted |
| `verification/read-attribute.md` | drafted |
| `verification/read-html.md` | drafted |
| `verification/read-enabled.md` | drafted |
| `verification/read-editable.md` | drafted |
| `verification/read-focused.md` | drafted |
| `verification/read-hovered.md` | drafted |
| `verification/read-visible.md` | drafted |
| `verification/inspect-layout.md` | drafted |
| `verification/scroll-page.md` | drafted |
| `verification/set-viewport.md` | drafted |
| `verification/reload-page.md` | drafted |
| `verification/` remaining checklists | not started |
| `foundations/evaluation.md` | not started |
| `foundations/session.md` | not started |
| `foundations/page-and-document.md` | not started |
| `foundations/snapshot.md` | not started |
| `foundations/user-agent-profile.md` | not started |
| `foundations/check.md` | not started |
| `cli/help.md` | drafted |
| `repl/simple-expression.md` | not started |
| `repl/help-and-discovery.md` | not started |
| `repl/output.md` | not started |
| `loading/open-page.md` | drafted |
| `loading/navigation.md` | drafted |
| `loading/history-navigation.md` | drafted |
| `loading/reload-page.md` | drafted |
| `loading/network-control.md` | drafted |
| `interaction/fill-text.md` | drafted |
| `interaction/type-text.md` | drafted |
| `interaction/focus-element.md` | drafted |
| `interaction/hover-element.md` | drafted |
| `interaction/scroll-page.md` | drafted |
| `interaction/set-viewport.md` | drafted |
| `interaction/click-element.md` | drafted |
| `interaction/press-key.md` | drafted |
| `interaction/submit-form.md` | drafted |
| `interaction/select-option.md` | drafted |
| `interaction/set-checked.md` | drafted |
| `inspection/query-elements.md` | drafted |
| `inspection/read-value.md` | drafted |
| `inspection/read-checked.md` | drafted |
| `inspection/read-text.md` | drafted |
| `inspection/read-page.md` | drafted |
| `inspection/read-attribute.md` | drafted |
| `inspection/read-html.md` | drafted |
| `inspection/read-enabled.md` | drafted |
| `inspection/read-editable.md` | drafted |
| `inspection/read-focused.md` | drafted |
| `inspection/read-hovered.md` | drafted |
| `inspection/read-visible.md` | drafted |
| `inspection/inspect-layout.md` | drafted |
| `inspection/inspect-grid.md` | not started |
| `inspection/compare-user-agents.md` | not started |
| `inspection/capture-snapshot.md` | drafted |
| `inspection/capture-screenshot.md` | drafted |
| `verification-features/design-lint.md` | drafted |
| `verification-features/evaluate-check.md` | not started |
| `verification-features/diagnostics.md` | not started |
| `verification-features/batch-checks.md` | not started |
| `automation/package-session.md` | not started |
| `automation/ai-session.md` | drafted |
| `automation/reproducible-script.md` | not started |
| `cross-cutting/determinism.md` | not started |
| `cross-cutting/resource-limits.md` | not started |
| `cross-cutting/isolation.md` | not started |
| `cross-cutting/compatibility.md` | not started |
| `cross-cutting/accessibility-inspection.md` | not started |
