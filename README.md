# browser.jr product description

A behavior specification for browser.jr, a small browser engine built for fast, programmable interface verification.

## Purpose

browser.jr loads a local web page and lints its rendered design. It reports measurable problems such as overflow, clipping, broken grids, unsafe tap targets, and inconsistent alignment. It can repeat the same checks across viewports and user-agent profiles.

The engine is a new implementation. It is not defined as a wrapper around an existing browser. The package and the interactive REPL use the same engine and inspection model.

The primary experience is `browser.jr lint <url>` against a running development server. Watch mode keeps the engine beside the developer's normal edit-and-refresh loop. The REPL and package expose the same observations for investigation, custom checks, and AI control.

This repository contains the intended behavior and an early Rust implementation slice. `./browser.jr help` and `./browser.jr --version` run locally. Page loading and design lint execution remain unavailable. No size budget, speed budget, or compatibility result exists yet.

### What this is not

- It is not a general consumer browser. Tabs, bookmarks, accounts, extensions, and browsing history are outside the first scope.
- It is not API reference documentation. API reference will come from the exported package types and REPL help after implementation begins.
- It is not a claim of full web-platform compatibility. Each supported feature needs a test and a stated compatibility boundary.
- It is not organized by package or module. Each behavior is described where the user meets it.

## Conventions

- Describe what the user enters, observes, and receives.
- Put implementation details in a `> Technical note:` block only when they change observable behavior.
- Use sentence case for headings.
- Use the [glossary](glossary.md) for terms such as *lint run*, *finding*, *page*, *snapshot*, *user-agent profile*, and *check*.
- Treat performance words as requirements until measurements give them numeric limits.
- End each feature document with open questions and its evidence state.
- State surprising or inconsistent behavior plainly.

## The work to be done

Each document describes one user-visible behavior. The set starts with the CLI, then defines the shared page model, design lint, inspection, and automation.

### Document template

Every feature document follows the same eight sections.

1. **Summary.** State what the behavior lets the user do and how they reach it.
2. **The simple case.** Narrate the common path without variants or failures.
3. **The interaction, event by event.** The unit is an invocation. Its phases are *invoke*, *exit immediately*, *begin running*, *while running*, and *finish*.
4. **Variants.** Use the same rows in every document: flags and options, project configuration, target matrix, and output channel.
5. **Cancel and interrupt.** Use this fixed list in this order:
   - Ctrl+C once
   - Ctrl+C again before the evaluation stops
   - the process receives SIGTERM
   - the terminal closes
   - stdin or stdout closes
   - the network fails or a request times out
   - the inspected page changes
   - another lint run targets the same page
   - the process exits outright
6. **Interactions with other systems.** Use this fixed order: configuration precedence, output and exit status, resource limits, network and storage, rendering compatibility, isolation, and accessibility inspection.
7. **Edge cases.** Cover limits, empty states, repeated input, and order-dependent results.
8. **Open questions and verification.** Separate intended behavior from evidence in code, tests, and the running product.

Every invocation gets one Mermaid `stateDiagram-v2`. The diagram includes only states the user can observe.

### Method

For each document:

1. Read the session state, evaluator, page lifecycle, and relevant engine component.
2. Read the matching behavior and conformance tests.
3. Draft the outside-in behavior.
4. Run ambiguous cases through the REPL and package boundary.
5. Record the source commit and unresolved differences.

If a behavior has no implementation, write only intended behavior supplied by the product owner. Label it as unverified. Do not invent defaults, limits, syntax, or compatibility.

### Verification

Drafting reads code and tests. Verification watches the running engine. The future `verification/` directory will hold one observable claim per checklist row.

A document becomes `drafted` after code evidence or an explicit product decision defines its behavior. It becomes `verified` only after all important checklist items pass or have a recorded triage decision.

`bug-triage.md` will collect behavior that code or runtime checks show to be inconsistent or defective. Missing implementation is planned work, not a bug.

### Order of work

1. **Pilot: show CLI help.** Settle invocation, output, errors, and interruption behavior without loading a page.
2. **Foundations.** Define evaluations, sessions, pages, documents, snapshots, user-agent profiles, and checks.
3. **Core workflow.** Load a page, inspect layout, and evaluate a check.
4. **Remaining behavior.** Add automation, lifecycle, diagnostics, compatibility boundaries, and verification checklists.

### Scope decisions

- **Primary surface.** `browser.jr lint <url>` checks a live local page. Watch mode repeats checks during development.
- **Product identity.** browser.jr is a new browser engine and package. Existing engines may inform conformance tests but do not define its architecture.
- **First job.** Design lint reports measurable rendered-page defects. Grid inspection and user-agent comparison are first-class checks.
- **AI access.** AI agents script the engine through the same session model as the REPL. A separate chat interface is outside scope.
- **Consumer browsing.** Tabs, bookmarks, accounts, extensions, saved history, and password management are outside scope.
- **Compatibility.** Full web-platform compatibility is not assumed. Supported behavior must be explicit and tested.
- **Performance.** "Small" and "fast" require numeric budgets before implementation can claim them.
- **Implementation.** The browser, layout, and lint core use Rust. An existing embedded JavaScript runtime may power the REPL.
- **Rendering boundary.** The first version computes structured layout evidence. Pixel comparison and full browser compatibility are outside scope.
- **Layout invalidation.** Watch mode uses Spineless Traversal. A clean full layout remains the correctness oracle and recovery path.
- **Interaction shape.** The unit is an invocation. Its five phases, interrupt list, variant rows, and cross-cutting order are fixed above.
- **Numbered rules.** Stable headings and checklist identifiers provide references. The prose itself is not numbered.

