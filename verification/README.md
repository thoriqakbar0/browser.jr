# browser.jr hand verification

This directory checks product claims against the running browser.jr CLI. Feature documents define intended behavior. These checklists record evidence only.

## Relationship to the product description

- The [root README](../README.md) owns scope, current status, and coverage.
- Feature documents own user-visible behavior.
- Files in this directory link each check to its owning claim.
- A failed check does not rewrite the claim. Record the conflict in [`bug-triage.md`](../bug-triage.md).
- A passing automated test supplies implementation evidence. It does not replace the required hand check.

## What is here

| File | Covers |
| --- | --- |
| [design-lint.md](design-lint.md) | `verification-features/design-lint.md` |
| [capture-snapshot.md](capture-snapshot.md) | `inspection/capture-snapshot.md` |
| [capture-screenshot.md](capture-screenshot.md) | `inspection/capture-screenshot.md` |
| [query-elements.md](query-elements.md) | `inspection/query-elements.md` |
| [navigation.md](navigation.md) | `loading/navigation.md` |
| [history-navigation.md](history-navigation.md) | `loading/history-navigation.md` |
| [ai-session.md](ai-session.md) | `automation/ai-session.md` |
| [fill-text.md](fill-text.md) | `interaction/fill-text.md` |
| [type-text.md](type-text.md) | `interaction/type-text.md` |
| [focus-element.md](focus-element.md) | `interaction/focus-element.md` |
| [hover-element.md](hover-element.md) | `interaction/hover-element.md` |
| [scroll-page.md](scroll-page.md) | `interaction/scroll-page.md` |
| [set-viewport.md](set-viewport.md) | `interaction/set-viewport.md` |
| [click-element.md](click-element.md) | `interaction/click-element.md` |
| [press-key.md](press-key.md) | `interaction/press-key.md` |
| [submit-form.md](submit-form.md) | `interaction/submit-form.md` |
| [read-value.md](read-value.md) | `inspection/read-value.md` |
| [select-option.md](select-option.md) | `interaction/select-option.md` |
| [check-state.md](check-state.md) | `interaction/set-checked.md` and `inspection/read-checked.md` |
| [read-text.md](read-text.md) | `inspection/read-text.md` |
| [read-page.md](read-page.md) | `inspection/read-page.md` |
| [read-attribute.md](read-attribute.md) | `inspection/read-attribute.md` |
| [read-html.md](read-html.md) | `inspection/read-html.md` |
| [read-enabled.md](read-enabled.md) | `inspection/read-enabled.md` |
| [read-editable.md](read-editable.md) | `inspection/read-editable.md` |
| [read-focused.md](read-focused.md) | `inspection/read-focused.md` |
| [read-hovered.md](read-hovered.md) | `inspection/read-hovered.md` |
| [read-visible.md](read-visible.md) | `inspection/read-visible.md` |
| [inspect-layout.md](inspect-layout.md) | `inspection/inspect-layout.md` |
| [reload-page.md](reload-page.md) | `loading/reload-page.md` |

The root README lists the remaining checklist clusters. Add a checklist after its feature document reaches `drafted`.

## How to run a pass

1. Start a controlled local test page and record its source commit.
2. Build browser.jr and record its source commit.
3. Run P1 items first, then P2, then P3.
4. Record `pass`, `fail`, or `blocked` with a short note.
5. Add each failure to [`bug-triage.md`](../bug-triage.md).
6. Mark a document verified only after every P1 and P2 result passes or reaches triage.

## Devices and conditions

- `tty` means a real interactive terminal with stdout attached.
- `pipe` means stdout is consumed by another process.
- `network` means the controlled server can be stopped during a request.
- `watch` means a source file can trigger the development server's normal page update.

## Driving browser.jr

CLI output, exit status, and saved machine-readable results can be checked by script. Progress, terminal replacement, and Ctrl+C behavior need a real terminal.

## Results so far

Automated tests cover CLI discovery, loopback loading, static HTML geometry, typed requests, transactional `x` and `width` invalidation, and explicit project width limits. DLINT-01 through DLINT-04 and DLINT-12 have partial automated evidence from 2026-08-29. DLINT-11 and DLINT-13 have partial automated evidence from 2026-08-31.

Snapshot tests cover full trees, interactive projections, list markers, state, scopes, document refs, URLs, depth, compact output, JSON, and replacement. SNAP-01 through SNAP-23 have partial automated evidence from 2026-08-31.

Screenshot tests cover viewport, full-page, locator, PNG, compositing, limits, and unsupported-paint blocking. SCREENSHOT-01 through SCREENSHOT-07 have partial automated evidence from 2026-08-31.

Navigation tests cover relative links, click and Enter activation, static stability, references, history, and failed-navigation recovery. NAV-01 through NAV-08 have partial evidence through 2026-09-01.

History tests cover bounds, back, forward, refetch, failures, branch truncation, reload, forms, and missing pages. HISTORY-01 through HISTORY-09 have partial evidence from 2026-08-31.

