# Action transactions

The action layer separates evidence checks from committed page changes. This page maps the shared transaction shape and the points where navigation extends beyond the local page mutation.

## Shared sequence

[[src/session.rs#execute_click_by_locator]] resolves the locator before it enters click behavior. [[src/session.rs#validate_native_click]] checks the target's supported click rules.

[[src/session.rs#CurrentPage#receives_events]] evaluates supported blockers. [[src/session.rs#Session#pointer_action_context]] captures event and target context before a control or navigation effect changes state.

```text
resolve target
  -> collect preflight evidence
  -> local control: validate effect, then commit state and events
  -> navigation: commit local action phases, then load and install a document
```

## Local controls

Text, checkbox, radio, and select operations can validate their complete effect against the current page. [[src/session.rs#Session#finish_pointer_click]] records the final pointer action phases after the control mutation succeeds.

Radio mutations update the selected control and clear other controls in the same group as one page-state operation.

## Navigation actions

Link and form actions combine local pointer phases with [[network-loading]]. The loader keeps document installation atomic, but the complete action can include local scroll, focus, or event changes before a later navigation load fails.

Code and documentation must not describe every navigation action as if all local action state rolls back with document installation. The narrower invariant is that a failed document load preserves the previously installed document and history position.

## Related maps

[[interaction-pipeline]] gives the broader action overview. [[session-state#Commit rule]] owns session mutation. [[decisions#State changes are transactional]] must be read with the navigation distinction above.