## Structure

```text
README.md                              this file
architecture.md                        proposed internal architecture and rationale
goal.md                                standing drafting instructions
AGENTS.md                              agent entry point
glossary.md                            shared vocabulary
bug-triage.md                          confirmed defects and product decisions
Cargo.toml                             Rust package and binary configuration
.debtmap.toml                          Rust technical-debt analysis settings
.githooks/pre-commit                   tracked Debtmap commit gate
browser.jr                             repository command wrapper

src/
  lib.rs                               package boundary
  main.rs                              binary entry point
  cli.rs                               argument parsing and local exits
  session.rs                           typed request execution
  layout.rs                            field program, clean layout, and width invalidation
  non_empty.rs                         non-empty evidence and result collections
  snapshot.rs                          immutable structured evidence
  rules.rs                             horizontal-overflow evaluation

tests/
  cli.rs                               compiled-process behavior
  package.rs                           public session and rule behavior

verification/
  README.md                            hand-verification protocol
  design-lint.md                       checks for the primary lint workflow
  foundations.md                       checks for the shared models
  loading-and-inspection.md            checks for the core workflow
  automation-and-lifecycle.md          checks for scripting and failures

foundations/
  evaluation.md                        input, output, errors, and interruption
  session.md                           engine state shared across evaluations
  page-and-document.md                 loaded pages, navigation, and document changes
  snapshot.md                          captured rendered state and its lifetime
  user-agent-profile.md                identity and capability changes applied to a page
  check.md                             pass, fail, evidence, and diagnostic results

cli/
  help.md                              pilot for invocation, discovery, and local exits

repl/
  simple-expression.md                 pilot for entering and evaluating JavaScript
  help-and-discovery.md                 discovering commands and inspectable values
  output.md                             human-readable and machine-readable results

loading/
  open-page.md                         loading a URL or supplied document
  navigation.md                        redirects and later page navigation
  network-control.md                   requests, failures, timeouts, and deterministic inputs

inspection/
  query-elements.md                    finding rendered elements
  inspect-layout.md                    geometry and computed layout values
  inspect-grid.md                      tracks, placement, gaps, and overflow
  compare-user-agents.md               rerunning observations under another profile
  capture-snapshot.md                  freezing evidence for later checks

verification-features/
  design-lint.md                       linting a live page during development
  evaluate-check.md                    comparing an observation with an expectation
  diagnostics.md                       explaining failures with relevant evidence
  batch-checks.md                       running several independent checks

automation/
  package-session.md                   controlling the engine through the package
  ai-session.md                        safe, structured control by an AI agent
  reproducible-script.md               saving and rerunning a verification sequence

cross-cutting/
  determinism.md                       stable inputs, ordering, and repeatable results
  resource-limits.md                   size, speed, memory, and execution limits
  isolation.md                         script, page, file, and process boundaries
  compatibility.md                     supported web behavior and user-agent fidelity
  accessibility-inspection.md          inspectable accessibility information
```

## Coverage

Status is one of `not started`, `drafted`, or `verified`.

| Document | Status |
| --- | --- |
| `architecture.md` | drafted |
| `glossary.md` | drafted |
| `bug-triage.md` | drafted |
| `verification/README.md` | drafted |
| `verification/design-lint.md` | drafted |
| `verification/` remaining 3 checklists | not started |
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
| `loading/open-page.md` | not started |
| `loading/navigation.md` | not started |
| `loading/network-control.md` | not started |
| `inspection/query-elements.md` | not started |
| `inspection/inspect-layout.md` | not started |
| `inspection/inspect-grid.md` | not started |
| `inspection/compare-user-agents.md` | not started |
| `inspection/capture-snapshot.md` | not started |
| `verification-features/design-lint.md` | drafted |
| `verification-features/evaluate-check.md` | not started |
| `verification-features/diagnostics.md` | not started |
| `verification-features/batch-checks.md` | not started |
| `automation/package-session.md` | not started |
| `automation/ai-session.md` | not started |
| `automation/reproducible-script.md` | not started |
| `cross-cutting/determinism.md` | not started |
| `cross-cutting/resource-limits.md` | not started |
| `cross-cutting/isolation.md` | not started |
| `cross-cutting/compatibility.md` | not started |
| `cross-cutting/accessibility-inspection.md` | not started |

## Reference

The source repository is this repository at `/Users/thor/work/browser.jr`. It contains the early implementation described below.

- `README.md`: product scope, interaction shape, and planned behavior documents
- `architecture.md`: proposed ownership, data model, module boundaries, and verification order
- `glossary.md`: current vocabulary and unresolved terms
- `goal.md`: evidence rules and future reading order

The current implementation proves CLI discovery, a synthetic clean-layout rule path, and one ordered width mutation. Page loading, CSS layout, watch mode, and compatibility remain open.

## Development

Run the Rust checks:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Inspect complexity and technical-debt signals:

```text
debtmap analyze . --format markdown --no-tui
```

Enable the tracked pre-commit hook once per checkout:

```text
git config core.hooksPath .githooks
```

The hook runs `debtmap validate . --config .debtmap.toml --format terminal` before each commit.
