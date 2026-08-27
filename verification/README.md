# Hand verification

The current design-lint document records intended behavior. This directory will check those claims against the running browser.jr CLI.

## What is here

| File | Covers |
| --- | --- |
| [design-lint.md](design-lint.md) | `verification-features/design-lint.md` |

The remaining checklist clusters will appear when their feature documents exist.

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

Automated tests cover CLI discovery, typed package requests, synthetic layout, and width invalidation. The live design-lint checklist remains blocked because page loading does not exist.
