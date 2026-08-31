use std::collections::BTreeMap;

use super::visibility::{VisibilityState, focus_visibility_state};
use super::{
    ElementSource, border_width, collapse_whitespace, parse_page_source, parse_style,
    resolve_bounding_boxes,
};
use crate::{BoundingBox, CaptureRect, PaintCommand, Rgba8};

pub(crate) fn paint_commands_from_html(
    html: &str,
    viewport_width: u64,
    capture_bounds: CaptureRect,
    fixed_offset_x: u64,
    fixed_offset_y: u64,
) -> Result<Vec<PaintCommand>, String> {
    let source = parse_page_source(html);
    if source.has_stylesheet {
        return Err("linked and embedded stylesheet paint is not implemented".into());
    }
    let bounding_boxes = resolve_bounding_boxes(&source.elements, false, viewport_width);
    let mut commands = vec![PaintCommand::FillRect {
        source: "page-canvas".into(),
        bounds: capture_bounds,
        color: WHITE,
    }];

    for (index, element) in source.elements.iter().enumerate() {
        let properties = parse_style(
            element
                .attributes
                .get("style")
                .map(String::as_str)
                .unwrap_or_default(),
        );
        let has_potential_paint = has_potential_paint(element, &properties);
        let bounding_box = match &bounding_boxes[index] {
            Ok(value) if value.width > 0 && value.height > 0 => *value,
            Ok(_) if !has_potential_paint => continue,
            Ok(_) => return Err(format!("{} has an empty paint box", element.id)),
            Err(_) if !has_potential_paint => continue,
            Err(reason) => {
                return Err(format!(
                    "paint geometry for {} is not implemented: {reason}",
                    element.id
                ));
            }
        };
        let scrolls_with_document = properties.get("position").map(String::as_str) != Some("fixed");
        let painted_box = translate_box(
            bounding_box,
            scrolls_with_document,
            fixed_offset_x,
            fixed_offset_y,
            &element.id,
        )?;
        if !intersects(painted_box, capture_bounds) {
            continue;
        }
        match focus_visibility_state(index, element, &source.elements, false) {
            VisibilityState::Visible => {}
            VisibilityState::Hidden => continue,
            VisibilityState::Unsupported { reason } => {
                return Err(format!(
                    "paint visibility for {} is not implemented: {reason}",
                    element.id
                ));
            }
        }
        let contribution = paint_contribution(element, &properties)?;
        let Some(contribution) = contribution else {
            continue;
        };
        contribution.append_commands(element, painted_box, capture_bounds, &mut commands)?;
    }

    Ok(commands)
}

