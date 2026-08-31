use std::collections::BTreeMap;
use std::fmt;

use dom_query::{Document, Matcher, NodeData, NodeId, NodeRef};
use sxd_document::dom;
use sxd_xpath::{Context, Factory, Value};

use super::ElementSource;

#[derive(Clone)]
pub(crate) struct SelectorIndex {
    document: Document,
    locator_by_node: BTreeMap<NodeId, usize>,
    mapping_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SelectorQueryError {
    DocumentMapping(String),
    HtmlSerialization,
    InvalidCssSelector,
    InvalidXPathExpression,
    XPathEvaluation(String),
    XPathResultIsNotElements,
}

impl SelectorIndex {
    pub(super) fn new(html: &str, sources: &[ElementSource]) -> Self {
        let document = Document::from(html);
        let candidates = document
            .root()
            .descendants_it()
            .filter(|node| node.node_name().is_some())
            .collect::<Vec<_>>();
        let mut locator_by_node = BTreeMap::new();
        let mut next_candidate = 0;
        let mut mapping_error = None;

        for (source_index, source) in sources.iter().enumerate() {
            if source.content_ordinal.is_none() {
                continue;
            }
            let matched = candidates[next_candidate..]
                .iter()
                .position(|node| node_matches_source(*node, source))
                .map(|offset| next_candidate + offset);
            let Some(candidate_index) = matched else {
                mapping_error = Some(format!(
                    "parsed DOM could not identify source element {}",
                    source.id
                ));
                break;
            };
            locator_by_node.insert(candidates[candidate_index].id, source_index);
            next_candidate = candidate_index + 1;
        }

        Self {
            document,
            locator_by_node,
            mapping_error,
        }
    }

    pub(crate) fn css_matches(&self, selector: &str) -> Result<Vec<usize>, SelectorQueryError> {
        self.require_mapping()?;
        let matcher = Matcher::new(selector).map_err(|_| SelectorQueryError::InvalidCssSelector)?;
        Ok(self
            .document
            .select_matcher(&matcher)
            .nodes()
            .iter()
            .filter_map(|node| self.locator_by_node.get(&node.id).copied())
            .collect())
    }

    pub(crate) fn xpath_matches(&self, expression: &str) -> Result<Vec<usize>, SelectorQueryError> {
        self.require_mapping()?;
        let package = sxd_document::Package::new();
        let document = package.as_document();
        let mut locator_by_element = Vec::new();
        copy_children(
            self.document.root(),
            XPathParent::Root(document.root()),
            document,
            &self.locator_by_node,
            &mut locator_by_element,
        );

        let xpath = Factory::new()
            .build(expression)
            .map_err(|_| SelectorQueryError::InvalidXPathExpression)?
            .ok_or(SelectorQueryError::InvalidXPathExpression)?;
        let value = xpath
            .evaluate(&Context::new(), document.root())
            .map_err(|error| SelectorQueryError::XPathEvaluation(error.to_string()))?;
        let Value::Nodeset(nodes) = value else {
            return Err(SelectorQueryError::XPathResultIsNotElements);
        };

        let mut matches = Vec::new();
        for node in nodes.document_order() {
            let sxd_xpath::nodeset::Node::Element(element) = node else {
                return Err(SelectorQueryError::XPathResultIsNotElements);
            };
            if let Some(locator_index) =
                locator_by_element
                    .iter()
                    .find_map(|(candidate, locator_index)| {
                        (*candidate == element).then_some(*locator_index)
                    })
            {
                matches.push(locator_index);
            }
        }
        Ok(matches)
    }

    pub(crate) fn inner_html(&self, source_index: usize) -> Result<String, SelectorQueryError> {
        let node = self.mapped_node(source_index)?;
        node.try_inner_html()
            .map(|html| html.to_string())
            .ok_or(SelectorQueryError::HtmlSerialization)
    }

