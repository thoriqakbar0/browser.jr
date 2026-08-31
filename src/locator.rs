use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleLocator {
    role: String,
    name: Option<AccessibleNameMatch>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Locator {
    Role(RoleLocator),
    Text(TextLocator),
    Label(LabelLocator),
    Placeholder(PlaceholderLocator),
    Alt(AltLocator),
    Title(TitleLocator),
    TestId(TestIdLocator),
    Css(CssLocator),
    XPath(XPathLocator),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextLocator(TextMatch);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelLocator(TextMatch);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderLocator(TextMatch);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AltLocator(TextMatch);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TitleLocator(TextMatch);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestIdLocator {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CssLocator {
    source: String,
    position: Option<LocatorPosition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XPathLocator {
    expression: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorPosition {
    First,
    Last,
    Nth(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorMatch {
    pub element: String,
    pub role: Option<String>,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleMatch {
    pub element: String,
    pub role: String,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AccessibleNameMatch {
    Contains(String),
    Exact(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextMatch {
    value: String,
    exact: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleLocatorError {
    EmptyRole,
    InvalidRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorValueError {
    EmptyValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CssLocatorError {
    EmptySelector,
    InvalidSelector,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XPathLocatorError {
    EmptyExpression,
    InvalidExpression,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LocatorCandidate<'a> {
    pub(crate) semantic: SemanticLocatorCandidate<'a>,
    pub(crate) source: SourceLocatorCandidate<'a>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SemanticLocatorCandidate<'a> {
    pub(crate) role: Option<&'a str>,
    pub(crate) name: &'a str,
    pub(crate) text: &'a str,
    pub(crate) label: Option<&'a str>,
    pub(crate) placeholder: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceLocatorCandidate<'a> {
    pub(crate) attributes: &'a BTreeMap<String, String>,
}

impl RoleLocator {
    pub fn new(role: impl Into<String>) -> Result<Self, RoleLocatorError> {
        let role = role.into();
        let role = role.trim();
        if role.is_empty() {
            return Err(RoleLocatorError::EmptyRole);
        }
        if !role
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-')
        {
            return Err(RoleLocatorError::InvalidRole);
        }
        Ok(Self {
            role: role.to_ascii_lowercase(),
            name: None,
        })
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(AccessibleNameMatch::Contains(normalize_name(name.into())));
        self
    }

    pub fn with_exact_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(AccessibleNameMatch::Exact(normalize_name(name.into())));
        self
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn name(&self) -> Option<&str> {
        match &self.name {
            Some(AccessibleNameMatch::Contains(name) | AccessibleNameMatch::Exact(name)) => {
                Some(name)
            }
            None => None,
        }
    }

    pub fn exact(&self) -> bool {
        matches!(self.name, Some(AccessibleNameMatch::Exact(_)))
    }

    pub(crate) fn matches(&self, role: &str, name: &str) -> bool {
        if !self.role.eq_ignore_ascii_case(role) {
            return false;
        }
        match &self.name {
            None => true,
            Some(AccessibleNameMatch::Contains(expected)) => name
                .to_lowercase()
                .contains(expected.to_lowercase().as_str()),
            Some(AccessibleNameMatch::Exact(expected)) => name == expected,
        }
    }
}

macro_rules! text_locator {
    ($locator:ident) => {
        impl $locator {
            pub fn new(value: impl Into<String>) -> Result<Self, LocatorValueError> {
                Ok(Self(TextMatch::new(value)?))
            }

            pub fn exact(mut self) -> Self {
                self.0.exact = true;
                self
            }

            pub fn value(&self) -> &str {
                &self.0.value
            }

            pub fn is_exact(&self) -> bool {
                self.0.exact
            }

            pub(crate) fn matches(&self, candidate: &str) -> bool {
                self.0.matches(candidate)
            }
        }
    };
}

text_locator!(TextLocator);
text_locator!(LabelLocator);
text_locator!(PlaceholderLocator);
text_locator!(AltLocator);
text_locator!(TitleLocator);

impl TestIdLocator {
    pub fn new(value: impl Into<String>) -> Result<Self, LocatorValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(LocatorValueError::EmptyValue);
        }
        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    fn matches(&self, candidate: &str) -> bool {
        candidate == self.value
    }
}

impl CssLocator {
    pub fn new(selector: impl Into<String>) -> Result<Self, CssLocatorError> {
        Self::with_position(selector, None)
    }

    pub fn first(selector: impl Into<String>) -> Result<Self, CssLocatorError> {
        Self::with_position(selector, Some(LocatorPosition::First))
    }

    pub fn last(selector: impl Into<String>) -> Result<Self, CssLocatorError> {
        Self::with_position(selector, Some(LocatorPosition::Last))
    }

    pub fn nth(index: usize, selector: impl Into<String>) -> Result<Self, CssLocatorError> {
        Self::with_position(selector, Some(LocatorPosition::Nth(index)))
    }

    pub fn selector(&self) -> &str {
        &self.source
    }

    pub fn position(&self) -> Option<LocatorPosition> {
        self.position
    }

    fn with_position(
        selector: impl Into<String>,
        position: Option<LocatorPosition>,
    ) -> Result<Self, CssLocatorError> {
        let source = selector.into();
        let source = source.trim();
        if source.is_empty() {
            return Err(CssLocatorError::EmptySelector);
        }
        dom_query::Matcher::new(source).map_err(|_| CssLocatorError::InvalidSelector)?;
        Ok(Self {
            source: source.into(),
            position,
        })
    }
}

impl XPathLocator {
    pub fn new(expression: impl Into<String>) -> Result<Self, XPathLocatorError> {
        let expression = expression.into();
        let expression = expression.trim();
        if expression.is_empty() {
            return Err(XPathLocatorError::EmptyExpression);
        }
        let parsed = sxd_xpath::Factory::new()
            .build(expression)
            .map_err(|_| XPathLocatorError::InvalidExpression)?;
        if parsed.is_none() {
            return Err(XPathLocatorError::InvalidExpression);
        }
        Ok(Self {
            expression: expression.into(),
        })
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
}

impl Locator {
    pub(crate) fn matches(&self, candidate: LocatorCandidate<'_>) -> bool {
        let semantic = candidate.semantic;
        let source = candidate.source;
        match self {
            Self::Role(locator) => semantic
                .role
                .is_some_and(|role| locator.matches(role, semantic.name)),
            Self::Text(locator) => locator.matches(semantic.text),
            Self::Label(locator) => semantic.label.is_some_and(|label| locator.matches(label)),
            Self::Placeholder(locator) => semantic
                .placeholder
                .is_some_and(|placeholder| locator.matches(placeholder)),
            Self::Alt(locator) => source
                .attributes
                .get("alt")
                .is_some_and(|alt| locator.matches(alt)),
            Self::Title(locator) => source
                .attributes
                .get("title")
                .is_some_and(|title| locator.matches(title)),
            Self::TestId(locator) => source
                .attributes
                .get("data-testid")
                .is_some_and(|test_id| locator.matches(test_id)),
            Self::Css(_) | Self::XPath(_) => false,
        }
    }

    pub(crate) fn uses_descendant_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub(crate) fn position(&self) -> Option<LocatorPosition> {
        match self {
            Self::Css(locator) => locator.position(),
            Self::Role(_)
            | Self::Text(_)
            | Self::Label(_)
            | Self::Placeholder(_)
            | Self::Alt(_)
            | Self::Title(_)
            | Self::TestId(_)
            | Self::XPath(_) => None,
        }
    }

    pub(crate) fn css(&self) -> Option<&CssLocator> {
        match self {
            Self::Css(locator) => Some(locator),
            _ => None,
        }
    }

    pub(crate) fn xpath(&self) -> Option<&XPathLocator> {
        match self {
            Self::XPath(locator) => Some(locator),
            _ => None,
        }
    }
}

impl From<RoleLocator> for Locator {
    fn from(locator: RoleLocator) -> Self {
        Self::Role(locator)
    }
}

impl From<TextLocator> for Locator {
    fn from(locator: TextLocator) -> Self {
        Self::Text(locator)
    }
}

impl From<LabelLocator> for Locator {
    fn from(locator: LabelLocator) -> Self {
        Self::Label(locator)
    }
}

impl From<PlaceholderLocator> for Locator {
    fn from(locator: PlaceholderLocator) -> Self {
        Self::Placeholder(locator)
    }
}

impl From<AltLocator> for Locator {
    fn from(locator: AltLocator) -> Self {
        Self::Alt(locator)
    }
}

impl From<TitleLocator> for Locator {
    fn from(locator: TitleLocator) -> Self {
        Self::Title(locator)
    }
}

impl From<TestIdLocator> for Locator {
    fn from(locator: TestIdLocator) -> Self {
        Self::TestId(locator)
    }
}

impl From<CssLocator> for Locator {
    fn from(locator: CssLocator) -> Self {
        Self::Css(locator)
    }
}

impl From<XPathLocator> for Locator {
    fn from(locator: XPathLocator) -> Self {
        Self::XPath(locator)
    }
}

impl LocatorMatch {
    pub(crate) fn new(element: &str, role: Option<&str>, name: &str, text: &str) -> Self {
        Self {
            element: element.into(),
            role: role.map(str::to_owned),
            name: name.into(),
            text: text.into(),
        }
    }

    pub(crate) fn into_role_match(self) -> RoleMatch {
        RoleMatch {
            element: self.element,
            role: self
                .role
                .expect("role locator results always carry a semantic role"),
            name: self.name,
            text: self.text,
        }
    }
}

impl std::fmt::Display for RoleLocator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "role {:?}", self.role)?;
        match &self.name {
            Some(AccessibleNameMatch::Contains(name)) => {
                write!(formatter, " with accessible name containing {name:?}")
            }
            Some(AccessibleNameMatch::Exact(name)) => {
                write!(formatter, " with exact accessible name {name:?}")
            }
            None => Ok(()),
        }
    }
}

impl std::fmt::Display for Locator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Role(locator) => locator.fmt(formatter),
            Self::Text(locator) => write_text_locator(formatter, "text", &locator.0),
            Self::Label(locator) => write_text_locator(formatter, "label", &locator.0),
            Self::Placeholder(locator) => write_text_locator(formatter, "placeholder", &locator.0),
            Self::Alt(locator) => write_text_locator(formatter, "alt text", &locator.0),
            Self::Title(locator) => write_text_locator(formatter, "title", &locator.0),
            Self::TestId(locator) => write!(formatter, "test id {:?}", locator.value),
            Self::Css(locator) => locator.fmt(formatter),
            Self::XPath(locator) => locator.fmt(formatter),
        }
    }
}

impl std::fmt::Display for RoleLocatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRole => formatter.write_str("role must not be empty"),
            Self::InvalidRole => {
                formatter.write_str("role must use only ASCII letters, digits, or hyphens")
            }
        }
    }
}

impl std::error::Error for RoleLocatorError {}

impl std::fmt::Display for LocatorValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyValue => formatter.write_str("locator value must not be empty"),
        }
    }
}

impl std::error::Error for LocatorValueError {}

impl std::fmt::Display for CssLocatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptySelector => "CSS selector must not be empty",
            Self::InvalidSelector => "CSS selector is invalid",
        })
    }
}

