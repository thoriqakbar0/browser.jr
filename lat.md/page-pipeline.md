# Page pipeline

The page pipeline derives supported DOM, style, layout, semantic, selector, visibility, and paint evidence from one HTML source.

## Normalize the source

[[src/page.rs#parse_page_source]] uses the tree sink in `src/page/dom.rs` to normalize ancestry and collect element source data. Metadata and rendered text come from this normalized source rather than from separate parsers.

## Compute supported style and geometry

[[src/page.rs#page_computed_styles]] applies the supported declaration and cascade subset from `src/page/style.rs`. It also records unsupported style evidence that can block dependent operations.

[[src/page.rs#layout_input_from_html]] prepares layout inputs. The page module then resolves supported static and fixed boxes against the viewport and containing blocks.

The package layout kernel in `src/layout.rs` is separate. It evaluates the small `x`, `width`, and `right` mutation program used by design-lint checks. [[decisions#Layout dependencies are compiled]] explains that choice.

## Build interactive evidence

[[src/page/interactive.rs#page_semantics_from_html_with_viewport]] derives accessibility roles, names, control state, focus order, action candidates, and source identities.

[[src/page/selectors.rs#SelectorIndex]] owns CSS and XPath queries over normalized ancestry. `src/page/visibility.rs` owns the supported visibility and actionability inputs. `src/page/paint.rs` emits the supported paint commands.

## One source, several projections

Interactive snapshots, accessibility snapshots, locators, actions, reads, and screenshot preparation consume projections from the same installed page.

Unsupported source features remain attached to the affected evidence. They do not disappear when a later projection omits unrelated fields.

## Related maps

These pages continue the same execution path from another ownership boundary.

- [[evidence-and-snapshots]] explains evidence support states.
- [[interaction-pipeline]] explains target resolution and action commits.
- [[screenshot-pipeline]] explains paint-scene completeness.
