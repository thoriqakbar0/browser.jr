use std::collections::BTreeMap;

use super::CssLocatorError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SimpleCssSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AttributeSelector {
    name: String,
    value: Option<String>,
}

impl SimpleCssSelector {
    pub(super) fn parse(source: &str) -> Result<Self, CssLocatorError> {
        let (tag, mut index) = parse_type_selector(source)?;
        let mut selector = Self {
            tag,
            id: None,
            classes: Vec::new(),
            attributes: Vec::new(),
        };
        while index < source.len() {
            index = selector.parse_modifier(source, index)?;
        }
        if selector.is_empty() && source != "*" {
            return Err(CssLocatorError::InvalidSelector);
        }
        Ok(selector)
    }

    pub(super) fn matches(&self, tag: &str, attributes: &BTreeMap<String, String>) -> bool {
        self.matches_tag(tag)
            && self.matches_id(attributes)
            && self.matches_classes(attributes)
            && self.matches_attributes(attributes)
    }

    fn parse_modifier(&mut self, source: &str, index: usize) -> Result<usize, CssLocatorError> {
        match source.as_bytes()[index] {
            b'#' => self.parse_id(source, index + 1),
            b'.' => self.parse_class(source, index + 1),
            b'[' => self.parse_attribute(source, index + 1),
            value if is_unsupported_operator(value) => Err(CssLocatorError::UnsupportedSelector),
            _ => Err(CssLocatorError::InvalidSelector),
        }
    }

    fn parse_id(&mut self, source: &str, start: usize) -> Result<usize, CssLocatorError> {
        let (id, end) = parse_identifier(source, start)?;
        if self.id.replace(id.into()).is_some() {
            return Err(CssLocatorError::InvalidSelector);
        }
        Ok(end)
    }

    fn parse_class(&mut self, source: &str, start: usize) -> Result<usize, CssLocatorError> {
        let (class, end) = parse_identifier(source, start)?;
        self.classes.push(class.into());
        Ok(end)
    }

    fn parse_attribute(&mut self, source: &str, start: usize) -> Result<usize, CssLocatorError> {
        let (attribute, end) = parse_attribute_selector(source, start)?;
        self.attributes.push(attribute);
        Ok(end)
    }

    fn is_empty(&self) -> bool {
        self.tag.is_none()
            && self.id.is_none()
            && self.classes.is_empty()
            && self.attributes.is_empty()
    }

    fn matches_tag(&self, tag: &str) -> bool {
        self.tag
            .as_deref()
            .is_none_or(|expected| expected.eq_ignore_ascii_case(tag))
    }

    fn matches_id(&self, attributes: &BTreeMap<String, String>) -> bool {
        self.id
            .as_deref()
            .is_none_or(|expected| attributes.get("id").is_some_and(|id| id == expected))
    }

    fn matches_classes(&self, attributes: &BTreeMap<String, String>) -> bool {
        let classes = attributes
            .get("class")
            .map(String::as_str)
            .unwrap_or_default();
        self.classes.iter().all(|expected| {
            classes
                .split_ascii_whitespace()
                .any(|class| class == expected)
        })
    }

    fn matches_attributes(&self, attributes: &BTreeMap<String, String>) -> bool {
        self.attributes.iter().all(|expected| {
            attributes.get(&expected.name).is_some_and(|actual| {
                expected
                    .value
                    .as_deref()
                    .is_none_or(|value| actual == value)
            })
        })
    }
}

fn parse_type_selector(source: &str) -> Result<(Option<String>, usize), CssLocatorError> {
    let Some(first) = source.as_bytes().first().copied() else {
        return Err(CssLocatorError::EmptySelector);
    };
    if first == b'*' {
        return Ok((None, 1));
    }
    if !is_identifier_byte(first) {
        return Ok((None, 0));
    }
    let (tag, end) = parse_identifier(source, 0)?;
    Ok((Some(tag.to_ascii_lowercase()), end))
}

fn parse_identifier(source: &str, start: usize) -> Result<(&str, usize), CssLocatorError> {
    let end = source.as_bytes()[start..]
        .iter()
        .position(|value| !is_identifier_byte(*value))
        .map_or(source.len(), |offset| start + offset);
    if end == start {
        return Err(CssLocatorError::InvalidSelector);
    }
    Ok((&source[start..end], end))
}

fn parse_attribute_selector(
    source: &str,
    start: usize,
) -> Result<(AttributeSelector, usize), CssLocatorError> {
    let (name, mut index) = parse_identifier(source, start)?;
    let name = name.to_ascii_lowercase();
    let bytes = source.as_bytes();
    if bytes.get(index) == Some(&b']') {
        return Ok((AttributeSelector { name, value: None }, index + 1));
    }
    if bytes.get(index) != Some(&b'=') {
        return Err(attribute_operator_error(bytes.get(index).copied()));
    }
    index += 1;
    let (value, end) = parse_attribute_value(source, index)?;
    Ok((
        AttributeSelector {
            name,
            value: Some(value.into()),
        },
        end,
    ))
}

fn parse_attribute_value(source: &str, start: usize) -> Result<(&str, usize), CssLocatorError> {
    let bytes = source.as_bytes();
    let Some(first) = bytes.get(start).copied() else {
        return Err(CssLocatorError::InvalidSelector);
    };
    let quoted = matches!(first, b'\'' | b'"');
    let value_start = start + usize::from(quoted);
    let terminator = if quoted { first } else { b']' };
    let value_end = bytes[value_start..]
        .iter()
        .position(|candidate| *candidate == terminator)
        .map(|offset| value_start + offset)
        .ok_or(CssLocatorError::InvalidSelector)?;
    let closing_bracket = value_end + usize::from(quoted);
    let value = &source[value_start..value_end];
    if value.is_empty() || bytes.get(closing_bracket) != Some(&b']') {
        return Err(CssLocatorError::InvalidSelector);
    }
    if !quoted && !value.bytes().all(is_identifier_byte) {
        return Err(CssLocatorError::UnsupportedSelector);
    }
    Ok((value, closing_bracket + 1))
}

fn attribute_operator_error(value: Option<u8>) -> CssLocatorError {
    if value.is_some_and(is_unsupported_operator) {
        CssLocatorError::UnsupportedSelector
    } else {
        CssLocatorError::InvalidSelector
    }
}

fn is_unsupported_operator(value: u8) -> bool {
    value.is_ascii_whitespace()
        || matches!(
            value,
            b'>' | b'+' | b'~' | b',' | b':' | b'|' | b'^' | b'$' | b'*'
        )
}

fn is_identifier_byte(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_')
}
