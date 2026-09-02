# Keyboard state

Keyboard handling combines pure text and selection transformations with session-owned held-key state, focus, deferred activation, and event recording.

## Pure text effects

`src/keyboard.rs` computes text edits without session access. [[src/keyboard.rs#TextSelectionState#move_left]] and the related movement methods use UTF-16 boundaries because browser selection offsets follow DOM string indexing.

Insertion, deletion, Home, End, arrows, and modifier handling return candidate values and selections. The session commits those candidates only when the focused control allows the operation.

## Held keys

[[src/session.rs#KeyboardState#record_down]] tracks keys whose `keydown` has occurred and whether a later `keyup` must record or activate behavior.

Held modifiers affect later keys. Releasing a key uses the stored down-state rather than reparsing the current focus as a new press.

## Deferred Space activation

[[src/session.rs#pending_space_activation]] records supported button, checkbox, and radio activation that occurs on key release. Focus ownership and control state are checked again before activation.

[[src/session.rs#execute_press_request]] handles complete presses and the separate key-down or key-up paths. [[src/session.rs#record_complete_press_events]] writes the supported keyboard and input event phases.

## Focus transitions

Tab and Shift+Tab use the current supported focus order. Other keys act only on the focused element types that define behavior.

A document replacement clears focus. A pending Space activation cannot retarget because key-up compares the stored target identity with the current focused target. Held modifier state remains session-owned until release.

## Related maps

[[interaction-pipeline]] explains actionability. [[domain#Native control state]] explains control values. [[domain#Native event transcript]] explains the observation boundary.
