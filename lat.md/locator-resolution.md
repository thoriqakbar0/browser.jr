# Locator resolution

Locator resolution converts a query into current-document source identities. Semantic matching and document selectors use separate engines, then converge before session selection.

The supported syntax, locator kinds, matching rules, and user-visible errors belong to [query elements](../inspection/query-elements.md). This page maps the internal resolution path.

## Query representation

[[src/locator.rs#Locator]] is the common query type. Semantic variants evaluate [[src/locator.rs#LocatorCandidate]] values produced by the page model.

CSS parsing lives in [[src/locator/css.rs#CssSelector#parse]]. XPath parsing lives in `src/locator/xpath.rs`. These parsers produce validated query representations without session access.

## Source mapping

[[src/page/selectors.rs#SelectorIndex#css_matches]] and [[src/page/selectors.rs#SelectorIndex#xpath_matches]] return indices into the normalized page source. Semantic candidates carry the same source identity.

The common identity lets query results feed reads, actions, snapshots, and screenshot preparation without matching the document again.

## Session selection

[[src/session.rs#Session#locator_matches_for]] gathers matches from the current page. A later helper applies the request's selection policy and converts the chosen source identity into the operation-specific target.

The session does not store locator results between requests. Each request resolves against the current document. [[evidence-and-snapshots#References]] explains why interactive references have a different lifetime.

## Error boundaries

Syntax errors remain in the CSS or XPath parser. Semantic evidence errors remain in the page projection. Selection and operation errors remain in the session request.

This separation preserves enough information for the owning product document to define the visible error without coupling every query engine to CLI output.
