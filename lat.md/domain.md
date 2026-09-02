# Domain concepts and invariants

This map connects the canonical terms in the project glossary to the source ownership and invariants that matter during implementation.

The repository [glossary](../glossary.md) owns user-facing definitions. These sections explain how the concepts relate inside the current code.

## Session and current page

A session is the only mutable owner of browsing state, and its current page is replaced atomically after successful navigation.

[[src/session.rs#Session]] holds the current page, history, viewport, snapshot state, keyboard state, event queue, and network access. The session can exist before any page is open.

A failed open, reload, history move, link activation, or form submission keeps the prior page and history position.

## Document epoch

A document epoch separates identities from different installed documents even when the URL or markup is the same.

The session advances the epoch when it installs a fresh document. [[src/snapshot.rs#InteractiveElementRef]] carries that epoch so a reference from an older document cannot act on the current document.

Reload creates a new epoch but does not add a history entry.

## Locator and reference

A locator is a reusable current-document query, while a reference is a snapshot-scoped identity for one projected element.

[[src/locator.rs#Locator]] covers role, text, label, placeholder, alt, title, test ID, CSS, and XPath variants. Non-positioned action and read paths require strict resolution.

A locator resolves again for every request. An `@eN` reference is valid only for the latest interactive snapshot from the same document epoch.

## Snapshot and evidence

A snapshot freezes selected observations so rules and callers can inspect one stable result without reading mutable page state.

[[src/snapshot.rs#EvidenceRef]] links rule evidence to a snapshot and semantic element. Interactive and accessibility snapshots project different views from the same current document semantics.

Evidence carries both a value and its support status. This distinction is part of the result, not an adapter-only diagnostic.

## Support state

Support state records whether browser.jr has enough modeled evidence to make a claim.

[[src/snapshot.rs#ObservationCell]] represents available, unsupported, indeterminate, or unstable evidence. A dependent rule or action must preserve that reason when it blocks.

`Unsupported` means the feature is outside the implemented compatibility boundary. It does not mean the underlying page is invalid.

## Layout program and mutation batch

The layout program defines legal recomputation order, while a mutation batch applies supported input changes as one candidate transaction.

[[src/layout.rs#LayoutProgram]] currently orders `x`, `width`, and `right`. [[src/layout.rs#LayoutMutation]] changes one supported input, and [[src/layout.rs#LayoutKernel#apply_mutations]] evaluates the whole batch before commit.

Clean and incremental layout use the same field program so tests can compare their final snapshots.

## Actionability and action point

Actionability is the evidence an action requires before it may commit state, and the action point is the center of the target's supported visible intersection.

Pointer-like actions use visibility, stability, geometry, prospective scroll, and receives-events evidence. Focus and keyboard paths have narrower boundaries because they do not claim pointer dispatch.

Unsupported target geometry does not become proof of complete hit testing.

## Native control state

Supported controls keep live values and checked or selected state in the current document model rather than rewriting source HTML.

The interactive page model owns text, checkbox, radio, and select state in [[src/page/interactive.rs#ControlState]]. Radio groups maintain exclusivity by form owner and non-empty name.

Snapshots and typed reads expose current control state. Navigation replaces it with state derived from the newly loaded document.

## Native event transcript

The native event transcript is a session-owned observation log for supported action phases, not a JavaScript event system.

[[src/session.rs#DomEventType]] defines the recorded subset. Records contain structural target identity and event metadata but omit control values and do not invoke handlers.

Taking events drains the queue. Navigation preserves already recorded events and their source document epoch.

## Paint scene and raster image

A paint scene is browser.jr-owned drawing evidence; a raster image is a complete bounded RGBA result produced from that scene.

[[src/screenshot.rs#PaintScene]] contains capture bounds and ordered commands. [[src/screenshot.rs#RasterImage]] validates non-empty dimensions and exact byte length.

The current paint vocabulary contains solid rectangle fills only. Screenshot preparation blocks when unsupported visible content would make that scene incomplete.

## Plugin session and relay

A plugin session batch starts one bounded native session for an ordered list, while a relay keeps one authenticated native session warm across plugin invocations.

[`runBrowserJrSession`](../plugin/cli.mjs) owns the batch boundary. [`serve`](../plugin/cli.mjs) owns relay lifecycle, loopback binding, token authentication, command serialization, and idle shutdown.

Both are transport adapters around the same browser.jr JSON session protocol. Neither adds CDP or a second browser state model.

## Deeper concept maps

These pages expand the state and identity concepts used by the domain model.

- [[session-state]] connects page installation, history, focus, and identity counters.
- [[locator-resolution]] separates reusable locators from capture-scoped references.
- [[action-transactions]] describes the boundary between local action state and navigation.
- [[keyboard-state]] describes held-key and selection state.
- [[evidence-and-snapshots]] describes support state and immutable captures.
- [[plugin-protocol]] describes transport state that remains outside the browser domain.
