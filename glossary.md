# Glossary

The vocabulary used across these documents. Each definition states intended meaning. Implementation evidence may refine it.

## The engine

**browser.jr.** A Rust proof of concept for a small browser that helps agents verify their work. Playwright and `agent-browser` compatibility are product goals.

**Engine.** The code that loads a page, interprets supported web content, computes rendered state, and exposes that state for inspection.

**REPL.** A decided but unimplemented read-evaluate-print loop. Its intended job is to investigate findings and run custom checks in one session.

**Caller.** The person, AI agent, or program that submits an evaluation. Caller identity must not change evaluation semantics.

**CLI.** The primary developer interface. `lint` checks layout. `snapshot` reports supported accessibility evidence. `session` accepts persistent line commands.

## Sessions and pages

**Session.** One isolated engine instance with its page, configuration, navigation history, keyboard state, snapshots, and native event transcript.

**Session mode.** One long-lived CLI invocation that reads one command per stdin line. It keeps one engine session alive. It is separate from the planned JavaScript REPL.

**Page.** A loaded browsing context within a session. A page owns its current document and user-agent profile.

**Current URL.** The URL of the document installed in one page. A successful navigation replaces it.

**Page title.** The normalized text of the first HTML title element. A successful navigation replaces it.

**Document.** The current parsed and rendered content of a page. Navigation may replace it.

**Normalized HTML ancestry.** The parser-built element tree after implied elements, optional end tags, and other HTML repairs.

**Navigation.** A request that may replace a page's current document. Links and supported GET form submissions implement bounded subsets.

**Reload.** A navigation that fetches the current URL again. Success installs a fresh document epoch.

## Rendering and evidence

**Rendered state.** The engine's inspectable result after applying supported parsing, style, layout, and rendering rules.

**Layout observation.** A structured value describing geometry or computed layout for one rendered element.

**Bounding box.** One visible element's viewport-relative border box with `x`, `y`, `width`, and `height`.

**Viewport.** The current page's configured CSS pixel viewing area. A session starts at 1280 by 720.

**Viewport resize.** A size change that recomputes supported static geometry without replacing the current document.

**Page scroll.** A bounded change to the current page's horizontal or vertical viewport offset.

**Scroll into view.** A target action that adjusts page offsets to reveal one supported normal-flow box when possible.

**Automatic action scrolling.** Local click, hover, changed check, and changed uncheck reveal a supported target box before mutation.

**Normal flow.** Static block boxes placed in document order inside their parent's content box. Fixed boxes do not consume flow space.

**Grid observation.** A layout observation for a CSS grid. It may include tracks, gaps, placement, alignment, and overflow once support is defined.

**Layout mutation.** A package request that changes one supported input to incremental layout. Current mutations set one element's `x` or `width` value.

**Mutation batch.** An ordered group of layout mutations applied as one transaction. A failure preserves the previously committed layout.

**Layout invalidation.** The internal process that marks affected layout fields dirty and recomputes them in dependency order.

**Snapshot.** An immutable capture of selected page evidence at a known point in the page's lifetime.

**Screenshot.** A PNG capture of the current viewport, full supported page, strict locator, or package rectangle.

**Paint scene.** One capture rectangle and ordered browser.jr-owned paint commands.

**Software rasterizer.** The lazy in-process renderer for the implemented solid-box paint subset.

**Screenshot paint limit.** One screenshot may contain at most 16,777,216 CSS pixels.

**Screenshot work limit.** One raster may visit at most 67,108,864 clipped fill pixels.

**Accessibility snapshot.** A whole-page or locator-scoped tree of supported roles, names, ordered text, state, and references.

**Generated list marker.** A document-level accessibility node for one visible native list item. It has no element reference.

**Interactive snapshot.** An agent-oriented projection of supported controls, links, headings, navigation landmarks, state, and references.

**Interactive element reference.** A snapshot-owned target identity such as `@e1`. It may identify a control, link, heading, or navigation landmark.

References use document-wide ordinals. Scoped snapshots may contain gaps. Another capture or document makes every prior reference stale.

**Locator.** A reusable current-document query. Implemented kinds cover semantic text, source attributes, CSS, and XPath.

**Role locator.** A role query over the supported semantic index. It may filter name, description, state, level, and accessibility-hidden inclusion.

**Accessible name.** Normalized text that identifies one semantic element. Supported content names keep descendant text and non-presentational image `alt` alternatives in document order.

**Accessible description.** Normalized text that adds information to an accessible name. The supported subset follows ARIA description precedence.

**Accessibility-hidden state.** Whether supported HTML, CSS, and ARIA evidence removes an element from default role matching.

**Role state filter.** A Boolean checked, disabled, expanded, pressed, or selected condition applied during role resolution.

**Text locator.** A normalized descendant-text query. A matching descendant takes priority over its matching ancestor.

**Label locator.** A query for a supported control through its associated label or ARIA label source.

**Placeholder locator.** A query for an input or textarea through its non-empty `placeholder` attribute.

**Alt locator.** A normalized text query over a parsed `alt` attribute.

**Title locator.** A normalized text query over a parsed `title` attribute.

**Test ID locator.** An exact, case-sensitive query over the `data-testid` attribute.

**CSS locator.** A CSS selector query over the current normalized HTML document. Resolution is strict unless the locator has a position.

**Positioned CSS locator.** A CSS locator that chooses its first, last, or zero-based nth document-order match.

**XPath locator.** An XPath 1.0 element query over a namespace-free mirror of the current normalized HTML document.

