# Glossary

The vocabulary used across these documents. Each definition states intended meaning. Implementation evidence may refine it.

## The engine

**browser.jr.** The product and package described by this repository. It is a new browser engine for programmable interface verification.

**Engine.** The code that loads a page, interprets supported web content, computes rendered state, and exposes that state for inspection.

**REPL.** A decided but unimplemented read-evaluate-print loop. Its intended job is to investigate findings and run custom checks in one session.

**Caller.** The person, AI agent, or program that submits an evaluation. Caller identity must not change evaluation semantics.

**CLI.** The primary developer interface. `lint` checks layout. `snapshot --interactive` reports supported semantic controls. `session` accepts persistent line commands.

## Sessions and pages

**Session.** One isolated engine instance with its pages, configuration, and evaluation history. Whether a session may contain more than one page is open.

**Session mode.** One long-lived CLI invocation that reads one command per stdin line. It keeps one engine session alive. It is separate from the planned JavaScript REPL.

**Page.** A loaded browsing context within a session. A page owns its current document and user-agent profile.

**Current URL.** The URL of the document installed in one page. A successful navigation replaces it.

**Page title.** The normalized text of the first HTML title element. A successful navigation replaces it.

**Document.** The current parsed and rendered content of a page. Navigation may replace it.

**Navigation.** A request that may replace a page's current document. Package and session-mode link clicks implement one bounded subset.

**Reload.** A navigation that fetches the current URL again. Success installs a fresh document epoch.

## Rendering and evidence

**Rendered state.** The engine's inspectable result after applying supported parsing, style, layout, and rendering rules.

**Layout observation.** A structured value describing geometry or computed layout for one rendered element.

**Grid observation.** A layout observation for a CSS grid. It may include tracks, gaps, placement, alignment, and overflow once support is defined.

**Layout mutation.** A package request that changes one supported input to incremental layout. Current mutations set one element's `x` or `width` value.

**Mutation batch.** An ordered group of layout mutations applied as one transaction. A failure preserves the previously committed layout.

**Layout invalidation.** The internal process that marks affected layout fields dirty and recomputes them in dependency order.

**Snapshot.** An immutable capture of selected page evidence at a known point in the page's lifetime.

**Interactive snapshot.** A snapshot containing the supported interactive roles, names, semantic identifiers, and ordered references.

**Interactive element reference.** A snapshot-owned target identity such as `@e1`. Another capture or document makes it stale.

**Locator.** A reusable current-document query. Implemented kinds cover semantic text, source attributes, and positioned compound CSS selectors.

**Role locator.** A role and optional accessible-name query over the supported semantic index.

**Text locator.** A normalized descendant-text query. A matching descendant takes priority over its matching ancestor.

**Label locator.** A query for a supported control through its associated label or ARIA label source.

**Placeholder locator.** A query for an input or textarea through its non-empty `placeholder` attribute.

**Alt locator.** A normalized text query over a parsed `alt` attribute.

**Title locator.** A normalized text query over a parsed `title` attribute.

**Test ID locator.** An exact, case-sensitive query over the `data-testid` attribute.

**Compound CSS selector.** One supported selector made from a tag, ID, classes, or attribute tests without combinators.

**Positioned CSS locator.** A compound CSS selector that chooses its first, last, or zero-based nth document-order match.

**Locator resolution.** Matching a locator against the current document. Non-positioned resolution rejects zero or multiple matches.

**Locator match.** One resolved element's identifier, optional role, accessible name, and normalized text. It is not an interactive reference.

**Role match.** A locator match whose semantic role is present and required.

**Locator action.** An action that resolves its locator when the request executes. Fill, check, uncheck, and supported link clicks are implemented.

**Actionability check.** Evidence required before an action mutates state. Current locator actions check supported visible, enabled, or editable state.

**Click.** An action request against one current interactive element reference. Only same-context link navigation is implemented.

**Fill.** An action that replaces a supported text control's current value. Fill does not dispatch browser events yet.

**Select.** An exact-value action on one current native single-select. Select does not dispatch browser events yet.

**Value inspection.** A read of one current text-control or native single-select value through a snapshot or typed request.

**Checked state.** The current Boolean state of a supported native checkbox. Snapshots and typed requests expose it.

**Element text.** Normalized descendant text from a loaded static element. It stays distinct from the accessible name.

**Element attribute.** One parsed static source attribute. Attribute reads distinguish present, missing, and blocked-sensitive values.

**Enabled state.** Whether a supported native element lacks its native disabled state. It is not full actionability.

**Visible state.** Whether supported static evidence proves a non-empty box without hidden computed visibility.

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

**Machine-readable output.** Decided but unimplemented structured output for programs and AI agents. Session mode emits flushed line-oriented text, not this structured format.

**Watch mode.** A decided but unimplemented lint invocation that checks the target again after relevant page changes settle.

**Exit status.** The process result used by shells and continuous integration. The CLI uses zero for pass and one for findings. It uses two for invalid input and three for blocked or unavailable results.