Session-mode tests cover persistent actions, observations, native event paths, JSON, snapshots, selectors, navigation, forms, focus, keyboard text, held keys, hover, viewport sizing, scrolling, geometry, stale labels, and recovery. AISESSION-01 through AISESSION-55 have partial evidence from 2026-08-31.

Fill tests cover values, focus, selection collapse, native event paths, snapshots, stale references, and session text. FILL-01 through FILL-09 have partial evidence from 2026-08-31.

Type tests cover reference and selector appends, selection preservation, empty text, editable boundaries, stale references, and session output. TYPE-01 through TYPE-07 have partial automated evidence from 2026-08-31.

Focus tests cover reference and locator targets, selection, rejection, document replacement, sequential order, radio groups, body boundaries, and output. FOCUS-01 through FOCUS-09 have partial evidence from 2026-08-31.

Hover tests cover references, locators, visibility, stability, auto-scroll, disabled targets, replacement, document changes, and output. HOVER-01 through HOVER-08 have partial evidence through 2026-09-01.

Click tests cover link and form navigation, native focus, checked controls, event records, static stability, auto-scroll, references, and output. CLICK-01 through CLICK-11 have partial evidence through 2026-09-01.

Key-press tests cover editing, focused text events, complete and held phases, state, selection, controls, submission, navigation, and output. PRESS-01 through PRESS-39 have partial evidence from 2026-08-31.

Form tests cover GET encoding, controls, overrides, ownership, implicit submission, failures, references, and history. SUBMIT-01 through SUBMIT-13 have partial evidence from 2026-08-31.

Value tests cover text and native-select reads, direct selectors, unsupported controls, stale references, and session output. VALUE-01 through VALUE-08 have partial automated evidence from 2026-08-31.

Select tests cover value, label, index, lists, event paths, direct selectors, disabled boundaries, atomic failures, and stale references. SELECT-01 through SELECT-15 have partial automated evidence from 2026-08-31.

Checked-state tests cover native controls, radio groups, event paths, references, selectors, static stability, auto-scroll, idempotence, and stale references. CHECK-01 through CHECK-15 have partial evidence through 2026-09-01.

Text tests cover selector and reference reads, descendants, inert raw text, empty text, accessible-name separation, repeated reads, and navigation. TEXT-01 through TEXT-07 have partial automated evidence from 2026-08-31.

Page-read tests cover one-shot, package, and session text across navigation, exclusions, references, and missing pages. READPAGE-01 through READPAGE-07 have partial automated evidence from 2026-08-31.

Attribute tests cover selector and reference reads for present, missing, normalized, invalid, and sensitive values. ATTR-01 through ATTR-06 have partial automated evidence from 2026-08-31.

HTML tests cover normalized serialization, selector and reference reads, outer-tag exclusion, and sensitive descendants. HTML-01 through HTML-05 have partial automated evidence from 2026-08-31.

Enabled-state tests cover direct selectors, active, disabled, unsupported, and reusable references. ENABLED-01 through ENABLED-05 have partial automated evidence from 2026-08-31.

Editable-state tests cover native controls, inherited contenteditable state, unsupported elements, references, and disabled-fieldset boundaries. EDITABLE-01 through EDITABLE-08 have partial evidence from 2026-08-31.

Focused-state tests cover body ownership, reference and locator reads, structural targets, Tab boundaries, and stale references. FOCUSED-01 through FOCUSED-07 have partial evidence from 2026-08-31.

Hovered-state tests cover references, semantic and structural locators, replacement, failed hover, and output. HOVERED-01 through HOVERED-05 have partial evidence from 2026-08-31.

Visibility tests cover selector and reference reads for static boxes, inline and embedded hidden states, quoted selectors, unsupported evidence, and reusable references. VISIBLE-01 through VISIBLE-10 have partial automated evidence from 2026-08-31.

Geometry tests cover reference, semantic, CSS, and XPath reads. They cover resized, scroll-relative, fixed, normal-flow, hidden, complete, and unsupported boxes. BOX-01 through BOX-12 have partial evidence from 2026-08-31.

Scroll tests cover page directions, clamping, viewport geometry, fixed boxes, actions, locators, aliases, and reload reset. SCROLL-01 through SCROLL-13 have partial evidence from 2026-08-31.

Viewport tests cover defaults, pre-open sizing, current-page reflow, state preservation, scroll clamping, invalid input, and output. VIEWPORT-01 through VIEWPORT-10 have partial evidence from 2026-08-31.

Reload tests cover replacement, stale references, failed-load recovery, and missing pages. RELOAD-01 through RELOAD-04 have partial automated evidence from 2026-08-31.

Locator tests cover semantic, state, hidden, attribute, CSS, and XPath matching over normalized ancestry. They also cover actions, reads, geometry, scrolling, static stability, native controls, non-presentational descendant image `alt` text, and references. QUERY-01 through QUERY-61 have partial evidence through 2026-09-01.

TTY conditions, interactive cancellation, and broader screenshot paint remain unverified.
