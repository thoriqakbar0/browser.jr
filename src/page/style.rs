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
        if let Some(expected) = quote {
            if value == expected {
                quote = None;
            }
            continue;
        }
        match value {
            b'\'' | b'"' if bracket_depth > 0 => quote = Some(value),
            b'[' => bracket_depth += 1,
            b']' if bracket_depth > 0 => bracket_depth -= 1,
            b',' if bracket_depth == 0 => {
                selectors.push(source[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if quote.is_some() || bracket_depth != 0 {
        return Err("CSS selector list is not closed".into());
    }
    selectors.push(source[start..].trim());
    if selectors.iter().any(|selector| selector.is_empty()) {
        return Err("CSS selector list contains an empty selector".into());
    }
    Ok(selectors)
}

fn strip_comments(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let value = bytes[index];
        if let Some(expected) = quote {
            output.push(value);
            if escaped {
                escaped = false;
            } else if value == b'\\' {
                escaped = true;
            } else if value == expected {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(value, b'\'' | b'"') {
            quote = Some(value);
            output.push(value);
            index += 1;
            continue;
        }
        if value == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let Some(end) = bytes[index + 2..]
                .windows(2)
                .position(|window| window == b"*/")
            else {
                return Err("CSS comment is not closed".into());
            };
            index += end + 4;
            continue;
        }
        output.push(value);
        index += 1;
    }
    Ok(String::from_utf8(output).expect("comment removal preserves UTF-8"))
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
    use super::strip_comments;

    #[test]
    fn css_comments_ignore_markers_inside_quoted_values() {
        let source =
            r#"[data-double="/*"] { display:none } [data-single='*/'] { width:1px } /* gone */"#;

        assert_eq!(
            strip_comments(source).unwrap(),
            r#"[data-double="/*"] { display:none } [data-single='*/'] { width:1px } "#
        );
    }
}