**Direct selector.** A CSS or XPath target supplied directly to a session action, box read, state read, content read, or count command.

**Locator resolution.** Matching a locator against the current document. Non-positioned resolution rejects zero or multiple matches.

**Locator match.** One resolved element's identifier, optional role, accessible name, and normalized text. It is not an interactive reference.

**Locator collection.** Zero or more locator matches in current document order. Collection resolution does not require one unique match.

**Role match.** A locator match whose semantic role is present and required.

**Locator action.** An action that resolves its locator when the request executes. Supported actions include click, fill, type, focus, hover, scroll into view, press, select, check, and uncheck.

**Actionability check.** Evidence required before some actions mutate state. Focus and press use their separate no-actionability boundary.

**Stable state.** Supported static evidence that target geometry cannot change during one synchronous action. Motion declarations block this evidence.

**Action point.** The center of the target's supported viewport intersection after prospective automatic scrolling.

**Receives-events check.** A static hit test over supported normal-flow and fixed boxes at the action point.

The check accepts the target or its descendant. It rejects a known outside blocker and ignores `pointer-events:none` boxes.

An overlapping box with unsupported stacking, clipping, or pointer-event evidence blocks the check.

Unsupported target geometry keeps the earlier action boundary. It does not claim complete document hit-test evidence.

Successful pointer actions commit automatic action scrolling after their checks.

**Click.** An action that navigates a same-context link, submits a supported form, or applies a native control effect.

**Fill.** An action that focuses a supported text control and replaces its current value. Success records `beforeinput` and `input`.

**Type.** An action that appends text to a supported text control's current value.

Type records supported events in the native event transcript. It does not deliver them to page scripts.

**Focus.** The current page's supported active target or document body. Focus does not dispatch browser events.

**Focused state.** Whether one resolved target owns the current page focus at the time of a read.

**Hover.** An action that stores one visible source element as the current page's pointer target.

**Hovered state.** Whether one resolved target is the exact current pointer target. It does not imply CSS pseudo-class matching.

**Sequential focus order.** Positive `tabindex` targets, then natural and zero-`tabindex` targets in document order.

**Text selection.** One text control's anchor and focus, exposed as ordered UTF-16 start and end offsets.

**Key press.** One bounded text edit, focus traversal, native activation, ignored default, link navigation, or form submission.

Complete printable text and same-target native-control presses record their portable event sequences.

Focus-changing, navigating, modified, and non-ASCII presses do not record incomplete sequences.

**Held key.** One normalized key kept down in session state until the matching key-up request.

Supported down phases record before storage. Their matching up records against the current focused target.

Native-control `Space` down stores one pending activation without changing native state.

Matching key-up applies one effect when the original target still owns focus.

Repeated down preserves one pending activation. Another focused target at key-up cancels it.

**Keyboard text input.** A focused `type` or `inserttext` request. It replaces the current supported text selection.

Non-empty `inserttext` records `beforeinput`, then `input` on editable text. Read-only text records only `beforeinput`.

`type` applies each Unicode scalar in order.

Printable ASCII records `keydown`, `keypress`, `beforeinput`, `input`, then `keyup` on editable text.

Editable non-ASCII input records `beforeinput`, then `input`.

Read-only printable ASCII records the shared `keydown`, `keypress`, then `keyup` sequence.

Read-only non-key input events differ across Playwright engines. browser.jr does not record them.

**Form owner.** The exact form associated through a `form` attribute or nearest ancestor form.

**Successful control.** A named, enabled, form-owned control that contributes entries under the implemented GET subset.

**Implicit submission.** Text-control `Enter` activation that uses the form's default submitter or blocker count.

**Form submission.** Explicit or implicit activation that serializes successful controls and performs same-context HTTP GET navigation.

**Navigation history.** The ordered successful open, link, and supported form-navigation URLs. Reload does not add an entry.

**Select.** An exact value, label, or index action on one native select. Success records `input` and `change`.

**Native event transcript.** Session-owned, data-minimized records of supported native action events.

Each record contains type, document epoch, target identity, source ordinal, ancestor path, and bubbling metadata. It contains no input value.

The transcript does not dispatch events to page scripts. `TakeDomEvents` and session `events` drain it.

**Value inspection.** A read of one current text-control or native-select value through a snapshot or typed request. Multiple selects report their first selected value.

**Checked state.** The current Boolean state of a supported native checkbox or radio. Snapshots and typed requests expose it.

**Radio group.** Radios sharing one non-empty name and form owner. Selecting one radio unchecks its group peers.

**Element text.** Normalized non-inert descendant text from a loaded static element. It excludes raw style, script, and metadata text.

**Page text.** Normalized static document text. It excludes metadata, scripts, styles, and source control values.

**Element HTML.** Normalized static child markup for one element. It excludes the selected element's outer tags.

**Element attribute.** One parsed static source attribute. Attribute reads distinguish present, missing, and blocked-sensitive values.

**Enabled state.** Whether a supported native element lacks its native disabled state. It is not full actionability.

**Editable state.** Whether a supported native control or HTML editing host accepts user editing under the modeled static state.

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

**Machine-readable output.** JSON results for programs and AI agents. One-shot snapshots emit one envelope. JSON session mode emits newline-delimited envelopes.

**Watch mode.** A decided but unimplemented lint invocation that checks the target again after relevant page changes settle.

**Exit status.** The process result used by shells and continuous integration. The CLI uses zero for pass and one for findings. It uses two for invalid input and three for blocked or unavailable results.
