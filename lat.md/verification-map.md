# Verification map

The repository separates product claims, architecture explanations, runtime checks, and conflict records. This page maps those document roles without copying their current results.

## Evidence chain

The README and feature documents own user-visible behavior. [[architecture]] and the focused graph pages explain implementation ownership. `verification/` records checks. `bug-triage.md` records confirmed conflicts between claims and evidence.

```text
product claim
  -> verification row
  -> test, command, or controlled probe
  -> recorded result
  -> bug triage when evidence conflicts
```

## Plugin evidence

[Agent-browser plugin verification](../verification/agent-browser-plugin.md) owns the current ABPLUGIN results. [Bug triage](../bug-triage.md) owns BJR-014, BJR-015, and BJR-016.

[[plugin-protocol]] maps the implementation behind those records. [[release-and-packaging]] maps the artifact and native dependency. This graph does not redefine the pass or fail state.

## Test locations

The plugin protocol tests live in [`plugin/test/plugin.test.mjs`](../plugin/test/plugin.test.mjs). Rust session behavior is covered by [`tests/cli.rs`](../tests/cli.rs), [`tests/package.rs`](../tests/package.rs), and module tests under `src/`.

The benchmark runner tests live in [`benchmarks/test`](../benchmarks/test). The full matrix remains a separate runtime check because unit tests do not execute every installed browser adapter.

## Validation commands

Run these checks after changing the graph or plugin documentation:

```sh
lat check
npm run check:versions
npm test
cd benchmarks && pnpm test
```

Run Cargo formatting, Clippy, and all-target tests when source-linked Rust behavior changes.

## lat limitation

`lat` 0.12.2 validates Rust symbol links but rejects `.mjs` symbol anchors. The graph uses normal file links for JavaScript modules until the tool supports `.mjs` symbols.