    pub(crate) fn inner_html_contains_sensitive_value(
        &self,
        source_index: usize,
    ) -> Result<bool, SelectorQueryError> {
        let node = self.mapped_node(source_index)?;
        Ok(node.descendants_it().any(|descendant| {
            if descendant
                .node_name()
                .is_none_or(|name| name.as_ref() != "input")
            {
                return false;
            }
            let attributes = node_attributes(descendant);
            attributes
                .get("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("password"))
                && attributes.contains_key("value")
        }))
    }

    fn mapped_node(&self, source_index: usize) -> Result<NodeRef<'_>, SelectorQueryError> {
        self.require_mapping()?;
        self.locator_by_node
            .iter()
            .find_map(|(node_id, candidate)| {
                (*candidate == source_index).then(|| self.document.tree.get(node_id))
            })
            .flatten()
            .ok_or_else(|| {
                SelectorQueryError::DocumentMapping(format!(
                    "parsed DOM has no mapped element for source index {source_index}"
                ))
            })
    }

    fn require_mapping(&self) -> Result<(), SelectorQueryError> {
        self.mapping_error.as_ref().map_or(Ok(()), |reason| {
            Err(SelectorQueryError::DocumentMapping(reason.clone()))
        })
    }
}

impl fmt::Debug for SelectorIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectorIndex")
            .field("mapped_elements", &self.locator_by_node.len())
            .field("mapping_error", &self.mapping_error)
            .finish()
    }
}

impl fmt::Display for SelectorQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocumentMapping(reason) => write!(formatter, "{reason}"),
            Self::HtmlSerialization => formatter.write_str("element HTML could not be serialized"),
            Self::InvalidCssSelector => formatter.write_str("CSS selector is invalid"),
            Self::InvalidXPathExpression => formatter.write_str("XPath expression is invalid"),
            Self::XPathEvaluation(reason) => {
                write!(formatter, "XPath evaluation failed: {reason}")
            }
            Self::XPathResultIsNotElements => {
                formatter.write_str("XPath expression did not return only elements")
            }
        }
    }
}

fn node_matches_source(node: NodeRef<'_>, source: &ElementSource) -> bool {
    node.node_name()
        .is_some_and(|name| name.as_ref() == source.tag)
        && node_attributes(node) == source.attributes
}

