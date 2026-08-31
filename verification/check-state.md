# Verification: checkbox state

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/set-checked.md and inspection/read-checked.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CHECK-01 | P1 | pipe | Check stores true ([The simple case](../interaction/set-checked.md#the-simple-case)). | Serve an unchecked native checkbox. | Open, snapshot, check, and read state. | The action and read report true. | partial: package and compiled-process tests passed, 2026-08-31 |
| CHECK-02 | P1 | pipe | Uncheck stores false ([The simple case](../interaction/set-checked.md#the-simple-case)). | Serve a native checkbox. | Check, uncheck, and read state through one reference. | The final read reports false. | partial: compiled-process test passed, 2026-08-31 |
| CHECK-03 | P1 | pipe | Repeated state writes are idempotent ([Edge cases](../interaction/set-checked.md#edge-cases)). | Serve an unchecked native checkbox. | Submit the same true state twice. | Both typed results equal true. | partial: package boundary test passed, 2026-08-31 |
| CHECK-04 | P1 | pipe | Disabled state remains readable but immutable ([Begin running](../inspection/read-checked.md#begin-running)). | Serve a checked disabled checkbox. | Read state, then request false. | The read returns true. The change reports unsupported. | partial: package boundary test passed, 2026-08-31 |
| CHECK-05 | P1 | pipe | Snapshots report native checkbox state ([Finish](../interaction/set-checked.md#finish)). | Serve one checkbox. | Snapshot, change state, then snapshot again. | Each snapshot reports its current Boolean state. | partial: package and compiled-process tests passed, 2026-08-31 |
| CHECK-06 | P1 | pipe | New snapshots stale older state references ([Edge cases](../inspection/read-checked.md#edge-cases)). | Serve one checkbox. | Capture twice, then read with the first reference. | The read reports a stale reference. | partial: package boundary test passed, 2026-08-31 |
| CHECK-07 | P1 | pipe | Direct selectors change and read checked state without a snapshot ([The simple case](../interaction/set-checked.md#the-simple-case)). | Serve one unchecked native checkbox. | Check it through CSS, then read checked state through CSS. | The action commits true and the read returns true. | partial: package, compiled-process, and controlled agent-browser comparison passed, 2026-08-31 |
| CHECK-08 | P1 | pipe | Changed checkbox state records input before change ([While running](../interaction/set-checked.md#while-running)). | Serve a nested unchecked checkbox. | Check once, check again, then drain events. | One input and one change event appear in order. The idempotent action adds none. | partial: package test passed, 2026-08-31 |

Not checkable yet:

- Click activation, focus, pointer events, and listener invocation do not exist.
- Radio-group behavior does not exist.
- ARIA checked-state observation does not exist.
