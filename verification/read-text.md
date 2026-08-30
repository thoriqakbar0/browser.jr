# Verification: read element text

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-text.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TEXT-01 | P1 | pipe | Direct reads return normalized descendant text ([The simple case](../inspection/read-text.md#the-simple-case)). | Serve a button containing nested text. | Open, snapshot, then read its text. | The package returns collapsed descendant text. | partial: package boundary test passed, 2026-08-31 |
| TEXT-02 | P1 | pipe | Accessible names do not replace descendant text ([Accessibility inspection](../inspection/read-text.md#interactions-with-other-systems)). | Give one button a different `aria-label`. | Read its text through the snapshot reference. | The text result uses descendants, not the label. | partial: package boundary test passed, 2026-08-31 |
| TEXT-03 | P1 | pipe | Elements without descendants return empty text ([Edge cases](../inspection/read-text.md#edge-cases)). | Serve a labeled input. | Snapshot and read the input text. | The result is an empty string. | partial: package boundary test passed, 2026-08-31 |
| TEXT-04 | P1 | pipe | Session mode reads text before and after navigation ([Finish](../inspection/read-text.md#finish)). | Serve linked pages with textual controls. | Snapshot and read each current reference. | Each result reports the installed document's element text. | partial: compiled-process test passed, 2026-08-31 |
| TEXT-05 | P1 | pipe | Text reads preserve current references ([While running](../inspection/read-text.md#while-running)). | Serve one button. | Read its text twice through one reference. | Both typed results match. | partial: package boundary test passed, 2026-08-31 |

Not checkable yet:

- CSS-aware rendered text does not exist.
- Non-interactive locators do not exist.
- Machine-readable responses do not exist.
