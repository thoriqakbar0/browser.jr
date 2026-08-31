use super::visibility::{
    UNKNOWN_BOX_GEOMETRY, VisibilityState, accessibility_visibility_state, focus_visibility_state,
    visibility_state,
};
use super::{
    ElementChildSource, ElementSource, SelectorIndex, collapse_whitespace, page_computed_styles,
    parse_page_source, resolve_bounding_boxes_with_styles,
};
use crate::BoundingBox;
use crate::keyboard::{FocusedElement, KeyboardKey, TextKeyOutcome, TextSelectionState};
use crate::locator::{
    AccessibilityLocatorCandidate, Locator, LocatorCandidate, RoleStateCandidate, RoleStateValues,
    SemanticLocatorCandidate, SourceLocatorCandidate,
};
use crate::non_empty::NonEmpty;
use crate::selection::SelectOptionTarget;
#[cfg(test)]
use crate::{DEFAULT_VIEWPORT_HEIGHT, DEFAULT_VIEWPORT_WIDTH};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct PageSemanticSource {
    pub(crate) document: PageDocumentSource,
    pub(crate) elements: PageElementSources,
    pub(crate) selector_index: SelectorIndex,
    pub(crate) sequential_focus: SequentialFocusSource,
    pub(crate) extent: PageExtent,
}

#[derive(Clone, Debug)]
pub(crate) struct PageDocumentSource {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) accessibility_tree: Vec<AccessibilityNodeSource>,
}

