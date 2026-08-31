# Verification: read element HTML

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-html.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HTML-01 | P1 | pipe | Reads return normalized child markup ([The simple case](../inspection/read-html.md#the-simple-case)). | Serve nested markup with entities and a comment. | Read the container through CSS. | The result preserves children and escapes entities. | partial: unit, package, and compiled-process tests passed, 2026-08-31 |
| HTML-02 | P1 | pipe | CSS and XPath share one normalized document ([Begin running](../inspection/read-html.md#begin-running)). | Serve one non-interactive container. | Read it through strict CSS and XPath. | Both replies contain identical markup. | partial: package and compiled-process tests passed, 2026-08-31 |
| HTML-03 | P1 | pipe | Current references read their source element ([Finish](../inspection/read-html.md#finish)). | Serve one button with nested markup. | Capture, then read HTML through its reference. | Output identifies the reference and contains child markup. | partial: package and compiled-process tests passed, 2026-08-31 |
| HTML-04 | P1 | pipe | Sensitive descendant values block serialization ([While running](../inspection/read-html.md#while-running)). | Nest a valued password input inside a target. | Read the target through a locator and reference. | Both requests return typed blocked errors without markup. | partial: package boundary test passed, 2026-08-31 |
| HTML-05 | P2 | pipe | Inner HTML excludes the selected outer element ([Edge cases](../inspection/read-html.md#edge-cases)). | Serve one container with a child. | Read the container HTML. | The child appears. The container tags do not. | partial: unit and package tests passed, 2026-08-31 |

Not checkable yet:

- Live script-mutated DOM serialization does not exist.
- Shadow DOM serialization does not exist.
- Machine-readable responses do not exist.
