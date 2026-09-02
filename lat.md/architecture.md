# Implemented architecture

browser.jr is one Rust library and CLI that turns bounded static web input into inspectable evidence, stateful actions, rule results, and optional solid-box screenshots.

## Execution path

External adapters parse input, create typed requests, and send them to one mutable session that owns the current browsing state.

The CLI enters through [[src/cli.rs#run_cli_with_input]]. Package callers use the public exports in `src/lib.rs`. Both paths converge on [[src/session.rs#Session]] and the request/reply contract in [[src/session.rs#SessionRequest]].

A typical page operation follows this path:

```text
CLI or package request
  -> Session
  -> bounded loader
  -> parsed page and supported style/layout evidence
  -> snapshot, action result, rule result, or paint scene
  -> human or JSON presentation
```

Presentation remains outside the engine state. Human and JSON formatting live in `cli.rs`, `cli_output.rs`, and `cli_session_json.rs`.

## Session ownership

One session is the transaction boundary for a page, history, viewport, snapshot identity, focus, held keys, pointer state, native event records, and network authority.

[[src/session.rs#Session]] owns these values directly. [[src/session.rs#Session#execute]] accepts only a type that implements [[src/session.rs#SessionRequest]], so each request selects its reply type at compile time.

The implementation is deliberately single-owner and synchronous at its core. It has no actor, mailbox, service graph, or shared session lock because no current caller needs concurrent commands within one session.

## Page loading

The loader accepts public HTTP and HTTPS by default and adds explicit loopback access only when the session receives that capability.

[[src/loading.rs#NetworkAccess]] stores the authority. [[src/loading.rs#load_html]] applies bounded DNS, connection, redirect, header, body, and timeout rules. The transport connects only to endpoints approved for the requested URL.

A redirect cannot cross between public and loopback network modes, and HTTPS cannot downgrade to HTTP. A failed load leaves the previously installed page unchanged.

## Page interpretation

Page interpretation builds normalized HTML ancestry, extracts metadata and text, applies the supported CSS subset, and produces semantic and layout inputs from the same source tree.

[[src/page.rs#layout_input_from_html]] creates layout inputs. [[src/page/interactive.rs#page_semantics_from_html_with_viewport]] creates semantic, control, focus, visibility, and geometry evidence. [[src/page/selectors.rs#SelectorIndex]] owns CSS and XPath queries over normalized ancestry.

The `page` module is split by evidence kind:

- `dom.rs` owns the html5ever tree sink.
- `style.rs` owns supported declarations, cascade precedence, and style blockers.
- `interactive.rs` owns accessibility projection, controls, action evidence, and hit-test candidates.
- `selectors.rs` owns document queries and source-element mapping.
- `visibility.rs` owns visibility and actionability evidence.
- `paint.rs` owns supported paint commands.

## Layout and rules

The package design-lint kernel uses a compiled dependency order for `x`, `width`, and derived `right`, then evaluates pure rules over immutable observations.

Loaded-page geometry is computed in `page.rs` and stored on the current page. Separately, [[src/layout.rs#LayoutProgram#compile]] validates the package mutation program, and [[src/layout.rs#LayoutKernel]] runs clean layout and transactional mutation batches.

[[src/rules.rs#evaluate_horizontal_overflow]] and [[src/rules.rs#evaluate_max_element_width]] consume snapshots. Rule results distinguish a comparison from a blocked evaluation, so missing evidence cannot silently become a pass.

## Snapshots and references

A snapshot is immutable evidence, while an interactive reference is a short-lived handle tied to one document epoch and one snapshot identity.

[[src/snapshot.rs#Snapshot]] stores layout observations. [[src/snapshot.rs#InteractiveSnapshot]] and [[src/snapshot.rs#AccessibilitySnapshot]] expose agent-oriented and tree-oriented projections.

[[src/snapshot.rs#InteractiveElementRef]] contains the document epoch, snapshot identifier, and document-order ordinal displayed as `@eN`. A new capture replaces the usable reference set. Successful document replacement invalidates references from the prior epoch.

Locator requests do not depend on stored `@eN` references. They resolve against the current document when each request executes.

## Stateful actions

Actions validate current evidence before committing focus, scroll, pointer, control, history, or document changes.

The session resolves a locator or reference, checks the relevant visibility, stability, geometry, and receives-events evidence, computes prospective scrolling, then commits state only after all required checks pass.

Text controls, checkboxes, radios, selects, links, GET forms, focus, hover, and the bounded keyboard model each keep their live state in the session-owned current page. Failed local actions preserve current state and snapshot references. A navigation load failure preserves the installed document and history position, but earlier scroll, focus, and event phases can remain.

## Native event transcript

Supported actions append data-minimized native event records instead of executing JavaScript or dispatching events to page handlers.

[[src/session.rs#DomEvent]] stores event type, document epoch, target identity, ancestor path, bubbling, composition, and optional related-target identity. It does not store input values. [[src/session.rs#TakeDomEvents]] drains the queue.

This transcript is an observation boundary. It provides portable action evidence without claiming a script runtime or complete browser event dispatch.

## Screenshot boundary

Screenshot preparation is part of the session, but raster process ownership stays outside the core browsing state and starts only when a caller renders a supported scene.

[[src/session.rs#PrepareScreenshot]] selects viewport, full-page, locator, or rectangle bounds and requests supported paint commands. [[src/screenshot.rs#PaintScene]] carries the capture bounds and ordered fills.

[[src/screenshot.rs#OnDemandRasterProcess]] lazily creates a [[src/screenshot.rs#RasterProcess]]. The current software implementation supports bounded solid rectangles and source-over RGBA composition. Unsupported visible content blocks capture instead of producing an incomplete image.

## Agent-browser plugin boundary

The npm package is a namespaced command plugin, not a replacement browser engine or CDP provider.

[`runBrowserJrSession`](../plugin/cli.mjs) runs a bounded command batch in one native JSON session. [`serve`](../plugin/cli.mjs) exposes an authenticated loopback relay, and [`exchangeWithRelay`](../plugin/cli.mjs) sends one serialized command to that warm session.

The plugin resolves the native executable from an explicit request, `BROWSER_JR_BIN`, a packaged release binary, or `browser-jr` on `PATH`. Native binary distribution remains a release concern outside the Rust session model.

## Detailed maps

These pages trace each architecture boundary through its primary source path.

- [[runtime-flow]] follows a request from adapter input to presentation.
- [[session-state]] maps mutable ownership and identity lifetime.
- [[network-loading]] maps URL authority and bounded fetching.
- [[page-pipeline]] maps normalized HTML to supported evidence.
- [[locator-resolution]] maps queries to current-document identities.
- [[interaction-pipeline]] and [[action-transactions]] map stateful commands.
- [[keyboard-state]] maps held keys, selection, and activation.
- [[evidence-and-snapshots]] maps support state, snapshots, and references.
- [[screenshot-pipeline]] maps capture preparation and rasterization.
- [[session-wire]] maps the human and JSON session adapters.
- [[plugin-protocol]] maps the agent-browser integration boundary.
- [[benchmark-harness]] maps correctness checks and timed work.
- [[release-and-packaging]] maps the npm artifact and native dependency.
- [[verification-map]] maps claims to runtime evidence and triage.
