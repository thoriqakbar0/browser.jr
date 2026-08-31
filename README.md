# browser.jr

A small browser for agents, built in Rust.

browser.jr is a proof of concept. It explores how agents can inspect a page, control it, and verify their work without a full consumer browser.

Playwright and `agent-browser` compatibility are goals. Neither protocol is complete today.

## Try it

Start a local web server, then run:

```sh
./browser.jr lint http://localhost:3000
```

Inspect interactive elements:

```sh
./browser.jr snapshot http://localhost:3000 --interactive
```

Pipe one interactive snapshot as JSON:

```sh
./browser.jr --json snapshot http://localhost:3000 --interactive
```

Keep one page alive across commands:

```sh
printf 'open http://localhost:3000\nsnapshot --interactive\nget title\nexit\n' \
  | ./browser.jr session
```

## What works

- bounded loopback HTTP loading
- static HTML parsing
- a small inline CSS and horizontal layout subset
- role, text, label, attribute, CSS, and XPath locators
- direct CSS and XPath targets for implemented session actions, reads, and counts
- text input, checkbox, native single-select, multiple-select, and same-context link actions
- URL, title, HTML, text, value, attribute, enabled, and visibility reads
- whole-page and CSS-scoped interactive snapshots with a tested accessibility subset
- one-shot snapshot JSON with structured success and failure envelopes
- two deterministic layout checks
- transactional incremental layout for `x` and `width`
- validated screenshot regions, paint commands, RGBA output, and lazy raster-process activation

## What does not work

- JavaScript
- DOM events
- a complete DOM, CSS cascade, or layout engine
- page paint-list construction, screenshot output, or compositing
- Playwright or `agent-browser` protocol compatibility
- remote websites or HTTPS
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

Evidence date: 31 August 2026.