fn node_attributes(node: NodeRef<'_>) -> BTreeMap<String, String> {
    node.attrs()
        .into_iter()
        .map(|attribute| {
            (
                attribute.name.local.to_string(),
                attribute.value.to_string(),
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
enum XPathParent<'d> {
    Root(dom::Root<'d>),
    Element(dom::Element<'d>),
}

fn copy_children<'d>(
    source_parent: NodeRef<'_>,
    target_parent: XPathParent<'d>,
    target_document: dom::Document<'d>,
    locator_by_node: &BTreeMap<NodeId, usize>,
    locator_by_element: &mut Vec<(dom::Element<'d>, usize)>,
) {
    for child in source_parent.children() {
        let data = child.query(|node| node.data.clone());
        match data {
            Some(NodeData::Element(element)) => {
                let target = target_document.create_element(element.name.local.as_ref());
                for attribute in element.attrs {
                    target.set_attribute_value(
                        attribute.name.local.as_ref(),
                        attribute.value.as_ref(),
                    );
                }
                append_element(target_parent, target);
                if let Some(locator_index) = locator_by_node.get(&child.id) {
                    locator_by_element.push((target, *locator_index));
                }
                copy_children(
                    child,
                    XPathParent::Element(target),
                    target_document,
                    locator_by_node,
                    locator_by_element,
                );
            }
            Some(NodeData::Text { contents }) => {
                if let XPathParent::Element(parent) = target_parent {
                    parent.append_child(target_document.create_text(contents.as_ref()));
                }
            }
            Some(NodeData::Comment { contents }) => match target_parent {
                XPathParent::Root(parent) => {
                    parent.append_child(target_document.create_comment(contents.as_ref()));
                }
                XPathParent::Element(parent) => {
                    parent.append_child(target_document.create_comment(contents.as_ref()));
                }
            },
            Some(NodeData::ProcessingInstruction { target, contents }) => {
                let instruction = target_document
                    .create_processing_instruction(target.as_ref(), Some(contents.as_ref()));
                match target_parent {
                    XPathParent::Root(parent) => parent.append_child(instruction),
                    XPathParent::Element(parent) => parent.append_child(instruction),
                }
            }
            Some(NodeData::Document | NodeData::Fragment | NodeData::Doctype { .. }) | None => {}
        }
    }
}

fn append_element(parent: XPathParent<'_>, child: dom::Element<'_>) {
    match parent {
        XPathParent::Root(parent) => parent.append_child(child),
        XPathParent::Element(parent) => parent.append_child(child),
    }
}

#[cfg(test)]
mod tests {
    use super::{SelectorIndex, SelectorQueryError};
    use crate::page::parse_page_source;

    #[test]
    fn resolves_css_combinators_groups_and_pseudo_classes() {
        let html = r#"
            <main>
                <section class="cards"><button id="first">One</button></section>
                <section class="cards"><button id="second">Two</button></section>
            </main>
        "#;
        let source = parse_page_source(html);
        let selectors = SelectorIndex::new(html, &source.elements);
        let first = source
            .elements
            .iter()
            .position(|element| element.id == "first")
            .unwrap();
        let second = source
            .elements
            .iter()
            .position(|element| element.id == "second")
            .unwrap();

        assert_eq!(
            selectors
                .css_matches("main > section.cards:nth-child(2) button, #first")
                .unwrap(),
            vec![first, second]
        );
    }

    #[test]
    fn resolves_xpath_against_the_same_html_tree() {
        let html = r#"
            <section data-kind="cards">
                <button id="first">One</button>
                <button id="second">Two</button>
            </section>
        "#;
        let source = parse_page_source(html);
        let selectors = SelectorIndex::new(html, &source.elements);
        let second = source
            .elements
            .iter()
            .position(|element| element.id == "second")
            .unwrap();

        assert_eq!(
            selectors
                .xpath_matches("//section[@data-kind='cards']/button[2]")
                .unwrap(),
            vec![second]
        );
    }

    #[test]
    fn rejects_xpath_scalar_results() {
        let html = "<button>One</button>";
        let source = parse_page_source(html);
        let selectors = SelectorIndex::new(html, &source.elements);

        assert_eq!(
            selectors.xpath_matches("count(//button)"),
            Err(SelectorQueryError::XPathResultIsNotElements)
        );
    }

    #[test]
    fn serializes_normalized_inner_html_for_a_mapped_element() {
        let html = r#"<section id="card"><span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note --></section>"#;
        let source = parse_page_source(html);
        let selectors = SelectorIndex::new(html, &source.elements);
        let card = source
            .elements
            .iter()
            .position(|element| element.id == "card")
            .unwrap();

        assert_eq!(
            selectors.inner_html(card).unwrap(),
            r#"<span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note -->"#
        );
    }

    #[test]
    fn detects_sensitive_values_in_normalized_descendants() {
        let html = r#"<div id="safe"></div><div id="secret"><input type="PASSWORD" value="private"></div>"#;
        let source = parse_page_source(html);
        let selectors = SelectorIndex::new(html, &source.elements);
        let safe = source
            .elements
            .iter()
            .position(|element| element.id == "safe")
            .unwrap();
        let secret = source
            .elements
            .iter()
            .position(|element| element.id == "secret")
            .unwrap();
        let input = source
            .elements
            .iter()
            .position(|element| element.tag == "input")
            .unwrap();

        assert!(!selectors.inner_html_contains_sensitive_value(safe).unwrap());
        assert!(
            selectors
                .inner_html_contains_sensitive_value(secret)
                .unwrap()
        );
        assert!(
            !selectors
                .inner_html_contains_sensitive_value(input)
                .unwrap()
        );
    }
}
