# browser.jr

A browser engine package for programmable interface verification.

browser.jr loads a local page, computes supported layout evidence, and applies deterministic design rules. The CLI and Rust package share one session model.

## Current status

Evidence date: 31 August 2026.

The Rust implementation loads loopback HTTP pages with a one MiB body cap. It parses static HTML and a stated inline-CSS subset. Parent-aware block flow and fixed pixel geometry support horizontal-overflow lint.

One project rule is available. `--max-width <element> <css-px>` checks an explicit width limit without inventing a design preference. Missing or unsupported evidence blocks the affected rule.

Package layout requests can apply `x` and `width` changes as one transactional batch. The kernel recomputes dirty `x`, `width`, and `right` fields in dependency order. Automated differential tests compare each incremental result with a clean layout.

`snapshot <url> --interactive` reports a stated native HTML and ARIA role subset. It assigns ordered references for agent use.

Package and session callers can resolve one supported semantic role locator without an earlier snapshot. The current subset includes interactive controls, headings, lists, landmarks, and common document structure. Resolution supports optional accessible-name matching and rejects ambiguous targets.

Role resolution returns the semantic identifier, role, accessible name, and normalized descendant text. It does not capture a snapshot or change interactive references.

Session and package callers can compose role locators with click, fill, check, uncheck, hover, and text operations. Role commands default to click.

Direct fill and checked-state actions mutate supported controls without a snapshot. Direct link clicks navigate after supported visibility and enabled checks.

Hover and unsupported click targets return typed errors. Missing visibility evidence blocks the action instead of reporting success.

Package sessions can click a current link reference. Same-context links navigate through the loopback loader and stale old references.

Package sessions can fill supported text inputs and textareas. Callers can read the current value directly or through a later snapshot.

Package sessions can check, uncheck, and inspect native checkboxes. Snapshots report their current Boolean state.

Package sessions can select exact values on native single selects. Snapshots and direct reads report the selected value.

Package sessions can also read the installed URL and parsed page title.

Package sessions can read normalized descendant text from a current interactive reference.

Package sessions can read static attributes while blocking password input values.

Package sessions can inspect the enabled state of supported native elements.

Package sessions can inspect supported static visibility. Missing style or box evidence blocks the read.

Package sessions can reload the current URL. Success installs a fresh document and stale references.

`session` reads page, role-locator, snapshot, link, text, select, checkbox, visibility, URL, and title commands from stdin. It preserves typed identity behind each `@eN` label.

Grid inspection, user-agent comparison, watch mode, JavaScript execution, machine-readable output, and the REPL remain unimplemented. Numeric budgets remain open.

## Quick start

Start the page's development server, then run:

```sh
./browser.jr lint http://localhost:3000
```

Set a viewport and a project width limit when the page needs them:

```sh
./browser.jr lint http://localhost:3000 \
  --viewport 1280 \
  --max-width content 720
```

Capture the supported interactive semantic elements:

```sh
./browser.jr snapshot http://localhost:3000 --interactive
```

Keep the page and references alive across commands:

```sh
printf 'open http://localhost:3000\nget url\nget title\nsnapshot --interactive\nfill @e1 hello\nget value @e1\nexit\n' | ./browser.jr session
```

Read one supported role without capturing first:

```sh
printf 'open http://localhost:3000\nfind role heading text --name settings\nexit\n' | ./browser.jr session
```

Act through the same current-document locator:

```sh
printf 'open http://localhost:3000\nfind role textbox fill hello --name Email\nfind role checkbox check --name Terms\nexit\n' | ./browser.jr session
```

The element argument is a semantic element identifier. An HTML `id` supplies that identifier when present.

## Purpose

browser.jr checks rendered evidence for measurable interface defects.

The engine is a new implementation. It is not defined as a wrapper around an existing browser.

The primary experience is `browser.jr lint <url>` against a running development server. Package callers use the same typed observations.

This repository owns the product description and its implementation. Product documents distinguish current evidence, decided behavior, and open questions.

### What this is not

- It is not a general consumer browser. Tabs, bookmarks, accounts, extensions, and browsing history are outside the first scope.
- It is not API reference documentation. Exported package types own implemented API details.
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

## Managing the product description

Each product fact has one owner. Other documents link to that owner instead of copying the same rule.

| Fact | Owner |
| --- | --- |
| Product identity, scope, current status, and coverage | This `README.md` |
| Observable behavior for one feature | The matching feature document |
| Shared terms | [`glossary.md`](glossary.md) |
| Internal ownership and data flow | [`architecture.md`](architecture.md) |
| Writing order and evidence rules | [`goal.md`](goal.md) |
| Runtime proof | [`verification/`](verification/README.md) |
| Confirmed conflicts and defects | [`bug-triage.md`](bug-triage.md) |

Use these evidence labels consistently:

- `implemented` means code and an automated test support the claim.
- `verified` means the running product passed the important hand checks.
- `decided` means the product owner defined behavior that code may not support yet.
- `open` means no product decision or sufficient evidence exists.

