# Glossary

The vocabulary used across these documents. Each definition states intended meaning. Implementation evidence may refine it.

## The engine

**browser.jr.** The product and package described by this repository. It is a new browser engine for programmable interface verification.

**Engine.** The code that loads a page, interprets supported web content, computes rendered state, and exposes that state for inspection.

**REPL.** A decided but unimplemented read-evaluate-print loop. Its intended job is to investigate findings and run custom checks in one session.

**Caller.** The person, AI agent, or program that submits an evaluation. Caller identity must not change evaluation semantics.

**CLI.** The primary developer interface. `lint` checks layout. `snapshot --interactive` reports supported semantic controls.

## Sessions and pages

**Session.** One isolated engine instance with its pages, configuration, and evaluation history. Whether a session may contain more than one page is open.

**Page.** A loaded browsing context within a session. A page owns its current document and user-agent profile.

**Document.** The current parsed and rendered content of a page. Navigation may replace it.

**Navigation.** A request that may replace a page's current document. Redirect behavior and commit boundaries remain open.

## Rendering and evidence

**Rendered state.** The engine's inspectable result after applying supported parsing, style, layout, and rendering rules.

**Layout observation.** A structured value describing geometry or computed layout for one rendered element.

**Grid observation.** A layout observation for a CSS grid. It may include tracks, gaps, placement, alignment, and overflow once support is defined.

**Snapshot.** An immutable capture of selected page evidence at a known point in the page's lifetime.

**Interactive snapshot.** A snapshot containing the supported interactive roles, names, semantic identifiers, and ordered references.

**Interactive element reference.** A session-owned target identity such as `@e1`. Repeated captures preserve it until another document opens.

**Evidence.** The structured observations and diagnostics that support a check result.

## Verification

**Design lint.** A lint run over rendered page data. It finds measurable defects without claiming whether a design is tasteful.

**Lint run.** One invocation that loads a target, computes rendered state, applies rules, and reports findings.

**Finding.** One rule result that needs attention. It identifies the rule, severity, target, viewport, expectation, observed value, and supporting evidence.

**Rule.** A deterministic comparison between rendered evidence and an expectation. Built-in rules cover general defects. Project rules express local design decisions.

**Target matrix.** The requested combinations of page URL, viewport, and user-agent profile for one lint run.

**Check.** An evaluation that compares an observation with an explicit expectation.

**Pass.** A check result stating that the observed value meets its expectation.

**Fail.** A check result stating that the observed value differs from its expectation. A failure includes evidence when the engine can produce it.

**Blocked.** A check result stating that browser.jr could not reach a valid comparison. Unsupported behavior, a load failure, or a resource limit may block a check.

**Diagnostic.** Structured information that explains an error, blocked check, or failed check.

**Objective rule.** A rule for measurable failure such as overflow, clipping, unreadable contrast, or an undersized tap target.

**Project rule.** A rule supplied by the project. The implemented `max-element-width` rule compares one semantic element with an explicit width limit.

## Compatibility

**User-agent profile.** The declared browser identity and related observable capabilities applied to a page. The exact fields and fidelity boundary remain open.

**Supported behavior.** Web behavior backed by a conformance test and exposed without an unsupported warning.

**Unsupported behavior.** Web behavior outside the engine's stated compatibility boundary. The engine reports it instead of silently claiming a valid result.

## Evaluations

These terms describe decided but unimplemented REPL behavior.

**Evaluation.** One JavaScript expression submitted to a session and its resulting value, error, or interruption.

**Enter.** The caller submits input. The REPL has enough input to attempt parsing.

**Reject immediately.** The evaluation ends before it begins running. Syntax errors and invalid session state are candidates for this phase.

**Begin evaluating.** The engine accepts the input and starts observable work.

**While evaluating.** The evaluation remains active and may produce progress, page activity, or partial observations.

**Finish.** The evaluation returns one value or one error and leaves the session in a defined state.

## Invocations

**Invocation.** One execution of the browser.jr CLI with its arguments, configuration, output, and exit status.

**Invoke.** The shell starts browser.jr. The CLI parses arguments and resolves project configuration.

**Exit immediately.** The invocation ends before loading a page because it completed locally or rejected its input.

**Begin running.** browser.jr accepts the target and starts the first observable page-loading work.

**While running.** The invocation loads pages, computes rendered state, evaluates rules, and may report progress.

**Finish the invocation.** browser.jr reports the complete result and exits, or waits for another run in watch mode.

## Events that end or interrupt an evaluation

**Cancel.** The caller requests a graceful stop, initially through one Ctrl+C in the REPL. The exact cleanup guarantee remains open.

**Force stop.** The caller repeats Ctrl+C before graceful cancellation completes. The remaining session guarantee remains open.

**Supersede.** Another evaluation starts while one is active. Whether browser.jr rejects, queues, or replaces it remains open.

**Navigation interrupt.** The page navigates while an evaluation depends on its current document. The result must identify whether it used the old document, the new document, or stopped.

**Environment failure.** Required network, input, output, or process access becomes unavailable during an evaluation.

**Document change.** Script, navigation, or another allowed actor changes inspected content during an evaluation.

## Output

**Human-readable output.** Text meant for a person. The CLI emits it now. The intended REPL will use the same result terms.

**Machine-readable output.** Decided but unimplemented structured output for programs and AI agents. The encoding remains open.

**Watch mode.** A decided but unimplemented lint invocation that checks the target again after relevant page changes settle.

**Exit status.** The process result used by shells and continuous integration. The CLI uses zero for pass and one for findings. It uses two for invalid input and three for blocked or unavailable results.
