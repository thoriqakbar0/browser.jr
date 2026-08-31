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

## Project notes

- [`architecture.md`](architecture.md) describes the proposed engine design.
- [`glossary.md`](glossary.md) defines shared terms.
- [`goal.md`](goal.md) explains how to maintain the product description.
- [`verification/`](verification/README.md) holds runtime checklists.
- [`bug-triage.md`](bug-triage.md) records confirmed conflicts.

Evidence date: 31 August 2026.
