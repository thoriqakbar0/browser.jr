use std::cell::Cell;
use std::collections::BTreeMap;

use html5ever::parse_document;
use html5ever::tendril::StrTendril;
use html5ever::tendril::TendrilSink;
use html5ever::tokenizer::{
    BufferQueue, StartTag, TagToken, Token, TokenSink, TokenSinkResult, Tokenizer,
};

use crate::locator::css::CssNode;
use crate::{BoundingBox, ElementInput, LayoutInput};

mod dom;
mod interactive;
mod paint;
mod selectors;
mod style;
mod visibility;

use style::computed_styles;

pub(crate) use interactive::{
    AccessibilityNodeSource, ControlState, InteractiveAction, InteractiveElementSource,
    LocatorElementSource, RadioGroup, SelectValueError, SequentialFocusSource, TextValueError,
    page_semantics_from_html_with_viewport,
};
#[cfg(test)]
pub(crate) use interactive::{
    CheckedState, TextValueState, interactive_elements_from_html, page_semantics_from_html,
    semantic_elements_from_html,
};
pub(crate) use paint::paint_commands_from_html;
pub(crate) use selectors::{SelectorIndex, SelectorQueryError};

#[derive(Clone, Debug)]
struct ElementSource {
    id: String,
    tag: String,
    attributes: BTreeMap<String, String>,
    parent: Option<usize>,
    content_ordinal: Option<usize>,
    content: ElementContentSource,
}

#[derive(Clone, Debug)]
struct ElementContentSource {
    text: String,
    direct_text: String,
    children: Vec<ElementChildSource>,
}

#[derive(Clone, Debug)]
enum ElementChildSource {
    Element(usize),
    Text(String),
}

#[derive(Clone, Copy, Debug)]
struct ContainingBlock {
    x: i64,
    width: u64,
}

#[derive(Clone, Copy, Debug)]
struct ResolvedBox {
    border_x: i64,
    border_width: u64,
    content_x: i64,
    content_width: u64,
}

#[derive(Debug, Default)]
struct PageStyles {
    has_linked_stylesheet: bool,
    stylesheets: Vec<String>,
    stylesheet_error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct BoundingContainingBlock {
    x: i64,
    y: i64,
    width: u64,
}

#[derive(Clone, Copy, Debug)]
struct VerticalEdges {
    padding_top: u64,
    padding_bottom: u64,
    border_top: u64,
    border_bottom: u64,
}

impl VerticalEdges {
    fn total(self, id: &str) -> Result<u64, String> {
        self.padding_top
            .checked_add(self.padding_bottom)
            .and_then(|value| value.checked_add(self.border_top))
            .and_then(|value| value.checked_add(self.border_bottom))
            .ok_or_else(|| format!("vertical size overflows for {id}"))
    }

