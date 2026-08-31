use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, CharacterTokens, EndTag, ParseError, StartTag, TagToken, Token, TokenSink,
    TokenSinkResult, Tokenizer,
};

use crate::{ElementInput, LayoutInput};

mod interactive;
mod selectors;
mod visibility;

pub(crate) use interactive::{
    CheckedState, ControlState, InteractiveAction, InteractiveElementSource, LocatorElementSource,
    SelectValueError, TextValueState, page_semantics_from_html,
};
#[cfg(test)]
pub(crate) use interactive::{interactive_elements_from_html, semantic_elements_from_html};
pub(crate) use selectors::{SelectorIndex, SelectorQueryError};

#[derive(Debug)]
struct ElementSource {
    id: String,
    tag: String,
    attributes: BTreeMap<String, String>,
    parent: Option<usize>,
    text: String,
}

#[derive(Debug)]
struct OpenElement {
    tag: String,
    content_index: Option<usize>,
    captures_title: bool,
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

#[derive(Default)]
struct PageSink {
    elements: RefCell<Vec<ElementSource>>,
    stack: RefCell<Vec<OpenElement>>,
    next_ordinal: Cell<usize>,
    metadata: PageMetadataSink,
}

#[derive(Debug, Default)]
struct PageMetadataSink {
    has_stylesheet: Cell<bool>,
    title_seen: Cell<bool>,
    title: RefCell<String>,
    parse_error: RefCell<Option<String>>,
}

struct ParsedPageSource {
    elements: Vec<ElementSource>,
    has_stylesheet: bool,
    title: String,
    parse_error: Option<String>,
}

impl TokenSink for PageSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        match token {
            TagToken(tag) if tag.kind == StartTag => self.start_tag(tag),
            TagToken(tag) if tag.kind == EndTag => self.end_tag(tag.name.as_ref()),
            CharacterTokens(text) => self.append_text(text.as_ref()),
            ParseError(error) => {
                let mut first_error = self.metadata.parse_error.borrow_mut();
                if first_error.is_none() {
                    *first_error = Some(error.to_string());
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

impl PageSink {
    fn start_tag(&self, tag: html5ever::tokenizer::Tag) {
        let tag_name = tag.name.to_string();
        if separates_text(&tag_name) {
            self.append_text(" ");
        }
        let attributes = tag
            .attrs
            .iter()
            .map(|attribute| {
                (
                    attribute.name.local.to_string(),
                    attribute.value.to_string(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        if tag_name == "style"
            || (tag_name == "link"
                && attributes.get("rel").is_some_and(|value| {
                    value
                        .split_ascii_whitespace()
                        .any(|part| part.eq_ignore_ascii_case("stylesheet"))
                }))
        {
            self.metadata.has_stylesheet.set(true);
        }

        let content_index = if is_content_element(&tag_name) {
            let ordinal = self.next_ordinal.get() + 1;
            self.next_ordinal.set(ordinal);
            let id = attributes
                .get("id")
                .cloned()
                .unwrap_or_else(|| format!("{tag_name}[{ordinal}]"));
            let parent = self
                .stack
                .borrow()
                .iter()
                .rev()
                .find_map(|open| open.content_index);
            let mut elements = self.elements.borrow_mut();
            let index = elements.len();
            elements.push(ElementSource {
                id,
                tag: tag_name.clone(),
                attributes: attributes.clone(),
                parent,
                text: String::new(),
            });
            Some(index)
        } else {
            None
        };

        let captures_title = tag_name == "title" && !self.metadata.title_seen.replace(true);
        if !is_void_element(&tag_name) && !tag.self_closing {
            self.stack.borrow_mut().push(OpenElement {
                tag: tag_name,
                content_index,
                captures_title,
            });
        }
    }

    fn end_tag(&self, tag_name: &str) {
        let mut stack = self.stack.borrow_mut();
        if let Some(index) = stack.iter().rposition(|open| open.tag == tag_name) {
            stack.truncate(index);
        }
        drop(stack);
        if separates_text(tag_name) {
            self.append_text(" ");
        }
    }

    fn append_text(&self, text: &str) {
        let stack = self.stack.borrow();
        let captures_title = stack.last().is_some_and(|open| open.captures_title);
        let content_indices = stack
            .iter()
            .filter_map(|open| open.content_index)
            .collect::<Vec<_>>();
        drop(stack);
        if captures_title {
            self.metadata.title.borrow_mut().push_str(text);
        }
        let mut elements = self.elements.borrow_mut();
        for index in content_indices {
            elements[index].text.push_str(text);
        }
    }
}

fn parse_page_source(html: &str) -> ParsedPageSource {
    let input = BufferQueue::default();
    input.push_back(StrTendril::from(html));
    let tokenizer = Tokenizer::new(PageSink::default(), Default::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();

    ParsedPageSource {
        elements: tokenizer.sink.elements.into_inner(),
        has_stylesheet: tokenizer.sink.metadata.has_stylesheet.get(),
        title: tokenizer.sink.metadata.title.into_inner(),
        parse_error: tokenizer.sink.metadata.parse_error.into_inner(),
    }
}

pub(crate) fn layout_input_from_html(html: &str, viewport_width: u64) -> LayoutInput {
    let ParsedPageSource {
        elements: sources,
        has_stylesheet,
        parse_error,
        ..
    } = parse_page_source(html);
    let mut resolved = Vec::<Result<ResolvedBox, String>>::with_capacity(sources.len());
    let mut elements = Vec::with_capacity(sources.len() + usize::from(parse_error.is_some()));

    for source in sources {
        let parent = source
            .parent
            .map(|index| resolved[index].as_ref().copied().map_err(Clone::clone))
            .transpose()
            .map(|parent| {
                parent.map_or(
                    ContainingBlock {
                        x: 0,
                        width: viewport_width,
                    },
                    |parent| ContainingBlock {
                        x: parent.content_x,
                        width: parent.content_width,
                    },
                )
            });
        let layout = if has_stylesheet {
            Err("linked and embedded stylesheets are not implemented".into())
        } else {
            parent.and_then(|parent| resolve_horizontal_box(&source, parent, viewport_width))
        };

        match &layout {
            Ok(layout) => elements.push(ElementInput::supported(
                source.id,
                layout.border_x,
                layout.border_width,
            )),
            Err(reason) => elements.push(ElementInput::unsupported(source.id, reason.clone())),
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

fn resolve_horizontal_box(
    source: &ElementSource,
    parent: ContainingBlock,
    viewport_width: u64,
) -> Result<ResolvedBox, String> {
    let properties = parse_style(
        source
            .attributes
            .get("style")
            .map(String::as_str)
            .unwrap_or_default(),
    );
    reject_unsupported_geometry(&properties)?;

    match properties.get("position").map(String::as_str) {
        Some("fixed") => resolve_fixed_box(source, &properties, viewport_width),
        None | Some("static") => resolve_normal_box(source, &properties, parent),
        Some(value) => Err(format!("position:{value} layout is not implemented")),
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

fn resolve_normal_box(
    source: &ElementSource,
    properties: &BTreeMap<String, String>,
    parent: ContainingBlock,
) -> Result<ResolvedBox, String> {
    if !is_block_element(&source.tag) {
        return Err(format!(
            "normal-flow {} layout is not implemented",
            source.tag
        ));
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

fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CheckedState, ControlState, InteractiveAction, TextValueState,
        interactive_elements_from_html, layout_input_from_html, page_semantics_from_html,
        semantic_elements_from_html,
    };
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
    fn stylesheets_block_inline_geometry() {
        let result = lint(
            r#"<style>#hero { width: 20px }</style><div id="hero" style="position:fixed;left:0;width:20px"></div>"#,
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
                value: "old".into()
            })
        );
        assert_eq!(
            elements[1].control_state,
            ControlState::Text(TextValueState::Editable {
                value: "draft".into()
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
            r#"<style>button { display: block }</style><button>Styled</button>"#,
        );
        assert!(styled[0].visible().is_err());

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
    fn page_title_uses_the_first_title_and_collapses_whitespace() {
        let semantics = page_semantics_from_html(
            "<title> browser\n  junior </title><title>Ignored</title><button>Save</button>",
        );

        assert_eq!(semantics.title, "browser junior");
        assert_eq!(semantics.interactive_elements.len(), 1);
    }

    #[test]
    fn page_title_is_empty_when_the_document_has_no_title() {
        assert_eq!(page_semantics_from_html("<main>Hello</main>").title, "");
    }
}
