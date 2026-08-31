# Verification: select option

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/select-option.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SELECT-01 | P1 | pipe | Exact option values update single-select state ([The simple case](../interaction/select-option.md#the-simple-case)). | Serve a single select with two enabled values. | Open, snapshot, select the second value, and read it. | The typed result and direct read report the second value. | partial: package boundary test passed, 2026-08-31 |
| SELECT-02 | P1 | pipe | Initial state follows selected and fallback rules ([Edge cases](../interaction/select-option.md#edge-cases)). | Serve selected, unselected, and listbox-shaped single selects. | Capture one interactive snapshot. | Explicit selection wins. A size-one select falls back. The listbox may be empty. | partial: unit and package tests passed, 2026-08-31 |
| SELECT-03 | P1 | pipe | Disabled options reject selection ([While running](../interaction/select-option.md#while-running)). | Serve direct-disabled and disabled-optgroup options. | Try each exact value, then read the current value. | Both actions fail. The previous value remains selected. | partial: package boundary test passed, 2026-08-31 |
| SELECT-04 | P1 | pipe | A disabled select remains readable and immutable ([While running](../interaction/select-option.md#while-running)). | Serve one disabled single select. | Read its value, then select that value. | The read succeeds. The action reports unsupported selection. | partial: package boundary test passed, 2026-08-31 |
| SELECT-05 | P1 | pipe | Rejected targets preserve current state ([Edge cases](../interaction/select-option.md#edge-cases)). | Serve a single select, multiple select, and button. | Try missing, multiple, and wrong-role actions. Then read the single select. | Every action fails with its boundary. The original single value remains. | partial: package boundary test passed, 2026-08-31 |
| SELECT-06 | P1 | pipe | A newer snapshot stales old selection references ([Finish](../interaction/select-option.md#finish)). | Serve one single select. | Capture, select, capture again, then use the old reference. | The last request reports a stale reference. | partial: package boundary test passed, 2026-08-31 |
| SELECT-07 | P2 | pipe | Session mode accepts spaced exact values ([Finish](../interaction/select-option.md#finish)). | Serve one option whose value contains a space. | Send `select @e1 large value`, read it, and capture again. | Output quotes `large value`. Direct and snapshot reads agree. | partial: compiled-process test passed, 2026-08-31 |

Not checkable yet:

- Multiple selection does not exist.
- Label and index matching do not exist.
- Browser event dispatch and native validation do not exist.