    fn content_offset(self, id: &str) -> Result<u64, String> {
        self.border_top
            .checked_add(self.padding_top)
            .ok_or_else(|| format!("vertical size overflows for {id}"))
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedBoundingBox {
    border: BoundingBox,
    content_x: i64,
    content_y: i64,
    content_width: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PositioningScheme {
    Normal,
    Fixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FlowPlacement {
    InFlow { next_y: i64 },
    OutOfFlow,
}

#[derive(Debug, Default)]
struct PageMetadata {
    styles: PageStyles,
    title_seen: bool,
    title: String,
    readable_text: String,
    next_content_ordinal: usize,
}

struct PageSourceCollector<'a> {
    elements: &'a mut Vec<ElementSource>,
    content_ancestors: &'a mut Vec<usize>,
    metadata: &'a mut PageMetadata,
    explicit_body: bool,
}

#[derive(Clone, Copy, Default)]
struct TraversalContext {
    parent: Option<usize>,
    stylesheet: Option<usize>,
    captures_title: bool,
    inert: bool,
}

struct ParsedPageSource {
    elements: Vec<ElementSource>,
    styles: PageStyles,
    has_stylesheet: bool,
    title: String,
    readable_text: String,
    parse_error: Option<String>,
}

#[derive(Default)]
struct ExplicitBodySink {
    found: Cell<bool>,
}

impl TokenSink for ExplicitBodySink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        if let TagToken(tag) = token
            && tag.kind == StartTag
            && tag.name.as_ref() == "body"
        {
            self.found.set(true);
        }
        TokenSinkResult::Continue
    }
}

fn parse_page_source(html: &str) -> ParsedPageSource {
    let explicit_body = has_explicit_body(html);
    let dom = parse_document(dom::Dom::default(), Default::default()).one(StrTendril::from(html));
    let mut elements = Vec::new();
    let mut content_ancestors = Vec::new();
    let mut metadata = PageMetadata::default();
    {
        let mut collector = PageSourceCollector {
            elements: &mut elements,
            content_ancestors: &mut content_ancestors,
            metadata: &mut metadata,
            explicit_body,
        };
        for child in dom.document.children.borrow().iter() {
            collector.collect(child, TraversalContext::default());
        }
    }

    let has_stylesheet =
        metadata.styles.has_linked_stylesheet || !metadata.styles.stylesheets.is_empty();
    ParsedPageSource {
        elements,
        has_stylesheet,
        styles: metadata.styles,
        title: metadata.title,
        readable_text: metadata.readable_text,
        parse_error: None,
    }
}

fn has_explicit_body(html: &str) -> bool {
    let input = BufferQueue::default();
    input.push_back(StrTendril::from(html));
    let tokenizer = Tokenizer::new(ExplicitBodySink::default(), Default::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.found.get()
}

impl PageSourceCollector<'_> {
    fn collect(&mut self, node: &dom::Handle, context: TraversalContext) {
        match &node.data {
            dom::NodeData::Element {
                name, attributes, ..
            } => {
                let tag = name.local.to_string();
                let attributes = attributes
                    .borrow()
                    .iter()
                    .map(|attribute| {
                        (
                            attribute.name.local.to_string(),
                            attribute.value.to_string(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                self.collect_element(node, tag, attributes, context);
            }
            dom::NodeData::Text(text) => self.collect_text(text.borrow().as_ref(), context),
            dom::NodeData::Document | dom::NodeData::Other => {}
        }
    }

    fn collect_element(
        &mut self,
        node: &dom::Handle,
        tag: String,
        attributes: BTreeMap<String, String>,
        context: TraversalContext,
    ) {
        let stylesheet = self.record_styles(&tag, &attributes, context.stylesheet);
        let index = self.push_element(tag.clone(), attributes, context.parent);
        self.separate_text(&tag);
        self.content_ancestors.push(index);
        let child_context = TraversalContext {
            parent: Some(index),
            stylesheet,
            captures_title: self.capture_title(&tag),
            inert: context.inert || is_inert_text_element(&tag),
        };
        for child in node.children.borrow().iter() {
            self.collect(child, child_context);
        }
        self.content_ancestors.pop();
        self.separate_text(&tag);
    }

    fn push_element(
        &mut self,
        tag: String,
        attributes: BTreeMap<String, String>,
        parent: Option<usize>,
    ) -> usize {
        let content_ordinal = (is_content_element(&tag) || (tag == "body" && self.explicit_body))
            .then(|| {
                self.metadata.next_content_ordinal += 1;
                self.metadata.next_content_ordinal
            });
        let id = attributes.get("id").cloned().unwrap_or_else(|| {
            content_ordinal.map_or_else(|| tag.clone(), |ordinal| format!("{tag}[{ordinal}]"))
        });
        let index = self.elements.len();
        if let Some(parent) = parent {
            self.elements[parent]
                .content
                .children
                .push(ElementChildSource::Element(index));
        }
        self.elements.push(ElementSource {
            id,
            tag,
            attributes,
            parent,
            content_ordinal,
            content: ElementContentSource {
                text: String::new(),
                direct_text: String::new(),
                children: Vec::new(),
            },
        });
        index
    }

    fn record_styles(
        &mut self,
        tag: &str,
        attributes: &BTreeMap<String, String>,
        inherited: Option<usize>,
    ) -> Option<usize> {
        if tag == "link" && is_stylesheet_link(attributes) {
            self.metadata.styles.has_linked_stylesheet = true;
        }
        if tag != "style" {
            return inherited;
        }
        record_style_support(attributes, &mut self.metadata.styles);
        let index = self.metadata.styles.stylesheets.len();
        self.metadata.styles.stylesheets.push(String::new());
        Some(index)
    }

    fn capture_title(&mut self, tag: &str) -> bool {
        if tag != "title" || self.metadata.title_seen {
            return false;
        }
        self.metadata.title_seen = true;
        true
    }

    fn collect_text(&mut self, text: &str, context: TraversalContext) {
        if context.captures_title {
            self.metadata.title.push_str(text);
        }
        if let Some(index) = context.stylesheet {
            self.metadata.styles.stylesheets[index].push_str(text);
        }
        if !context.inert {
            self.metadata.readable_text.push_str(text);
            append_rendered_text(self.elements, self.content_ancestors, text);
        }
    }

    fn separate_text(&mut self, tag: &str) {
        if separates_text(tag) {
            self.metadata.readable_text.push(' ');
            append_rendered_text(self.elements, self.content_ancestors, " ");
        }
    }
}

fn is_stylesheet_link(attributes: &BTreeMap<String, String>) -> bool {
    attributes.get("rel").is_some_and(|value| {
        value
            .split_ascii_whitespace()
            .any(|part| part.eq_ignore_ascii_case("stylesheet"))
    })
}

fn append_rendered_text(elements: &mut [ElementSource], ancestors: &[usize], text: &str) {
    for index in ancestors {
        elements[*index].content.text.push_str(text);
    }
    if let Some(index) = ancestors.last().copied() {
        elements[index].content.direct_text.push_str(text);
        match elements[index].content.children.last_mut() {
            Some(ElementChildSource::Text(current)) => current.push_str(text),
            Some(ElementChildSource::Element(_)) | None => elements[index]
                .content
                .children
                .push(ElementChildSource::Text(text.into())),
        }
    }
}

fn record_style_support(attributes: &BTreeMap<String, String>, styles: &mut PageStyles) {
    let unsupported_media = attributes.get("media").is_some_and(|media| {
        !media.is_empty()
            && !media.eq_ignore_ascii_case("all")
            && !media.eq_ignore_ascii_case("screen")
    });
    let unsupported_type = attributes
        .get("type")
        .is_some_and(|value| !value.is_empty() && !value.eq_ignore_ascii_case("text/css"));
    if styles.stylesheet_error.is_none() && unsupported_media {
        styles.stylesheet_error = Some("style media conditions are not implemented".into());
    } else if styles.stylesheet_error.is_none() && unsupported_type {
        styles.stylesheet_error = Some("non-CSS style elements are not implemented".into());
    }
}

fn page_computed_styles(
    elements: &[ElementSource],
    styles: &PageStyles,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    if styles.has_linked_stylesheet {
        Err("linked stylesheet loading is not implemented".into())
    } else if let Some(error) = &styles.stylesheet_error {
        Err(error.clone())
    } else {
        computed_styles(elements, &styles.stylesheets)
    }
}

pub(crate) fn layout_input_from_html(html: &str, viewport_width: u64) -> LayoutInput {
    let ParsedPageSource {
        elements: sources,
        styles,
        parse_error,
        ..
    } = parse_page_source(html);
    let computed = page_computed_styles(&sources, &styles);
    let mut resolved = Vec::<Result<Option<ResolvedBox>, String>>::with_capacity(sources.len());
    let mut elements = Vec::with_capacity(sources.len() + usize::from(parse_error.is_some()));

    for (index, source) in sources.iter().enumerate() {
        let layout = containing_block(source.parent, &sources, &resolved, viewport_width).and_then(
            |parent| {
                if source.content_ordinal.is_none() {
                    return Ok(None);
                }
                computed.as_ref().map_err(Clone::clone).and_then(|styles| {
                    resolve_horizontal_box(source, &styles[index], parent, viewport_width).map(Some)
                })
            },
        );

        if source.content_ordinal.is_some() {
            match &layout {
                Ok(Some(layout)) => elements.push(ElementInput::supported(
                    source.id.clone(),
                    layout.border_x,
                    layout.border_width,
                )),
                Ok(None) => unreachable!("content elements resolve a layout result"),
                Err(reason) => {
                    elements.push(ElementInput::unsupported(source.id.clone(), reason.clone()))
                }
            }
        }
        resolved.push(layout);
    }

    if let Some(error) = parse_error {
        elements.push(ElementInput::unsupported(
            "document",
            format!("HTML tokenization reported: {error}"),
        ));
    }
    if elements.is_empty() {
        elements.push(ElementInput::unsupported(
            "document",
            "the page has no measurable content elements",
        ));
    }

    LayoutInput {
        viewport_width,
        elements,
    }
}

fn containing_block(
    mut parent: Option<usize>,
    sources: &[ElementSource],
    resolved: &[Result<Option<ResolvedBox>, String>],
    viewport_width: u64,
) -> Result<ContainingBlock, String> {
    while let Some(index) = parent {
        match &resolved[index] {
            Ok(Some(parent)) => {
                return Ok(ContainingBlock {
                    x: parent.content_x,
                    width: parent.content_width,
                });
            }
            Ok(None) => parent = sources[index].parent,
            Err(reason) => return Err(reason.clone()),
        }
    }
    Ok(ContainingBlock {
        x: 0,
        width: viewport_width,
    })
}

fn resolve_horizontal_box(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    parent: ContainingBlock,
    viewport_width: u64,
) -> Result<ResolvedBox, String> {
    reject_unsupported_geometry(properties)?;

    match properties.get("position").map(String::as_str) {
        Some("fixed") => resolve_fixed_box(source, properties, viewport_width),
        None | Some("static") => resolve_normal_box(source, properties, parent),
        Some(value) => Err(format!("position:{value} layout is not implemented")),
    }
}

impl CssNode for ElementSource {
    fn css_tag(&self) -> &str {
        &self.tag
    }

    fn css_attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }

    fn css_parent(&self) -> Option<usize> {
        self.parent
    }
}

fn resolve_fixed_box(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    viewport_width: u64,
) -> Result<ResolvedBox, String> {
    let left = required_length(properties, "left")?;
    let width = required_non_negative_length(properties, "width")?;
    resolve_box_model(
        source,
        properties,
        ContainingBlock {
            x: 0,
            width: viewport_width,
        },
        left,
        0,
        Some(width),
    )
}

fn resolve_bounding_boxes(
    sources: &[ElementSource],
    has_stylesheet: bool,
    viewport_width: u64,
) -> Vec<Result<BoundingBox, String>> {
    if has_stylesheet {
        return sources
            .iter()
            .map(|_| Err("linked and embedded stylesheets are not implemented".into()))
            .collect();
    }

    let mut children = vec![Vec::new(); sources.len()];
    let mut roots = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        if source.content_ordinal.is_none() {
            continue;
        }
        let mut parent = source.parent;
        while let Some(parent_index) = parent {
            if sources[parent_index].content_ordinal.is_some() {
                break;
            }
            parent = sources[parent_index].parent;
        }
        match parent {
            Some(parent) => children[parent].push(index),
            None => roots.push(index),
        }
    }
    let mut resolved = sources
        .iter()
        .map(|source| {
            source
                .content_ordinal
                .is_none()
                .then(|| Err(format!("{} is not a retained content element", source.id)))
        })
        .collect::<Vec<_>>();
    let viewport = BoundingContainingBlock {
        x: 0,
        y: 0,
        width: viewport_width,
    };
    let _ = resolve_block_children(
        &roots,
        viewport,
        sources,
        &children,
        viewport_width,
        &mut resolved,
    );

    resolved
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result.unwrap_or_else(|| {
                Err(format!(
                    "bounding box resolution did not visit {}",
                    sources[index].id
                ))
            })
        })
        .collect()
}

fn resolve_bounding_boxes_with_styles(
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
    viewport_width: u64,
) -> Vec<Result<BoundingBox, String>> {
    let styles = match styles {
        Ok(styles) => styles,
        Err(reason) => {
            return sources.iter().map(|_| Err(reason.into())).collect();
        }
    };
    let mut styled_sources = sources.to_vec();
    for (source, properties) in styled_sources.iter_mut().zip(styles) {
        if properties.is_empty() {
            source.attributes.remove("style");
        } else {
            let serialized = properties
                .iter()
                .map(|(name, value)| format!("{name}:{value}"))
                .collect::<Vec<_>>()
                .join(";");
            source.attributes.insert("style".into(), serialized);
        }
    }
    resolve_bounding_boxes(&styled_sources, false, viewport_width)
}

fn resolve_block_children(
    child_indices: &[usize],
    containing_block: BoundingContainingBlock,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    viewport_width: u64,
    resolved: &mut [Option<Result<BoundingBox, String>>],
) -> Result<i64, String> {
    let mut cursor_y = containing_block.y;
    let mut blocked_flow = None::<String>;

    for &index in child_indices {
        let source = &sources[index];
        let properties = parse_style(
            source
                .attributes
                .get("style")
                .map(String::as_str)
                .unwrap_or_default(),
        );
        if source_does_not_generate_box(source, &properties) {
            mark_subtree_without_box(
                index,
                sources,
                children,
                resolved,
                "the element does not generate a layout box",
            );
            continue;
        }

        let scheme = match positioning_scheme(&properties) {
            Ok(scheme) => scheme,
            Err(reason) => {
                resolved[index] = Some(Err(reason.clone()));
                mark_descendants_unsupported(index, sources, children, resolved);
                if blocked_flow.is_none() && unsupported_position_blocks_flow(&properties) {
                    blocked_flow = Some(reason);
                }
                continue;
            }
        };
        if scheme == PositioningScheme::Normal
            && let Some(reason) = &blocked_flow
        {
            let reason = format!("previous normal-flow sibling geometry is unsupported: {reason}");
            resolved[index] = Some(Err(reason));
            mark_descendants_unsupported(index, sources, children, resolved);
            continue;
        }

        match resolve_element_bounding_box(
            index,
            source,
            &properties,
            scheme,
            containing_block,
            cursor_y,
            sources,
            children,
            viewport_width,
            resolved,
        ) {
            Ok(FlowPlacement::InFlow { next_y }) => cursor_y = next_y,
            Ok(FlowPlacement::OutOfFlow) => {}
            Err(reason) => {
                resolved[index] = Some(Err(reason.clone()));
                mark_descendants_unsupported(index, sources, children, resolved);
                if scheme == PositioningScheme::Normal {
                    blocked_flow = Some(reason);
                }
            }
        }
    }

    blocked_flow.map_or(Ok(cursor_y), Err)
}

#[allow(clippy::too_many_arguments)]
fn resolve_element_bounding_box(
    index: usize,
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    scheme: PositioningScheme,
    containing_block: BoundingContainingBlock,
    cursor_y: i64,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    viewport_width: u64,
    resolved: &mut [Option<Result<BoundingBox, String>>],
) -> Result<FlowPlacement, String> {
    reject_unsupported_bounding_box_geometry(properties)?;
    let edges = vertical_edges(properties)?;
    let edge_height = edges.total(&source.id)?;

    match scheme {
        PositioningScheme::Fixed => resolve_fixed_element_bounding_box(
            index,
            source,
            properties,
            edges,
            edge_height,
            sources,
            children,
            viewport_width,
            resolved,
        ),
        PositioningScheme::Normal => resolve_normal_element_bounding_box(
            index,
            source,
            properties,
            edges,
            edge_height,
            containing_block,
            cursor_y,
            sources,
            children,
            viewport_width,
            resolved,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_fixed_element_bounding_box(
    index: usize,
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    edges: VerticalEdges,
    edge_height: u64,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    viewport_width: u64,
    resolved: &mut [Option<Result<BoundingBox, String>>],
) -> Result<FlowPlacement, String> {
    reject_fixed_margins(properties)?;
    let horizontal = resolve_fixed_box(source, properties, viewport_width)?;
    let border_y = required_length(properties, "top")?;
    let specified_height = required_non_negative_length(properties, "height")?;
    let (_, border_height) =
        explicit_vertical_size(source, properties, edge_height, specified_height)?;
    let content_y = checked_add_vertical(border_y, edges.content_offset(&source.id)?, &source.id)?;
    let layout = ResolvedBoundingBox {
        border: BoundingBox {
            x: horizontal.border_x,
            y: border_y,
            width: horizontal.border_width,
            height: border_height,
        },
        content_x: horizontal.content_x,
        content_y,
        content_width: horizontal.content_width,
    };
    resolved[index] = Some(Ok(layout.border));
    let _ = resolve_element_children(
        index,
        layout,
        properties,
        sources,
        children,
        viewport_width,
        resolved,
    );
    Ok(FlowPlacement::OutOfFlow)
}

#[allow(clippy::too_many_arguments)]
fn resolve_normal_element_bounding_box(
    index: usize,
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    edges: VerticalEdges,
    edge_height: u64,
    containing_block: BoundingContainingBlock,
    cursor_y: i64,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    viewport_width: u64,
    resolved: &mut [Option<Result<BoundingBox, String>>],
) -> Result<FlowPlacement, String> {
    let horizontal = resolve_normal_box(
        source,
        properties,
        ContainingBlock {
            x: containing_block.x,
            width: containing_block.width,
        },
    )?;
    let (margin_top, margin_bottom) = normal_vertical_margins(source, properties)?;
    let border_y = cursor_y
        .checked_add(margin_top)
        .ok_or_else(|| format!("vertical coordinates overflow for {}", source.id))?;
    let content_y = checked_add_vertical(border_y, edges.content_offset(&source.id)?, &source.id)?;
    let specified_height = optional_non_negative_length(properties, "height")?;
    let explicit_size = specified_height
        .map(|height| explicit_vertical_size(source, properties, edge_height, height))
        .transpose()?;
    let provisional = ResolvedBoundingBox {
        border: BoundingBox {
            x: horizontal.border_x,
            y: border_y,
            width: horizontal.border_width,
            height: 0,
        },
        content_x: horizontal.content_x,
        content_y,
        content_width: horizontal.content_width,
    };
    let child_flow = resolve_element_children(
        index,
        provisional,
        properties,
        sources,
        children,
        viewport_width,
        resolved,
    );
    let border_height =
        resolved_normal_height(source, explicit_size, child_flow, content_y, edge_height)?;
    let border = BoundingBox {
        x: horizontal.border_x,
        y: border_y,
        width: horizontal.border_width,
        height: border_height,
    };
    resolved[index] = Some(Ok(border));
    let next_y = checked_add_vertical(border_y, border_height, &source.id)?
        .checked_add(margin_bottom)
        .ok_or_else(|| format!("vertical coordinates overflow for {}", source.id))?;
    Ok(FlowPlacement::InFlow { next_y })
}

fn resolved_normal_height(
    source: &ElementSource,
    explicit_size: Option<(u64, u64)>,
    child_flow: Result<i64, String>,
    content_y: i64,
    edge_height: u64,
) -> Result<u64, String> {
    if let Some((_, border_height)) = explicit_size {
        return Ok(border_height);
    }
    if !collapse_whitespace(&source.content.direct_text).is_empty() {
        return Err(format!(
            "intrinsic text height is not implemented for {}",
            source.id
        ));
    }
    let child_end = child_flow?;
    let content_height = u64::try_from(i128::from(child_end) - i128::from(content_y))
        .map_err(|_| format!("vertical size overflows for {}", source.id))?;
    content_height
        .checked_add(edge_height)
        .ok_or_else(|| format!("vertical size overflows for {}", source.id))
}

#[allow(clippy::too_many_arguments)]
fn resolve_element_children(
    index: usize,
    layout: ResolvedBoundingBox,
    properties: &BTreeMap<String, String>,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    viewport_width: u64,
    resolved: &mut [Option<Result<BoundingBox, String>>],
) -> Result<i64, String> {
    if properties
        .get("content-visibility")
        .is_some_and(|value| value == "hidden")
    {
        for &child in &children[index] {
            mark_subtree_without_box(
                child,
                sources,
                children,
                resolved,
                "an ancestor suppresses its layout contents",
            );
        }
        return Ok(layout.content_y);
    }
    resolve_block_children(
        &children[index],
        BoundingContainingBlock {
            x: layout.content_x,
            y: layout.content_y,
            width: layout.content_width,
        },
        sources,
        children,
        viewport_width,
        resolved,
    )
}

fn source_does_not_generate_box(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
) -> bool {
    properties
        .get("display")
        .is_some_and(|value| value == "none")
        || (source.attributes.contains_key("hidden") && !properties.contains_key("display"))
}

fn positioning_scheme(properties: &BTreeMap<String, String>) -> Result<PositioningScheme, String> {
    match properties.get("position").map(String::as_str) {
        None | Some("static") => Ok(PositioningScheme::Normal),
        Some("fixed") => Ok(PositioningScheme::Fixed),
        Some(value) => Err(format!("position:{value} layout is not implemented")),
    }
}

fn unsupported_position_blocks_flow(properties: &BTreeMap<String, String>) -> bool {
    properties.get("position").map(String::as_str) != Some("absolute")
}

fn vertical_edges(properties: &BTreeMap<String, String>) -> Result<VerticalEdges, String> {
    Ok(VerticalEdges {
        padding_top: non_negative_length(properties, "padding-top")?,
        padding_bottom: non_negative_length(properties, "padding-bottom")?,
        border_top: border_width(properties, "top")?,
        border_bottom: border_width(properties, "bottom")?,
    })
}

fn explicit_vertical_size(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    edge_height: u64,
    specified_height: u64,
) -> Result<(u64, u64), String> {
    let border_box = properties
        .get("box-sizing")
        .is_some_and(|value| value == "border-box");
    if border_box {
        let content_height = specified_height.checked_sub(edge_height).ok_or_else(|| {
            format!(
                "border-box height is smaller than its edges for {}",
                source.id
            )
        })?;
        Ok((content_height, specified_height))
    } else {
        let border_height = specified_height
            .checked_add(edge_height)
            .ok_or_else(|| format!("vertical size overflows for {}", source.id))?;
        Ok((specified_height, border_height))
    }
}

fn normal_vertical_margins(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
) -> Result<(i64, i64), String> {
    if source.tag == "body" {
        return Ok((
            optional_length(properties, "margin-top")?.unwrap_or(8),
            optional_length(properties, "margin-bottom")?.unwrap_or(8),
        ));
    }
    if properties.contains_key("margin-top") || properties.contains_key("margin-bottom") {
        return Err("vertical margin collapsing is not implemented".into());
    }
    Ok((0, 0))
}

fn reject_fixed_margins(properties: &BTreeMap<String, String>) -> Result<(), String> {
    for name in ["margin-left", "margin-right", "margin-top", "margin-bottom"] {
        if properties.contains_key(name) {
            return Err(format!("fixed-position {name} geometry is not implemented"));
        }
    }
    Ok(())
}

fn mark_subtree_without_box(
    index: usize,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    resolved: &mut [Option<Result<BoundingBox, String>>],
    reason: &str,
) {
    if resolved[index].is_none() {
        resolved[index] = Some(Err(format!("{reason} for {}", sources[index].id)));
    }
    for &child in &children[index] {
        mark_subtree_without_box(child, sources, children, resolved, reason);
    }
}

fn mark_descendants_unsupported(
    index: usize,
    sources: &[ElementSource],
    children: &[Vec<usize>],
    resolved: &mut [Option<Result<BoundingBox, String>>],
) {
    for &child in &children[index] {
        mark_subtree_without_box(
            child,
            sources,
            children,
            resolved,
            &format!("ancestor {} bounding box is unsupported", sources[index].id),
        );
    }
}

fn checked_add_vertical(value: i64, addition: u64, id: &str) -> Result<i64, String> {
    value
        .checked_add(
            i64::try_from(addition)
                .map_err(|_| format!("vertical coordinates overflow for {id}"))?,
        )
        .ok_or_else(|| format!("vertical coordinates overflow for {id}"))
}

fn resolve_normal_box(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    parent: ContainingBlock,
) -> Result<ResolvedBox, String> {
    if !is_block_element(&source.tag) {
        if properties.get("display").map(String::as_str) != Some("block") {
            return Err(format!(
                "normal-flow {} layout is not implemented",
                source.tag
            ));
        }
        if properties.get("box-sizing").map(String::as_str) != Some("border-box")
            || !properties.contains_key("width")
            || !properties.contains_key("height")
        {
            return Err(format!(
                "normal-flow {} needs explicit border-box width and height",
                source.tag
            ));
        }
    }
    let default_margin = if source.tag == "body" { 8 } else { 0 };
    let margin_left = optional_length(properties, "margin-left")?.unwrap_or(default_margin);
    let margin_right = optional_length(properties, "margin-right")?.unwrap_or(default_margin);
    let width = optional_non_negative_length(properties, "width")?;
    let layout = resolve_box_model(source, properties, parent, margin_left, margin_right, width)?;
    if width.is_none() {
        let occupied = checked_add_i64_u64(layout.border_x, layout.border_width, &source.id)?;
        let occupied = occupied
            .checked_add(margin_right)
            .ok_or_else(|| format!("horizontal coordinates overflow for {}", source.id))?;
        let parent_right = checked_add_i64_u64(parent.x, parent.width, &source.id)?;
        if occupied != parent_right {
            return Err(format!(
                "auto width could not fill the containing block for {}",
                source.id
            ));
        }
    }
    Ok(layout)
}

fn resolve_box_model(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    parent: ContainingBlock,
    offset: i64,
    trailing_offset: i64,
    specified_width: Option<u64>,
) -> Result<ResolvedBox, String> {
    let padding_left = non_negative_length(properties, "padding-left")?;
    let padding_right = non_negative_length(properties, "padding-right")?;
    let border_left = border_width(properties, "left")?;
    let border_right = border_width(properties, "right")?;
    let additions = padding_left
        .checked_add(padding_right)
        .and_then(|value| value.checked_add(border_left))
        .and_then(|value| value.checked_add(border_right))
        .ok_or_else(|| format!("horizontal size overflows for {}", source.id))?;
    let available = subtract_offsets(parent.width, offset, trailing_offset, &source.id)?;
    let border_box = properties
        .get("box-sizing")
        .is_some_and(|value| value == "border-box");
    let border_width = match (specified_width, border_box) {
        (Some(width), true) if width < additions => {
            return Err(format!(
                "border-box width is smaller than its edges for {}",
                source.id
            ));
        }
        (Some(width), true) => width,
        (Some(width), false) => width
            .checked_add(additions)
            .ok_or_else(|| format!("horizontal size overflows for {}", source.id))?,
        (None, _) => available,
    };
    let content_width = border_width
        .checked_sub(additions)
        .ok_or_else(|| format!("horizontal edges exceed available width for {}", source.id))?;
    let border_x = parent
        .x
        .checked_add(offset)
        .ok_or_else(|| format!("horizontal coordinates overflow for {}", source.id))?;
    let content_x = border_x
        .checked_add(
            i64::try_from(border_left + padding_left)
                .map_err(|_| format!("horizontal coordinates overflow for {}", source.id))?,
        )
        .ok_or_else(|| format!("horizontal coordinates overflow for {}", source.id))?;

    Ok(ResolvedBox {
        border_x,
        border_width,
        content_x,
        content_width,
    })
}

fn subtract_offsets(width: u64, left: i64, right: i64, id: &str) -> Result<u64, String> {
    let remaining = i128::from(width) - i128::from(left) - i128::from(right);
    u64::try_from(remaining).map_err(|_| format!("horizontal offsets exceed width for {id}"))
}

fn checked_add_i64_u64(left: i64, right: u64, id: &str) -> Result<i64, String> {
    left.checked_add(
        i64::try_from(right).map_err(|_| format!("horizontal coordinates overflow for {id}"))?,
    )
    .ok_or_else(|| format!("horizontal coordinates overflow for {id}"))
}

fn parse_style(style: &str) -> BTreeMap<String, String> {
    style
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .map(|(name, value)| {
            (
                name.trim().to_ascii_lowercase(),
                value.trim().to_ascii_lowercase(),
            )
        })
        .collect()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn reject_unsupported_geometry(properties: &BTreeMap<String, String>) -> Result<(), String> {
    for unsupported in [
        "right",
        "inset",
        "margin",
        "padding",
        "border",
        "border-left",
        "border-right",
        "transform",
        "min-width",
        "max-width",
        "float",
        "columns",
        "column-width",
    ] {
        if properties.contains_key(unsupported) {
            return Err(format!("inline {unsupported} geometry is not implemented"));
        }
    }
    if let Some(display) = properties.get("display")
        && display != "block"
    {
        return Err(format!("display:{display} layout is not implemented"));
    }
    if let Some(box_sizing) = properties.get("box-sizing")
        && box_sizing != "border-box"
        && box_sizing != "content-box"
    {
        return Err(format!("box-sizing:{box_sizing} is not implemented"));
    }
    Ok(())
}

fn reject_unsupported_bounding_box_geometry(
    properties: &BTreeMap<String, String>,
) -> Result<(), String> {
    reject_unsupported_geometry(properties)?;
    for unsupported in [
        "bottom",
        "border-top",
        "border-bottom",
        "min-height",
        "max-height",
    ] {
        if properties.contains_key(unsupported) {
            return Err(format!("inline {unsupported} geometry is not implemented"));
        }
    }
    Ok(())
}

fn required_length(properties: &BTreeMap<String, String>, name: &str) -> Result<i64, String> {
    properties
        .get(name)
        .and_then(|value| parse_px_i64(value))
        .ok_or_else(|| format!("inline {name} must be an integer px value"))
}

fn required_non_negative_length(
    properties: &BTreeMap<String, String>,
    name: &str,
) -> Result<u64, String> {
    optional_non_negative_length(properties, name)?
        .ok_or_else(|| format!("inline {name} must be a non-negative integer px value"))
}

fn optional_non_negative_length(
    properties: &BTreeMap<String, String>,
    name: &str,
) -> Result<Option<u64>, String> {
    optional_length(properties, name)?
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| format!("inline {name} must be a non-negative integer px value"))
        })
        .transpose()
}

fn non_negative_length(properties: &BTreeMap<String, String>, name: &str) -> Result<u64, String> {
    Ok(optional_non_negative_length(properties, name)?.unwrap_or(0))
}

fn border_width(properties: &BTreeMap<String, String>, side: &str) -> Result<u64, String> {
    let width_name = format!("border-{side}-width");
    let style_name = format!("border-{side}-style");
    let width = optional_non_negative_length(properties, &width_name)?;
    match (width, properties.get(&style_name).map(String::as_str)) {
        (None, None | Some("none") | Some("hidden")) => Ok(0),
        (Some(_), None | Some("none") | Some("hidden")) => Ok(0),
        (Some(width), Some(_)) => Ok(width),
        (None, Some(_)) => Err(format!(
            "inline {width_name} must be explicit when {style_name} paints a border"
        )),
    }
}

fn optional_length(
    properties: &BTreeMap<String, String>,
    name: &str,
) -> Result<Option<i64>, String> {
    properties
        .get(name)
        .map(|value| {
            parse_px_i64(value).ok_or_else(|| format!("inline {name} must be an integer px value"))
        })
        .transpose()
}

fn parse_px_i64(value: &str) -> Option<i64> {
    if value == "0" {
        return Some(0);
    }
    value.strip_suffix("px")?.trim().parse().ok()
}

fn is_content_element(tag_name: &str) -> bool {
    !matches!(
        tag_name,
        "html"
            | "head"
            | "body"
            | "base"
            | "link"
            | "meta"
            | "title"
            | "style"
            | "script"
            | "noscript"
            | "template"
    )
}

fn is_block_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "body"
            | "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "dl"
            | "fieldset"
            | "footer"
            | "form"
            | "header"
            | "main"
            | "nav"
            | "section"
    )
}

fn is_inert_text_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "head" | "title" | "style" | "script" | "noscript" | "template"
    )
}

