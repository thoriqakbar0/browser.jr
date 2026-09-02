# Interaction pipeline

An interaction resolves a target and checks its preflight evidence. Local controls validate before commit. Navigation can commit local action phases before loading completes.

## Resolve the target

[[src/locator.rs#Locator]] represents reusable role, text, label, placeholder, alt, title, test ID, CSS, and XPath queries. [[src/snapshot.rs#InteractiveElementRef]] represents one element from the latest interactive capture.

[[src/session.rs#ResolvedLocator]] joins a successful locator result to the installed page. Non-positioned action and read paths require strict selection unless the request defines another rule.

## Check actionability

Pointer-like actions use supported visibility, stability, geometry, scrolling, and receives-events evidence. [[src/session.rs#action_point]] selects the center of the target's supported visible intersection.

Keyboard and focus operations use narrower evidence because they do not claim pointer dispatch. Text and selection behavior delegates to the pure transformations in `src/keyboard.rs`.

## Commit state

Local control actions validate their complete local effect before commit. Checkbox, radio, select, and text actions then update live native control state.

Navigation actions have a narrower boundary. They can commit scroll, focus, pointer state, and native events before URL preparation or loading finishes. [[action-transactions]] maps this ordering. [[network-loading]] maps document installation after the local phases.

## Record native events

Supported actions append [[src/session.rs#DomEvent]] records after the relevant phases commit. [[src/session.rs#DomEventType]] defines the recorded subset.

The transcript does not execute page scripts. [[src/session.rs#TakeDomEvents]] drains the session-owned queue and returns the records with their source document epoch.

## Failure boundary

Unsupported or indeterminate preflight evidence blocks the action before its local commit. A later navigation error can occur after local action phases have committed. [[decisions#Unsupported evidence blocks results]] owns the evidence rule, and [[action-transactions]] owns the navigation distinction.