impl std::error::Error for CssLocatorError {}

impl std::fmt::Display for XPathLocatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyExpression => "XPath expression must not be empty",
            Self::InvalidExpression => "XPath expression is invalid",
        })
    }
}

impl std::error::Error for XPathLocatorError {}

impl std::fmt::Display for CssLocator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.position {
            None => write!(formatter, "CSS selector {:?}", self.source),
            Some(LocatorPosition::First) => {
                write!(formatter, "first element matching {:?}", self.source)
            }
            Some(LocatorPosition::Last) => {
                write!(formatter, "last element matching {:?}", self.source)
            }
            Some(LocatorPosition::Nth(index)) => {
                write!(formatter, "element {index} matching {:?}", self.source)
            }
        }
    }
}

impl std::fmt::Display for XPathLocator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "XPath expression {:?}", self.expression)
    }
}

impl TextMatch {
    fn new(value: impl Into<String>) -> Result<Self, LocatorValueError> {
        let value = normalize_name(value.into());
        if value.is_empty() {
            return Err(LocatorValueError::EmptyValue);
        }
        Ok(Self {
            value,
            exact: false,
        })
    }

    fn matches(&self, candidate: &str) -> bool {
        let candidate = normalize_name(candidate.into());
        if self.exact {
            candidate == self.value
        } else {
            candidate
                .to_lowercase()
                .contains(self.value.to_lowercase().as_str())
        }
    }
}

