use super::visibility::{VisibilityState, visibility_state};
use super::{ElementSource, SelectorIndex, collapse_whitespace, parse_page_source};
use crate::locator::{Locator, LocatorCandidate, SemanticLocatorCandidate, SourceLocatorCandidate};
use crate::non_empty::NonEmpty;
use crate::selection::SelectOptionTarget;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct PageSemanticSource {
    pub(crate) title: String,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) semantic_elements: Vec<SemanticElementSource>,
    pub(crate) locator_elements: Vec<LocatorElementSource>,
    pub(crate) interactive_elements: Vec<InteractiveElementSource>,
    pub(crate) selector_index: SelectorIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocatorElementSource {
    pub(crate) element: String,
    pub(crate) interactive_index: Option<usize>,
    pub(crate) parent: Option<usize>,
    evidence: LocatorEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatorEvidence {
    role: Option<String>,
    name: String,
    text: String,
    label: Option<String>,
    placeholder: Option<String>,
    visibility: VisibilityState,
    source: LocatorSourceEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatorSourceEvidence {
    tag: String,
    attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SemanticElementSource {
    pub(crate) element: String,
    pub(crate) interactive_index: Option<usize>,
    tag: String,
    role: String,
    name: String,
    text: String,
    attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractiveElementSource {
    semantics: SemanticElementSource,
    pub(crate) action: InteractiveAction,
    pub(crate) control_state: ControlState,
    visibility: VisibilityState,
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
pub(crate) enum SelectState {
    Editable(NativeSelect),
    NonEditable {
        select: NativeSelect,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SelectValueError {
    Blocked { reason: String },
    Unsupported { reason: String },
    OptionNotFound { target: SelectOptionTarget },
    OptionDisabled { target: SelectOptionTarget },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeSelect {
    options: Vec<NativeSelectOption>,
    selection: SelectSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectSelection {
    Single(Option<usize>),
    Multiple(Vec<usize>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeSelectOption {
    value: String,
    label: String,
    disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ControlState {
    Text(TextValueState),
    Checkbox(CheckedState),
    Select(SelectState),
    Unavailable,
}

impl InteractiveElementSource {
    pub(crate) fn element(&self) -> &str {
        &self.semantics.element
    }

    pub(crate) fn role(&self) -> &str {
        self.semantics.role()
    }

    pub(crate) fn name(&self) -> &str {
        self.semantics.name()
    }

    pub(crate) fn text(&self) -> &str {
        self.semantics.text()
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

    pub(crate) fn visible(&self) -> Result<bool, &str> {
        match &self.visibility {
            VisibilityState::Visible => Ok(true),
            VisibilityState::Hidden => Ok(false),
            VisibilityState::Unsupported { reason } => Err(reason),
        }
    }

    pub(crate) fn value(&self) -> Option<&str> {
        match &self.control_state {
            ControlState::Text(
                TextValueState::Editable { value } | TextValueState::NonEditable { value, .. },
            ) => Some(value),
            ControlState::Select(state) => state.value(),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn select_value(&mut self, value: &str) -> Result<&str, SelectValueError> {
        match &mut self.control_state {
            ControlState::Select(state) => state.select_value(value),
            ControlState::Text(_) | ControlState::Checkbox(_) | ControlState::Unavailable => {
                Err(SelectValueError::Unsupported {
                    reason: format!(
                        "select execution for role {} is not implemented",
                        self.semantics.role
                    ),
                })
            }
        }
    }

    pub(crate) fn select_options(
        &mut self,
        targets: &NonEmpty<SelectOptionTarget>,
    ) -> Result<NonEmpty<String>, SelectValueError> {
        match &mut self.control_state {
            ControlState::Select(state) => state.select_options(targets),
            ControlState::Text(_) | ControlState::Checkbox(_) | ControlState::Unavailable => {
                Err(SelectValueError::Unsupported {
                    reason: format!(
                        "select execution for role {} is not implemented",
                        self.semantics.role
                    ),
                })
            }
        }
    }

    pub(crate) fn checked(&self) -> Option<bool> {
        match self.control_state {
            ControlState::Checkbox(
                CheckedState::Editable { checked } | CheckedState::NonEditable { checked, .. },
            ) => Some(checked),
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => None,
        }
    }
}

impl SemanticElementSource {
    pub(crate) fn role(&self) -> &str {
        &self.role
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

impl LocatorElementSource {
    pub(crate) fn role(&self) -> Option<&str> {
        self.evidence.role.as_deref()
    }

    pub(crate) fn name(&self) -> &str {
        &self.evidence.name
    }

    pub(crate) fn text(&self) -> &str {
        &self.evidence.text
    }

    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.evidence
            .source
            .attributes
            .get(name)
            .map(String::as_str)
    }

    pub(crate) fn attribute_is_sensitive(&self, name: &str) -> bool {
        name == "value"
            && self.evidence.source.tag == "input"
            && self
                .evidence
                .source
                .attributes
                .get("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("password"))
    }

    pub(crate) fn enabled(&self) -> Option<bool> {
        match self.evidence.source.tag.as_str() {
            "button" | "input" | "select" | "textarea" | "option" | "optgroup" => {
                Some(!self.evidence.source.attributes.contains_key("disabled"))
            }
            "a" if self.evidence.source.attributes.contains_key("href") => Some(true),
            _ => None,
        }
    }

    pub(crate) fn visible(&self) -> Result<bool, &str> {
        match &self.evidence.visibility {
            VisibilityState::Visible => Ok(true),
            VisibilityState::Hidden => Ok(false),
            VisibilityState::Unsupported { reason } => Err(reason),
        }
    }

    pub(crate) fn matches(&self, locator: &Locator) -> bool {
        locator.matches(LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: self.role(),
                name: self.name(),
                text: self.text(),
                label: self.evidence.label.as_deref(),
                placeholder: self.evidence.placeholder.as_deref(),
            },
            source: SourceLocatorCandidate {
                attributes: &self.evidence.source.attributes,
            },
        })
    }
}

impl SelectState {
    fn value(&self) -> Option<&str> {
        match self {
            Self::Editable(select) | Self::NonEditable { select, .. } => Some(select.value()),
        }
    }

    fn select_value(&mut self, value: &str) -> Result<&str, SelectValueError> {
        match self {
            Self::Editable(select) => select.select_value(value),
            Self::NonEditable { reason, .. } => Err(SelectValueError::Blocked {
                reason: reason.clone(),
            }),
        }
    }

    fn select_options(
        &mut self,
        targets: &NonEmpty<SelectOptionTarget>,
    ) -> Result<NonEmpty<String>, SelectValueError> {
        match self {
            Self::Editable(select) => select.select_options(targets),
            Self::NonEditable { reason, .. } => Err(SelectValueError::Blocked {
                reason: reason.clone(),
            }),
        }
    }
}

impl NativeSelect {
    fn value(&self) -> &str {
        self.first_selected_index()
            .map(|index| self.options[index].value.as_str())
            .unwrap_or_default()
    }

    fn select_value(&mut self, value: &str) -> Result<&str, SelectValueError> {
        let targets = NonEmpty::one(SelectOptionTarget::Value(value.to_owned()));
        self.select_options(&targets)?;
        Ok(self.value())
    }

    fn select_options(
        &mut self,
        targets: &NonEmpty<SelectOptionTarget>,
    ) -> Result<NonEmpty<String>, SelectValueError> {
        let mut requested_indices = Vec::with_capacity(targets.len());
        for target in targets.iter() {
            let index =
                self.option_index(target)
                    .ok_or_else(|| SelectValueError::OptionNotFound {
                        target: target.clone(),
                    })?;
            if self.options[index].disabled {
                return Err(SelectValueError::OptionDisabled {
                    target: target.clone(),
                });
            }
            if !requested_indices.contains(&index) {
                requested_indices.push(index);
            }
        }

        if matches!(self.selection, SelectSelection::Single(_)) {
            let index = requested_indices
                .iter()
                .copied()
                .min()
                .expect("non-empty requested values resolve at least one option");
            self.selection = SelectSelection::Single(Some(index));
            return Ok(NonEmpty::one(self.options[index].value.clone()));
        }

        let selected_values = requested_indices
            .iter()
            .map(|index| self.options[*index].value.clone())
            .collect();
        let mut selected_indices = requested_indices;
        selected_indices.sort_unstable();
        self.selection = SelectSelection::Multiple(selected_indices);
        Ok(NonEmpty::from_vec(selected_values)
            .expect("non-empty requested values resolve at least one option"))
    }

    fn first_selected_index(&self) -> Option<usize> {
        match &self.selection {
            SelectSelection::Single(selected) => *selected,
            SelectSelection::Multiple(selected) => selected.first().copied(),
        }
    }

    fn option_index(&self, target: &SelectOptionTarget) -> Option<usize> {
        match target {
            SelectOptionTarget::Value(value) => self
                .options
                .iter()
                .position(|option| option.value == *value),
            SelectOptionTarget::Label(label) => self
                .options
                .iter()
                .position(|option| option.label == *label),
            SelectOptionTarget::Index(index) => (*index < self.options.len()).then_some(*index),
        }
    }
}

#[cfg(test)]
pub(crate) fn interactive_elements_from_html(html: &str) -> Vec<InteractiveElementSource> {
    page_semantics_from_html(html).interactive_elements
}

#[cfg(test)]
pub(crate) fn semantic_elements_from_html(html: &str) -> Vec<SemanticElementSource> {
    page_semantics_from_html(html).semantic_elements
}

pub(crate) fn page_semantics_from_html(html: &str) -> PageSemanticSource {
    let source = parse_page_source(html);
    let selector_index = SelectorIndex::new(html, &source.elements);
    let (semantic_elements, locator_elements, interactive_elements) =
        element_sources(&source.elements, source.has_stylesheet);
    PageSemanticSource {
        title: collapse_whitespace(&source.title),
        semantic_elements,
        locator_elements,
        interactive_elements,
        selector_index,
    }
}

fn element_sources(
    sources: &[ElementSource],
    has_stylesheet: bool,
) -> (
    Vec<SemanticElementSource>,
    Vec<LocatorElementSource>,
    Vec<InteractiveElementSource>,
) {
    let mut semantic_elements = Vec::new();
    let mut locator_elements = Vec::with_capacity(sources.len());
    let mut interactive_elements = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let role = semantic_role(index, source, sources);
        let visibility = visibility_state(index, source, sources, has_stylesheet);
        let interactive_index = role
            .as_deref()
            .is_some_and(is_interactive_role)
            .then_some(interactive_elements.len());
        let name = role
            .as_deref()
            .map(|role| accessible_name(source, sources, role))
            .unwrap_or_default();
        let text = locator_text(source);
        locator_elements.push(LocatorElementSource {
            element: source.id.clone(),
            interactive_index,
            parent: source.parent,
            evidence: LocatorEvidence {
                role: role.clone(),
                name: name.clone(),
                text: text.clone(),
                label: locator_label(source, sources),
                placeholder: locator_placeholder(source),
                visibility: visibility.clone(),
                source: LocatorSourceEvidence {
                    tag: source.tag.clone(),
                    attributes: source.attributes.clone(),
                },
            },
        });
        if let Some(role) = role {
            let interactive_index =
                is_interactive_role(&role).then_some(interactive_elements.len());
            let semantics = SemanticElementSource {
                element: source.id.clone(),
                interactive_index,
                tag: source.tag.clone(),
                name,
                role: role.clone(),
                text,
                attributes: source.attributes.clone(),
            };
            if interactive_index.is_some() {
                let action = interactive_action(source, &role);
                let control_state = control_state(index, source, sources);
                interactive_elements.push(InteractiveElementSource {
                    semantics: semantics.clone(),
                    action,
                    control_state,
                    visibility,
                });
            }
            semantic_elements.push(semantics);
        }
    }
    (semantic_elements, locator_elements, interactive_elements)
}

fn locator_text(source: &ElementSource) -> String {
    if source.tag == "input" && matches!(input_type(source).as_deref(), Some("button" | "submit")) {
        return attribute_name(source, "value");
    }
    collapse_whitespace(&source.text)
}

fn locator_label(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    if !matches!(
        source.tag.as_str(),
        "button" | "input" | "meter" | "output" | "progress" | "select" | "textarea"
    ) {
        return None;
    }
    author_name(source, sources)
}

fn locator_placeholder(source: &ElementSource) -> Option<String> {
    if !matches!(source.tag.as_str(), "input" | "textarea") {
        return None;
    }
    non_empty_attribute(source, "placeholder").map(collapse_whitespace)
}

fn control_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> ControlState {
    if source.tag == "input" && input_type(source).as_deref() == Some("checkbox") {
        return ControlState::Checkbox(checkbox_state(source));
    }
    if source.tag == "select" {
        return ControlState::Select(select_state(source_index, source, sources));
    }
    let text = text_value_state(source);
    if text == TextValueState::Unavailable {
        ControlState::Unavailable
    } else {
        ControlState::Text(text)
    }
}

fn select_state(
    select_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> SelectState {
    let multiple = source.attributes.contains_key("multiple");
    let mut options = Vec::new();
    let mut selected = Vec::new();
    for option in sources.iter().filter(|candidate| {
        candidate.tag == "option"
            && nearest_select_ancestor(candidate.parent, sources) == Some(select_index)
    }) {
        if option.attributes.contains_key("selected") {
            if multiple {
                selected.push(options.len());
            } else {
                selected.clear();
                selected.push(options.len());
            }
        }
        options.push(NativeSelectOption {
            value: option
                .attributes
                .get("value")
                .cloned()
                .unwrap_or_else(|| collapse_whitespace(&option.text)),
            label: option
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| collapse_whitespace(&option.text)),
            disabled: option_is_disabled(option, sources),
        });
    }

    if selected.is_empty()
        && !multiple
        && select_display_size(source) == 1
        && let Some(index) = options.iter().position(|option| !option.disabled)
    {
        selected.push(index);
    }
    let selection = if multiple {
        SelectSelection::Multiple(selected)
    } else {
        SelectSelection::Single(selected.first().copied())
    };
    let select = NativeSelect { options, selection };
    if source.attributes.contains_key("disabled") {
        SelectState::NonEditable {
            select,
            reason: "disabled select controls cannot change value".into(),
        }
    } else {
        SelectState::Editable(select)
    }
}

fn nearest_select_ancestor(mut parent: Option<usize>, sources: &[ElementSource]) -> Option<usize> {
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "select" {
            return Some(index);
        }
        parent = ancestor.parent;
    }
    None
}

fn option_is_disabled(option: &ElementSource, sources: &[ElementSource]) -> bool {
    if option.attributes.contains_key("disabled") {
        return true;
    }
    let mut parent = option.parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "select" {
            return false;
        }
        if ancestor.tag == "optgroup" && ancestor.attributes.contains_key("disabled") {
            return true;
        }
        parent = ancestor.parent;
    }
    false
}

fn select_display_size(source: &ElementSource) -> u64 {
    source
        .attributes
        .get("size")
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(1)
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

fn semantic_role(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> Option<String> {
    explicit_semantic_role(source)
        .or_else(|| native_interactive_role(source))
        .or_else(|| native_structural_role(source_index, source, sources))
}

fn explicit_semantic_role(source: &ElementSource) -> Option<String> {
    source.attributes.get("role").and_then(|roles| {
        roles
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .find(|role| is_supported_role(role))
    })
}

fn native_interactive_role(source: &ElementSource) -> Option<String> {
    match source.tag.as_str() {
        "a" if source.attributes.contains_key("href") => Some("link".into()),
        "button" => Some("button".into()),
        "select"
            if source.attributes.contains_key("multiple") || select_display_size(source) > 1 =>
        {
            Some("listbox".into())
        }
        "select" => Some("combobox".into()),
        "textarea" => Some("textbox".into()),
        "input" => input_role(input_type(source)),
        _ => None,
    }
}

fn native_structural_role(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> Option<String> {
    let role = match source.tag.as_str() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "ul" | "ol" | "menu" => "list",
        "li" => "listitem",
        "nav" => "navigation",
        "main" => "main",
        "aside" => "complementary",
        "header" if !has_sectioning_ancestor(source_index, sources) => "banner",
        "footer" if !has_sectioning_ancestor(source_index, sources) => "contentinfo",
        "section" if author_name(source, sources).is_some() => "region",
        "form" if author_name(source, sources).is_some() => "form",
        "article" => "article",
        "blockquote" => "blockquote",
        "dialog" => "dialog",
        "fieldset" | "details" => "group",
        "figure" => "figure",
        "hr" => "separator",
        "progress" => "progressbar",
        "output" => "status",
        "table" => "table",
        "thead" | "tbody" | "tfoot" => "rowgroup",
        "tr" => "row",
        "td" => "cell",
        "th" if source.attributes.get("scope").is_some_and(|scope| {
            scope.eq_ignore_ascii_case("row") || scope.eq_ignore_ascii_case("rowgroup")
        }) =>
        {
            "rowheader"
        }
        "th" => "columnheader",
        "img"
            if source
                .attributes
                .get("alt")
                .is_some_and(|alt| alt.trim().is_empty()) =>
        {
            return None;
        }
        "img" => "img",
        "p" => "paragraph",
        _ => return None,
    };
    Some(role.into())
}

fn has_sectioning_ancestor(source_index: usize, sources: &[ElementSource]) -> bool {
    let mut parent = sources[source_index].parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if matches!(
            ancestor.tag.as_str(),
            "article" | "aside" | "main" | "nav" | "section"
        ) {
            return true;
        }
        parent = ancestor.parent;
    }
    false
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

fn is_supported_role(role: &str) -> bool {
    is_interactive_role(role)
        || matches!(
            role,
            "alert"
                | "article"
                | "banner"
                | "blockquote"
                | "cell"
                | "columnheader"
                | "complementary"
                | "contentinfo"
                | "dialog"
                | "figure"
                | "form"
                | "group"
                | "heading"
                | "img"
                | "list"
                | "listitem"
                | "main"
                | "navigation"
                | "paragraph"
                | "progressbar"
                | "region"
                | "row"
                | "rowgroup"
                | "rowheader"
                | "separator"
                | "status"
                | "table"
        )
}

fn accessible_name(source: &ElementSource, sources: &[ElementSource], role: &str) -> String {
    if let Some(name) = author_name(source, sources) {
        return name;
    }

    if source.tag == "input" {
        return input_accessible_name(source);
    }
    if matches!(source.tag.as_str(), "select" | "textarea") {
        return title_name(source);
    }

    if source.tag == "img" {
        return attribute_name(source, "alt");
    }
    if name_from_content(role) {
        let text = collapse_whitespace(&source.text);
        if !text.is_empty() {
            return text;
        }
    }
    title_name(source)
}

fn author_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    aria_labelled_name(source, sources)
        .or_else(|| non_empty_attribute(source, "aria-label").map(collapse_whitespace))
        .or_else(|| html_label_name(source, sources))
}

fn aria_labelled_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    let references = non_empty_attribute(source, "aria-labelledby")?;
    let name = references
        .split_ascii_whitespace()
        .filter_map(|reference| {
            sources.iter().find(|candidate| {
                candidate
                    .attributes
                    .get("id")
                    .is_some_and(|id| id == reference)
            })
        })
        .map(|candidate| collapse_whitespace(&candidate.text))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!name.is_empty()).then_some(name)
}

fn html_label_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    if !matches!(
        source.tag.as_str(),
        "button" | "input" | "meter" | "output" | "progress" | "select" | "textarea"
    ) {
        return None;
    }
    explicit_label_name(source, sources).or_else(|| ancestor_label_name(source, sources))
}

fn name_from_content(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "cell"
            | "checkbox"
            | "columnheader"
            | "heading"
            | "link"
            | "listitem"
            | "menuitem"
            | "option"
            | "radio"
            | "row"
            | "rowheader"
            | "switch"
            | "tab"
            | "treeitem"
    )
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
