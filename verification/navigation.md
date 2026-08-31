# Verification: link navigation

Run package checks through one `Session`. Run CLI checks through one session-mode process. Record the fixture and browser.jr commit.

## loading/navigation.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NAV-01 | P1 | pipe | A current link reference navigates ([The simple case](../loading/navigation.md#the-simple-case)). | Serve a page linking to another controlled page. | Open, snapshot, click the link, and snapshot again. | The result reports navigation. The next snapshot contains the second document. | partial: package boundary test passed, 2026-08-31 |
| NAV-02 | P1 | pipe | A new snapshot invalidates older references ([Edge cases](../loading/navigation.md#edge-cases)). | Serve one page with an interactive element. | Capture twice. Submit one reference from each snapshot. | The old reference is stale. The current reference reaches action dispatch. | partial: package boundary test passed, 2026-08-31 |
| NAV-03 | P1 | network | Failed navigation preserves current state ([Finish](../loading/navigation.md#finish)). | Serve a link to a disallowed host. | Capture and click the link twice. | Both attempts report navigation failure. The first attempt does not stale the reference. | partial: package boundary test passed, 2026-08-31 |
| NAV-04 | P1 | pipe | Unsupported clicks never become navigation ([While running](../loading/navigation.md#while-running)). | Serve a button, download link, and new-context link. | Capture and click each current reference. | Each returns unsupported behavior without replacing the page. | partial: parser and package boundary tests passed, 2026-08-31 |
| NAV-05 | P2 | pipe | Successful navigation invalidates old evidence ([Finish](../loading/navigation.md#finish)). | Create layout and interactive evidence before navigation. | Navigate, then use both old evidence forms. | Old interactive references are stale. Old layout evidence is absent. | interactive and open invalidation tests passed separately, 2026-08-31 |
| NAV-06 | P1 | pipe | Session mode carries a reference into link navigation ([Invoke](../loading/navigation.md#invoke)). | Serve a page linking to a second controlled page. | Send open, snapshot, click, and snapshot through one process. | Navigation succeeds. The next snapshot reports the second document. | partial: compiled-process fixture passed, 2026-08-31 |

Not checkable yet:

- JavaScript events and form submission do not exist.
- Reference click does not enforce visibility, stability, or hit-testing actionability checks.
