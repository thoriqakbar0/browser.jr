use std::collections::BTreeMap;

use super::CssLocatorError;

const MAX_SELECTOR_COMPOUNDS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CssSelector {
    compounds: Vec<SimpleCssSelector>,
    combinators: Vec<Combinator>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Combinator {
    Child,
    Descendant,
}

pub(crate) trait CssNode {
    fn css_tag(&self) -> &str;
    fn css_attributes(&self) -> &BTreeMap<String, String>;
    fn css_parent(&self) -> Option<usize>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SimpleCssSelector {
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

impl CssSelector {
    pub(crate) fn parse(source: &str) -> Result<Self, CssLocatorError> {
        let source = source.trim();
        if source.is_empty() {
            return Err(CssLocatorError::EmptySelector);
        }

        let mut compounds = Vec::new();
        let mut combinators = Vec::new();
        let mut index = 0;
        loop {
            let (compound, end) = parse_compound(source, index)?;
            compounds.push(compound);
            if compounds.len() > MAX_SELECTOR_COMPOUNDS {
                return Err(CssLocatorError::UnsupportedSelector);
            }

            let Some((combinator, next_index)) = parse_combinator(source, end)? else {
                break;
            };
            combinators.push(combinator);
            index = next_index;
        }

        Ok(Self {
            compounds,
            combinators,
        })
    }

    pub(crate) fn matches<N: CssNode>(&self, index: usize, nodes: &[N]) -> bool {
        let mut ancestry = Vec::new();
        let mut current = Some(index);
        while let Some(index) = current {
            ancestry.push(index);
            current = nodes[index].css_parent();
        }
        ancestry.reverse();

        let mut previous = ancestry
            .iter()
            .map(|index| {
                let node = &nodes[*index];
                self.compounds[0].matches(node.css_tag(), node.css_attributes())
            })
            .collect::<Vec<_>>();
        for part in 1..self.compounds.len() {
            let mut ancestor_matches = false;
            let mut matches = vec![false; ancestry.len()];
            for (position, index) in ancestry.iter().copied().enumerate() {
                let relation_matches = match self.combinators[part - 1] {
                    Combinator::Child => position
                        .checked_sub(1)
                        .is_some_and(|parent| previous[parent]),
                    Combinator::Descendant => ancestor_matches,
                };
                let node = &nodes[index];
                matches[position] = relation_matches
                    && self.compounds[part].matches(node.css_tag(), node.css_attributes());
                ancestor_matches |= previous[position];
            }
            previous = matches;
        }
        previous.last().copied().unwrap_or(false)
    }

    pub(crate) fn specificity(&self) -> (usize, usize, usize) {
        self.compounds.iter().fold((0, 0, 0), |total, part| {
            let specificity = part.specificity();
            (
                total.0 + specificity.0,
                total.1 + specificity.1,
                total.2 + specificity.2,
            )
        })
    }
}

fn parse_compound(
    source: &str,
    start: usize,
) -> Result<(SimpleCssSelector, usize), CssLocatorError> {
    let end = compound_end(source, start)?;
    Ok((SimpleCssSelector::parse(&source[start..end])?, end))
}

fn parse_combinator(
    source: &str,
    end: usize,
) -> Result<Option<(Combinator, usize)>, CssLocatorError> {
    if end == source.len() {
        return Ok(None);
    }

    let bytes = source.as_bytes();
    let had_space = bytes[end].is_ascii_whitespace();
    let mut index = skip_spaces(bytes, end);
    let combinator = if bytes.get(index) == Some(&b'>') {
        index = skip_spaces(bytes, index + 1);
        Combinator::Child
    } else if had_space {
        Combinator::Descendant
    } else {
        return Err(CssLocatorError::UnsupportedSelector);
    };
    if index == source.len() || bytes.get(index) == Some(&b'>') {
        return Err(CssLocatorError::InvalidSelector);
    }
    Ok(Some((combinator, index)))
}

impl SimpleCssSelector {
    fn parse(source: &str) -> Result<Self, CssLocatorError> {
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

    fn matches(&self, tag: &str, attributes: &BTreeMap<String, String>) -> bool {
        self.matches_tag(tag)
            && self.matches_id(attributes)
            && self.matches_classes(attributes)
            && self.matches_attributes(attributes)
    }

    fn specificity(&self) -> (usize, usize, usize) {
        (
            usize::from(self.id.is_some()),
            self.classes.len() + self.attributes.len(),
            usize::from(self.tag.is_some()),
        )
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

fn compound_end(source: &str, start: usize) -> Result<usize, CssLocatorError> {
    let mut scanner = CompoundScanner::default();
    if let Some(offset) = source.as_bytes()[start..]
        .iter()
        .copied()
        .position(|value| scanner.reached_end(value))
    {
        return Ok(start + offset);
    }
    scanner.finish(source.len())
}

#[derive(Default)]
struct CompoundScanner {
    bracket_depth: usize,
    quote: Option<u8>,
}

impl CompoundScanner {
    fn reached_end(&mut self, value: u8) -> bool {
        if let Some(expected) = self.quote {
            if value == expected {
                self.quote = None;
            }
            return false;
        }
        match value {
            b'\'' | b'"' if self.bracket_depth > 0 => self.quote = Some(value),
            b'[' => self.bracket_depth += 1,
            b']' if self.bracket_depth > 0 => self.bracket_depth -= 1,
            b'>' if self.bracket_depth == 0 => return true,
            value if value.is_ascii_whitespace() && self.bracket_depth == 0 => return true,
            _ => {}
        }
        false
    }

    fn finish(&self, end: usize) -> Result<usize, CssLocatorError> {
        if self.quote.is_some() || self.bracket_depth != 0 {
            Err(CssLocatorError::InvalidSelector)
        } else {
            Ok(end)
        }
    }
}

fn skip_spaces(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
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
    matches!(value, b'+' | b'~' | b',' | b':' | b'|' | b'^' | b'$' | b'*')
}

fn is_identifier_byte(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_')
}

#[cfg(test)]
mod tests {
    use super::{CssNode, CssSelector, MAX_SELECTOR_COMPOUNDS};
    use crate::locator::CssLocatorError;
    use std::cell::Cell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    struct CountingNode {
        attributes: BTreeMap<String, String>,
        parent: Option<usize>,
        parent_reads: Rc<Cell<usize>>,
    }

    impl CssNode for CountingNode {
        fn css_tag(&self) -> &str {
            "div"
        }

        fn css_attributes(&self) -> &BTreeMap<String, String> {
            &self.attributes
        }

        fn css_parent(&self) -> Option<usize> {
            self.parent_reads.set(self.parent_reads.get() + 1);
            self.parent
        }
    }

    #[test]
    fn selector_compound_limit_accepts_64_and_rejects_65() {
        let accepted = vec!["div"; MAX_SELECTOR_COMPOUNDS].join(" ");
        let rejected = vec!["div"; MAX_SELECTOR_COMPOUNDS + 1].join(" ");

        assert!(CssSelector::parse(&accepted).is_ok());
        assert_eq!(
            CssSelector::parse(&rejected),
            Err(CssLocatorError::UnsupportedSelector)
        );
    }

    #[test]
    fn selector_parser_preserves_typed_error_boundaries() {
        for (source, expected) in [
            ("", CssLocatorError::EmptySelector),
            ("div >", CssLocatorError::InvalidSelector),
            ("div > > span", CssLocatorError::InvalidSelector),
            ("[title='open", CssLocatorError::InvalidSelector),
            ("[title", CssLocatorError::InvalidSelector),
            ("div + span", CssLocatorError::UnsupportedSelector),
            ("div, span", CssLocatorError::UnsupportedSelector),
        ] {
            assert_eq!(CssSelector::parse(source), Err(expected), "{source}");
        }
    }

    #[test]
    fn descendant_matching_reads_the_ancestry_once() {
        let parent_reads = Rc::new(Cell::new(0));
        let attributes = BTreeMap::from([("class".into(), "a".into())]);
        let nodes = (0usize..12)
            .map(|index| CountingNode {
                attributes: attributes.clone(),
                parent: index.checked_sub(1),
                parent_reads: Rc::clone(&parent_reads),
            })
            .collect::<Vec<_>>();
        let selector = CssSelector::parse(".missing .a .a .a .a .a .a .a .a").unwrap();

        assert!(!selector.matches(nodes.len() - 1, &nodes));
        assert!(parent_reads.get() <= nodes.len());
    }
}