#[derive(Clone, Debug)]
pub(crate) struct PageElementSources {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) semantic_elements: Vec<SemanticElementSource>,
    pub(crate) locator_elements: Vec<LocatorElementSource>,
    pub(crate) interactive_elements: Vec<InteractiveElementSource>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PageExtent {
    pub(crate) document_width: u64,
    pub(crate) document_height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccessibilityNodeSource {
    pub(crate) depth: u64,
    pub(crate) origin: AccessibilityNodeOrigin,
    pub(crate) role: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AccessibilityNodeOrigin {
    Element {
        owner_source_index: usize,
        reference_source_index: Option<usize>,
    },
    DocumentGenerated,
}

impl AccessibilityNodeOrigin {
    pub(crate) const fn owner_source_index(&self) -> Option<usize> {
        match self {
            Self::Element {
                owner_source_index, ..
            } => Some(*owner_source_index),
            Self::DocumentGenerated => None,
        }
    }

    pub(crate) const fn reference_source_index(&self) -> Option<usize> {
        match self {
            Self::Element {
                reference_source_index,
                ..
            } => *reference_source_index,
            Self::DocumentGenerated => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SequentialFocusSource {
    Supported { order: Vec<usize> },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocatorElementSource {
    pub(crate) element: String,
    pub(crate) interactive_index: Option<usize>,
    pub(crate) parent: Option<usize>,
    pub(crate) content_ordinal: Option<usize>,
    pub(crate) form_owner: Option<usize>,
    evidence: LocatorEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatorEvidence {
    semantic: LocatorSemanticEvidence,
    visibility: VisibilityState,
    stability: StabilityState,
    bounding_box: BoundingBoxEvidence,
    source: LocatorSourceEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatorSemanticEvidence {
    accessibility: AccessibilityEvidence,
    text: String,
    label: Option<String>,
    placeholder: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AccessibilityEvidence {
    role: Option<String>,
    name: String,
    description: String,
    role_state: RoleStateEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoleStateEvidence {
    values: RoleStateValues,
    level: Option<u32>,
    visibility: VisibilityState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BoundingBoxEvidence {
    Visible {
        value: BoundingBox,
        scrolls_with_document: bool,
    },
    Hidden,
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StabilityState {
    Stable,
    Unsupported(String),
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
    pub(crate) source_index: usize,
    pub(crate) content_ordinal: usize,
    pub(crate) action: InteractiveAction,
    pub(crate) control_state: ControlState,
    visibility: VisibilityState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InteractiveAction {
    Navigate { href: String },
    SubmitForm { form_owner: usize },
    Activate,
    ToggleCheckbox,
    SelectRadio,
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TextValueState {
    Editable {
        value: String,
        selection: TextSelectionState,
    },
    NonEditable {
        value: String,
        selection: TextSelectionState,
        reason: String,
    },
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TextValueError {
    Blocked { reason: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CheckedState {
    Editable { checked: bool },
    NonEditable { checked: bool, reason: String },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RadioGroup {
    Named {
        name: String,
        form_owner: Option<usize>,
    },
    Singleton {
        source_index: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RadioState {
    pub(crate) checked: CheckedState,
    pub(crate) group: RadioGroup,
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
    Radio(RadioState),
    Select(SelectState),
    Unavailable,
}

impl CheckedState {
    pub(crate) fn checked(&self) -> bool {
        match self {
            Self::Editable { checked } | Self::NonEditable { checked, .. } => *checked,
        }
    }

    pub(crate) fn set_checked(&mut self, replacement: bool) {
        match self {
            Self::Editable { checked } | Self::NonEditable { checked, .. } => {
                *checked = replacement;
            }
        }
    }

    pub(crate) fn block_reason(&self) -> Option<&str> {
        match self {
            Self::Editable { .. } => None,
            Self::NonEditable { reason, .. } => Some(reason),
        }
    }
}

impl RadioState {
    pub(crate) fn checked(&self) -> bool {
        self.checked.checked()
    }

    pub(crate) fn set_checked(&mut self, replacement: bool) {
        self.checked.set_checked(replacement);
    }

    pub(crate) fn block_reason(&self) -> Option<&str> {
        self.checked.block_reason()
    }
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

    pub(crate) fn focused_element(&self) -> FocusedElement {
        FocusedElement::new(self.element(), self.role(), self.name())
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

    pub(crate) fn has_positive_tabindex(&self) -> bool {
        self.semantics
            .attributes
            .get("tabindex")
            .and_then(|value| value.trim().parse::<i64>().ok())
            .is_some_and(|value| value > 0)
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

    pub(crate) fn focus_block_reason(&self) -> Option<String> {
        if self.semantics.attributes.contains_key("disabled")
            && matches!(
                self.semantics.tag.as_str(),
                "button" | "input" | "select" | "textarea"
            )
        {
            return Some("disabled native controls cannot take focus".into());
        }
        let native_focus_target = matches!(
            self.semantics.tag.as_str(),
            "button" | "input" | "select" | "textarea"
        ) || (self.semantics.tag == "a"
            && self.semantics.attributes.contains_key("href"));
        let explicit_focus_target = self
            .semantics
            .attributes
            .get("tabindex")
            .is_some_and(|value| value.parse::<i32>().is_ok());
        (!native_focus_target && !explicit_focus_target).then(|| {
            format!(
                "focus execution for role {} is not implemented",
                self.semantics.role
            )
        })
    }

    pub(crate) fn is_multiline_text_control(&self) -> bool {
        self.semantics.tag == "textarea"
    }

    pub(crate) fn is_single_line_text_control(&self) -> bool {
        self.semantics.tag == "input" && matches!(self.control_state, ControlState::Text(_))
    }

    pub(crate) fn keyboard_text_editable(&self) -> Option<bool> {
        match self.control_state {
            ControlState::Text(TextValueState::Editable { .. }) => Some(true),
            ControlState::Text(TextValueState::NonEditable { .. }) => Some(false),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn visible(&self) -> Result<bool, &str> {
        match &self.visibility {
            VisibilityState::Visible => Ok(true),
            VisibilityState::Hidden => Ok(false),
            VisibilityState::Unsupported { reason } => Err(reason),
        }
    }

    fn role_checked(&self) -> Option<bool> {
        match &self.control_state {
            ControlState::Checkbox(state) => Some(state.checked()),
            ControlState::Radio(state) => Some(state.checked()),
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn value(&self) -> Option<&str> {
        match &self.control_state {
            ControlState::Text(
                TextValueState::Editable { value, .. } | TextValueState::NonEditable { value, .. },
            ) => Some(value),
            ControlState::Select(state) => state.value(),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn form_values(&self, submitter: bool) -> Result<Vec<String>, String> {
        let input_type = self
            .semantics
            .attributes
            .get("type")
            .map(|value| value.to_ascii_lowercase());
        match self.semantics.tag.as_str() {
            "textarea" => Ok(vec![self.value().unwrap_or_default().into()]),
            "select" => match &self.control_state {
                ControlState::Select(state) => Ok(state.selected_values()),
                _ => unreachable!("native selects retain select state"),
            },
            "button" => Ok(submitter
                .then(|| self.attribute("value").unwrap_or_default().into())
                .into_iter()
                .collect()),
            "input" => self.input_form_values(input_type.as_deref(), submitter),
            _ => Ok(Vec::new()),
        }
    }

    fn input_form_values(
        &self,
        input_type: Option<&str>,
        submitter: bool,
    ) -> Result<Vec<String>, String> {
        match input_type.unwrap_or("text") {
            "button" | "reset" => Ok(Vec::new()),
            "submit" => Ok(submitter
                .then(|| self.attribute("value").unwrap_or_default().into())
                .into_iter()
                .collect()),
            "image" => Err("image submit coordinates are not implemented".into()),
            "file" => Err("file input form submission is not implemented".into()),
            "checkbox" | "radio" if self.checked() != Some(true) => Ok(Vec::new()),
            "checkbox" | "radio" => Ok(vec![self.attribute("value").unwrap_or("on").into()]),
            "" | "email" | "search" | "tel" | "text" | "url" => {
                Ok(vec![self.value().unwrap_or_default().into()])
            }
            "password" => Ok(vec![self.attribute("value").unwrap_or_default().into()]),
            other => Err(format!(
                "form submission for input type {other:?} is not implemented"
            )),
        }
    }

    pub(crate) fn replace_text(&mut self, replacement: String) -> Result<&str, TextValueError> {
        let role = self.semantics.role.clone();
        match &mut self.control_state {
            ControlState::Text(TextValueState::Editable { value, selection }) => {
                *value = replacement;
                selection.collapse_at_end(value);
                Ok(value)
            }
            ControlState::Text(TextValueState::NonEditable { reason, .. }) => {
                Err(TextValueError::Blocked {
                    reason: reason.clone(),
                })
            }
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => Err(TextValueError::Unsupported {
                reason: format!("fill execution for role {role} is not implemented"),
            }),
        }
    }

    pub(crate) fn append_text(&mut self, text: &str) -> Result<&str, TextValueError> {
        let role = self.semantics.role.clone();
        match &mut self.control_state {
            ControlState::Text(TextValueState::Editable { value, .. }) => {
                value.push_str(text);
                Ok(value)
            }
            ControlState::Text(TextValueState::NonEditable { reason, .. }) => {
                Err(TextValueError::Blocked {
                    reason: reason.clone(),
                })
            }
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => Err(TextValueError::Unsupported {
                reason: format!("type execution for role {role} is not implemented"),
            }),
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) -> Option<(String, TextKeyOutcome)> {
        match &mut self.control_state {
            ControlState::Text(TextValueState::Editable { value, selection }) => {
                let outcome = selection.insert_text(value, text, true);
                Some((value.clone(), outcome))
            }
            ControlState::Text(TextValueState::NonEditable {
                value, selection, ..
            }) => {
                let outcome = selection.insert_text(value, text, false);
                Some((value.clone(), outcome))
            }
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn type_text(&mut self, text: &str) -> Option<(String, TextKeyOutcome)> {
        let multiline = self.is_multiline_text_control();
        match &mut self.control_state {
            ControlState::Text(TextValueState::Editable { value, selection }) => {
                let outcome = selection.type_text(value, text, multiline, true);
                Some((value.clone(), outcome))
            }
            ControlState::Text(TextValueState::NonEditable {
                value, selection, ..
            }) => {
                let outcome = selection.type_text(value, text, multiline, false);
                Some((value.clone(), outcome))
            }
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => None,
        }
    }

    pub(crate) fn press_key(
        &mut self,
        key: &KeyboardKey,
    ) -> Result<(String, TextKeyOutcome), TextValueError> {
        let role = self.semantics.role.clone();
        let multiline = self.is_multiline_text_control();
        let result = match &mut self.control_state {
            ControlState::Text(TextValueState::Editable { value, selection }) => key
                .apply_to_text(value, selection, multiline, true)
                .map(|outcome| (value.clone(), outcome)),
            ControlState::Text(TextValueState::NonEditable {
                value, selection, ..
            }) => key
                .apply_to_text(value, selection, multiline, false)
                .map(|outcome| (value.clone(), outcome)),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => {
                return Err(TextValueError::Unsupported {
                    reason: format!("press execution for role {role} is not implemented"),
                });
            }
        };
        result.map_err(|error| TextValueError::Unsupported {
            reason: error.reason,
        })
    }

    pub(crate) fn select_value(&mut self, value: &str) -> Result<&str, SelectValueError> {
        match &mut self.control_state {
            ControlState::Select(state) => state.select_value(value),
            ControlState::Text(_)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Unavailable => Err(SelectValueError::Unsupported {
                reason: format!(
                    "select execution for role {} is not implemented",
                    self.semantics.role
                ),
            }),
        }
    }

    pub(crate) fn select_options(
        &mut self,
        targets: &NonEmpty<SelectOptionTarget>,
    ) -> Result<NonEmpty<String>, SelectValueError> {
        match &mut self.control_state {
            ControlState::Select(state) => state.select_options(targets),
            ControlState::Text(_)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Unavailable => Err(SelectValueError::Unsupported {
                reason: format!(
                    "select execution for role {} is not implemented",
                    self.semantics.role
                ),
            }),
        }
    }

    pub(crate) fn checked(&self) -> Option<bool> {
        match &self.control_state {
            ControlState::Checkbox(state) => Some(state.checked()),
            ControlState::Radio(state) => Some(state.checked()),
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => None,
        }
    }
}

fn native_editable_state(tag: &str, attributes: &BTreeMap<String, String>) -> Option<bool> {
    match tag {
        "input" | "textarea" => {
            Some(!attributes.contains_key("disabled") && !attributes.contains_key("readonly"))
        }
        "select" => Some(!attributes.contains_key("disabled")),
        _ => None,
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
    pub(crate) fn is_body(&self) -> bool {
        self.evidence.source.tag == "body"
    }

    pub(crate) fn tag(&self) -> &str {
        &self.evidence.source.tag
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.evidence.source.attributes.contains_key("disabled")
    }

    pub(crate) fn is_native_submit_button(&self) -> bool {
        let input_type = self
            .attribute("type")
            .map(|value| value.to_ascii_lowercase());
        match self.tag() {
            "button" => !matches!(input_type.as_deref(), Some("button" | "reset")),
            "input" => matches!(input_type.as_deref(), Some("image" | "submit")),
            _ => false,
        }
    }

    pub(crate) fn blocks_implicit_submission(&self) -> bool {
        if self.tag() != "input" {
            return false;
        }
        let input_type = self
            .attribute("type")
            .map(|value| value.to_ascii_lowercase());
        matches!(
            input_type.as_deref().unwrap_or("text"),
            "" | "text"
                | "search"
                | "tel"
                | "url"
                | "email"
                | "password"
                | "date"
                | "month"
                | "week"
                | "time"
                | "datetime-local"
                | "number"
        )
    }

    pub(crate) fn role(&self) -> Option<&str> {
        self.evidence.semantic.accessibility.role.as_deref()
    }

    pub(crate) fn name(&self) -> &str {
        &self.evidence.semantic.accessibility.name
    }

    pub(crate) fn text(&self) -> &str {
        &self.evidence.semantic.text
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

    pub(crate) fn native_editable(&self) -> Option<bool> {
        native_editable_state(&self.evidence.source.tag, &self.evidence.source.attributes)
    }

    pub(crate) fn content_editable_value(&self) -> Option<bool> {
        self.evidence
            .source
            .attributes
            .get("contenteditable")
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                "" | "true" | "plaintext-only" => Some(true),
                "false" => Some(false),
                _ => None,
            })
    }

    pub(crate) fn is_disabled_fieldset(&self) -> bool {
        self.evidence.source.tag == "fieldset"
            && self.evidence.source.attributes.contains_key("disabled")
    }

    pub(crate) fn visible(&self) -> Result<bool, &str> {
        match &self.evidence.visibility {
            VisibilityState::Visible => Ok(true),
            VisibilityState::Hidden => Ok(false),
            VisibilityState::Unsupported { reason } => Err(reason),
        }
    }

    pub(crate) fn stable(&self) -> Result<(), &str> {
        match &self.evidence.stability {
            StabilityState::Stable => Ok(()),
            StabilityState::Unsupported(reason) => Err(reason),
        }
    }

    pub(crate) fn bounding_box(
        &self,
        scroll_x: u64,
        scroll_y: u64,
    ) -> Result<Option<BoundingBox>, &str> {
        match &self.evidence.bounding_box {
            BoundingBoxEvidence::Visible {
                value,
                scrolls_with_document,
            } => Ok(Some(if *scrolls_with_document {
                BoundingBox {
                    x: subtract_scroll(value.x, scroll_x),
                    y: subtract_scroll(value.y, scroll_y),
                    width: value.width,
                    height: value.height,
                }
            } else {
                *value
            })),
            BoundingBoxEvidence::Hidden => Ok(None),
            BoundingBoxEvidence::Unsupported(reason) => Err(reason),
        }
    }

    pub(crate) fn document_bounding_box(&self) -> Result<Option<(BoundingBox, bool)>, &str> {
        match &self.evidence.bounding_box {
            BoundingBoxEvidence::Visible {
                value,
                scrolls_with_document,
            } => Ok(Some((*value, *scrolls_with_document))),
            BoundingBoxEvidence::Hidden => Ok(None),
            BoundingBoxEvidence::Unsupported(reason) => Err(reason),
        }
    }

    pub(crate) fn matches<'a>(
        &'a self,
        locator: &Locator,
        interactive: Option<&'a InteractiveElementSource>,
    ) -> Result<bool, &'a str> {
        if self.content_ordinal.is_none() {
            return Ok(false);
        }

        let semantic = &self.evidence.semantic;
        let role_state = &semantic.accessibility.role_state;
        let visible_to_accessibility = match &role_state.visibility {
            VisibilityState::Visible => Ok(true),
            VisibilityState::Hidden => Ok(false),
            VisibilityState::Unsupported { reason } => Err(reason.as_str()),
        };
        locator.matches(LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                accessibility: AccessibilityLocatorCandidate {
                    role: self.role(),
                    name: self.name(),
                    description: &semantic.accessibility.description,
                    role_state: RoleStateCandidate {
                        values: RoleStateValues {
                            checked: interactive
                                .and_then(InteractiveElementSource::role_checked)
                                .or(role_state.values.checked),
                            ..role_state.values
                        },
                        level: role_state.level,
                        visible_to_accessibility,
                    },
                },
                text: self.text(),
                label: semantic.label.as_deref(),
                placeholder: semantic.placeholder.as_deref(),
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

    fn selected_values(&self) -> Vec<String> {
        match self {
            Self::Editable(select) | Self::NonEditable { select, .. } => select.selected_values(),
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

    fn selected_values(&self) -> Vec<String> {
        let selected = match &self.selection {
            SelectSelection::Single(selected) => selected.iter().copied().collect::<Vec<_>>(),
            SelectSelection::Multiple(selected) => selected.clone(),
        };
        selected
            .into_iter()
            .filter_map(|index| {
                let option = &self.options[index];
                (!option.disabled).then(|| option.value.clone())
            })
            .collect()
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
    page_semantics_from_html(html).elements.interactive_elements
}

#[cfg(test)]
pub(crate) fn semantic_elements_from_html(html: &str) -> Vec<SemanticElementSource> {
    page_semantics_from_html(html).elements.semantic_elements
}

#[cfg(test)]
pub(crate) fn page_semantics_from_html(html: &str) -> PageSemanticSource {
    page_semantics_from_html_with_viewport(html, DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT)
}

pub(crate) fn page_semantics_from_html_with_viewport(
    html: &str,
    viewport_width: u64,
    viewport_height: u64,
) -> PageSemanticSource {
    let source = parse_page_source(html);
    let selector_index = SelectorIndex::new(html, &source.elements);
    let styles = page_computed_styles(&source.elements, &source.styles);
    let style_evidence = styles.as_deref().map_err(String::as_str);
    let (
        semantic_elements,
        locator_elements,
        interactive_elements,
        document_width,
        document_height,
    ) = element_sources(
        &source.elements,
        style_evidence,
        viewport_width,
        viewport_height,
    );
    let sequential_focus = sequential_focus_source(
        &source.elements,
        style_evidence,
        &locator_elements,
        &interactive_elements,
    );
    let accessibility_tree = accessibility_tree_source(&source.elements, style_evidence);
    PageSemanticSource {
        document: PageDocumentSource {
            title: collapse_whitespace(&source.title),
            text: collapse_whitespace(&source.readable_text),
            accessibility_tree,
        },
        elements: PageElementSources {
            semantic_elements,
            locator_elements,
            interactive_elements,
        },
        selector_index,
        sequential_focus,
        extent: PageExtent {
            document_width,
            document_height,
        },
    }
}

fn accessibility_tree_source(
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
) -> Vec<AccessibilityNodeSource> {
    let mut builder = AccessibilityTreeBuilder {
        sources,
        styles,
        nodes: Vec::new(),
    };
    for (source_index, source) in sources.iter().enumerate() {
        if source.parent.is_none() {
            builder.append_subtree(source_index, 0);
        }
    }
    builder.append_list_markers();
    builder.nodes
}

fn resolved_focus_visibility(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
) -> VisibilityState {
    match styles {
        Ok(styles) => focus_visibility_state(source_index, source, sources, styles),
        Err(reason) => VisibilityState::Unsupported {
            reason: reason.into(),
        },
    }
}

fn resolved_accessibility_visibility(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
) -> VisibilityState {
    match styles {
        Ok(styles) => accessibility_visibility_state(source_index, source, sources, styles),
        Err(reason) => VisibilityState::Unsupported {
            reason: reason.into(),
        },
    }
}

struct AccessibilityTreeBuilder<'a> {
    sources: &'a [ElementSource],
    styles: Result<&'a [BTreeMap<String, String>], &'a str>,
    nodes: Vec<AccessibilityNodeSource>,
}

impl AccessibilityTreeBuilder<'_> {
    fn append_subtree(&mut self, source_index: usize, depth: u64) {
        let source = &self.sources[source_index];
        if resolved_accessibility_visibility(source_index, source, self.sources, self.styles)
            == VisibilityState::Hidden
        {
            return;
        }
        let Some(child_depth) = self.emit_role(source_index, depth) else {
            return;
        };
        self.append_children(source_index, child_depth);
    }

    fn emit_role(&mut self, source_index: usize, depth: u64) -> Option<u64> {
        let source = &self.sources[source_index];
        let Some(role) = accessibility_tree_role(source_index, source, self.sources) else {
            return Some(depth);
        };
        self.nodes.push(AccessibilityNodeSource {
            depth,
            origin: AccessibilityNodeOrigin::Element {
                owner_source_index: source_index,
                reference_source_index: Some(source_index),
            },
            role: role.clone(),
            name: accessibility_tree_name(source, self.sources, &role),
        });
        if accessibility_tree_role_is_leaf(&role)
            || (role == "heading" && !accessibility_descendant_has_role(source, self.sources))
        {
            return None;
        }
        Some(depth.saturating_add(1))
    }

    fn append_children(&mut self, source_index: usize, child_depth: u64) {
        let children = self.sources[source_index].content.children.clone();
        for child in children {
            match child {
                ElementChildSource::Element(index) => self.append_subtree(index, child_depth),
                ElementChildSource::Text(text) => {
                    let text = collapse_whitespace(&text);
                    if !text.is_empty() {
                        self.nodes.push(AccessibilityNodeSource {
                            depth: child_depth,
                            origin: AccessibilityNodeOrigin::Element {
                                owner_source_index: source_index,
                                reference_source_index: None,
                            },
                            role: "StaticText".into(),
                            name: text,
                        });
                    }
                }
            }
        }
    }

    fn append_list_markers(&mut self) {
        let markers = self
            .sources
            .iter()
            .enumerate()
            .filter_map(|(source_index, _)| self.list_marker_name(source_index))
            .collect::<Vec<_>>();
        self.nodes
            .extend(markers.into_iter().map(|name| AccessibilityNodeSource {
                depth: 0,
                origin: AccessibilityNodeOrigin::DocumentGenerated,
                role: "ListMarker".into(),
                name,
            }));
    }

    fn list_marker_name(&self, source_index: usize) -> Option<String> {
        let source = &self.sources[source_index];
        if source.tag != "li"
            || resolved_accessibility_visibility(source_index, source, self.sources, self.styles)
                == VisibilityState::Hidden
        {
            return None;
        }
        let (list_index, ordered) = native_list_owner(source_index, self.sources)?;
        if !ordered {
            return Some("• ".into());
        }
        let ordinal = (0..=source_index)
            .filter(|candidate_index| {
                let candidate = &self.sources[*candidate_index];
                candidate.tag == "li"
                    && resolved_accessibility_visibility(
                        *candidate_index,
                        candidate,
                        self.sources,
                        self.styles,
                    ) != VisibilityState::Hidden
                    && native_list_owner(*candidate_index, self.sources)
                        .is_some_and(|(candidate_list, _)| candidate_list == list_index)
            })
            .count();
        Some(format!("{ordinal}. "))
    }
}

fn native_list_owner(source_index: usize, sources: &[ElementSource]) -> Option<(usize, bool)> {
    let mut parent = sources[source_index].parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if matches!(ancestor.tag.as_str(), "menu" | "ol" | "ul") {
            return Some((index, ancestor.tag == "ol"));
        }
        parent = ancestor.parent;
    }
    None
}

fn accessibility_descendant_has_role(source: &ElementSource, sources: &[ElementSource]) -> bool {
    source.content.children.iter().any(|child| {
        let ElementChildSource::Element(index) = child else {
            return false;
        };
        let child = &sources[*index];
        accessibility_tree_role(*index, child, sources).is_some()
            || accessibility_descendant_has_role(child, sources)
    })
}

fn accessibility_tree_role(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> Option<String> {
    semantic_role(source_index, source, sources)
        .or_else(|| (source.tag == "section").then(|| "region".into()))
}

fn accessibility_tree_name(
    source: &ElementSource,
    sources: &[ElementSource],
    role: &str,
) -> String {
    if matches!(
        role,
        "caption"
            | "code"
            | "definition"
            | "deletion"
            | "emphasis"
            | "insertion"
            | "mark"
            | "strong"
            | "subscript"
            | "superscript"
            | "term"
            | "time"
    ) {
        return collapse_whitespace(&source.content.text);
    }
    if matches!(
        role,
        "blockquote"
            | "cell"
            | "generic"
            | "list"
            | "listitem"
            | "main"
            | "paragraph"
            | "presentation"
            | "region"
            | "row"
            | "rowgroup"
            | "table"
    ) {
        return author_name(source, sources).unwrap_or_default();
    }
    accessible_name(source, sources, role)
}

fn accessibility_tree_role_is_leaf(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "checkbox"
            | "code"
            | "combobox"
            | "deletion"
            | "emphasis"
            | "img"
            | "insertion"
            | "link"
            | "mark"
            | "meter"
            | "progressbar"
            | "radio"
            | "searchbox"
            | "separator"
            | "slider"
            | "spinbutton"
            | "strong"
            | "subscript"
            | "superscript"
            | "switch"
            | "textbox"
            | "time"
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SequentialFocusEligibility {
    Excluded,
    Candidate { tabindex: i64 },
    Unsupported { reason: String },
}

fn sequential_focus_source(
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
    locator_elements: &[LocatorElementSource],
    interactive_elements: &[InteractiveElementSource],
) -> SequentialFocusSource {
    let mut candidates = Vec::new();
    let mut radio_groups = BTreeMap::<RadioGroup, Vec<(usize, i64, bool)>>::new();
    for (source_index, source) in sources.iter().enumerate() {
        let visibility = resolved_focus_visibility(source_index, source, sources, styles);
        match sequential_focus_eligibility(source_index, source, sources, visibility) {
            SequentialFocusEligibility::Excluded => {}
            SequentialFocusEligibility::Unsupported { reason } => {
                return SequentialFocusSource::Unsupported { reason };
            }
            SequentialFocusEligibility::Candidate { tabindex } => {
                let Some(interactive_index) = locator_elements[source_index].interactive_index
                else {
                    return SequentialFocusSource::Unsupported {
                        reason: format!(
                            "sequential focus for {} without a supported interactive role is not implemented",
                            source.tag
                        ),
                    };
                };
                if locator_elements[source_index]
                    .evidence
                    .semantic
                    .accessibility
                    .role
                    .as_deref()
                    == Some("radio")
                {
                    if source.attributes.contains_key("tabindex") {
                        return SequentialFocusSource::Unsupported {
                            reason: "explicit radio tabindex order is not implemented".into(),
                        };
                    }
                    let ControlState::Radio(state) =
                        &interactive_elements[interactive_index].control_state
                    else {
                        return SequentialFocusSource::Unsupported {
                            reason: "native radio focus state is unavailable".into(),
                        };
                    };
                    radio_groups.entry(state.group.clone()).or_default().push((
                        interactive_index,
                        tabindex,
                        state.checked(),
                    ));
                    continue;
                }
                candidates.push((interactive_index, tabindex));
            }
        }
    }
    for radios in radio_groups.into_values() {
        let selected = radios
            .iter()
            .find(|(_, _, checked)| *checked)
            .or_else(|| radios.first())
            .expect("radio group candidates are non-empty");
        candidates.push((selected.0, selected.1));
    }
    candidates.sort_by_key(|(source_index, tabindex)| {
        if *tabindex > 0 {
            (0, *tabindex, *source_index)
        } else {
            (1, 0, *source_index)
        }
    });
    SequentialFocusSource::Supported {
        order: candidates
            .into_iter()
            .map(|(interactive_index, _)| interactive_index)
            .collect(),
    }
}

fn sequential_focus_eligibility(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    visibility: VisibilityState,
) -> SequentialFocusEligibility {
    let tabindex = match parsed_tabindex(source) {
        Ok(tabindex) => tabindex,
        Err(reason) => return SequentialFocusEligibility::Unsupported { reason },
    };
    let natural = natural_sequential_focus_target(source);
    if !natural && tabindex.is_none() {
        return SequentialFocusEligibility::Excluded;
    }
    if source_or_ancestor_has_attribute(source_index, sources, "inert") {
        return SequentialFocusEligibility::Excluded;
    }
    if native_control_is_disabled(source) {
        return SequentialFocusEligibility::Excluded;
    }
    if native_form_control(source)
        && ancestor_has_tagged_attribute(source.parent, sources, "fieldset", "disabled")
    {
        return SequentialFocusEligibility::Unsupported {
            reason: "disabled fieldset sequential focus order is not implemented".into(),
        };
    }
    if tabindex.is_some_and(|value| value < 0) {
        return SequentialFocusEligibility::Excluded;
    }
    match visibility {
        VisibilityState::Hidden => SequentialFocusEligibility::Excluded,
        VisibilityState::Unsupported { reason } => {
            SequentialFocusEligibility::Unsupported { reason }
        }
        VisibilityState::Visible => SequentialFocusEligibility::Candidate {
            tabindex: tabindex.unwrap_or(0),
        },
    }
}

fn parsed_tabindex(source: &ElementSource) -> Result<Option<i64>, String> {
    let Some(raw) = source.attributes.get("tabindex") else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty()
        || !raw.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_digit() || (index == 0 && matches!(byte, b'+' | b'-'))
        })
    {
        return Ok(None);
    }
    raw.parse::<i64>()
        .map(Some)
        .map_err(|_| "tabindex is outside the supported integer range".into())
}

fn natural_sequential_focus_target(source: &ElementSource) -> bool {
    match source.tag.as_str() {
        "a" | "area" => source.attributes.contains_key("href"),
        "button" | "select" | "textarea" => true,
        "input" => input_type(source).as_deref() != Some("hidden"),
        "audio" | "video" => source.attributes.contains_key("controls"),
        "iframe" | "object" | "embed" | "summary" => true,
        _ => source
            .attributes
            .get("contenteditable")
            .is_some_and(|value| {
                value.is_empty()
                    || value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("plaintext-only")
            }),
    }
}

fn native_form_control(source: &ElementSource) -> bool {
    matches!(
        source.tag.as_str(),
        "button" | "input" | "select" | "textarea"
    )
}

fn native_control_is_disabled(source: &ElementSource) -> bool {
    native_form_control(source) && source.attributes.contains_key("disabled")
}

fn source_or_ancestor_has_attribute(
    source_index: usize,
    sources: &[ElementSource],
    attribute: &str,
) -> bool {
    let mut current = Some(source_index);
    while let Some(index) = current {
        let source = &sources[index];
        if source.attributes.contains_key(attribute) {
            return true;
        }
        current = source.parent;
    }
    false
}

fn ancestor_has_tagged_attribute(
    mut current: Option<usize>,
    sources: &[ElementSource],
    tag: &str,
    attribute: &str,
) -> bool {
    while let Some(index) = current {
        let source = &sources[index];
        if source.tag == tag && source.attributes.contains_key(attribute) {
            return true;
        }
        current = source.parent;
    }
    false
}

fn element_sources(
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
    viewport_width: u64,
    viewport_height: u64,
) -> (
    Vec<SemanticElementSource>,
    Vec<LocatorElementSource>,
    Vec<InteractiveElementSource>,
    u64,
    u64,
) {
    let mut semantic_elements = Vec::new();
    let mut locator_elements = Vec::with_capacity(sources.len());
    let mut interactive_elements = Vec::new();
    let bounding_boxes = resolve_bounding_boxes_with_styles(sources, styles, viewport_width);
    let (document_width, document_height) = document_extent(
        sources,
        &bounding_boxes,
        styles,
        viewport_width,
        viewport_height,
    );
    for (index, source) in sources.iter().enumerate() {
        let role = source
            .content_ordinal
            .and_then(|_| semantic_role(index, source, sources));
        let base_visibility = match styles {
            Ok(styles) => visibility_state(index, source, sources, styles),
            Err(reason) => VisibilityState::Unsupported {
                reason: reason.into(),
            },
        };
        let role_state = role_state_evidence(
            index,
            source,
            sources,
            role.as_deref(),
            resolved_accessibility_visibility(index, source, sources, styles),
        );
        let visibility = visibility_from_box(base_visibility, &bounding_boxes[index]);
        let bounding_box = match &visibility {
            VisibilityState::Hidden => BoundingBoxEvidence::Hidden,
            VisibilityState::Visible | VisibilityState::Unsupported { .. } => match &bounding_boxes
                [index]
            {
                Ok(value) if value.width > 0 && value.height > 0 => BoundingBoxEvidence::Visible {
                    value: *value,
                    scrolls_with_document: source_scrolls_with_document(index, styles),
                },
                Ok(_) => BoundingBoxEvidence::Hidden,
                Err(reason) => BoundingBoxEvidence::Unsupported(reason.clone()),
            },
        };
        let stability = stability_state(index, sources, styles);
        let interactive_index = role
            .as_deref()
            .is_some_and(is_snapshot_reference_role)
            .then_some(interactive_elements.len());
        let name = role
            .as_deref()
            .map(|role| accessible_name(source, sources, role))
            .unwrap_or_default();
        let description = accessible_description(index, source, sources, role.as_deref());
        let text = locator_text(source);
        locator_elements.push(LocatorElementSource {
            element: source.id.clone(),
            interactive_index,
            parent: source.parent,
            content_ordinal: source.content_ordinal,
            form_owner: native_form_control(source)
                .then(|| form_owner_index(source, sources))
                .flatten(),
            evidence: LocatorEvidence {
                semantic: LocatorSemanticEvidence {
                    accessibility: AccessibilityEvidence {
                        role: role.clone(),
                        name: name.clone(),
                        description,
                        role_state,
                    },
                    text: text.clone(),
                    label: locator_label(source, sources),
                    placeholder: locator_placeholder(source),
                },
                visibility: visibility.clone(),
                stability,
                bounding_box,
                source: LocatorSourceEvidence {
                    tag: source.tag.clone(),
                    attributes: source.attributes.clone(),
                },
            },
        });
        if let Some(role) = role {
            let interactive_index =
                is_snapshot_reference_role(&role).then_some(interactive_elements.len());
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
                let action = interactive_action(source, sources, &role);
                let control_state = control_state(index, source, sources);
                interactive_elements.push(InteractiveElementSource {
                    semantics: semantics.clone(),
                    source_index: index,
                    content_ordinal: source
                        .content_ordinal
                        .expect("interactive elements are retained content"),
                    action,
                    control_state,
                    visibility,
                });
            }
            semantic_elements.push(semantics);
        }
    }
    normalize_radio_groups(&mut interactive_elements);
    (
        semantic_elements,
        locator_elements,
        interactive_elements,
        document_width,
        document_height,
    )
}

fn stability_state(
    source_index: usize,
    sources: &[ElementSource],
    styles: Result<&[BTreeMap<String, String>], &str>,
) -> StabilityState {
    let styles = match styles {
        Ok(styles) => styles,
        Err(reason) => return StabilityState::Unsupported(reason.into()),
    };
    let mut current = Some(source_index);
    while let Some(index) = current {
        let source = &sources[index];
        if let Some(name) = styles[index]
            .keys()
            .find(|name| motion_property(name.as_str()))
        {
            return StabilityState::Unsupported(format!(
                "inline {name} stability is not implemented for {}",
                source.id
            ));
        }
        current = source.parent;
    }
    StabilityState::Stable
}

fn motion_property(name: &str) -> bool {
    name == "animation"
        || name.starts_with("animation-")
        || name == "transition"
        || name.starts_with("transition-")
}

fn source_scrolls_with_document(
    source_index: usize,
    styles: Result<&[BTreeMap<String, String>], &str>,
) -> bool {
    styles
        .ok()
        .and_then(|styles| styles[source_index].get("position"))
        .is_none_or(|position| position != "fixed")
}

fn document_extent(
    sources: &[ElementSource],
    bounding_boxes: &[Result<BoundingBox, String>],
    styles: Result<&[BTreeMap<String, String>], &str>,
    viewport_width: u64,
    viewport_height: u64,
) -> (u64, u64) {
    sources
        .iter()
        .enumerate()
        .zip(bounding_boxes)
        .filter(|((index, _), _)| source_scrolls_with_document(*index, styles))
        .filter_map(|(_, bounding_box)| bounding_box.as_ref().ok())
        .fold(
            (viewport_width, viewport_height),
            |(width, height), bounding_box| {
                (
                    width.max(positive_edge(bounding_box.x, bounding_box.width)),
                    height.max(positive_edge(bounding_box.y, bounding_box.height)),
                )
            },
        )
}

fn positive_edge(origin: i64, size: u64) -> u64 {
    let edge = i128::from(origin) + i128::from(size);
    u64::try_from(edge.max(0)).unwrap_or(u64::MAX)
}

fn subtract_scroll(origin: i64, scroll: u64) -> i64 {
    let coordinate = i128::from(origin) - i128::from(scroll);
    coordinate.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn visibility_from_box(
    visibility: VisibilityState,
    bounding_box: &Result<BoundingBox, String>,
) -> VisibilityState {
    match visibility {
        VisibilityState::Unsupported { reason } if reason == UNKNOWN_BOX_GEOMETRY => {
            match bounding_box {
                Ok(value) if value.width > 0 && value.height > 0 => VisibilityState::Visible,
                Ok(_) => VisibilityState::Hidden,
                Err(reason) => VisibilityState::Unsupported {
                    reason: reason.clone(),
                },
            }
        }
        visibility => visibility,
    }
}

fn normalize_radio_groups(elements: &mut [InteractiveElementSource]) {
    for index in 0..elements.len() {
        let group = match &elements[index].control_state {
            ControlState::Radio(state) if state.checked() => state.group.clone(),
            ControlState::Text(_)
            | ControlState::Checkbox(_)
            | ControlState::Radio(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => continue,
        };
        for previous in &mut elements[..index] {
            if let ControlState::Radio(state) = &mut previous.control_state
                && state.group == group
            {
                state.set_checked(false);
            }
        }
    }
}

fn locator_text(source: &ElementSource) -> String {
    if source.tag == "input" && matches!(input_type(source).as_deref(), Some("button" | "submit")) {
        return attribute_name(source, "value");
    }
    collapse_whitespace(&source.content.text)
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
    if source.tag == "input" && input_type(source).as_deref() == Some("radio") {
        return ControlState::Radio(radio_state(source_index, source, sources));
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
                .unwrap_or_else(|| collapse_whitespace(&option.content.text)),
            label: option
                .attributes
                .get("label")
                .cloned()
                .unwrap_or_else(|| collapse_whitespace(&option.content.text)),
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

fn radio_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> RadioState {
    let checked = source.attributes.contains_key("checked");
    let checked = if source.attributes.contains_key("disabled") {
        CheckedState::NonEditable {
            checked,
            reason: "disabled radios cannot change state".into(),
        }
    } else {
        CheckedState::Editable { checked }
    };
    let group = source
        .attributes
        .get("name")
        .filter(|name| !name.is_empty())
        .map_or(RadioGroup::Singleton { source_index }, |name| {
            RadioGroup::Named {
                name: name.clone(),
                form_owner: form_owner_index(source, sources),
            }
        });
    RadioState { checked, group }
}

fn text_value_state(source: &ElementSource) -> TextValueState {
    let initial_value = match source.tag.as_str() {
        "textarea" => Some(source.content.text.clone()),
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
    let selection = TextSelectionState::collapsed_at(0);
    if source.attributes.contains_key("disabled") {
        return TextValueState::NonEditable {
            value,
            selection,
            reason: "disabled controls cannot be filled".into(),
        };
    }
    if source.attributes.contains_key("readonly") {
        return TextValueState::NonEditable {
            value,
            selection,
            reason: "read-only controls cannot be filled".into(),
        };
    }
    TextValueState::Editable { value, selection }
}

fn interactive_action(
    source: &ElementSource,
    sources: &[ElementSource],
    role: &str,
) -> InteractiveAction {
    if let Some(href) = source.attributes.get("href").filter(|_| source.tag == "a") {
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
        return InteractiveAction::Navigate { href: href.clone() };
    }
    if source.tag == "input" && input_type(source).as_deref() == Some("checkbox") {
        return InteractiveAction::ToggleCheckbox;
    }
    if source.tag == "input" && input_type(source).as_deref() == Some("radio") {
        return InteractiveAction::SelectRadio;
    }
    if native_button_activation(source) {
        if let Some(form_owner) = form_owner_index(source, sources) {
            return match native_button_kind(source) {
                NativeButtonKind::Submit => InteractiveAction::SubmitForm { form_owner },
                NativeButtonKind::Reset => InteractiveAction::Unsupported {
                    reason: "form reset is not implemented".into(),
                },
                NativeButtonKind::Image => InteractiveAction::Unsupported {
                    reason: "image submit coordinates are not implemented".into(),
                },
                NativeButtonKind::Activate => InteractiveAction::Activate,
            };
        }
        return InteractiveAction::Activate;
    }
    InteractiveAction::Unsupported {
        reason: format!("click execution for role {role} is not implemented"),
    }
}

fn native_button_activation(source: &ElementSource) -> bool {
    source.tag == "button"
        || (source.tag == "input"
            && matches!(
                input_type(source).as_deref(),
                Some("button" | "image" | "reset" | "submit")
            ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeButtonKind {
    Activate,
    Submit,
    Reset,
    Image,
}

fn native_button_kind(source: &ElementSource) -> NativeButtonKind {
    if source.tag == "button" {
        return match input_type(source).as_deref() {
            Some("button") => NativeButtonKind::Activate,
            Some("reset") => NativeButtonKind::Reset,
            _ => NativeButtonKind::Submit,
        };
    }
    match input_type(source).as_deref() {
        Some("image") => NativeButtonKind::Image,
        Some("reset") => NativeButtonKind::Reset,
        Some("submit") => NativeButtonKind::Submit,
        _ => NativeButtonKind::Activate,
    }
}

fn form_owner_index(source: &ElementSource, sources: &[ElementSource]) -> Option<usize> {
    if let Some(form_id) = source.attributes.get("form") {
        return sources.iter().position(|candidate| {
            candidate.tag == "form"
                && candidate
                    .attributes
                    .get("id")
                    .is_some_and(|id| id == form_id)
        });
    }
    ancestor_form_index(source.parent, sources)
}

fn ancestor_form_index(mut parent: Option<usize>, sources: &[ElementSource]) -> Option<usize> {
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "form" {
            return Some(index);
        }
        parent = ancestor.parent;
    }
    None
}

fn role_state_evidence(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    role: Option<&str>,
    visibility: VisibilityState,
) -> RoleStateEvidence {
    let checked = role.and_then(|role| role_checked_state(source, role));
    let selected = role.and_then(|role| role_selected_state(source, role));
    RoleStateEvidence {
        values: RoleStateValues {
            checked,
            disabled: role
                .is_some_and(|role| role_disabled_state(source_index, source, sources, role)),
            expanded: role.and_then(|role| role_expanded_state(source, role)),
            pressed: role.and_then(|role| role_pressed_state(source, role)),
            selected,
        },
        level: role.and_then(|role| role_level(source, role)),
        visibility,
    }
}

fn role_checked_state(source: &ElementSource, role: &str) -> Option<bool> {
    if source.tag == "input" && matches!(input_type(source).as_deref(), Some("checkbox" | "radio"))
    {
        return Some(source.attributes.contains_key("checked"));
    }
    if !matches!(
        role,
        "checkbox"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "option"
            | "radio"
            | "switch"
            | "treeitem"
    ) {
        return None;
    }
    match source.attributes.get("aria-checked").map(String::as_str) {
        Some("true") => Some(true),
        Some("mixed") => None,
        _ => Some(false),
    }
}

fn role_selected_state(source: &ElementSource, role: &str) -> Option<bool> {
    if source.tag == "option" {
        return Some(source.attributes.contains_key("selected"));
    }
    matches!(
        role,
        "columnheader" | "gridcell" | "option" | "row" | "rowheader" | "tab" | "treeitem"
    )
    .then(|| aria_true_attribute(source, "aria-selected"))
}

fn role_disabled_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    role: &str,
) -> bool {
    if native_disabled_state(source_index, source, sources) {
        return true;
    }
    if !role_supports_disabled(role) {
        return false;
    }
    let mut current = Some(source_index);
    while let Some(index) = current {
        let candidate = &sources[index];
        if let Some(value) = candidate.attributes.get("aria-disabled") {
            return value.trim().eq_ignore_ascii_case("true");
        }
        current = candidate.parent;
    }
    false
}

fn native_disabled_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> bool {
    if !matches!(
        source.tag.as_str(),
        "button" | "input" | "optgroup" | "option" | "select" | "textarea"
    ) {
        return false;
    }
    if source.attributes.contains_key("disabled") {
        return true;
    }
    if source.tag == "option"
        && ancestor_has_tagged_attribute(source.parent, sources, "optgroup", "disabled")
    {
        return true;
    }
    let mut parent = source.parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "fieldset" && ancestor.attributes.contains_key("disabled") {
            let first_legend = sources
                .iter()
                .position(|candidate| candidate.parent == Some(index) && candidate.tag == "legend");
            return !first_legend
                .is_some_and(|legend| source_is_descendant_of(source_index, legend, sources));
        }
        parent = ancestor.parent;
    }
    false
}

fn source_is_descendant_of(
    source_index: usize,
    ancestor_index: usize,
    sources: &[ElementSource],
) -> bool {
    let mut parent = sources[source_index].parent;
    while let Some(index) = parent {
        if index == ancestor_index {
            return true;
        }
        parent = sources[index].parent;
    }
    false
}

fn role_supports_disabled(role: &str) -> bool {
    matches!(
        role,
        "application"
            | "button"
            | "checkbox"
            | "columnheader"
            | "combobox"
            | "grid"
            | "gridcell"
            | "group"
            | "link"
            | "listbox"
            | "menu"
            | "menubar"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "option"
            | "radio"
            | "radiogroup"
            | "row"
            | "rowheader"
            | "scrollbar"
            | "searchbox"
            | "separator"
            | "slider"
            | "spinbutton"
            | "switch"
            | "tab"
            | "tablist"
            | "textbox"
            | "toolbar"
            | "tree"
            | "treegrid"
            | "treeitem"
    )
}

fn role_level(source: &ElementSource, role: &str) -> Option<u32> {
    if role == "heading"
        && let Some(level) = source
            .tag
            .strip_prefix('h')
            .and_then(|level| level.parse::<u32>().ok())
            .filter(|level| (1..=6).contains(level))
    {
        return Some(level);
    }
    source
        .attributes
        .get("aria-level")
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|level| *level > 0)
}

fn role_expanded_state(source: &ElementSource, role: &str) -> Option<bool> {
    if !matches!(
        role,
        "application"
            | "button"
            | "checkbox"
            | "columnheader"
            | "combobox"
            | "gridcell"
            | "link"
            | "listbox"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "row"
            | "rowheader"
            | "switch"
            | "tab"
            | "treeitem"
    ) {
        return None;
    }
    source
        .attributes
        .get("aria-expanded")
        .map(|value| value.as_str() == "true")
}

fn role_pressed_state(source: &ElementSource, role: &str) -> Option<bool> {
    if role != "button" {
        return None;
    }
    match source.attributes.get("aria-pressed").map(String::as_str) {
        Some("true") => Some(true),
        Some("mixed") => None,
        _ => Some(false),
    }
}

fn aria_true_attribute(source: &ElementSource, name: &str) -> bool {
    source
        .attributes
        .get(name)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

fn semantic_role(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> Option<String> {
    let explicit = explicit_semantic_role(source);
    if explicit
        .as_deref()
        .is_some_and(|role| !matches!(role, "none" | "presentation"))
    {
        return explicit;
    }
    let implicit = native_interactive_role(source, sources)
        .or_else(|| native_structural_role(source_index, source, sources));
    match explicit {
        Some(role)
            if !has_presentation_conflict(source_index, source, sources, implicit.as_deref()) =>
        {
            Some(role)
        }
        Some(_) | None => implicit,
    }
}

fn explicit_semantic_role(source: &ElementSource) -> Option<String> {
    source.attributes.get("role").and_then(|roles| {
        roles
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .find(|role| is_supported_role(role))
    })
}

fn native_interactive_role(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    match source.tag.as_str() {
        "a" | "area" if source.attributes.contains_key("href") => Some("link".into()),
        "button" => Some("button".into()),
        "select"
            if source.attributes.contains_key("multiple") || select_display_size(source) > 1 =>
        {
            Some("listbox".into())
        }
        "select" => Some("combobox".into()),
        "textarea" => Some("textbox".into()),
        "input" => input_role(source, sources),
        _ => None,
    }
}

fn native_structural_role(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> Option<String> {
    if let Some(role) = simple_native_structural_role(&source.tag) {
        return Some(role.into());
    }
    let role = match source.tag.as_str() {
        "header" if !has_sectioning_ancestor(source_index, sources) => "banner",
        "footer" if !has_sectioning_ancestor(source_index, sources) => "contentinfo",
        "section" if author_name(source, sources).is_some() => "region",
        "form" if author_name(source, sources).is_some() => "form",
        "td" if table_has_grid_role(source_index, sources) => "gridcell",
        "td" => "cell",
        "th" if source.attributes.get("scope").is_some_and(|scope| {
            scope.eq_ignore_ascii_case("row") || scope.eq_ignore_ascii_case("rowgroup")
        }) =>
        {
            "rowheader"
        }
        "th" => "columnheader",
        "img" if image_is_presentational(source) => "presentation",
        "img" => "img",
        _ => return None,
    };
    Some(role.into())
}

const SIMPLE_NATIVE_STRUCTURAL_ROLES: &[(&str, &str)] = &[
    ("article", "article"),
    ("aside", "complementary"),
    ("blockquote", "blockquote"),
    ("caption", "caption"),
    ("code", "code"),
    ("datalist", "listbox"),
    ("dd", "definition"),
    ("del", "deletion"),
    ("details", "group"),
    ("dfn", "term"),
    ("dialog", "dialog"),
    ("dt", "term"),
    ("em", "emphasis"),
    ("fieldset", "group"),
    ("figure", "figure"),
    ("h1", "heading"),
    ("h2", "heading"),
    ("h3", "heading"),
    ("h4", "heading"),
    ("h5", "heading"),
    ("h6", "heading"),
    ("hr", "separator"),
    ("html", "document"),
    ("ins", "insertion"),
    ("li", "listitem"),
    ("main", "main"),
    ("mark", "mark"),
    ("math", "math"),
    ("menu", "list"),
    ("meter", "meter"),
    ("nav", "navigation"),
    ("ol", "list"),
    ("optgroup", "group"),
    ("output", "status"),
    ("p", "paragraph"),
    ("progress", "progressbar"),
    ("search", "search"),
    ("strong", "strong"),
    ("sub", "subscript"),
    ("sup", "superscript"),
    ("svg", "img"),
    ("table", "table"),
    ("tbody", "rowgroup"),
    ("tfoot", "rowgroup"),
    ("thead", "rowgroup"),
    ("time", "time"),
    ("tr", "row"),
    ("ul", "list"),
];

fn simple_native_structural_role(tag: &str) -> Option<&'static str> {
    SIMPLE_NATIVE_STRUCTURAL_ROLES
        .binary_search_by(|(candidate, _)| candidate.cmp(&tag))
        .ok()
        .map(|index| SIMPLE_NATIVE_STRUCTURAL_ROLES[index].1)
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

fn input_role(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    let input_type = input_type(source);
    match input_type.as_deref() {
        Some("hidden") => None,
        Some("checkbox") => Some("checkbox".into()),
        Some("radio") => Some("radio".into()),
        Some("range") => Some("slider".into()),
        Some("number") => Some("spinbutton".into()),
        Some("search") if !input_has_datalist(source, sources) => Some("searchbox".into()),
        Some("button" | "file" | "image" | "reset" | "submit") => Some("button".into()),
        Some("" | "email" | "search" | "tel" | "text" | "url")
            if input_has_datalist(source, sources) =>
        {
            Some("combobox".into())
        }
        None if input_has_datalist(source, sources) => Some("combobox".into()),
        _ => Some("textbox".into()),
    }
}

fn input_has_datalist(source: &ElementSource, sources: &[ElementSource]) -> bool {
    let Some(list) = source.attributes.get("list") else {
        return false;
    };
    sources.iter().any(|candidate| {
        candidate.tag == "datalist" && candidate.attributes.get("id").is_some_and(|id| id == list)
    })
}

fn image_is_presentational(source: &ElementSource) -> bool {
    source
        .attributes
        .get("alt")
        .is_some_and(|alt| alt.is_empty())
        && non_empty_attribute(source, "title").is_none()
        && !source.attributes.contains_key("tabindex")
        && !has_global_aria_attribute(source, None)
}

const GLOBAL_ARIA_ATTRIBUTES: &[&str] = &[
    "aria-atomic",
    "aria-busy",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-details",
    "aria-dropeffect",
    "aria-flowto",
    "aria-grabbed",
    "aria-hidden",
    "aria-keyshortcuts",
    "aria-live",
    "aria-owns",
    "aria-relevant",
];

const ARIA_LABEL_PROHIBITED_ROLES: &[&str] = &[
    "caption",
    "code",
    "deletion",
    "emphasis",
    "generic",
    "insertion",
    "paragraph",
    "presentation",
    "strong",
    "subscript",
    "superscript",
];

fn has_global_aria_attribute(source: &ElementSource, role: Option<&str>) -> bool {
    GLOBAL_ARIA_ATTRIBUTES
        .iter()
        .any(|name| source.attributes.contains_key(*name))
        || (!role.is_some_and(|role| ARIA_LABEL_PROHIBITED_ROLES.contains(&role))
            && (source.attributes.contains_key("aria-label")
                || source.attributes.contains_key("aria-labelledby")))
        || (role != Some("generic") && source.attributes.contains_key("aria-roledescription"))
}

fn has_presentation_conflict(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    implicit_role: Option<&str>,
) -> bool {
    has_global_aria_attribute(source, implicit_role)
        || element_is_focusable(source_index, source, sources)
}

fn element_is_focusable(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
) -> bool {
    if native_disabled_state(source_index, source, sources) {
        return false;
    }
    match source.tag.as_str() {
        "button" | "details" | "select" | "textarea" => true,
        "a" | "area" => source.attributes.contains_key("href"),
        "input" => input_type(source).as_deref() != Some("hidden"),
        _ => source
            .attributes
            .get("tabindex")
            .is_some_and(|value| value.trim().is_empty() || value.trim().parse::<f64>().is_ok()),
    }
}

fn table_has_grid_role(source_index: usize, sources: &[ElementSource]) -> bool {
    let mut parent = sources[source_index].parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "table" {
            return explicit_semantic_role(ancestor)
                .is_some_and(|role| matches!(role.as_str(), "grid" | "treegrid"));
        }
        parent = ancestor.parent;
    }
    false
}

fn is_snapshot_reference_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "checkbox"
            | "combobox"
            | "heading"
            | "link"
            | "listbox"
            | "menuitem"
            | "navigation"
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
    matches!(
        role,
        "alert"
            | "alertdialog"
            | "application"
            | "article"
            | "banner"
            | "blockquote"
            | "button"
            | "caption"
            | "cell"
            | "checkbox"
            | "code"
            | "columnheader"
            | "combobox"
            | "complementary"
            | "contentinfo"
            | "definition"
            | "deletion"
            | "dialog"
            | "directory"
            | "document"
            | "emphasis"
            | "feed"
            | "figure"
            | "form"
            | "generic"
            | "grid"
            | "gridcell"
            | "group"
            | "heading"
            | "img"
            | "insertion"
            | "link"
            | "list"
            | "listbox"
            | "listitem"
            | "log"
            | "main"
            | "mark"
            | "marquee"
            | "math"
            | "menu"
            | "menubar"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "meter"
            | "navigation"
            | "none"
            | "note"
            | "option"
            | "paragraph"
            | "presentation"
            | "progressbar"
            | "radio"
            | "radiogroup"
            | "region"
            | "row"
            | "rowgroup"
            | "rowheader"
            | "scrollbar"
            | "search"
            | "searchbox"
            | "separator"
            | "slider"
            | "spinbutton"
            | "status"
            | "strong"
            | "subscript"
            | "superscript"
            | "switch"
            | "tab"
            | "table"
            | "tablist"
            | "tabpanel"
            | "term"
            | "textbox"
            | "time"
            | "timer"
            | "toolbar"
            | "tooltip"
            | "tree"
            | "treegrid"
            | "treeitem"
    )
}

fn accessible_name(source: &ElementSource, sources: &[ElementSource], role: &str) -> String {
    if role_prohibits_naming(role) {
        return String::new();
    }
    if let Some(name) = author_name(source, sources) {
        return name;
    }

    if source.tag == "input" {
        return input_accessible_name(source);
    }
    if source.tag == "area" {
        let alt = attribute_name(source, "alt");
        return if alt.is_empty() {
            title_name(source)
        } else {
            alt
        };
    }
    if matches!(source.tag.as_str(), "select" | "textarea") {
        return title_name(source);
    }

    if source.tag == "img" {
        let alt = attribute_name(source, "alt");
        return if alt.is_empty() {
            title_name(source)
        } else {
            alt
        };
    }
    if name_from_content(role) {
        let text = accessible_name_from_content(source, sources);
        if !text.is_empty() {
            return text;
        }
    }
    title_name(source)
}

fn accessible_name_from_content(source: &ElementSource, sources: &[ElementSource]) -> String {
    let mut name = String::new();
    append_accessible_name_content(source, sources, &mut name);
    collapse_whitespace(&name)
}

fn append_accessible_name_content(
    source: &ElementSource,
    sources: &[ElementSource],
    name: &mut String,
) {
    for child in &source.content.children {
        match child {
            ElementChildSource::Text(text) => name.push_str(text),
            ElementChildSource::Element(index) => {
                let child = &sources[*index];
                if child.tag == "img"
                    && semantic_role(*index, child, sources).as_deref() == Some("img")
                {
                    let alt = attribute_name(child, "alt");
                    if !alt.is_empty() {
                        name.push(' ');
                        name.push_str(&alt);
                        name.push(' ');
                    }
                } else {
                    append_accessible_name_content(child, sources, name);
                }
            }
        }
    }
}

fn accessible_description(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    role: Option<&str>,
) -> String {
    if let Some(references) = source.attributes.get("aria-describedby") {
        return references
            .split_ascii_whitespace()
            .filter_map(|reference| {
                sources.iter().enumerate().find(|(_, candidate)| {
                    candidate
                        .attributes
                        .get("id")
                        .is_some_and(|id| id == reference)
                })
            })
            .map(|(_, candidate)| description_reference_name(candidate, sources))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    if let Some(description) = source.attributes.get("aria-description") {
        return collapse_whitespace(description);
    }
    if role.is_some_and(|role| accessible_name_uses_title(source_index, source, sources, role)) {
        return String::new();
    }
    title_name(source)
}

fn description_reference_name(source: &ElementSource, sources: &[ElementSource]) -> String {
    if let Some(name) = author_name(source, sources) {
        return name;
    }
    if source.tag == "img"
        || (source.tag == "input" && input_type(source).as_deref() == Some("image"))
    {
        return attribute_name(source, "alt");
    }
    let content = collapse_whitespace(&source.content.text);
    if !content.is_empty() {
        return content;
    }
    title_name(source)
}

fn accessible_name_uses_title(
    _source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    role: &str,
) -> bool {
    if role_prohibits_naming(role) {
        return false;
    }
    if author_name(source, sources).is_some() {
        return false;
    }
    if source.tag == "input" {
        return match input_type(source).as_deref() {
            Some("image" | "reset" | "submit") => false,
            Some("button") => attribute_name(source, "value").is_empty(),
            _ => true,
        };
    }
    if matches!(source.tag.as_str(), "select" | "textarea") {
        return true;
    }
    if source.tag == "img" {
        return false;
    }
    !name_from_content(role) || collapse_whitespace(&source.content.text).is_empty()
}

fn role_prohibits_naming(role: &str) -> bool {
    matches!(
        role,
        "caption"
            | "code"
            | "definition"
            | "deletion"
            | "emphasis"
            | "generic"
            | "insertion"
            | "mark"
            | "paragraph"
            | "presentation"
            | "strong"
            | "subscript"
            | "suggestion"
            | "superscript"
            | "term"
            | "time"
    )
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
        .map(|candidate| collapse_whitespace(&candidate.content.text))
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
        .map(|label| collapse_whitespace(&label.content.text))
}

fn ancestor_label_name(source: &ElementSource, sources: &[ElementSource]) -> Option<String> {
    let mut parent = source.parent;
    while let Some(index) = parent {
        let ancestor = &sources[index];
        if ancestor.tag == "label" {
            return Some(collapse_whitespace(&ancestor.content.text));
        }
        parent = ancestor.parent;
    }
    None
}

fn input_accessible_name(source: &ElementSource) -> String {
    let input_type = input_type(source);
    if input_type.as_deref() == Some("image") {
        let alt = attribute_name(source, "alt");
        return if alt.is_empty() {
            title_name(source)
        } else {
            alt
        };
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
