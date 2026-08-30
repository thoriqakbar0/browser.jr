# Verification: read value

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## inspection/read-value.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| VALUE-01 | P1 | pipe | Direct reads return the current value ([The simple case](../inspection/read-value.md#the-simple-case)). | Serve a text input with an initial value. | Open, snapshot, fill, then read through the same reference. | The direct read returns the replacement value. | partial: package boundary test passed, 2026-08-31 |
| VALUE-02 | P1 | pipe | Session mode reports escaped current text ([Finish](../inspection/read-value.md#finish)). | Serve one text input. | Snapshot, fill spaced text, then send `get value`. | Output contains the reference and quoted replacement value. | partial: compiled-process test passed, 2026-08-31 |
| VALUE-03 | P1 | pipe | Read-only text controls remain readable ([Begin running](../inspection/read-value.md#begin-running)). | Serve a read-only text input with a value. | Snapshot, read, then try to fill the same reference. | Read succeeds. Fill reports unsupported. | partial: package boundary test passed, 2026-08-31 |
| VALUE-04 | P1 | pipe | Controls without text values return unsupported behavior ([Edge cases](../inspection/read-value.md#edge-cases)). | Serve a button. | Snapshot and request its value. | The package returns `UnsupportedValue`. | partial: package boundary test passed, 2026-08-31 |
| VALUE-05 | P1 | pipe | New snapshots stale older value references ([Edge cases](../inspection/read-value.md#edge-cases)). | Serve one text input. | Capture twice, then read with the first reference. | The read reports a stale reference. | partial: package boundary test passed, 2026-08-31 |

Not checkable yet:

- Password values remain intentionally unavailable.
- Other native form-control value types do not exist.
- Machine-readable responses do not exist.