fn has_potential_paint(element: &ElementSource, properties: &BTreeMap<String, String>) -> bool {
    !collapse_whitespace(&element.content.direct_text).is_empty()
        || matches!(
            element.tag.as_str(),
            "button"
                | "input"
                | "select"
                | "textarea"
                | "audio"
                | "canvas"
                | "embed"
                | "iframe"
                | "img"
                | "object"
                | "svg"
                | "video"
        )
        || properties.keys().any(|name| {
            name.starts_with("background")
                || name.starts_with("border")
                || matches!(
                    name.as_str(),
                    "box-shadow"
                        | "clip"
                        | "clip-path"
                        | "filter"
                        | "isolation"
                        | "mask"
                        | "mix-blend-mode"
                        | "opacity"
                        | "outline"
                        | "overflow"
                        | "text-shadow"
                        | "z-index"
                )
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PaintContribution {
    background: Option<Rgba8>,
    borders: BorderPaint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct BorderPaint {
    top: u64,
    right: u64,
    bottom: u64,
    left: u64,
    color: Option<Rgba8>,
}

impl PaintContribution {
    fn append_commands(
        self,
        element: &ElementSource,
        bounds: CaptureRect,
        capture_bounds: CaptureRect,
        commands: &mut Vec<PaintCommand>,
    ) -> Result<(), String> {
        if let Some(color) = self.background {
            commands.push(PaintCommand::FillRect {
                source: format!("{}#background", element.id),
                bounds: if element.tag == "body" {
                    capture_bounds
                } else {
                    bounds
                },
                color,
            });
        }
        let Some(color) = self.borders.color else {
            return Ok(());
        };
        append_border_commands(element, bounds, self.borders, color, commands)
    }
}

fn paint_contribution(
    element: &ElementSource,
    properties: &BTreeMap<String, String>,
) -> Result<Option<PaintContribution>, String> {
    reject_unsupported_paint(element, properties)?;
    if !collapse_whitespace(&element.content.direct_text).is_empty() {
        return Err(format!("text paint for {} is not implemented", element.id));
    }
    if matches!(
        element.tag.as_str(),
        "button"
            | "input"
            | "select"
            | "textarea"
            | "audio"
            | "canvas"
            | "embed"
            | "iframe"
            | "img"
            | "object"
            | "svg"
            | "video"
    ) {
        return Err(format!(
            "native {} paint for {} is not implemented",
            element.tag, element.id
        ));
    }
    let background = properties
        .get("background-color")
        .map(|value| parse_color(value, "background-color", &element.id))
        .transpose()?
        .filter(|color| color.alpha > 0);
    let borders = border_paint(properties, &element.id)?;
    if background.is_none() && borders.color.is_none() {
        Ok(None)
    } else {
        Ok(Some(PaintContribution {
            background,
            borders,
        }))
    }
}

fn reject_unsupported_paint(
    element: &ElementSource,
    properties: &BTreeMap<String, String>,
) -> Result<(), String> {
    for name in [
        "background",
        "background-image",
        "background-position",
        "background-repeat",
        "background-size",
        "border-radius",
        "box-shadow",
        "clip",
        "clip-path",
        "filter",
        "isolation",
        "mask",
        "mix-blend-mode",
        "outline",
        "text-shadow",
        "z-index",
    ] {
        if properties.contains_key(name) {
            return Err(format!(
                "inline {name} paint is not implemented for {}",
                element.id
            ));
        }
    }
    if properties
        .get("opacity")
        .is_some_and(|value| value != "1" && value != "1.0")
    {
        return Err(format!(
            "inline opacity paint is not implemented for {}",
            element.id
        ));
    }
    if properties
        .get("overflow")
        .is_some_and(|value| value != "visible")
    {
        return Err(format!(
            "inline overflow clipping is not implemented for {}",
            element.id
        ));
    }
    Ok(())
}

fn border_paint(
    properties: &BTreeMap<String, String>,
    element: &str,
) -> Result<BorderPaint, String> {
    let top = border_width(properties, "top")?;
    let right = border_width(properties, "right")?;
    let bottom = border_width(properties, "bottom")?;
    let left = border_width(properties, "left")?;
    let widths = [
        ("top", top),
        ("right", right),
        ("bottom", bottom),
        ("left", left),
    ];
    let mut color = None;
    for (side, width) in widths {
        if width == 0 {
            continue;
        }
        if properties
            .get(&format!("border-{side}-style"))
            .map(String::as_str)
            != Some("solid")
        {
            return Err(format!(
                "paint for a non-solid {side} border is not implemented for {element}"
            ));
        }
        let value = properties
            .get(&format!("border-{side}-color"))
            .ok_or_else(|| format!("inline border-{side}-color is required to paint {element}"))?;
        let side_color = parse_color(value, &format!("border-{side}-color"), element)?;
        if let Some(existing) = color
            && existing != side_color
        {
            return Err(format!(
                "different border side colors are not implemented for {element}"
            ));
        }
        color = Some(side_color);
    }
    Ok(BorderPaint {
        top,
        right,
        bottom,
        left,
        color: color.filter(|color| color.alpha > 0),
    })
}

fn append_border_commands(
    element: &ElementSource,
    bounds: CaptureRect,
    border: BorderPaint,
    color: Rgba8,
    commands: &mut Vec<PaintCommand>,
) -> Result<(), String> {
    let inner_height = bounds
        .height()
        .checked_sub(border.top)
        .and_then(|height| height.checked_sub(border.bottom))
        .ok_or_else(|| format!("paint borders exceed the height of {}", element.id))?;
    let right_x = add_coordinate(bounds.right(), border.right, false, &element.id)?;
    let bottom_y = add_coordinate(bounds.bottom(), border.bottom, false, &element.id)?;
    let side_y = add_coordinate(bounds.y(), border.top, true, &element.id)?;
    for (side, rect) in [
        (
            "top",
            optional_rect(bounds.x(), bounds.y(), bounds.width(), border.top)?,
        ),
        (
            "bottom",
            optional_rect(bounds.x(), bottom_y, bounds.width(), border.bottom)?,
        ),
        (
            "left",
            optional_rect(bounds.x(), side_y, border.left, inner_height)?,
        ),
        (
            "right",
            optional_rect(right_x, side_y, border.right, inner_height)?,
        ),
    ] {
        if let Some(bounds) = rect {
            commands.push(PaintCommand::FillRect {
                source: format!("{}#border-{side}", element.id),
                bounds,
                color,
            });
        }
    }
    Ok(())
}

fn add_coordinate(value: i64, amount: u64, add: bool, element: &str) -> Result<i64, String> {
    let amount =
        i64::try_from(amount).map_err(|_| format!("paint coordinates overflow for {element}"))?;
    if add {
        value.checked_add(amount)
    } else {
        value.checked_sub(amount)
    }
    .ok_or_else(|| format!("paint coordinates overflow for {element}"))
}

fn optional_rect(x: i64, y: i64, width: u64, height: u64) -> Result<Option<CaptureRect>, String> {
    if width == 0 || height == 0 {
        return Ok(None);
    }
    CaptureRect::new(x, y, width, height)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn translate_box(
    bounds: BoundingBox,
    scrolls_with_document: bool,
    fixed_offset_x: u64,
    fixed_offset_y: u64,
    element: &str,
) -> Result<CaptureRect, String> {
    let (x, y) = if scrolls_with_document {
        (bounds.x, bounds.y)
    } else {
        (
            add_coordinate(bounds.x, fixed_offset_x, true, element)?,
            add_coordinate(bounds.y, fixed_offset_y, true, element)?,
        )
    };
    CaptureRect::new(x, y, bounds.width, bounds.height).map_err(|error| error.to_string())
}

fn intersects(left: CaptureRect, right: CaptureRect) -> bool {
    left.x() < right.right()
        && left.right() > right.x()
        && left.y() < right.bottom()
        && left.bottom() > right.y()
}

fn parse_color(value: &str, property: &str, element: &str) -> Result<Rgba8, String> {
    parse_hex_color(value)
        .or_else(|| parse_rgb_color(value))
        .or_else(|| named_color(value))
        .ok_or_else(|| {
            format!("inline {property} color {value:?} is not implemented for {element}")
        })
}

fn parse_hex_color(value: &str) -> Option<Rgba8> {
    let digits = value.strip_prefix('#')?;
    if !digits.is_ascii() {
        return None;
    }
    let expanded;
    let digits = match digits.len() {
        3 | 4 => {
            expanded = digits
                .chars()
                .flat_map(|digit| [digit, digit])
                .collect::<String>();
            expanded.as_str()
        }
        6 | 8 => digits,
        _ => return None,
    };
    let red = u8::from_str_radix(&digits[0..2], 16).ok()?;
    let green = u8::from_str_radix(&digits[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&digits[4..6], 16).ok()?;
    let alpha = if digits.len() == 8 {
        u8::from_str_radix(&digits[6..8], 16).ok()?
    } else {
        255
    };
    Some(Rgba8 {
        red,
        green,
        blue,
        alpha,
    })
}

fn parse_rgb_color(value: &str) -> Option<Rgba8> {
    let (function, alpha) = if let Some(inner) = value.strip_prefix("rgba(") {
        (inner.strip_suffix(')')?, true)
    } else {
        (value.strip_prefix("rgb(")?.strip_suffix(')')?, false)
    };
    let parts = function.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != if alpha { 4 } else { 3 } {
        return None;
    }
    let channel = |value: &str| value.parse::<u8>().ok();
    let alpha_value = if alpha {
        let value = parts[3].parse::<f64>().ok()?;
        if !(0.0..=1.0).contains(&value) {
            return None;
        }
        (value * 255.0).round() as u8
    } else {
        255
    };
    Some(Rgba8 {
        red: channel(parts[0])?,
        green: channel(parts[1])?,
        blue: channel(parts[2])?,
        alpha: alpha_value,
    })
}

fn named_color(value: &str) -> Option<Rgba8> {
    let (red, green, blue, alpha) = match value {
        "transparent" => (0, 0, 0, 0),
        "black" => (0, 0, 0, 255),
        "white" => (255, 255, 255, 255),
        "red" => (255, 0, 0, 255),
        "green" => (0, 128, 0, 255),
        "blue" => (0, 0, 255, 255),
        "yellow" => (255, 255, 0, 255),
        "gray" | "grey" => (128, 128, 128, 255),
        _ => return None,
    };
    Some(Rgba8 {
        red,
        green,
        blue,
        alpha,
    })
}

const WHITE: Rgba8 = Rgba8 {
    red: 255,
    green: 255,
    blue: 255,
    alpha: 255,
};

#[cfg(test)]
mod tests {
    use super::{paint_commands_from_html, parse_color};
    use crate::{CaptureRect, PaintCommand, Rgba8};

    #[test]
    fn builds_ordered_solid_background_commands() {
        let capture = CaptureRect::new(0, 0, 40, 30).unwrap();
        let commands = paint_commands_from_html(
            r#"<body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0;background-color:#102030"><main id="card" style="width:20px;height:10px;background-color:rgba(255, 0, 0, 0.5)"></main></body>"#,
            40,
            capture,
            0,
            0,
        )
        .unwrap();

        assert_eq!(commands.len(), 3);
        assert!(matches!(
            &commands[1],
            PaintCommand::FillRect { source, bounds, .. }
                if source == "body[1]#background" && *bounds == capture
        ));
        assert!(matches!(
            &commands[2],
            PaintCommand::FillRect { source, bounds, .. }
                if source == "card#background" && bounds.width() == 20 && bounds.height() == 10
        ));
    }

    #[test]
    fn blocks_text_instead_of_returning_incomplete_paint() {
        let error = paint_commands_from_html(
            r#"<main style="height:20px;background-color:red">hello</main>"#,
            40,
            CaptureRect::new(0, 0, 40, 30).unwrap(),
            0,
            0,
        )
        .unwrap_err();

        assert!(error.contains("text paint"));
    }

    #[test]
    fn parses_supported_css_colors() {
        assert_eq!(
            parse_color("#1234", "background-color", "box").unwrap(),
            Rgba8 {
                red: 17,
                green: 34,
                blue: 51,
                alpha: 68,
            }
        );
        assert_eq!(
            parse_color("rgb(1, 2, 3)", "background-color", "box").unwrap(),
            Rgba8 {
                red: 1,
                green: 2,
                blue: 3,
                alpha: 255,
            }
        );
        assert!(parse_color("#aéaaa", "background-color", "box").is_err());
    }
}
