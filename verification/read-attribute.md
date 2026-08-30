# Verification: read element attribute

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-attribute.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ATTR-01 | P1 | pipe | Present attributes return their parsed value ([The simple case](../inspection/read-attribute.md#the-simple-case)). | Serve a link with `href`. | Open, snapshot, and read `href`. | The package returns the source value. | partial: package and compiled-process tests passed, 2026-08-31 |
| ATTR-02 | P1 | pipe | Missing attributes remain distinct ([While running](../inspection/read-attribute.md#while-running)). | Serve a link without `title`. | Read `title` through its reference. | The package returns `None`. Session mode reports `null`. | partial: package and compiled-process tests passed, 2026-08-31 |
| ATTR-03 | P1 | pipe | Attribute names normalize ASCII case ([Begin running](../inspection/read-attribute.md#begin-running)). | Serve a `data-kind` attribute. | Request `DATA-KIND`. | The result name is lowercase and the value is present. | partial: package boundary test passed, 2026-08-31 |
| ATTR-04 | P1 | pipe | Password values remain blocked ([While running](../inspection/read-attribute.md#while-running)). | Serve a password input with a source value. | Read `type`, then read `value`. | Type succeeds. Value returns `SensitiveAttribute`. | partial: package boundary test passed, 2026-08-31 |
| ATTR-05 | P1 | pipe | Invalid names fail before attribute lookup ([Exit immediately](../inspection/read-attribute.md#exit-immediately)). | Open one interactive page. | Request a name containing whitespace. | The package returns `InvalidAttributeName`. | partial: package boundary test passed, 2026-08-31 |

Not checkable yet:

- Live DOM property inspection does not exist.
- Non-interactive locators do not exist.
- Machine-readable responses do not exist.
