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

Automated tests cover CLI discovery, loopback loading, static HTML geometry, typed requests, width invalidation, and explicit project width limits. DLINT-01 through DLINT-04 and DLINT-12 have partial automated evidence from 2026-08-29.

Interactive snapshot tests cover role and name extraction, ordered references, repeated captures, and document replacement. SNAP-01 through SNAP-04 have partial automated evidence from 2026-08-31.

TTY conditions, interactive cancellation, and broader rendering remain unverified.
