# Verification: reload page

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## loading/reload-page.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RELOAD-01 | P1 | pipe | Reload installs a fresh response for the current URL ([The simple case](../loading/reload-page.md#the-simple-case)). | Serve two responses from one URL. | Open, inspect, reload, and inspect again. | The second title and semantics replace the first. | partial: package and compiled-process tests passed, 2026-08-31 |
| RELOAD-02 | P1 | pipe | Successful reload invalidates old references ([Finish](../loading/reload-page.md#finish)). | Serve two interactive responses. | Capture, reload, then use the old typed reference. | The old reference reports stale. | partial: package boundary test passed, 2026-08-31 |
| RELOAD-03 | P1 | pipe | Failed reload preserves current state ([While running](../loading/reload-page.md#while-running)). | Open a page, capture, then stop its server. | Reload and read through the current reference. | Reload fails. The read still returns current document text. | partial: package boundary test passed, 2026-08-31 |
| RELOAD-04 | P1 | pipe | Reload requires an open page ([Exit immediately](../loading/reload-page.md#exit-immediately)). | Create an empty package session. | Submit `ReloadPage`. | The package returns `NoPage`. | partial: package boundary test passed, 2026-08-31 |

Not checkable yet:

- Cache semantics do not exist.
- Client-side history does not exist.
- Graceful cancellation does not exist.
