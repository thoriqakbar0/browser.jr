use crate::layout::{LayoutError, LayoutInput, LayoutKernel, LayoutMutation, LayoutProgram};
use crate::loading::{LoadError, load_local_html};
use crate::page::{
    CheckedState, ControlState, InteractiveAction, InteractiveElementSource, TextValueState,
    page_semantics_from_html,
};
use crate::rules::{
    RuleResult, WidthFinding, evaluate_horizontal_overflow, evaluate_max_element_width,
};
use crate::snapshot::{InteractiveElementRef, InteractiveSnapshot, Snapshot, SnapshotId};
use http::Uri;

mod private {
    pub trait Sealed {}
}

pub trait SessionRequest: private::Sealed {
    type Reply;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError>;
}

#[derive(Debug)]
pub struct Session {
    layout: LayoutKernel,
    identities: IdentityCounters,
    last_snapshot: Option<Snapshot>,
    latest_interactive_snapshot: Option<SnapshotId>,
    current_page: Option<CurrentPage>,
}

#[derive(Debug)]
struct IdentityCounters {
    next_snapshot_id: u64,
    next_document_epoch: u64,
}

#[derive(Debug)]
struct CurrentPage {
    epoch: u64,
    url: String,
    title: String,
    interactive_elements: Vec<InteractiveElementSource>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            layout: LayoutKernel::new(LayoutProgram::initial()),
            identities: IdentityCounters {
                next_snapshot_id: 1,
                next_document_epoch: 1,
            },
            last_snapshot: None,
            latest_interactive_snapshot: None,
            current_page: None,
        }
    }

    pub fn execute<R>(&mut self, request: R) -> Result<R::Reply, SessionError>
    where
        R: SessionRequest,
    {
        request.execute(self)
    }

    fn open_page(&mut self, url: String) -> Result<OpenedPage, LoadError> {
        let html = load_local_html(&url)?;
        let semantics = page_semantics_from_html(&html);
        let epoch = self.identities.next_document_epoch;
        self.identities.next_document_epoch = self
            .identities
            .next_document_epoch
            .checked_add(1)
            .expect("document epoch exhausted");
        let reply = OpenedPage {
            url: url.clone(),
            interactive_element_count: semantics.interactive_elements.len(),
        };
        self.layout = LayoutKernel::new(LayoutProgram::initial());
        self.last_snapshot = None;
        self.latest_interactive_snapshot = None;
        self.current_page = Some(CurrentPage {
            epoch,
            url,
            title: semantics.title,
            interactive_elements: semantics.interactive_elements,
        });
        Ok(reply)
    }

    fn element_index_for(&self, reference: InteractiveElementRef) -> Result<usize, SessionError> {
        let page = self.current_page.as_ref().ok_or(SessionError::NoPage)?;
        if reference.document_epoch() != page.epoch
            || self.latest_interactive_snapshot != Some(reference.snapshot())
        {
            return Err(SessionError::StaleElementReference { reference });
        }
        let index = reference
            .ordinal()
            .checked_sub(1)
            .and_then(|ordinal| usize::try_from(ordinal).ok())
            .expect("interactive snapshot references use nonzero usize ordinals");
        page.interactive_elements
            .get(index)
            .map(|_| index)
            .ok_or(SessionError::StaleElementReference { reference })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenPage {
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenedPage {
    pub url: String,
    pub interactive_element_count: usize,
}

impl private::Sealed for OpenPage {}

impl SessionRequest for OpenPage {
    type Reply = OpenedPage;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.open_page(self.url).map_err(SessionError::Load)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReloadPage;

impl private::Sealed for ReloadPage {}

impl SessionRequest for ReloadPage {
    type Reply = OpenedPage;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let url = session
            .current_page
            .as_ref()
            .ok_or(SessionError::NoPage)?
            .url
            .clone();
        session.open_page(url).map_err(SessionError::Load)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetPageUrl;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageUrl {
    pub url: String,
}

impl private::Sealed for GetPageUrl {}

impl SessionRequest for GetPageUrl {
    type Reply = PageUrl;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(PageUrl {
            url: page.url.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetPageTitle;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageTitle {
    pub title: String,
}

impl private::Sealed for GetPageTitle {}

impl SessionRequest for GetPageTitle {
    type Reply = PageTitle;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(PageTitle {
            title: page.title.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureInteractiveSnapshot;

impl private::Sealed for CaptureInteractiveSnapshot {}

impl SessionRequest for CaptureInteractiveSnapshot {
    type Reply = InteractiveSnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
        let snapshot = InteractiveSnapshot::from_document(
            snapshot_id,
            page.epoch,
            page.url.clone(),
            &page.interactive_elements,
        );
        session.latest_interactive_snapshot = Some(snapshot_id);
        Ok(snapshot)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClickElement {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickResult {
    Navigated {
        reference: InteractiveElementRef,
        page: OpenedPage,
    },
}

impl private::Sealed for ClickElement {}

impl SessionRequest for ClickElement {
    type Reply = ClickResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let action = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index]
            .action
            .clone();
        match action {
            InteractiveAction::Navigate { href } => {
                let current_url = session
                    .current_page
                    .as_ref()
                    .expect("validated reference requires a current page")
                    .url
                    .clone();
                let target = resolve_navigation_url(&current_url, &href).map_err(|error| {
                    SessionError::Navigation {
                        reference: self.reference,
                        error,
                    }
                })?;
                let page = session
                    .open_page(target)
                    .map_err(|error| SessionError::Navigation {
                        reference: self.reference,
                        error,
                    })?;
                Ok(ClickResult::Navigated {
                    reference: self.reference,
                    page,
                })
            }
            InteractiveAction::Unsupported { reason } => Err(SessionError::UnsupportedClick {
                reference: self.reference,
                reason,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillElement {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillResult {
    pub reference: InteractiveElementRef,
    pub value: String,
}

impl private::Sealed for FillElement {}

impl SessionRequest for FillElement {
    type Reply = FillResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page");
        let element = &mut page.interactive_elements[index];
        match &mut element.control_state {
            ControlState::Text(TextValueState::Editable { value }) => {
                *value = self.value;
                Ok(FillResult {
                    reference: self.reference,
                    value: value.clone(),
                })
            }
            ControlState::Text(TextValueState::NonEditable { reason, .. }) => {
                Err(SessionError::UnsupportedFill {
                    reference: self.reference,
                    reason: reason.clone(),
                })
            }
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Unavailable => Err(SessionError::UnsupportedFill {
                reference: self.reference,
                reason: format!(
                    "fill execution for role {} is not implemented",
                    element.role()
                ),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementValue {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementValue {
    pub reference: InteractiveElementRef,
    pub value: String,
}

impl private::Sealed for GetElementValue {}

impl SessionRequest for GetElementValue {
    type Reply = ElementValue;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match &element.control_state {
            ControlState::Text(
                TextValueState::Editable { value } | TextValueState::NonEditable { value, .. },
            ) => Ok(ElementValue {
                reference: self.reference,
                value: value.clone(),
            }),
            ControlState::Text(TextValueState::Unavailable)
            | ControlState::Checkbox(_)
            | ControlState::Unavailable => Err(SessionError::UnsupportedValue {
                reference: self.reference,
                reason: format!(
                    "value inspection for role {} is not implemented",
                    element.role()
                ),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementText {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementText {
    pub reference: InteractiveElementRef,
    pub text: String,
}

impl private::Sealed for GetElementText {}

impl SessionRequest for GetElementText {
    type Reply = ElementText;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        Ok(ElementText {
            reference: self.reference,
            text: element.text().into(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetElementAttribute {
    pub reference: InteractiveElementRef,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementAttribute {
    pub reference: InteractiveElementRef,
    pub name: String,
    pub value: Option<String>,
}

impl private::Sealed for GetElementAttribute {}

impl SessionRequest for GetElementAttribute {
    type Reply = ElementAttribute;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        if self.name.is_empty() || self.name.chars().any(char::is_whitespace) {
            return Err(SessionError::InvalidAttributeName { name: self.name });
        }
        let name = self.name.to_ascii_lowercase();
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        if element.attribute_is_sensitive(&name) {
            return Err(SessionError::SensitiveAttribute {
                reference: self.reference,
                name,
            });
        }
        Ok(ElementAttribute {
            reference: self.reference,
            value: element.attribute(&name).map(str::to_owned),
            name,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementEnabled {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementEnabled {
    pub reference: InteractiveElementRef,
    pub enabled: bool,
}

impl private::Sealed for GetElementEnabled {}

impl SessionRequest for GetElementEnabled {
    type Reply = ElementEnabled;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        let enabled = element
            .enabled()
            .ok_or(SessionError::UnsupportedEnabledState {
                reference: self.reference,
                reason: format!(
                    "enabled-state inspection for role {} is not implemented",
                    element.role()
                ),
            })?;
        Ok(ElementEnabled {
            reference: self.reference,
            enabled,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetElementChecked {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedResult {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

impl private::Sealed for SetElementChecked {}

impl SessionRequest for SetElementChecked {
    type Reply = SetCheckedResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &mut session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match &mut element.control_state {
            ControlState::Checkbox(CheckedState::Editable { checked }) => {
                *checked = self.checked;
                Ok(SetCheckedResult {
                    reference: self.reference,
                    checked: *checked,
                })
            }
            ControlState::Checkbox(CheckedState::NonEditable { reason, .. }) => {
                Err(SessionError::UnsupportedCheck {
                    reference: self.reference,
                    reason: reason.clone(),
                })
            }
            ControlState::Text(_) | ControlState::Unavailable => {
                Err(SessionError::UnsupportedCheck {
                    reference: self.reference,
                    reason: format!(
                        "checked-state mutation for role {} is not implemented",
                        element.role()
                    ),
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementChecked {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementChecked {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

impl private::Sealed for GetElementChecked {}

impl SessionRequest for GetElementChecked {
    type Reply = ElementChecked;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match element.control_state {
            ControlState::Checkbox(
                CheckedState::Editable { checked } | CheckedState::NonEditable { checked, .. },
            ) => Ok(ElementChecked {
                reference: self.reference,
                checked,
            }),
            ControlState::Text(_) | ControlState::Unavailable => {
                Err(SessionError::UnsupportedCheckedState {
                    reference: self.reference,
                    reason: format!(
                        "checked-state inspection for role {} is not implemented",
                        element.role()
                    ),
                })
            }
        }
    }
}

fn resolve_navigation_url(base: &str, href: &str) -> Result<String, LoadError> {
    if href.contains('#') {
        return Err(LoadError::UnsupportedTarget(
            "link fragments are not implemented".into(),
        ));
    }
    let base = base
        .parse::<Uri>()
        .map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    if let Ok(absolute) = href.parse::<Uri>()
        && absolute.scheme().is_some()
    {
        return Ok(absolute.to_string());
    }
    let scheme = base
        .scheme_str()
        .ok_or_else(|| LoadError::InvalidUrl("the base URL has no scheme".into()))?;
    if href.starts_with("//") {
        return Ok(format!("{scheme}:{href}"));
    }
    let authority = base
        .authority()
        .ok_or_else(|| LoadError::InvalidUrl("the base URL has no authority".into()))?;
    let base_path = base
        .path_and_query()
        .map(|value| value.path())
        .unwrap_or("/");
    let base_path_and_query = base.path_and_query().map_or("/", |value| value.as_str());
    let path_and_query = resolve_path_and_query(base_path, base_path_and_query, href);
    Ok(format!("{scheme}://{authority}{path_and_query}"))
}

fn resolve_path_and_query(base_path: &str, base_path_and_query: &str, href: &str) -> String {
    if href.is_empty() {
        return base_path_and_query.into();
    }
    if href.starts_with('?') {
        return format!("{base_path}{href}");
    }
    let (href_path, query) = href
        .split_once('?')
        .map_or((href, None), |(path, query)| (path, Some(query)));
    let joined = if href_path.starts_with('/') {
        href_path.into()
    } else {
        let directory_end = base_path.rfind('/').map_or(0, |index| index + 1);
        format!("{}{href_path}", &base_path[..directory_end])
    };
    let mut result = normalize_path(&joined);
    if let Some(query) = query {
        result.push('?');
        result.push_str(query);
    }
    result
}

fn normalize_path(path: &str) -> String {
    let keep_trailing_slash = path.ends_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            value => segments.push(value),
        }
    }
    let mut normalized = format!("/{}", segments.join("/"));
    if keep_trailing_slash && normalized != "/" {
        normalized.push('/');
    }
    normalized
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LintLayout {
    pub input: LayoutInput,
}

impl private::Sealed for LintLayout {}

impl SessionRequest for LintLayout {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.clean_layout(self.input)?;
        let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
        let snapshot = Snapshot::from_layout(snapshot_id, layout);
        let result = evaluate_horizontal_overflow(&snapshot);
        session.last_snapshot = Some(snapshot);
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckElementWidth {
    pub element: String,
    pub maximum_width: u64,
}

impl private::Sealed for CheckElementWidth {}

impl SessionRequest for CheckElementWidth {
    type Reply = RuleResult<WidthFinding>;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let snapshot = session
            .last_snapshot
            .as_ref()
            .ok_or(SessionError::NoSnapshot)?;
        Ok(evaluate_max_element_width(
            snapshot,
            &self.element,
            self.maximum_width,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyMutation {
    pub mutation: LayoutMutation,
}

impl private::Sealed for ApplyMutation {}

impl SessionRequest for ApplyMutation {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.apply_mutation(self.mutation)?;
        let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
        let snapshot = Snapshot::from_layout(snapshot_id, layout);
        let result = evaluate_horizontal_overflow(&snapshot);
        session.last_snapshot = Some(snapshot);
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    Load(LoadError),
    Navigation {
        reference: InteractiveElementRef,
        error: LoadError,
    },
    Layout(LayoutError),
    NoPage,
    NoSnapshot,
    StaleElementReference {
        reference: InteractiveElementRef,
    },
    UnsupportedClick {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedFill {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedValue {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedCheck {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedCheckedState {
        reference: InteractiveElementRef,
        reason: String,
    },
    InvalidAttributeName {
        name: String,
    },
    SensitiveAttribute {
        reference: InteractiveElementRef,
        name: String,
    },
    UnsupportedEnabledState {
        reference: InteractiveElementRef,
        reason: String,
    },
}

impl From<LoadError> for SessionError {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<LayoutError> for SessionError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_navigation_url;
    use crate::LoadError;

    #[test]
    fn resolves_relative_paths_and_queries() {
        let base = "http://localhost:3000/guide/current?old=1";

        assert_eq!(
            resolve_navigation_url(base, "../next?q=1").unwrap(),
            "http://localhost:3000/next?q=1"
        );
        assert_eq!(
            resolve_navigation_url(base, "child").unwrap(),
            "http://localhost:3000/guide/child"
        );
        assert_eq!(
            resolve_navigation_url(base, "?new=1").unwrap(),
            "http://localhost:3000/guide/current?new=1"
        );
        assert_eq!(resolve_navigation_url(base, "").unwrap(), base);
    }

    #[test]
    fn preserves_absolute_and_network_targets_for_loader_policy() {
        let base = "http://localhost:3000/current";

        assert_eq!(
            resolve_navigation_url(base, "http://example.com/away").unwrap(),
            "http://example.com/away"
        );
        assert_eq!(
            resolve_navigation_url(base, "//example.com/away").unwrap(),
            "http://example.com/away"
        );
    }

    #[test]
    fn rejects_fragments_until_same_document_navigation_exists() {
        let result = resolve_navigation_url("http://localhost:3000/current", "#details");

        assert!(matches!(result, Err(LoadError::UnsupportedTarget(_))));
    }
}
