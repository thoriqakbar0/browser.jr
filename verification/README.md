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
| [query-elements.md](query-elements.md) | `inspection/query-elements.md` |
| [navigation.md](navigation.md) | `loading/navigation.md` |
| [ai-session.md](ai-session.md) | `automation/ai-session.md` |
| [fill-text.md](fill-text.md) | `interaction/fill-text.md` |
| [read-value.md](read-value.md) | `inspection/read-value.md` |
| [select-option.md](select-option.md) | `interaction/select-option.md` |
| [check-state.md](check-state.md) | `interaction/set-checked.md` and `inspection/read-checked.md` |
| [read-text.md](read-text.md) | `inspection/read-text.md` |
| [read-attribute.md](read-attribute.md) | `inspection/read-attribute.md` |
| [read-html.md](read-html.md) | `inspection/read-html.md` |
| [read-enabled.md](read-enabled.md) | `inspection/read-enabled.md` |
| [read-visible.md](read-visible.md) | `inspection/read-visible.md` |
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

Interactive snapshot tests cover semantics, control state, scopes, references, JSON envelopes, empty pages, and replacement. SNAP-01 through SNAP-16 have partial automated evidence from 2026-08-31.

Navigation tests cover relative links, fresh and stale references, unsupported clicks, and failed-navigation recovery. NAV-01 through NAV-06 have partial automated evidence from 2026-08-31.

Session-mode tests cover persistent actions, observations, scoped snapshots, direct selectors, locator actions, events, reload, metadata, stale labels, and recovery. AISESSION-01 through AISESSION-29 have partial evidence from 2026-08-31.

Fill tests cover initial values, replacement, event records, snapshot evidence, stale references, and session-mode text. FILL-01 through FILL-06 have partial automated evidence from 2026-08-31.

Value tests cover text and native-select reads, direct selectors, unsupported controls, stale references, and session output. VALUE-01 through VALUE-08 have partial automated evidence from 2026-08-31.

Select tests cover value, label, index, lists, direct selectors, disabled boundaries, atomic failures, events, and stale references. SELECT-01 through SELECT-14 have partial automated evidence from 2026-08-31.

Checkbox tests cover reference and selector writes, reads, snapshots, disabled controls, idempotence, events, and stale references. CHECK-01 through CHECK-08 have partial automated evidence from 2026-08-31.

Text tests cover selector and reference reads, descendants, inert raw text, empty text, accessible-name separation, repeated reads, and navigation. TEXT-01 through TEXT-07 have partial automated evidence from 2026-08-31.

Attribute tests cover selector and reference reads for present, missing, normalized, invalid, and sensitive values. ATTR-01 through ATTR-06 have partial automated evidence from 2026-08-31.

HTML tests cover normalized serialization, selector and reference reads, outer-tag exclusion, and sensitive descendants. HTML-01 through HTML-05 have partial automated evidence from 2026-08-31.

Enabled-state tests cover direct selectors, active, disabled, unsupported, and reusable references. ENABLED-01 through ENABLED-05 have partial automated evidence from 2026-08-31.

Visibility tests cover selector and reference reads for static boxes, inline and embedded hidden states, quoted selector values, unsupported evidence, and reusable references. VISIBLE-01 through VISIBLE-09 have partial automated evidence from 2026-08-31.

Reload tests cover replacement, stale references, failed-load recovery, and missing pages. RELOAD-01 through RELOAD-04 have partial automated evidence from 2026-08-31.

Locator tests cover semantic, attribute, CSS, and XPath matching with normalized ancestry. They also cover direct selector actions and reads, strict and collection resolution, document order, navigation, and reference lifetimes. QUERY-01 through QUERY-37 have partial automated evidence from 2026-08-31.

TTY conditions, interactive cancellation, and broader rendering remain unverified.
