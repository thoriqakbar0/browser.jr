# Verification: select option

Run these checks against a controlled loopback page. Record the fixture and browser.jr commit.

## interaction/select-option.md

| ID | P | Device | Claim | Setup | Steps | Expected | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SELECT-01 | P1 | pipe | Exact option values update single-select state ([The simple case](../interaction/select-option.md#the-simple-case)). | Serve a single select with two enabled values. | Open, snapshot, select the second value, and read it. | The typed result and direct read report the second value. | partial: package boundary test passed, 2026-08-31 |
| SELECT-02 | P1 | pipe | Initial state follows selected and fallback rules ([Edge cases](../interaction/select-option.md#edge-cases)). | Serve selected, unselected, and listbox-shaped single selects. | Capture one interactive snapshot. | Explicit selection wins. A size-one select falls back. The listbox may be empty. | partial: unit and package tests passed, 2026-08-31 |
| SELECT-03 | P1 | pipe | Disabled options reject selection ([While running](../interaction/select-option.md#while-running)). | Serve direct-disabled and disabled-optgroup options. | Try each exact value, then read the current value. | Both actions fail. The previous value remains selected. | partial: package boundary test passed, 2026-08-31 |
| SELECT-04 | P1 | pipe | A disabled select remains readable and immutable ([While running](../interaction/select-option.md#while-running)). | Serve one disabled single select. | Read its value, then select that value. | The read succeeds. The action reports unsupported selection. | partial: package boundary test passed, 2026-08-31 |
| SELECT-05 | P1 | pipe | Rejected targets preserve current state ([Edge cases](../interaction/select-option.md#edge-cases)). | Serve a single select and button. | Try missing and wrong-role actions. Then read the single select. | Both actions fail with their boundary. The original single value remains. | partial: package boundary test passed, 2026-08-31 |
| SELECT-06 | P1 | pipe | A newer snapshot stales old selection references ([Finish](../interaction/select-option.md#finish)). | Serve one single select. | Capture, select, capture again, then use the old reference. | The last request reports a stale reference. | partial: package boundary test passed, 2026-08-31 |
| SELECT-07 | P2 | pipe | Session mode accepts spaced exact values ([Finish](../interaction/select-option.md#finish)). | Serve one option whose value contains a space. | Send `select @e1 large value`, read it, and capture again. | Output quotes `large value`. Direct and snapshot reads agree. | partial: compiled-process test passed, 2026-08-31 |
| SELECT-08 | P1 | pipe | Direct selectors select and read exact values without a snapshot ([Finish](../interaction/select-option.md#finish)). | Serve one native single-select. | Select the second value through CSS, then read it through CSS. | The action and read report the second value. | partial: package, compiled-process, and controlled agent-browser comparison passed, 2026-08-31 |
| SELECT-09 | P1 | pipe | Multiple selects commit a non-empty exact-value list ([While running](../interaction/select-option.md#while-running)). | Serve a multiple select with two enabled options. | Select both values in reverse document order. | The typed and session results contain both values in request order. | partial: package and compiled-process tests passed, 2026-08-31 |
| SELECT-10 | P1 | pipe | List selection is transactional ([While running](../interaction/select-option.md#while-running)). | Select two values, then request one valid and one missing or disabled value. | Read the value after each failure. | Each request fails at its exact value. The prior selection remains. | partial: package boundary tests passed, 2026-08-31 |
| SELECT-11 | P1 | pipe | Single selects commit one document-order match from a value list ([While running](../interaction/select-option.md#while-running)). | Serve a single select with two enabled options. | Request both values in reverse document order. | The first matching option in document order becomes selected and is the only returned value. | partial: package boundary test passed, 2026-08-31 |
| SELECT-12 | P1 | pipe | Package selection matches exact labels and zero-based indexes ([Begin running](../interaction/select-option.md#begin-running)). | Serve labeled and unlabeled options in one multiple select. | Select by fallback label, label attribute, and index through reference and locator requests. | Every target resolves its option. Results contain committed values in request order. | partial: package boundary test passed, 2026-08-31 |
| SELECT-13 | P1 | pipe | Missing and disabled typed targets preserve selection ([While running](../interaction/select-option.md#while-running)). | Commit a multiple selection with one disabled option. | Request a missing label, disabled index, and out-of-range index. Then read the value. | Each request returns its typed boundary. The committed selection remains. | partial: package boundary test passed, 2026-08-31 |

Not checkable yet:

- Empty-list deselection does not exist.
- Session label and index syntax does not exist.
- Browser event dispatch and native validation do not exist.
