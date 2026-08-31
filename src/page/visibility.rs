use std::collections::BTreeMap;

use super::{ElementSource, collapse_whitespace, parse_style};

pub(super) const UNKNOWN_BOX_GEOMETRY: &str =
    "inline geometry prevents a proven non-empty bounding box";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VisibilityState {
    Visible,
    Hidden,
    Unsupported { reason: String },
}

pub(super) fn visibility_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: &[BTreeMap<String, String>],
) -> VisibilityState {
    match inherited_visibility_state(source_index, source, sources, styles, true) {
        VisibilityState::Visible => default_box_visibility(source),
        result => result,
    }
}

pub(super) enum FocusStyleEvidence<'a> {
    Computed(&'a [BTreeMap<String, String>]),
    InlineOnly,
    UnsupportedStylesheet,
}

impl<'a> From<&'a [BTreeMap<String, String>]> for FocusStyleEvidence<'a> {
    fn from(styles: &'a [BTreeMap<String, String>]) -> Self {
        Self::Computed(styles)
    }
}

impl From<bool> for FocusStyleEvidence<'_> {
    fn from(has_stylesheet: bool) -> Self {
        if has_stylesheet {
            Self::UnsupportedStylesheet
        } else {
            Self::InlineOnly
        }
    }
}

pub(super) fn focus_visibility_state<'a>(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: impl Into<FocusStyleEvidence<'a>>,
) -> VisibilityState {
    match styles.into() {
        FocusStyleEvidence::Computed(styles) => {
            inherited_visibility_state(source_index, source, sources, styles, false)
        }
        FocusStyleEvidence::InlineOnly => {
            let styles = sources
                .iter()
                .map(|source| {
                    parse_style(
                        source
                            .attributes
                            .get("style")
                            .map(String::as_str)
                            .unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>();
            inherited_visibility_state(source_index, source, sources, &styles, false)
        }
        FocusStyleEvidence::UnsupportedStylesheet => VisibilityState::Unsupported {
            reason: "linked and embedded stylesheet visibility is not implemented".into(),
        },
    }
}

pub(super) fn accessibility_visibility_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: &[BTreeMap<String, String>],
) -> VisibilityState {
    let mut current = Some(source_index);
    while let Some(index) = current {
        let candidate = &sources[index];
        if candidate
            .attributes
            .get("aria-hidden")
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
        {
            return VisibilityState::Hidden;
        }
        current = candidate.parent;
    }
    focus_visibility_state(source_index, source, sources, styles)
}

fn inherited_visibility_state(
    source_index: usize,
    source: &ElementSource,
    sources: &[ElementSource],
    styles: &[BTreeMap<String, String>],
    inspect_geometry: bool,
) -> VisibilityState {
    let mut ancestry = vec![source_index];
    let mut parent = source.parent;
    while let Some(index) = parent {
        ancestry.push(index);
        parent = sources[index].parent;
    }
    ancestry.reverse();

    let mut evidence = VisibilityEvidence::default();
    for index in ancestry {
        if inspect_visibility_candidate(
            &sources[index],
            &styles[index],
            &mut evidence,
            inspect_geometry,
        ) {
            return VisibilityState::Hidden;
        }
    }

    if evidence.inherited_hidden {
        return VisibilityState::Hidden;
    }
    if let Some(reason) = evidence.unsupported {
        return VisibilityState::Unsupported { reason };
    }
    VisibilityState::Visible
}

#[derive(Default)]
struct VisibilityEvidence {
    inherited_hidden: bool,
    unsupported: Option<String>,
}

fn inspect_visibility_candidate(
    candidate: &ElementSource,
    properties: &BTreeMap<String, String>,
    evidence: &mut VisibilityEvidence,
    inspect_geometry: bool,
) -> bool {
    if candidate_is_definitely_hidden(candidate, properties) {
        return true;
    }
    if evidence.unsupported.is_none() {
        evidence.unsupported =
            unsupported_visibility_reason(candidate, properties, inspect_geometry);
    }
    apply_inherited_visibility(properties, evidence);
    false
}

fn candidate_is_definitely_hidden(
    candidate: &ElementSource,
    properties: &BTreeMap<String, String>,
) -> bool {
    let display = properties.get("display").map(String::as_str);
    display == Some("none")
        || properties
            .get("content-visibility")
            .is_some_and(|value| value == "hidden")
        || candidate
            .attributes
            .get("hidden")
            .is_some_and(|value| !value.eq_ignore_ascii_case("until-found") && display.is_none())
}

fn unsupported_visibility_reason(
    candidate: &ElementSource,
    properties: &BTreeMap<String, String>,
    inspect_geometry: bool,
) -> Option<String> {
    hidden_until_found_reason(candidate)
        .or_else(|| content_visibility_reason(properties))
        .or_else(|| display_reason(properties))
        .or_else(|| {
            inspect_geometry
                .then(|| geometry_reason(properties))
                .flatten()
        })
}

fn hidden_until_found_reason(candidate: &ElementSource) -> Option<String> {
    candidate
        .attributes
        .get("hidden")
        .is_some_and(|value| value.eq_ignore_ascii_case("until-found"))
        .then(|| "hidden-until-found visibility is not implemented".to_owned())
}

fn content_visibility_reason(properties: &BTreeMap<String, String>) -> Option<String> {
    let value = properties.get("content-visibility")?;
    (value != "visible" && value != "hidden")
        .then(|| format!("content-visibility:{value} is not implemented"))
}

fn display_reason(properties: &BTreeMap<String, String>) -> Option<String> {
    let value = properties.get("display")?;
    if value == "contents" {
        return Some("display:contents visibility is not implemented".to_owned());
    }
    (!supported_display_value(value) && value != "none")
        .then(|| format!("display:{value} is not implemented"))
}

fn geometry_reason(properties: &BTreeMap<String, String>) -> Option<String> {
    properties
        .keys()
        .any(|name| box_geometry_property(name))
        .then(|| UNKNOWN_BOX_GEOMETRY.to_owned())
}

fn apply_inherited_visibility(
    properties: &BTreeMap<String, String>,
    evidence: &mut VisibilityEvidence,
) {
    let Some(value) = properties.get("visibility") else {
        return;
    };
    match value.as_str() {
        "visible" => evidence.inherited_hidden = false,
        "hidden" | "collapse" => evidence.inherited_hidden = true,
        _ if evidence.unsupported.is_none() => {
            evidence.unsupported = Some(format!("visibility:{value} is not implemented"));
        }
        _ => {}
    }
}

fn default_box_visibility(source: &ElementSource) -> VisibilityState {
    if matches!(
        source.tag.as_str(),
        "button" | "input" | "select" | "textarea"
    ) || !collapse_whitespace(&source.content.text).is_empty()
    {
        return VisibilityState::Visible;
    }
    if matches!(
        source.tag.as_str(),
        "audio" | "canvas" | "embed" | "iframe" | "img" | "object" | "svg" | "video"
    ) {
        return VisibilityState::Unsupported {
            reason: format!("intrinsic {} geometry is not implemented", source.tag),
        };
    }
    VisibilityState::Hidden
}

fn supported_display_value(value: &str) -> bool {
    matches!(
        value,
        "block"
            | "inline"
            | "inline-block"
            | "flex"
            | "inline-flex"
            | "grid"
            | "inline-grid"
            | "flow-root"
            | "list-item"
            | "table"
            | "inline-table"
            | "table-row-group"
            | "table-header-group"
            | "table-footer-group"
            | "table-row"
            | "table-cell"
            | "table-column-group"
            | "table-column"
            | "table-caption"
    )
}

fn box_geometry_property(name: &str) -> bool {
    matches!(
        name,
        "width"
            | "height"
            | "min-width"
            | "max-width"
            | "min-height"
            | "max-height"
            | "padding"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "padding-bottom"
            | "border"
            | "border-width"
            | "border-left"
            | "border-right"
            | "border-top"
            | "border-bottom"
            | "font"
            | "font-size"
            | "line-height"
            | "transform"
            | "scale"
            | "zoom"
    )
}
