# Verification: interactive snapshot

Run these checks against a controlled loopback page. Record the page source and browser.jr commit.

## inspection/capture-snapshot.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SNAP-01 | P1 | tty | The command reports supported interactive elements ([The simple case](../inspection/capture-snapshot.md#the-simple-case)). | Serve labeled input and button elements. | Run `browser.jr snapshot <url> --interactive`. | Output contains their roles, names, and ordered references. | partial: compiled-process fixture passed; TTY pending, 2026-08-31 |
| SNAP-02 | P1 | pipe | References remain stable in one document ([While running](../inspection/capture-snapshot.md#while-running)). | Open one page through the package session. | Capture twice and compare typed references. | The snapshot identifiers change. Element references remain equal. | partial: package boundary test passed, 2026-08-31 |
| SNAP-03 | P1 | pipe | Navigation changes reference identity ([While running](../inspection/capture-snapshot.md#while-running)). | Prepare two controlled pages. | Open each page and capture it in one session. | References from different document epochs do not compare equal. | partial: package boundary test passed, 2026-08-31 |
| SNAP-04 | P1 | tty | A load failure cannot produce a snapshot ([Finish](../inspection/capture-snapshot.md#finish)). | Use an unreachable loopback URL. | Run the snapshot command. | Stderr identifies the load failure. The command exits three. | automated loading path covered; TTY pending, 2026-08-31 |
| SNAP-05 | P2 | tty | Empty interactive content is a successful empty snapshot ([Edge cases](../inspection/capture-snapshot.md#edge-cases)). | Serve static HTML without supported interactive elements. | Run the snapshot command. | Output reports `elements=0` and exits zero. | not run |
| SNAP-06 | P1 | pipe | Page replacement cannot leak old evidence ([Edge cases](../inspection/capture-snapshot.md#edge-cases)). | Create layout evidence, then open another page. | Request old layout evidence and test one failed replacement. | Old layout evidence is absent. A failed open preserves the current page. | partial: package boundary tests passed, 2026-08-31 |

Not checkable by hand yet:

- CSS visibility and JavaScript mutation do not exist.
- Complete accessible-name and accessibility-tree conformance remain unsupported.
- Persistent CLI sessions and action commands remain unimplemented.
