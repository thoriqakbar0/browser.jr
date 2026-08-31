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
| [navigation.md](navigation.md) | `loading/navigation.md` |
| [ai-session.md](ai-session.md) | `automation/ai-session.md` |
| [fill-text.md](fill-text.md) | `interaction/fill-text.md` |
| [read-value.md](read-value.md) | `inspection/read-value.md` |
| [select-option.md](select-option.md) | `interaction/select-option.md` |
| [check-state.md](check-state.md) | `interaction/set-checked.md` and `inspection/read-checked.md` |
| [read-text.md](read-text.md) | `inspection/read-text.md` |
| [read-attribute.md](read-attribute.md) | `inspection/read-attribute.md` |
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

Automated tests cover CLI discovery, loopback loading, static HTML geometry, typed requests, transactional `x` and `width` invalidation, and explicit project width limits. DLINT-01 through DLINT-04 and DLINT-12 have partial automated evidence from 2026-08-29. DLINT-11 has partial package and kernel evidence from 2026-08-31.

Interactive snapshot tests cover semantics, control state, references, empty pages, and replacement. SNAP-01 through SNAP-10 have partial automated evidence from 2026-08-31.

Navigation tests cover relative links, fresh and stale references, unsupported clicks, and failed-navigation recovery. NAV-01 through NAV-06 have partial automated evidence from 2026-08-31.

Session-mode tests cover persistent actions, observations, reload, metadata, stale labels, and recovery. AISESSION-01 through AISESSION-15 have partial evidence from 2026-08-31.

Fill tests cover initial values, replacement, snapshot evidence, stale references, and session-mode text. FILL-01 through FILL-05 have partial automated evidence from 2026-08-31.

Value tests cover text and native single-select reads, unsupported controls, stale references, and session output. VALUE-01 through VALUE-06 have partial automated evidence from 2026-08-31.

Select tests cover initial state, exact values, disabled boundaries, failures, and stale references. SELECT-01 through SELECT-07 have partial automated evidence from 2026-08-31.

Checkbox tests cover writes, reads, snapshots, disabled controls, idempotence, and stale references. CHECK-01 through CHECK-06 have partial automated evidence from 2026-08-31.

Text tests cover descendants, empty text, accessible-name separation, repeated reads, and navigation. TEXT-01 through TEXT-05 have partial automated evidence from 2026-08-31.

Attribute tests cover present, missing, normalized, invalid, and sensitive values. ATTR-01 through ATTR-05 have partial automated evidence from 2026-08-31.

Enabled-state tests cover active, disabled, unsupported, and reusable references. ENABLED-01 through ENABLED-04 have partial automated evidence from 2026-08-31.

Visibility tests cover static boxes, hidden states, unsupported evidence, and reusable references. VISIBLE-01 through VISIBLE-06 have partial automated evidence from 2026-08-31.

Reload tests cover replacement, stale references, failed-load recovery, and missing pages. RELOAD-01 through RELOAD-04 have partial automated evidence from 2026-08-31.

TTY conditions, interactive cancellation, and broader rendering remain unverified.