fn write_text_locator(
    formatter: &mut std::fmt::Formatter<'_>,
    kind: &str,
    matcher: &TextMatch,
) -> std::fmt::Result {
    if matcher.exact {
        write!(formatter, "exact {kind} {:?}", matcher.value)
    } else {
        write!(formatter, "{kind} containing {:?}", matcher.value)
    }
}

fn normalize_name(name: String) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        AltLocator, CssLocator, CssLocatorError, LabelLocator, Locator, LocatorCandidate,
        LocatorPosition, LocatorValueError, PlaceholderLocator, RoleLocator, RoleLocatorError,
        SemanticLocatorCandidate, SourceLocatorCandidate, TestIdLocator, TextLocator, TitleLocator,
        XPathLocator, XPathLocatorError,
    };
    use std::collections::BTreeMap;

    #[test]
    fn role_tokens_are_normalized_and_validated() {
        assert_eq!(RoleLocator::new(" BUTTON ").unwrap().role(), "button");
        assert_eq!(
            RoleLocator::new("doc-pagebreak").unwrap().role(),
            "doc-pagebreak"
        );
        assert_eq!(RoleLocator::new(" "), Err(RoleLocatorError::EmptyRole));
        assert_eq!(
            RoleLocator::new("button link"),
            Err(RoleLocatorError::InvalidRole)
        );
    }

    #[test]
    fn name_matching_has_explicit_case_rules() {
        let contains = RoleLocator::new("button")
            .unwrap()
            .with_name(" save   changes ");
        let exact = RoleLocator::new("button")
            .unwrap()
            .with_exact_name("  Save\nChanges  ");

        assert!(contains.matches("BUTTON", "Save Changes Now"));
        assert_eq!(contains.name(), Some("save changes"));
        assert!(!contains.exact());
        assert!(exact.matches("button", "Save Changes"));
        assert_eq!(exact.name(), Some("Save Changes"));
        assert!(!exact.matches("button", "save changes"));
        assert!(exact.exact());
    }

    #[test]
    fn text_backed_locators_normalize_and_match_with_explicit_case_rules() {
        let text = TextLocator::new(" save   changes ").unwrap();
        let exact_label = LabelLocator::new(" Email\naddress ").unwrap().exact();
        let placeholder = PlaceholderLocator::new("Search docs").unwrap();

        assert!(text.matches("Save Changes Now"));
        assert_eq!(text.value(), "save changes");
        assert!(!text.is_exact());
        assert!(exact_label.matches("Email address"));
        assert!(!exact_label.matches("email address"));
        assert!(exact_label.is_exact());
        assert!(placeholder.matches("search docs and examples"));
        assert_eq!(TextLocator::new("  "), Err(LocatorValueError::EmptyValue));
    }

    #[test]
    fn locator_variants_match_only_their_owned_evidence() {
        let role = Locator::from(RoleLocator::new("button").unwrap().with_name("save"));
        let text = Locator::from(TextLocator::new("draft").unwrap());
        let label = Locator::from(LabelLocator::new("email").unwrap());
        let placeholder = Locator::from(PlaceholderLocator::new("search").unwrap());
        let attributes = BTreeMap::new();
        let source = SourceLocatorCandidate {
            attributes: &attributes,
        };

        assert!(role.matches(LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: Some("button"),
                name: "Save",
                text: "Draft",
                label: None,
                placeholder: None,
            },
            source,
        }));
        assert!(text.matches(LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: None,
                name: "",
                text: "Draft",
                label: None,
                placeholder: None,
            },
            source,
        }));
        let labeled = LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: Some("textbox"),
                name: "Email",
                text: "",
                label: Some("Email"),
                placeholder: None,
            },
            source,
        };
        assert!(label.matches(labeled));
        let placeholder_candidate = LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: Some("searchbox"),
                name: "",
                text: "",
                label: None,
                placeholder: Some("Search docs"),
            },
            source,
        };
        assert!(placeholder.matches(placeholder_candidate));
        assert!(!label.matches(placeholder_candidate));
    }

    #[test]
    fn alt_title_and_test_id_use_their_owned_attributes() {
        let alt = Locator::from(AltLocator::new("product image").unwrap());
        let title = Locator::from(TitleLocator::new("Issue count").unwrap().exact());
        let test_id = Locator::from(TestIdLocator::new("SAVE-card").unwrap());
        let attributes = BTreeMap::from([
            ("alt".into(), "Product Image Large".into()),
            ("title".into(), "Issue count".into()),
            ("data-testid".into(), "SAVE-card".into()),
        ]);
        let candidate = LocatorCandidate {
            semantic: SemanticLocatorCandidate {
                role: None,
                name: "",
                text: "",
                label: None,
                placeholder: None,
            },
            source: SourceLocatorCandidate {
                attributes: &attributes,
            },
        };

        assert!(alt.matches(candidate));
        assert!(title.matches(candidate));
        assert!(
            !Locator::from(TitleLocator::new("issue count").unwrap().exact()).matches(candidate)
        );
        assert!(test_id.matches(candidate));
        assert!(!Locator::from(TestIdLocator::new("save-card").unwrap()).matches(candidate));
    }

    #[test]
    fn css_locators_validate_document_selectors_and_positions() {
        let locator = CssLocator::nth(2, "BUTTON.primary[data-kind='save'][disabled]").unwrap();
        let strict = CssLocator::new("form > input[title='hello world']:first-child").unwrap();

        assert_eq!(
            locator.selector(),
            "BUTTON.primary[data-kind='save'][disabled]"
        );
        assert_eq!(locator.position(), Some(LocatorPosition::Nth(2)));
        assert_eq!(strict.position(), None);
        assert_eq!(CssLocator::first(""), Err(CssLocatorError::EmptySelector));
        assert!(CssLocator::last("article .card, button:first-child").is_ok());
        assert_eq!(
            CssLocator::last("[name=]"),
            Err(CssLocatorError::InvalidSelector)
        );
    }

    #[test]
    fn xpath_locators_validate_expressions() {
        let locator = XPathLocator::new("//main/section[@data-kind='card'][2]").unwrap();

        assert_eq!(locator.expression(), "//main/section[@data-kind='card'][2]");
        assert_eq!(
            XPathLocator::new("  "),
            Err(XPathLocatorError::EmptyExpression)
        );
        assert_eq!(
            XPathLocator::new("//["),
            Err(XPathLocatorError::InvalidExpression)
        );
    }
}
