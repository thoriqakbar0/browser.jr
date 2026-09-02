# Session state

One [[src/session.rs#Session]] owns every mutable value that can affect a later browser command. The core does not share page state through globals, actors, or adapter-owned caches.

## Owned state

The session owns the current page, [[src/session.rs#NavigationHistory]], viewport, [[src/session.rs#LatestInteractiveSnapshot]], [[src/session.rs#KeyboardState]], pointer and focus state, native event records, identity counters, and network authority.

[[src/session.rs#CurrentPage]] groups the installed document with its live controls, styles, semantics, geometry, scroll position, URL, and title. A session may exist without a current page.

## Document installation

Navigation prepares a candidate document before replacing `CurrentPage`. Failed loading, parsing, or installation keeps the previous page and history position.

A successful document replacement advances the document epoch. Reload advances the epoch without adding a history entry. Back and forward install the selected history entry only after the target document succeeds.

## Identity lifetimes

[[src/session.rs#IdentityCounters]] separates document identity from capture identity. [[src/snapshot.rs#InteractiveElementRef]] contains both the document epoch and the interactive snapshot identifier.

A new interactive capture replaces the active reference set. A new document invalidates every reference from the prior epoch. Locators do not share this lifetime because they resolve against the current document for each request.

## Commit rule

Actions compute their required evidence and prospective state before mutating `CurrentPage`. A failed action keeps the current focus, scroll, control state, history, and reference set unless the owning operation documents a narrower effect.

[[decisions#State changes are transactional]] records this constraint. [[interaction-pipeline]] shows the action sequence that applies it.

## Adapter ownership

Session mode and the agent-browser relay keep one native session alive, but they do not own browser-domain state. They own transport lifetime and command ordering only. [[plugin-protocol]] maps that boundary.
