# Verification: read page text

Run these checks against controlled loopback pages. Record the fixture and browser.jr commit.

## inspection/read-page.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| READPAGE-01 | P1 | pipe | One-shot read returns normalized static page text ([The simple case](../inspection/read-page.md#the-simple-case)). | Serve inline text, a script, button, and input value. | Run `browser.jr read <url>`. | Visible source text returns on one line. Script and input values do not appear. | partial: compiled-process test passed, 2026-08-31 |
| READPAGE-02 | P1 | package | Package reads need no snapshot or locator ([Invoke](../inspection/read-page.md#invoke)). | Open one controlled page. | Submit `GetPageText`. | `PageText` contains the normalized document text. | partial: package test passed, 2026-08-31 |
| READPAGE-03 | P1 | package | Navigation replaces page text ([Finish](../inspection/read-page.md#finish)). | Serve two linked pages. | Read, follow the link, then read again. | Each result contains its installed document text. | partial: package test passed, 2026-08-31 |
| READPAGE-04 | P1 | pipe | Current session reads preserve references ([Begin running](../inspection/read-page.md#begin-running)). | Open a page with one button and capture it. | Run `read`, then read the button reference. | Both reads succeed through the same reference set. | partial: compiled-process test passed, 2026-08-31 |
| READPAGE-05 | P1 | pipe | A session URL read opens and reports the replacement document ([Begin running](../inspection/read-page.md#begin-running)). | Serve two responses under one controlled origin. | Open the first URL, then run `read <second-url>`. | The command prints only the second document text. | partial: compiled-process test passed, 2026-08-31 |
| READPAGE-06 | P2 | package | Static normalization excludes metadata and preserves inline adjacency ([While running](../inspection/read-page.md#while-running)). | Parse metadata, scripts, styles, inline elements, and blocks. | Inspect `PageText`. | Excluded content is absent. Whitespace and adjacency follow the documented model. | partial: unit and package tests passed, 2026-08-31 |
| READPAGE-07 | P1 | package | Package reads require a current page ([Exit immediately](../inspection/read-page.md#exit-immediately)). | Create an empty session. | Submit `GetPageText`. | The request returns `SessionError::NoPage`. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Rendered visibility and generated content are incomplete.
- Structured Markdown output does not exist.
- Shadow DOM, frames, and JavaScript mutations do not exist.
- Machine-readable read output does not exist.