fn separates_text(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "br"
            | "dd"
            | "details"
            | "dialog"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CheckedState, ControlState, InteractiveAction, SequentialFocusSource, TextValueState,
        interactive_elements_from_html, layout_input_from_html, page_semantics_from_html,
        parse_page_source, semantic_elements_from_html,
    };
    use crate::locator::css::CssSelector;
    use crate::{Comparison, LintLayout, RuleConstraint, RuleResult, Session};

    fn lint(html: &str) -> RuleResult {
        Session::new()
            .execute(LintLayout {
                input: layout_input_from_html(html, 320),
            })
            .unwrap()
    }

    #[test]
    fn extracts_fixed_pixel_geometry() {
        let result = lint(r#"<div id="hero" style="position:fixed;left:280px;width:80px"></div>"#);

        match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => assert_eq!(findings[0].affected_element.as_str(), "hero"),
            other => panic!("expected overflow, got {other:?}"),
        }
    }

    #[test]
    fn normal_block_fills_its_parent() {
        let result = lint(r#"<main id="content"></main>"#);

        assert!(matches!(
            result,
            RuleResult::Compared {
                comparison: Comparison::Pass,
                ..
            }
        ));
    }

    #[test]
    fn nested_block_uses_parent_content_box() {
        let result = lint(
            r#"<main id="shell" style="padding-left:20px;width:300px"><section id="wide" style="width:301px"></section></main>"#,
        );

        match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => {
                assert_eq!(findings.len(), 1);
                assert_eq!(findings[0].affected_element.as_str(), "wide");
                assert_eq!(findings[0].observed_left, 20);
                assert_eq!(findings[0].observed_right, 321);
            }
            other => panic!("expected nested overflow, got {other:?}"),
        }
    }

    #[test]
    fn body_uses_its_default_horizontal_margin() {
        let result = lint(r#"<body><main id="wide" style="width:313px"></main></body>"#);

        match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => {
                assert_eq!(findings[0].affected_element.as_str(), "wide");
                assert_eq!(findings[0].observed_left, 8);
                assert_eq!(findings[0].observed_right, 321);
            }
            other => panic!("expected body-margin overflow, got {other:?}"),
        }
    }

    #[test]
    fn border_width_without_border_style_does_not_expand_the_box() {
        let result =
            lint(r#"<main id="content" style="width:320px;border-left-width:1px"></main>"#);

        assert!(matches!(
            result,
            RuleResult::Compared {
                comparison: Comparison::Pass,
                ..
            }
        ));
    }

    #[test]
    fn inline_flow_blocks_instead_of_passing() {
        let result = lint(r#"<span id="label"></span>"#);

        assert!(matches!(
            result,
            RuleResult::Blocked {
                causes,
                ..
            } if matches!(&causes[0], RuleConstraint::Unsupported { element, .. } if element == "label")
        ));
    }

    #[test]
    fn absolute_positioning_blocks_without_false_geometry() {
        let result = lint(r#"<div id="hero" style="position:absolute;left:0;width:20px"></div>"#);

        assert!(matches!(result, RuleResult::Blocked { .. }));
    }

    #[test]
    fn embedded_stylesheets_apply_specificity_and_geometry() {
        let result = lint(
            r#"<style>div { position:fixed; left:0; width:400px } #hero { width:20px }</style><div id="hero"></div>"#,
        );

        assert!(matches!(
            result,
            RuleResult::Compared {
                comparison: Comparison::Pass,
                ..
            }
        ));
    }

    #[test]
    fn parsed_page_uses_html_tree_ancestry() {
        let source = parse_page_source(
            r#"<ul id="list"><li id="first"><button>A</button><li id="second"><button id="target">B</button></ul>"#,
        );
        let target = source
            .elements
            .iter()
            .position(|element| {
                element
                    .attributes
                    .get("id")
                    .is_some_and(|id| id == "target")
            })
            .unwrap();

        assert!(
            !CssSelector::parse("#first #target")
                .unwrap()
                .matches(target, &source.elements)
        );
        assert!(
            CssSelector::parse("html body #target")
                .unwrap()
                .matches(target, &source.elements)
        );
    }

    #[test]
    fn parsed_page_applies_implied_table_and_paragraph_structure() {
        let table = parse_page_source("<table><tr><td id=cell>Cell</td></tr></table>");
        let cell = table
            .elements
            .iter()
            .position(|element| element.attributes.get("id").is_some_and(|id| id == "cell"))
            .unwrap();
        assert!(
            CssSelector::parse("table > tbody > tr > #cell")
                .unwrap()
                .matches(cell, &table.elements)
        );

        let paragraph = parse_page_source("<p id=copy>Text<div id=block>Block</div>");
        let block = paragraph
            .elements
            .iter()
            .position(|element| element.attributes.get("id").is_some_and(|id| id == "block"))
            .unwrap();
        assert!(
            !CssSelector::parse("#copy > #block")
                .unwrap()
                .matches(block, &paragraph.elements)
        );
    }

    #[test]
    fn unsupported_stylesheet_syntax_blocks_layout() {
        let result = lint(
            r#"<style>@media (width > 1px) { #hero { width:20px } }</style><div id="hero"></div>"#,
        );

        assert!(matches!(result, RuleResult::Blocked { .. }));
    }

    #[test]
    fn unsupported_style_media_blocks_layout() {
        let result = lint(
            r#"<style media="print">#hero { position:fixed; left:0; width:20px }</style><div id="hero"></div>"#,
        );

        assert!(matches!(result, RuleResult::Blocked { .. }));
    }

    #[test]
    fn interactive_elements_use_native_roles_and_accessible_names() {
        let elements = interactive_elements_from_html(
            r#"
                <label for="email">Email address</label>
                <input id="email">
                <button id="save"><span>Save changes</span></button>
                <a id="docs" href="/docs" aria-label="Read documentation">Docs</a>
                <input id="secret" type="hidden">
            "#,
        );

        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].element(), "email");
        assert_eq!(elements[0].role(), "textbox");
        assert_eq!(elements[0].name(), "Email address");
        assert_eq!(elements[1].role(), "button");
        assert_eq!(elements[1].name(), "Save changes");
        assert_eq!(elements[1].text(), "Save changes");
        assert_eq!(elements[2].role(), "link");
        assert_eq!(elements[2].name(), "Read documentation");
        assert_eq!(elements[2].text(), "Docs");
    }

    #[test]
    fn content_names_include_non_presentational_image_alt_text() {
        let elements = interactive_elements_from_html(
            r#"
                <button id="save"><img alt="Save image"></button>
                <button id="mixed">Open <span><img alt="account"></span></button>
                <button id="decorative"><img alt="" title=""></button>
                <button id="presentational"><img role="presentation" alt="Ignored"></button>
            "#,
        );

        assert_eq!(elements[0].name(), "Save image");
        assert_eq!(elements[1].name(), "Open account");
        assert_eq!(elements[2].name(), "");
        assert_eq!(elements[3].name(), "");
    }

    #[test]
    fn semantic_text_separates_blocks_and_preserves_inline_adjacency() {
        let elements = semantic_elements_from_html(
            r#"<nav aria-label="Primary"><span>Read</span><a href="/docs">Docs</a><ul><li>Rust</li><li>Go</li></ul></nav>"#,
        );
        let navigation = elements
            .iter()
            .find(|element| element.role() == "navigation")
            .unwrap();
        let list = elements
            .iter()
            .find(|element| element.role() == "list")
            .unwrap();

        assert_eq!(navigation.text(), "ReadDocs Rust Go");
        assert_eq!(list.name(), "");
        assert_eq!(list.text(), "Rust Go");
    }

    #[test]
    fn semantic_names_follow_role_specific_sources() {
        let elements = semantic_elements_from_html(
            r#"
                <span id="one">Site</span><span id="two">map</span>
                <nav aria-labelledby="one two">Links</nav>
                <header><h1>Home</h1></header>
            "#,
        );
        let navigation = elements
            .iter()
            .find(|element| element.role() == "navigation")
            .unwrap();
        let banner = elements
            .iter()
            .find(|element| element.role() == "banner")
            .unwrap();

        assert_eq!(navigation.name(), "Site map");
        assert_eq!(banner.name(), "");
        assert_eq!(banner.text(), "Home");
    }

    #[test]
    fn explicit_interactive_role_is_included() {
        let elements = interactive_elements_from_html(
            r#"<div id="toggle" role="unsupported SWITCH" aria-label="Dark mode"></div>"#,
        );

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].role(), "switch");
        assert_eq!(elements[0].name(), "Dark mode");
    }

    #[test]
    fn form_values_do_not_become_accessible_names() {
        let elements = interactive_elements_from_html(
            r#"<select><option>One</option></select><textarea>draft</textarea><input type="submit">"#,
        );

        assert_eq!(elements[0].name(), "");
        assert_eq!(elements[1].name(), "");
        assert_eq!(elements[2].name(), "Submit");
    }

    #[test]
    fn text_controls_expose_initial_value_capabilities() {
        let elements = interactive_elements_from_html(
            r#"<input value="old"><textarea>draft</textarea><input type="password"><input disabled>"#,
        );

        assert_eq!(
            elements[0].control_state,
            ControlState::Text(TextValueState::Editable {
                value: "old".into(),
                selection: crate::keyboard::TextSelectionState::collapsed_at(0),
            })
        );
        assert_eq!(
            elements[1].control_state,
            ControlState::Text(TextValueState::Editable {
                value: "draft".into(),
                selection: crate::keyboard::TextSelectionState::collapsed_at(0),
            })
        );
        assert!(matches!(
            elements[2].control_state,
            ControlState::Unavailable
        ));
        assert!(matches!(
            elements[3].control_state,
            ControlState::Text(TextValueState::NonEditable { .. })
        ));
    }

    #[test]
    fn sequential_focus_orders_positive_tabindex_before_natural_targets() {
        let page = page_semantics_from_html(
            r##"
                <button id="natural1">one</button>
                <input id="negative" tabindex="-1">
                <input id="positive2" tabindex="2">
                <button id="positive2b" tabindex="2">two-b</button>
                <a id="link" href="#x">link</a>
                <input id="disabled" disabled>
                <input id="positive1" tabindex="1">
                <textarea id="natural2"></textarea>
                <div id="zero" role="button" tabindex="0">zero</div>
                <input id="hidden" hidden>
                <div inert><button id="inert-child">inert</button></div>
            "##,
        );

        assert_eq!(
            page.sequential_focus,
            SequentialFocusSource::Supported {
                order: vec![6, 2, 3, 0, 4, 7, 8],
            }
        );
    }

    #[test]
    fn sequential_focus_blocks_when_a_focusable_target_cannot_be_represented() {
        let page = page_semantics_from_html(
            r#"<button>Save</button><div contenteditable="true">Draft</div>"#,
        );

        assert!(matches!(
            page.sequential_focus,
            SequentialFocusSource::Unsupported { reason }
                if reason.contains("without a supported interactive role")
        ));
    }

    #[test]
    fn sequential_focus_ignores_inline_box_geometry() {
        let page = page_semantics_from_html(
            r#"<button style="position:absolute;width:0;height:0">Save</button>"#,
        );

        assert_eq!(
            page.sequential_focus,
            SequentialFocusSource::Supported { order: vec![0] }
        );
    }

    #[test]
    fn native_checkboxes_expose_checked_state_capabilities() {
        let elements = interactive_elements_from_html(
            r#"<input type="checkbox"><input type="checkbox" checked disabled><div role="checkbox" aria-checked="true"></div>"#,
        );

        assert_eq!(
            elements[0].control_state,
            ControlState::Checkbox(CheckedState::Editable { checked: false })
        );
        assert!(matches!(
            elements[1].control_state,
            ControlState::Checkbox(CheckedState::NonEditable { checked: true, .. })
        ));
        assert_eq!(elements[2].control_state, ControlState::Unavailable);
    }

    #[test]
    fn native_radios_normalize_groups_and_supply_one_natural_tab_stop() {
        let page = page_semantics_from_html(
            r#"
                <form id="first">
                    <label><input id="a" type="radio" name="group" checked>A</label>
                    <label><input id="b" type="radio" name="group" checked>B</label>
                </form>
                <form id="second">
                    <label><input id="c" type="radio" name="group">C</label>
                </form>
                <label><input id="d" type="radio">D</label>
                <button id="after" type="button">After</button>
            "#,
        );

        assert_eq!(page.elements.interactive_elements[0].checked(), Some(false));
        assert_eq!(page.elements.interactive_elements[1].checked(), Some(true));
        assert_eq!(page.elements.interactive_elements[2].checked(), Some(false));
        assert_eq!(page.elements.interactive_elements[3].checked(), Some(false));
        assert!(matches!(
            page.elements.interactive_elements[0].control_state,
            ControlState::Radio(_)
        ));
        assert_eq!(
            page.elements.interactive_elements[1].action,
            InteractiveAction::SelectRadio
        );
        assert_eq!(
            page.sequential_focus,
            SequentialFocusSource::Supported {
                order: vec![1, 2, 3, 4],
            }
        );

        let explicit = page_semantics_from_html(
            r#"<input type="radio" name="group" tabindex="0" aria-label="Choice">"#,
        );
        assert!(matches!(
            explicit.sequential_focus,
            SequentialFocusSource::Unsupported { reason }
                if reason == "explicit radio tabindex order is not implemented"
        ));
    }

    #[test]
    fn native_selects_expose_single_and_multiple_value_state() {
        let elements = interactive_elements_from_html(
            r#"
                <select aria-label="Size">
                    <option value="small" selected>Small</option>
                    <option value="large" selected>Large</option>
                </select>
                <select aria-label="Fallback">
                    <option value="blocked" disabled>Blocked</option>
                    <option> Ready </option>
                </select>
                <select aria-label="List" size="2"><option>One</option></select>
                <select aria-label="Many" multiple>
                    <option selected>A</option><option selected>B</option>
                </select>
            "#,
        );

        assert_eq!(elements[0].role(), "combobox");
        assert_eq!(elements[0].value(), Some("large"));
        assert_eq!(elements[1].value(), Some("Ready"));
        assert_eq!(elements[2].role(), "listbox");
        assert_eq!(elements[2].value(), Some(""));
        assert_eq!(elements[3].role(), "listbox");
        assert_eq!(elements[3].value(), Some("A"));
    }

    #[test]
    fn interactive_visibility_requires_static_style_and_box_evidence() {
        let elements = interactive_elements_from_html(
            r#"
                <button>Visible</button>
                <button hidden>Hidden</button>
                <button hidden style="display:block">Overridden hidden</button>
                <div style="display:none"><button>Ancestor hidden</button></div>
                <div role="button" aria-label="Empty"></div>
                <div style="visibility:hidden">
                    <button>Inherited hidden</button>
                    <button style="visibility:visible">Visible override</button>
                </div>
                <button style="width:0">Unknown box</button>
                <a href="/next">Visible link</a>
            "#,
        );

        assert_eq!(elements[0].visible(), Ok(true));
        assert_eq!(elements[1].visible(), Ok(false));
        assert_eq!(elements[2].visible(), Ok(true));
        assert_eq!(elements[3].visible(), Ok(false));
        assert_eq!(elements[4].visible(), Ok(false));
        assert_eq!(elements[5].visible(), Ok(false));
        assert_eq!(elements[6].visible(), Ok(true));
        assert!(elements[7].visible().is_err());
        assert_eq!(elements[8].visible(), Ok(true));

        let styled = interactive_elements_from_html(
            r#"<style>main > button { display: none }</style><main><button>Styled</button></main>"#,
        );
        assert_eq!(styled[0].visible(), Ok(false));

        let inert_style = interactive_elements_from_html(
            r#"<div role="button" aria-label="Empty"><style>.hidden { display:none }</style></div>"#,
        );
        assert_eq!(inert_style[0].text(), "");
        assert_eq!(inert_style[0].visible(), Ok(false));

        let quoted_comment = interactive_elements_from_html(
            r#"<style>[data-x="/*"] { display:none }</style><button data-x="/*">Hidden</button>"#,
        );
        assert_eq!(quoted_comment[0].visible(), Ok(false));

        let until_found =
            interactive_elements_from_html(r#"<button hidden="until-found">Found later</button>"#);
        assert!(until_found[0].visible().is_err());
    }

    #[test]
    fn link_actions_preserve_href_for_session_navigation() {
        let elements = interactive_elements_from_html(r#"<a href="../next?q=1">Next</a>"#);

        assert_eq!(
            elements[0].action,
            InteractiveAction::Navigate {
                href: "../next?q=1".into(),
            }
        );
    }

    #[test]
    fn new_context_and_download_links_are_unsupported_actions() {
        let elements = interactive_elements_from_html(
            r#"<a href="/new" target="_blank">New</a><a href="/file" download>File</a>"#,
        );

        assert!(matches!(
            elements[0].action,
            InteractiveAction::Unsupported { .. }
        ));
        assert!(matches!(
            elements[1].action,
            InteractiveAction::Unsupported { .. }
        ));
    }

    #[test]
    fn native_click_actions_keep_form_defaults_explicit() {
        let elements = interactive_elements_from_html(
            r#"
                <button type="button">Save</button>
                <input type="checkbox" aria-label="Terms">
                <button>Outside form</button>
                <form><button>Submit</button></form>
                <form id="account"></form>
                <input type="reset" form="account" value="Reset">
                <div role="button" tabindex="0">Custom</div>
            "#,
        );

        assert_eq!(elements[0].action, InteractiveAction::Activate);
        assert_eq!(elements[1].action, InteractiveAction::ToggleCheckbox);
        assert_eq!(elements[2].action, InteractiveAction::Activate);
        assert!(matches!(
            elements[3].action,
            InteractiveAction::SubmitForm { .. }
        ));
        assert!(matches!(
            elements[4].action,
            InteractiveAction::Unsupported { .. }
        ));
        assert!(matches!(
            elements[5].action,
            InteractiveAction::Unsupported { .. }
        ));
    }

    #[test]
    fn page_title_uses_the_first_title_and_collapses_whitespace() {
        let semantics = page_semantics_from_html(
            "<title> browser\n  junior </title><title>Ignored</title><button>Save</button>",
        );

        assert_eq!(semantics.document.title, "browser junior");
        assert_eq!(semantics.elements.interactive_elements.len(), 1);
    }

    #[test]
    fn page_title_is_empty_when_the_document_has_no_title() {
        assert_eq!(
            page_semantics_from_html("<main>Hello</main>")
                .document
                .title,
            ""
        );
    }

    #[test]
    fn page_text_normalizes_static_content_and_excludes_metadata_and_scripts() {
        let semantics = page_semantics_from_html(
            "<title>Docs</title><main>Hello <span>world</span><script>ignore()</script><style>.x{}</style><input value=\"secret\"></main><footer>End</footer>",
        );

        assert_eq!(semantics.document.text, "Hello world End");
    }
}
