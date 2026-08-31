use std::collections::BTreeMap;

use crate::locator::css::CssSelector;

use super::ElementSource;

#[derive(Clone, Debug)]
struct CssRule {
    selectors: Vec<CssSelector>,
    declarations: BTreeMap<String, String>,
    order: usize,
}

#[derive(Clone, Debug)]
struct CascadedValue {
    value: String,
    specificity: (usize, usize, usize),
    order: usize,
}

pub(super) fn computed_styles(
    elements: &[ElementSource],
    stylesheets: &[String],
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let mut rules = Vec::new();
    for stylesheet in stylesheets {
        parse_stylesheet(stylesheet, &mut rules)?;
    }

    let mut cascaded = vec![BTreeMap::<String, CascadedValue>::new(); elements.len()];
    for rule in rules {
        for (index, destination) in cascaded.iter_mut().enumerate() {
            let specificity = rule
                .selectors
                .iter()
                .filter(|selector| selector.matches(index, elements))
                .map(CssSelector::specificity)
                .max();
            let Some(specificity) = specificity else {
                continue;
            };
            apply_declarations(destination, &rule.declarations, specificity, rule.order);
        }
    }

    for (index, element) in elements.iter().enumerate() {
        let inline = parse_declarations(
            element
                .attributes
                .get("style")
                .map(String::as_str)
                .unwrap_or_default(),
        )?;
        apply_declarations(
            &mut cascaded[index],
            &inline,
            (usize::MAX, usize::MAX, usize::MAX),
            usize::MAX,
        );
    }

    Ok(cascaded
        .into_iter()
        .map(|properties| {
            properties
                .into_iter()
                .map(|(name, value)| (name, value.value))
                .collect()
        })
        .collect())
}

