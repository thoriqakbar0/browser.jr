# Key design decisions

These decisions describe implemented constraints that future changes should preserve unless the project deliberately replaces them.

## Unsupported evidence blocks results

browser.jr reports unavailable evidence instead of approximating it or treating absence as success.

[[src/snapshot.rs#ObservationCell]] distinguishes available, unsupported, indeterminate, and unstable observations. [[src/rules.rs#RuleResult]] separates compared results from blocked results.

This fail-closed rule is why unsupported layout, visibility, paint, or actionability stops the dependent operation. It protects the tool from making confident claims beyond its static engine boundary.

## Requests select their reply types

The core uses typed request structs with associated replies instead of a command enum paired with a broad reply enum.

[[src/session.rs#SessionRequest]] defines the association, and [[src/session.rs#Session#execute]] delegates execution through it. Invalid request/reply combinations therefore cannot enter the engine as runtime states.

Wire commands stay in adapters. They must become typed requests before they can change session state.

## State changes are transactional

A failed operation preserves the last committed state rather than leaving partial layout, scrolling, control, history, or document changes.

[[src/layout.rs#LayoutKernel#apply_mutations]] computes a candidate field state before commit. Session actions similarly validate actionability and prospective scrolling before they mutate the page.

This rule makes failures inspectable and keeps retries predictable.

## One session owns mutable browser state

The initial architecture concentrates mutable browser state in one session instead of distributing it through actors or shared services.

[[src/session.rs#Session]] owns the page, history, viewport, identities, keyboard state, and event queue. This keeps command ordering explicit and avoids locks for concurrency that no current caller requires.

Adapters may keep a session alive, but they do not own its domain state.

## Network authority is explicit and narrow

Loopback loading is a session capability; private and reserved network access is not inferred from a caller-selected URL.

[[src/loading.rs#NetworkAccess]] defaults to public-only. The loader validates every redirect and connects only to an approved bounded endpoint set.

The policy prevents a public request from redirecting into loopback or private infrastructure and prevents a loopback-enabled request from changing network mode during redirects.

## The engine models a supported static subset

browser.jr prefers a small stated compatibility boundary over a broad but unreliable imitation of a consumer browser.

HTML normalization, selected CSS, static block and fixed geometry, accessibility semantics, native controls, and bounded navigation are modeled directly. JavaScript, complete paint, and unsupported CSS remain outside the claim.

Dependent evidence is marked unavailable when unsupported syntax or page features could change the answer.

## Layout dependencies are compiled

Incremental layout follows a declared field program rather than an ad hoc dirty-node queue.

[[src/layout.rs#LayoutProgram]] owns field order and dependencies. [[src/layout.rs#LayoutKernel]] propagates work only when a complete observed value changes and commits a batch only after every recomputation succeeds.

The current slice is intentionally small: `x` and `width` are inputs, and `right` is derived. More identities and insertion or removal behavior remain future work.

## Snapshots are immutable and references expire

Captured evidence never changes in place, and action handles cannot silently retarget after another capture or navigation.

[[src/snapshot.rs#SnapshotId]] identifies captures. [[src/snapshot.rs#InteractiveElementRef]] binds a displayed ordinal to both snapshot and document identity.

This trades long-lived convenience for deterministic evidence and explicit stale-reference failures.

## Rasterization is lazy and bounded

Normal inspection and actions do not start a renderer, and a screenshot cannot allocate or paint unbounded work.

[[src/screenshot.rs#OnDemandRasterProcess]] starts its process on first render. [[src/screenshot.rs#MAX_SCREENSHOT_PIXELS]] and [[src/screenshot.rs#MAX_SCREENSHOT_PAINT_PIXELS]] bound image and fill work before allocation.

Paint preparation blocks unsupported visible content, so the PNG boundary does not hide missing text, images, controls, stylesheets, clipping, or effects.

## Agent-browser integration uses commands, not CDP

browser.jr integrates through `agent-browser.plugin.v1` command capabilities because it does not expose the Chrome DevTools Protocol.

[`pluginMain`](../plugin/cli.mjs) advertises `browserjr.session` and `browserjr.command`. It does not register a `browser.provider`, and it does not replace standard `agent-browser` commands.

The warm relay exists to reuse one browser.jr session while preserving an authenticated, loopback-only boundary. It serializes commands because the native JSON session has one ordered stdin/stdout stream.
