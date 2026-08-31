use std::collections::BTreeMap;

use super::{ElementSource, collapse_whitespace};

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
    let mut ancestry = vec![source_index];
    let mut parent = source.parent;
    while let Some(index) = parent {
        ancestry.push(index);
        parent = sources[index].parent;
    }
    ancestry.reverse();

    let mut evidence = VisibilityEvidence::default();
    for index in ancestry {
        if inspect_visibility_candidate(&sources[index], &styles[index], &mut evidence) {
            return VisibilityState::Hidden;
        }
    }

    if evidence.inherited_hidden {
        return VisibilityState::Hidden;
    }
    if let Some(reason) = evidence.unsupported {
        return VisibilityState::Unsupported { reason };
    }
    default_box_visibility(source)
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
) -> bool {
    if candidate_is_definitely_hidden(candidate, properties) {
        return true;
    }
    if evidence.unsupported.is_none() {
        evidence.unsupported = unsupported_visibility_reason(candidate, properties);
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
) -> Option<String> {
    hidden_until_found_reason(candidate)
        .or_else(|| content_visibility_reason(properties))
        .or_else(|| display_reason(properties))
        .or_else(|| geometry_reason(properties))
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
        .then(|| "inline geometry prevents a proven non-empty bounding box".to_owned())
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
    ) || !collapse_whitespace(&source.text).is_empty()
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
