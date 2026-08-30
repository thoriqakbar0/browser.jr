# Bug triage

Evidence date: 31 August 2026.

No open suspected defects exist. The current runtime slice passes its automated checks.

Missing behavior belongs in the coverage table. A live-page result that conflicts with decided behavior becomes a triage entry.

## Summary

| ID | Title | Severity | Area | Decision needed | Issue |
| --- | --- | --- | --- | --- | --- |

## Resolved conflicts

| ID | Conflict | Resolution | Evidence |
| --- | --- | --- | --- |
| BJR-001 | macOS loopback loads intermittently returned `EINVAL`. | The loader avoids socket-option updates that can race on macOS. | 100 consecutive CLI suites passed after the final fix, 2026-08-31. |
