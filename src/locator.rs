#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleLocator {
    role: String,
    name: Option<AccessibleNameMatch>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleLocatorError {
    EmptyRole,
    InvalidRole,
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

impl RoleMatch {
    pub(crate) fn new(element: &str, role: &str, name: &str, text: &str) -> Self {
        Self {
            element: element.into(),
            role: role.into(),
            name: name.into(),
            text: text.into(),
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

fn normalize_name(name: String) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{RoleLocator, RoleLocatorError};

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
}
