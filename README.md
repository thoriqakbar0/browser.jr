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
| `verification/query-elements.md` | drafted |
| `verification/navigation.md` | drafted |
| `verification/ai-session.md` | drafted |
| `verification/fill-text.md` | drafted |
| `verification/read-value.md` | drafted |
| `verification/select-option.md` | drafted |
| `verification/check-state.md` | drafted |
| `verification/read-text.md` | drafted |
| `verification/read-attribute.md` | drafted |
| `verification/read-html.md` | drafted |
| `verification/read-enabled.md` | drafted |
| `verification/read-visible.md` | drafted |
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
| `loading/reload-page.md` | drafted |
| `loading/network-control.md` | not started |
| `interaction/fill-text.md` | drafted |
| `interaction/select-option.md` | drafted |
| `interaction/set-checked.md` | drafted |
| `inspection/query-elements.md` | drafted |
| `inspection/read-value.md` | drafted |
| `inspection/read-checked.md` | drafted |
| `inspection/read-text.md` | drafted |
| `inspection/read-attribute.md` | drafted |
| `inspection/read-html.md` | drafted |
| `inspection/read-enabled.md` | drafted |
| `inspection/read-visible.md` | drafted |
| `inspection/inspect-layout.md` | not started |
| `inspection/inspect-grid.md` | not started |
| `inspection/compare-user-agents.md` | not started |
| `inspection/capture-snapshot.md` | drafted |
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
