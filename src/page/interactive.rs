use super::{ElementSource, parse_page_source};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PageSemanticSource {
    pub(crate) title: String,
    pub(crate) interactive_elements: Vec<InteractiveElementSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractiveElementSource {
    pub(crate) element: String,
    semantics: InteractiveSemantics,
    pub(crate) action: InteractiveAction,
    pub(crate) control_state: ControlState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InteractiveSemantics {
    tag: String,
    role: String,
    name: String,
    text: String,
    attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InteractiveAction {
    Navigate { href: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TextValueState {
    Editable { value: String },
    NonEditable { value: String, reason: String },
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CheckedState {
    Editable { checked: bool },
    NonEditable { checked: bool, reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ControlState {
    Text(TextValueState),
    Checkbox(CheckedState),
    Unavailable,
}

impl InteractiveElementSource {
    pub(crate) fn role(&self) -> &str {
        &self.semantics.role
    }

    pub(crate) fn name(&self) -> &str {
        &self.semantics.name
    }

    pub(crate) fn text(&self) -> &str {
        &self.semantics.text
    }

    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.semantics.attributes.get(name).map(String::as_str)
    }

    pub(crate) fn attribute_is_sensitive(&self, name: &str) -> bool {
        name == "value"
            && self.semantics.tag == "input"
            && self
                .semantics
                .attributes
                .get("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("password"))
    }

    pub(crate) fn enabled(&self) -> Option<bool> {
        match self.semantics.tag.as_str() {
            "button" | "input" | "select" | "textarea" => {
                Some(!self.semantics.attributes.contains_key("disabled"))
            }
            "a" if self.semantics.attributes.contains_key("href") => Some(true),
            _ => None,
        }
    }

    pub(crate) fn value(&self) -> Option<&str> {
        match &self.control_state {
            ControlState::Text(
                TextValueState::Editable { value } | TextValueState::NonEditable { value, .. },
            ) => Some(value),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn checked(&self) -> Option<bool> {
        match self.control_state {
            ControlState::Checkbox(
                CheckedState::Editable { checked } | CheckedState::NonEditable { checked, .. },
            ) => Some(checked),
            ControlState::Text(_) | ControlState::Unavailable => None,
        }
    }
}

#[cfg(test)]
pub(crate) fn interactive_elements_from_html(html: &str) -> Vec<InteractiveElementSource> {
    page_semantics_from_html(html).interactive_elements
}

pub(crate) fn page_semantics_from_html(html: &str) -> PageSemanticSource {
    let source = parse_page_source(html);
    PageSemanticSource {
        title: collapse_whitespace(&source.title),
        interactive_elements: interactive_elements_from_sources(&source.elements),
    }
}

fn interactive_elements_from_sources(sources: &[ElementSource]) -> Vec<InteractiveElementSource> {
    sources
        .iter()
        .filter_map(|source| {
            let role = interactive_role(source)?;
            let action = interactive_action(source, &role);
            let control_state = control_state(source);
            Some(InteractiveElementSource {
                element: source.id.clone(),
                semantics: InteractiveSemantics {
                    tag: source.tag.clone(),
                    role,
                    name: accessible_name(source, sources),
                    text: collapse_whitespace(&source.text),
                    attributes: source.attributes.clone(),
                },
                action,
                control_state,
            })
        })
        .collect()
}

fn control_state(source: &ElementSource) -> ControlState {
    if source.tag == "input" && input_type(source).as_deref() == Some("checkbox") {
        return ControlState::Checkbox(checkbox_state(source));
    }
    let text = text_value_state(source);
    if text == TextValueState::Unavailable {
        ControlState::Unavailable
    } else {
        ControlState::Text(text)
    }
}

fn checkbox_state(source: &ElementSource) -> CheckedState {
    let checked = source.attributes.contains_key("checked");
    if source.attributes.contains_key("disabled") {
        return CheckedState::NonEditable {
            checked,
            reason: "disabled checkboxes cannot change state".into(),
        };
    }
    CheckedState::Editable { checked }
}

fn text_value_state(source: &ElementSource) -> TextValueState {
    let initial_value = match source.tag.as_str() {
        "textarea" => Some(source.text.clone()),
        "input"
            if matches!(
                input_type(source).as_deref(),
                None | Some("" | "text" | "email" | "search" | "tel" | "url")
            ) =>
        {
            Some(source.attributes.get("value").cloned().unwrap_or_default())
        }
        _ => None,
    };
    let Some(value) = initial_value else {
        return TextValueState::Unavailable;
    };
    if source.attributes.contains_key("disabled") {
        return TextValueState::NonEditable {
            value,
            reason: "disabled controls cannot be filled".into(),
        };
    }
    if source.attributes.contains_key("readonly") {
        return TextValueState::NonEditable {
            value,
            reason: "read-only controls cannot be filled".into(),
        };
    }
    TextValueState::Editable { value }
}

fn interactive_action(source: &ElementSource, role: &str) -> InteractiveAction {
    let Some(href) = source.attributes.get("href").filter(|_| source.tag == "a") else {
        return InteractiveAction::Unsupported {
            reason: format!("click execution for role {role} is not implemented"),
        };
    };
    if source.attributes.contains_key("download") {
        return InteractiveAction::Unsupported {
            reason: "link downloads are not implemented".into(),
        };
    }
    if let Some(target) = source.attributes.get("target")
        && !target.is_empty()
        && !target.eq_ignore_ascii_case("_self")
    {
        return InteractiveAction::Unsupported {
            reason: "link target browsing contexts are not implemented".into(),
        };
    }
    InteractiveAction::Navigate { href: href.clone() }
}

fn interactive_role(source: &ElementSource) -> Option<String> {
    explicit_interactive_role(source).or_else(|| native_interactive_role(source))
}

fn explicit_interactive_role(source: &ElementSource) -> Option<String> {
    source.attributes.get("role").and_then(|roles| {
        roles
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .find(|role| is_interactive_role(role))
    })
}

fn native_interactive_role(source: &ElementSource) -> Option<String> {
    match source.tag.as_str() {
        "a" if source.attributes.contains_key("href") => Some("link".into()),
        "button" => Some("button".into()),
        "select" => Some("combobox".into()),
        "textarea" => Some("textbox".into()),
        "input" => input_role(input_type(source)),
        _ => None,
    }
}

fn input_role(input_type: Option<String>) -> Option<String> {
    match input_type.as_deref() {
        Some("hidden") => None,
        Some("checkbox") => Some("checkbox".into()),
        Some("radio") => Some("radio".into()),
        Some("range") => Some("slider".into()),
        Some("number") => Some("spinbutton".into()),
        Some("search") => Some("searchbox".into()),
        Some("button" | "image" | "reset" | "submit") => Some("button".into()),
        _ => Some("textbox".into()),
    }
}

fn is_interactive_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "checkbox"
            | "combobox"
            | "link"
            | "listbox"
            | "menuitem"
            | "option"
            | "radio"
            | "searchbox"
            | "slider"
            | "spinbutton"
            | "switch"
            | "tab"
            | "textbox"
            | "treeitem"
    )
}

fn accessible_name(source: &ElementSource, sources: &[ElementSource]) -> String {
    if let Some(name) = labelled_name(source, sources) {
        return name;
    }

    if source.tag == "input" {
        return input_accessible_name(source);
    }
    if matches!(source.tag.as_str(), "select" | "textarea") {
        return title_name(source);
    }

    let text = collapse_whitespace(&source.text);
    if !text.is_empty() {
        return text;
    }
    title_name(source)
}

fn labelled_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    non_empty_attribute(source, "aria-label")
        .map(collapse_whitespace)
        .or_else(|| explicit_label_name(source, sources))
        .or_else(|| ancestor_label_name(source, sources))
}

fn explicit_label_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    let id = source.attributes.get("id")?;
    sources
        .iter()
        .find(|candidate| {
            candidate.tag == "label"
                && candidate
                    .attributes
                    .get("for")
                    .is_some_and(|target| target == id)
        })
        .map(|label| collapse_whitespace(&label.text))
}

fn ancestor_label_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    let mut parent = source.parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "label" {
            return Some(collapse_whitespace(&ancestor.text));
        }
        parent = ancestor.parent;
    }
    None
}

fn input_accessible_name(source: &ElementSource) -> String {
    let input_type = input_type(source);
    if input_type.as_deref() == Some("image") {
        return attribute_name(source, "alt");
    }
    if matches!(input_type.as_deref(), Some("button" | "reset" | "submit")) {
        let value = attribute_name(source, "value");
        if !value.is_empty() {
            return value;
        }
    }
    match input_type.as_deref() {
        Some("submit") => "Submit".into(),
        Some("reset") => "Reset".into(),
        _ => title_name(source),
    }
}

fn input_type(source: &ElementSource) -> Option<String> {
    source
        .attributes
        .get("type")
        .map(|value| value.to_ascii_lowercase())
}

fn attribute_name(source: &ElementSource, attribute: &str) -> String {
    non_empty_attribute(source, attribute)
        .map(collapse_whitespace)
        .unwrap_or_default()
}

fn title_name(source: &ElementSource) -> String {
    attribute_name(source, "title")
}

fn non_empty_attribute<'a>(source: &'a ElementSource, name: &str) -> Option<&'a str> {
    source
        .attributes
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