These labels describe behavior claims. The coverage table uses `not started`, `drafted`, and `verified` to describe document completion.

When behavior changes:

1. Update the document that owns the behavior.
2. Update `glossary.md` when the change adds or changes a term.
3. Update this README when scope, current status, structure, or coverage changes.
4. Add or update the matching verification claim.
5. Record conflicting runtime evidence in `bug-triage.md`.
6. Change a status to `verified` only after the required hand checks pass or reach triage.

Runtime checks, automated tests, and code provide evidence for current behavior. An explicit product decision defines intended behavior. Record any conflict in `bug-triage.md`. Label intended but unimplemented behavior instead of describing it as current.

## Product document plan

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

Drafting reads code and tests. Verification watches the running engine. The `verification/` directory holds one observable claim per checklist row.

A document becomes `drafted` after code evidence or an explicit product decision defines its behavior. It becomes `verified` only after all important checklist items pass or have a recorded triage decision.

`bug-triage.md` records behavior that conflicts with a product decision or runtime evidence. Missing implementation belongs in the coverage table, not bug triage.

### Order of work

1. **Pilot: show CLI help.** Settle invocation, output, errors, and interruption behavior without loading a page.
2. **Foundations.** Define evaluations, sessions, pages, documents, snapshots, user-agent profiles, and checks.
3. **Core workflow.** Load a page, inspect layout, and evaluate a check.
4. **Remaining behavior.** Add automation, lifecycle, diagnostics, compatibility boundaries, and verification checklists.

### Scope decisions

These decisions define intended scope. The [current status](#current-status) section records implementation evidence.

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
README.md                              product entry point, status, scope, and document ownership
architecture.md                        proposed internal architecture and rationale
goal.md                                product-description workflow and evidence rules
AGENTS.md                              agent entry point
glossary.md                            shared vocabulary
bug-triage.md                          confirmed conflicts and defects
Cargo.toml                             Rust package and binary configuration
.debtmap.toml                          Rust technical-debt analysis settings
.githooks/pre-commit                   tracked Debtmap commit gate
browser.jr                             repository command wrapper

src/
  lib.rs                               package boundary
  main.rs                              binary entry point
  cli.rs                               argument parsing and local exits
  cli_session.rs                       persistent stdin command adapter
  loading.rs                           loopback HTTP page loading with a body cap
  page.rs                              HTML parsing, ancestry, and horizontal box extraction
  page/interactive.rs                  roles, names, action metadata, and control state
  page/visibility.rs                   supported static visibility evidence
  session.rs                           typed page, action, snapshot, and rule requests
  layout.rs                            field program, clean layout, and transactional invalidation
  locator.rs                           typed role and accessible-name matching
  non_empty.rs                         non-empty evidence and result collections
  snapshot.rs                          immutable layout and interactive evidence
  rules.rs                             built-in overflow and project width evaluation

tests/
  cli.rs                               compiled-process behavior
  package.rs                           public page, action, snapshot, and rule behavior

verification/
  README.md                            hand-verification protocol
  design-lint.md                       checks for the primary lint workflow
  capture-snapshot.md                  checks for interactive semantic snapshots
  query-elements.md                    checks for semantic role locator resolution
  navigation.md                        checks for package link navigation
  ai-session.md                        checks for persistent CLI action state
  fill-text.md                         checks for package and CLI fill behavior
  read-value.md                        checks for current supported-control value inspection
  select-option.md                     checks for native single-select behavior
  check-state.md                       checks for native checkbox state
  read-text.md                         checks for descendant text inspection
  read-attribute.md                    checks for static attribute inspection
  read-enabled.md                      checks for native enabled-state inspection
  read-visible.md                      checks for supported static visibility
  reload-page.md                       checks for current-page reload behavior
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
  reload-page.md                       replacing the current document from its URL
  network-control.md                   requests, failures, timeouts, and deterministic inputs

interaction/
  fill-text.md                         replacing supported text-control values
  select-option.md                     selecting one exact native option value
  set-checked.md                       replacing native checkbox state

inspection/
  query-elements.md                    finding rendered elements
  read-value.md                        reading a current supported-control value
  read-checked.md                      reading native checkbox state
  read-text.md                         reading normalized descendant text
  read-attribute.md                    reading static source attributes
  read-enabled.md                      reading supported native disabled state
  read-visible.md                      reading supported static visible state
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

## Reference

The source repository is this repository at `/Users/thor/work/browser.jr`. It contains the early implementation described below.

- `README.md`: product scope, interaction shape, and behavior documents
- `architecture.md`: proposed ownership, data model, module boundaries, and verification order
- `glossary.md`: current vocabulary and unresolved terms
- `goal.md`: evidence rules and reading order

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

The hook checks Debtmap configuration, Rust formatting, Clippy warnings, and tests. It rejects debt density above 30 per 1,000 lines.
