# Verification: history navigation

Run these checks against controlled loopback pages. Record the fixture and browser.jr commit.

## loading/history-navigation.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HISTORY-01 | P1 | pipe | A history bound preserves current evidence ([Exit immediately](../loading/history-navigation.md#exit-immediately)). | Open one page and capture a reference. | Go back, then read through the reference. | Back reports `NoEntry`. The reference remains usable. | partial: package and compiled-process tests passed, 2026-08-31 |
| HISTORY-02 | P1 | pipe | Back and forward reload adjacent successful navigation entries ([The simple case](../loading/history-navigation.md#the-simple-case)). | Serve two linked pages and return bodies. | Follow the link, go back, then go forward. | Each move loads the expected URL and returned document. | partial: package and compiled-process tests passed, 2026-08-31 |
| HISTORY-03 | P1 | pipe | Successful history movement invalidates old references ([Finish](../loading/history-navigation.md#finish)). | Capture a reference before a history move. | Move to another entry, then read the old reference. | The old reference is stale. | partial: package test passed, 2026-08-31 |
| HISTORY-04 | P1 | network | Failed history loading preserves current state ([While running](../loading/history-navigation.md#while-running)). | Open two pages, then stop the first server. | Capture on the second page and go back. | Loading fails. The second URL and reference remain usable. | partial: package test passed, 2026-08-31 |
| HISTORY-05 | P1 | pipe | A new navigation truncates forward entries ([Begin running](../loading/history-navigation.md#begin-running)). | Build two entries and move back. | Open a branch URL, then go forward. | Forward reports `NoEntry` at the branch. | partial: package and compiled-process tests passed, 2026-08-31 |
| HISTORY-06 | P1 | pipe | Session output distinguishes moves from bounds ([Finish](../loading/history-navigation.md#finish)). | Serve a two-page history and one branch. | Run back and forward across entries and bounds. | Output reports each URL and Boolean `navigated` state. | partial: parser and compiled-process tests passed, 2026-08-31 |
| HISTORY-07 | P2 | pipe | Reload does not add a history entry ([Begin running](../loading/history-navigation.md#begin-running)). | Serve two responses from one URL. | Open, reload, then go back. | Back reports `NoEntry`. The reloaded document remains installed. | partial: package test passed, 2026-08-31 |
| HISTORY-08 | P1 | package | History movement requires a current page ([Exit immediately](../loading/history-navigation.md#exit-immediately)). | Create an empty session. | Submit `GoBack` and `GoForward`. | Both requests return `SessionError::NoPage`. | partial: package test passed, 2026-08-31 |
| HISTORY-09 | P1 | package | Supported GET form navigation records one history entry ([Begin running](../loading/history-navigation.md#begin-running)). | Serve a form and destination. | Submit, go back, then submit again with Space. | Back restores the form URL. Each submit reaches the same destination. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Redirect history does not exist.
- Same-document fragment history does not exist.
- Cached document restoration does not exist.
- Navigation timeouts and cancellation do not exist.