fn parse_stylesheet(source: &str, rules: &mut Vec<CssRule>) -> Result<(), String> {
    let source = strip_comments(source)?;
    let mut remaining = source.trim();
    while !remaining.is_empty() {
        if remaining.starts_with('@') {
            return Err("CSS at-rules are not implemented".into());
        }
        let open = remaining
            .find('{')
            .ok_or_else(|| "CSS rule is missing an opening brace".to_owned())?;
        let close = remaining[open + 1..]
            .find('}')
            .map(|offset| open + 1 + offset)
            .ok_or_else(|| "CSS rule is missing a closing brace".to_owned())?;
        let selector_source = remaining[..open].trim();
        let selectors = split_selector_list(selector_source)?
            .into_iter()
            .map(|selector| {
                CssSelector::parse(selector)
                    .map_err(|error| format!("unsupported CSS selector {selector:?}: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if selectors.is_empty() {
            return Err("CSS rule has no selector".into());
        }
        let declarations = parse_declarations(&remaining[open + 1..close])?;
        rules.push(CssRule {
            selectors,
            declarations,
            order: rules.len(),
        });
        remaining = remaining[close + 1..].trim();
    }
    Ok(())
}

fn split_selector_list(source: &str) -> Result<Vec<&str>, String> {
    let mut selectors = Vec::new();
    let mut start = 0;
    let mut bracket_depth = 0;
    let mut quote = None;
    for (index, value) in source.bytes().enumerate() {
        if selector_boundary(value, &mut quote, &mut bracket_depth) {
            selectors.push(source[start..index].trim());
            start = index + 1;
        }
    }
    require_closed_selector_list(quote, bracket_depth)?;
    selectors.push(source[start..].trim());
    require_non_empty_selectors(&selectors)?;
    Ok(selectors)
}

fn selector_boundary(value: u8, quote: &mut Option<u8>, bracket_depth: &mut usize) -> bool {
    if let Some(expected) = *quote {
        if value == expected {
            *quote = None;
        }
        return false;
    }
    match value {
        b'\'' | b'"' if *bracket_depth > 0 => *quote = Some(value),
        b'[' => *bracket_depth += 1,
        b']' if *bracket_depth > 0 => *bracket_depth -= 1,
        b',' if *bracket_depth == 0 => return true,
        _ => {}
    }
    false
}

fn require_closed_selector_list(quote: Option<u8>, bracket_depth: usize) -> Result<(), String> {
    if quote.is_some() || bracket_depth != 0 {
        Err("CSS selector list is not closed".into())
    } else {
        Ok(())
    }
}

fn require_non_empty_selectors(selectors: &[&str]) -> Result<(), String> {
    if selectors.iter().any(|selector| selector.is_empty()) {
        Err("CSS selector list contains an empty selector".into())
    } else {
        Ok(())
    }
}

fn strip_comments(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut quote = CssQuote::default();
    let mut index = 0;
    while index < bytes.len() {
        let value = bytes[index];
        if quote.consumes(value) {
            output.push(value);
            index += 1;
            continue;
        }
        match comment_end(bytes, index)? {
            Some(end) => index = end,
            None => {
                output.push(value);
                index += 1;
            }
        }
    }
    Ok(String::from_utf8(output).expect("comment removal preserves UTF-8"))
}

#[derive(Default)]
struct CssQuote {
    delimiter: Option<u8>,
    escaped: bool,
}

impl CssQuote {
    fn consumes(&mut self, value: u8) -> bool {
        let Some(delimiter) = self.delimiter else {
            if matches!(value, b'\'' | b'"') {
                self.delimiter = Some(value);
                return true;
            }
            return false;
        };
        if self.escaped {
            self.escaped = false;
        } else if value == b'\\' {
            self.escaped = true;
        } else if value == delimiter {
            self.delimiter = None;
        }
        true
    }
}

fn comment_end(source: &[u8], index: usize) -> Result<Option<usize>, String> {
    if source.get(index..index + 2) != Some(b"/*") {
        return Ok(None);
    }
    let Some(end) = source[index + 2..]
        .windows(2)
        .position(|window| window == b"*/")
    else {
        return Err("CSS comment is not closed".into());
    };
    Ok(Some(index + end + 4))
}

fn parse_declarations(source: &str) -> Result<BTreeMap<String, String>, String> {
    let mut declarations = BTreeMap::new();
    for declaration in source.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let (name, value) = declaration
            .split_once(':')
            .ok_or_else(|| format!("CSS declaration {declaration:?} is missing a colon"))?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        if name.is_empty() || value.is_empty() {
            return Err(format!("CSS declaration {declaration:?} is incomplete"));
        }
        if value.contains("!important") {
            return Err("CSS !important cascade is not implemented".into());
        }
        declarations.insert(name, value);
    }
    Ok(declarations)
}

fn apply_declarations(
    destination: &mut BTreeMap<String, CascadedValue>,
    declarations: &BTreeMap<String, String>,
    specificity: (usize, usize, usize),
    order: usize,
) {
    for (name, value) in declarations {
        let replace = destination
            .get(name)
            .is_none_or(|current| (specificity, order) >= (current.specificity, current.order));
        if replace {
            destination.insert(
                name.clone(),
                CascadedValue {
                    value: value.clone(),
                    specificity,
                    order,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{split_selector_list, strip_comments};

    #[test]
    fn css_comments_ignore_markers_inside_quoted_values() {
        let source =
            r#"[data-double="/*"] { display:none } [data-single='*/'] { width:1px } /* gone */"#;

        assert_eq!(
            strip_comments(source).unwrap(),
            r#"[data-double="/*"] { display:none } [data-single='*/'] { width:1px } "#
        );
    }

    #[test]
    fn css_comments_preserve_escaped_quotes_and_report_unclosed_comments() {
        assert_eq!(
            strip_comments(r#"[data-value="escaped \" /* text"] /* gone */"#).unwrap(),
            r#"[data-value="escaped \" /* text"] "#
        );
        assert_eq!(
            strip_comments("div { color:red } /*").unwrap_err(),
            "CSS comment is not closed"
        );
    }

    #[test]
    fn selector_lists_keep_attribute_commas_and_exact_errors() {
        assert_eq!(
            split_selector_list(r#"[data-value="one,two"], button"#).unwrap(),
            vec![r#"[data-value="one,two"]"#, "button"]
        );
        assert_eq!(
            split_selector_list("button,").unwrap_err(),
            "CSS selector list contains an empty selector"
        );
        assert_eq!(
            split_selector_list("[data-value='open]").unwrap_err(),
            "CSS selector list is not closed"
        );
    }
}
