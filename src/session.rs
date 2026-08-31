use crate::layout::{
    LayoutError, LayoutInput, LayoutKernel, LayoutMutation, LayoutProgram, LayoutSnapshot,
};
use crate::loading::{LoadError, load_local_html};
use crate::locator::{Locator, LocatorMatch, LocatorPosition, RoleLocator, RoleMatch};
use crate::non_empty::NonEmpty;
use crate::page::{
    CheckedState, ControlState, InteractiveAction, InteractiveElementSource, LocatorElementSource,
    SelectValueError, SelectorIndex, SelectorQueryError, TextValueState, page_semantics_from_html,
};
use crate::rules::{
    RuleResult, WidthFinding, evaluate_horizontal_overflow, evaluate_max_element_width,
};
use crate::selection::SelectOptionTarget;
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
    latest_interactive_snapshot: Option<LatestInteractiveSnapshot>,
    current_page: Option<CurrentPage>,
}

#[derive(Debug)]
struct LatestInteractiveSnapshot {
    id: SnapshotId,
    element_indices: Vec<usize>,
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
    locator_elements: Vec<LocatorElementSource>,
    interactive_elements: Vec<InteractiveElementSource>,
    selector_index: SelectorIndex,
}

#[derive(Debug)]
struct ResolvedLocator {
    matched: LocatorMatch,
    source_index: usize,
    interactive_index: Option<usize>,
}

#[derive(Debug)]
enum LocatorOperationError {
    NoPage,
    NotFound,
    Ambiguous {
        match_count: usize,
    },
    Query {
        reason: String,
    },
    InspectionBlocked {
        inspection: LocatorInspection,
        reason: String,
    },
    SensitiveAttribute {
        name: String,
    },
    SelectOptionNotFound {
        target: SelectOptionTarget,
    },
    SelectOptionDisabled {
        target: SelectOptionTarget,
    },
    Navigation(LoadError),
    ActionBlocked {
        action: LocatorAction,
        check: ActionabilityCheck,
        reason: String,
    },
    UnsupportedAction {
        action: LocatorAction,
        reason: String,
    },
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
            locator_elements: semantics.locator_elements,
            interactive_elements: semantics.interactive_elements,
            selector_index: semantics.selector_index,
        });
        Ok(reply)
    }

    fn element_index_for(&self, reference: InteractiveElementRef) -> Result<usize, SessionError> {
        let page = self.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let Some(snapshot) = &self.latest_interactive_snapshot else {
            return Err(SessionError::StaleElementReference { reference });
        };
        if reference.document_epoch() != page.epoch || snapshot.id != reference.snapshot() {
            return Err(SessionError::StaleElementReference { reference });
        }
        let ordinal = reference
            .ordinal()
            .checked_sub(1)
            .and_then(|ordinal| usize::try_from(ordinal).ok())
            .expect("interactive snapshot references use nonzero usize ordinals");
        snapshot
            .element_indices
            .get(ordinal)
            .copied()
            .filter(|index| page.interactive_elements.get(*index).is_some())
            .ok_or(SessionError::StaleElementReference { reference })
    }

    fn locator_matches_for(
        &self,
        locator: &Locator,
    ) -> Result<Vec<ResolvedLocator>, LocatorOperationError> {
        let page = self
            .current_page
            .as_ref()
            .ok_or(LocatorOperationError::NoPage)?;
        let mut matches = if let Some(css) = locator.css() {
            page.selector_index.css_matches(css.selector())?
        } else if let Some(xpath) = locator.xpath() {
            page.selector_index.xpath_matches(xpath.expression())?
        } else {
            page.locator_elements
                .iter()
                .enumerate()
                .filter_map(|(index, element)| element.matches(locator).then_some(index))
                .collect::<Vec<_>>()
        };
        if locator.uses_descendant_text() {
            let candidates = matches.clone();
            matches.retain(|candidate| {
                !candidates.iter().any(|other| {
                    other != candidate
                        && locator_element_is_descendant(&page.locator_elements, *other, *candidate)
                })
            });
        }
        if let Some(position) = locator.position() {
            let selected = match position {
                LocatorPosition::First => matches.first().copied(),
                LocatorPosition::Last => matches.last().copied(),
                LocatorPosition::Nth(index) => matches.get(index).copied(),
            };
            matches.clear();
            matches.extend(selected);
        }
        Ok(matches
            .into_iter()
            .map(|index| {
                let element = &page.locator_elements[index];
                ResolvedLocator {
                    matched: LocatorMatch::new(
                        &element.element,
                        element.role(),
                        element.name(),
                        element.text(),
                    ),
                    source_index: index,
                    interactive_index: element.interactive_index,
                }
            })
            .collect())
    }

    fn locator_match_for(
        &self,
        locator: &Locator,
    ) -> Result<ResolvedLocator, LocatorOperationError> {
        let mut matches = self.locator_matches_for(locator)?;
        if matches.is_empty() {
            return Err(LocatorOperationError::NotFound);
        }
        if matches.len() > 1 {
            return Err(LocatorOperationError::Ambiguous {
                match_count: matches.len(),
            });
        }
        Ok(matches.pop().expect("one locator match remains"))
    }

    fn locator_interactive_index(
        &self,
        resolved: &ResolvedLocator,
        action: LocatorAction,
    ) -> Result<usize, LocatorOperationError> {
        resolved
            .interactive_index
            .ok_or_else(|| LocatorOperationError::UnsupportedAction {
                action,
                reason: resolved.matched.role.as_ref().map_or_else(
                    || "matched element has no implemented interactive behavior".into(),
                    |role| format!("role {role} has no implemented interactive behavior"),
                ),
            })
    }
}

fn locator_element_is_descendant(
    elements: &[LocatorElementSource],
    mut candidate: usize,
    ancestor: usize,
) -> bool {
    while let Some(parent) = elements[candidate].parent {
        if parent == ancestor {
            return true;
        }
        candidate = parent;
    }
    false
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
        let element_indices = (0..session
            .current_page
            .as_ref()
            .ok_or(SessionError::NoPage)?
            .interactive_elements
            .len())
            .collect();
        Ok(capture_interactive_snapshot(session, element_indices))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureInteractiveSnapshotWithin {
    pub locator: Locator,
}

impl private::Sealed for CaptureInteractiveSnapshotWithin {}

impl SessionRequest for CaptureInteractiveSnapshotWithin {
    type Reply = InteractiveSnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = self.locator;
        let resolved = match session.locator_match_for(&locator) {
            Ok(resolved) => resolved,
            Err(error) => return Err(locator_session_error(locator, error)),
        };
        let page = session
            .current_page
            .as_ref()
            .expect("resolved locator requires a current page");
        let element_indices = page
            .locator_elements
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                *index == resolved.source_index
                    || locator_element_is_descendant(
                        &page.locator_elements,
                        *index,
                        resolved.source_index,
                    )
            })
            .filter_map(|(_, element)| element.interactive_index)
            .collect();
        Ok(capture_interactive_snapshot(session, element_indices))
    }
}

fn capture_interactive_snapshot(
    session: &mut Session,
    element_indices: Vec<usize>,
) -> InteractiveSnapshot {
    let page = session
        .current_page
        .as_ref()
        .expect("snapshot capture requires a current page");
    let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
    let snapshot = InteractiveSnapshot::from_document_indices(
        snapshot_id,
        page.epoch,
        page.url.clone(),
        &page.interactive_elements,
        &element_indices,
    );
    session.latest_interactive_snapshot = Some(LatestInteractiveSnapshot {
        id: snapshot_id,
        element_indices,
    });
    snapshot
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindByRole {
    pub locator: RoleLocator,
}

impl private::Sealed for FindByRole {}

impl SessionRequest for FindByRole {
    type Reply = RoleMatch;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match session.locator_match_for(&locator) {
            Ok(resolved) => Ok(resolved.matched.into_role_match()),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorAction {
    Click,
    Fill,
    Select,
    Check,
    Uncheck,
    Hover,
}

pub type RoleAction = LocatorAction;

impl std::fmt::Display for LocatorAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Click => "click",
            Self::Fill => "fill",
            Self::Select => "select",
            Self::Check => "check",
            Self::Uncheck => "uncheck",
            Self::Hover => "hover",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionabilityCheck {
    Visible,
    Enabled,
    Editable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorInspection {
    Html,
    Value,
    Checked,
    Enabled,
    Visible,
}

impl std::fmt::Display for LocatorInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Html => "HTML",
            Self::Value => "value",
            Self::Checked => "checked state",
            Self::Enabled => "enabled state",
            Self::Visible => "visibility",
        })
    }
}

impl std::fmt::Display for ActionabilityCheck {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Visible => "visible",
            Self::Enabled => "enabled",
            Self::Editable => "editable",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindAllByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorMatches {
    pub matches: Vec<LocatorMatch>,
}

impl private::Sealed for FindAllByLocator {}

impl SessionRequest for FindAllByLocator {
    type Reply = LocatorMatches;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_matches_for(&self.locator) {
            Ok(matches) => Ok(LocatorMatches {
                matches: matches
                    .into_iter()
                    .map(|resolved| resolved.matched)
                    .collect(),
            }),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountByLocator {
    pub locator: Locator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorCount {
    pub count: usize,
}

impl private::Sealed for CountByLocator {}

impl SessionRequest for CountByLocator {
    type Reply = LocatorCount;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_matches_for(&self.locator) {
            Ok(matches) => Ok(LocatorCount {
                count: matches.len(),
            }),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetHtmlByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorHtml {
    pub matched: LocatorMatch,
    pub html: String,
}

impl private::Sealed for GetHtmlByLocator {}

impl SessionRequest for GetHtmlByLocator {
    type Reply = LocatorHtml;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_html_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetValueByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorValue {
    pub matched: LocatorMatch,
    pub value: String,
}

impl private::Sealed for GetValueByLocator {}

impl SessionRequest for GetValueByLocator {
    type Reply = LocatorValue;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_value_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAttributeByLocator {
    pub locator: Locator,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorAttribute {
    pub matched: LocatorMatch,
    pub name: String,
    pub value: Option<String>,
}

impl private::Sealed for GetAttributeByLocator {}

impl SessionRequest for GetAttributeByLocator {
    type Reply = LocatorAttribute;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let name = normalize_attribute_name(self.name)?;
        match execute_get_attribute_by_locator(session, &self.locator, name) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetCheckedByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorChecked {
    pub matched: LocatorMatch,
    pub checked: bool,
}

impl private::Sealed for GetCheckedByLocator {}

impl SessionRequest for GetCheckedByLocator {
    type Reply = LocatorChecked;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_checked_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetEnabledByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorEnabled {
    pub matched: LocatorMatch,
    pub enabled: bool,
}

impl private::Sealed for GetEnabledByLocator {}

impl SessionRequest for GetEnabledByLocator {
    type Reply = LocatorEnabled;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_enabled_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetVisibleByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorVisible {
    pub matched: LocatorMatch,
    pub visible: bool,
}

impl private::Sealed for GetVisibleByLocator {}

impl SessionRequest for GetVisibleByLocator {
    type Reply = LocatorVisible;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_visible_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for FindByLocator {}

impl SessionRequest for FindByLocator {
    type Reply = LocatorMatch;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_match_for(&self.locator) {
            Ok(resolved) => Ok(resolved.matched),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickByLocatorResult {
    Navigated {
        matched: LocatorMatch,
        page: OpenedPage,
    },
}

impl private::Sealed for ClickByLocator {}

impl SessionRequest for ClickByLocator {
    type Reply = ClickByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_click_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByLocator {
    pub locator: Locator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByLocatorResult {
    pub matched: LocatorMatch,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectByLocator {
    pub locator: Locator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectByLocatorResult {
    pub matched: LocatorMatch,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsByLocator {
    pub locator: Locator,
    pub options: NonEmpty<SelectOptionTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsByLocatorResult {
    pub matched: LocatorMatch,
    pub selected: NonEmpty<String>,
}

impl private::Sealed for SelectByLocator {}

impl SessionRequest for SelectByLocator {
    type Reply = SelectByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_select_by_locator(session, &self.locator, self.value) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for SelectOptionsByLocator {}

impl SessionRequest for SelectOptionsByLocator {
    type Reply = SelectOptionsByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_select_options_by_locator(session, &self.locator, self.options) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for FillByLocator {}

impl SessionRequest for FillByLocator {
    type Reply = FillByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_fill_by_locator(session, &self.locator, self.value) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByLocator {
    pub locator: Locator,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByLocatorResult {
    pub matched: LocatorMatch,
    pub checked: bool,
}

impl private::Sealed for SetCheckedByLocator {}

impl SessionRequest for SetCheckedByLocator {
    type Reply = SetCheckedByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_set_checked_by_locator(session, &self.locator, self.checked) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByLocatorResult {
    pub matched: LocatorMatch,
}

impl private::Sealed for HoverByLocator {}

impl SessionRequest for HoverByLocator {
    type Reply = HoverByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_hover_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickByRole {
    pub locator: RoleLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickByRoleResult {
    Navigated {
        matched: RoleMatch,
        page: OpenedPage,
    },
}

impl private::Sealed for ClickByRole {}

impl SessionRequest for ClickByRole {
    type Reply = ClickByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_click_by_locator(session, &locator) {
            Ok(ClickByLocatorResult::Navigated { matched, page }) => {
                Ok(ClickByRoleResult::Navigated {
                    matched: matched.into_role_match(),
                    page,
                })
            }
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByRole {
    pub locator: RoleLocator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByRoleResult {
    pub matched: RoleMatch,
    pub value: String,
}

impl private::Sealed for FillByRole {}

impl SessionRequest for FillByRole {
    type Reply = FillByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_fill_by_locator(session, &locator, self.value) {
            Ok(result) => Ok(FillByRoleResult {
                matched: result.matched.into_role_match(),
                value: result.value,
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByRole {
    pub locator: RoleLocator,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByRoleResult {
    pub matched: RoleMatch,
    pub checked: bool,
}

impl private::Sealed for SetCheckedByRole {}

impl SessionRequest for SetCheckedByRole {
    type Reply = SetCheckedByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_set_checked_by_locator(session, &locator, self.checked) {
            Ok(result) => Ok(SetCheckedByRoleResult {
                matched: result.matched.into_role_match(),
                checked: result.checked,
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByRole {
    pub locator: RoleLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByRoleResult {
    pub matched: RoleMatch,
}

impl private::Sealed for HoverByRole {}

impl SessionRequest for HoverByRole {
    type Reply = HoverByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_hover_by_locator(session, &locator) {
            Ok(result) => Ok(HoverByRoleResult {
                matched: result.matched.into_role_match(),
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

fn execute_get_value_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorValue, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = resolved.interactive_index.ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Value,
            "matched element has no implemented value state",
        )
    })?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    let value = element.value().ok_or_else(|| {
        let reason = format!(
            "value inspection for role {} is not implemented",
            element.role()
        );
        LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Value,
            reason,
        }
    })?;
    Ok(LocatorValue {
        matched: resolved.matched,
        value: value.into(),
    })
}

fn execute_get_html_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorHtml, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let page = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page");
    if page
        .selector_index
        .inner_html_contains_sensitive_value(resolved.source_index)?
    {
        return Err(LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Html,
            reason: "inner HTML contains a password value attribute".into(),
        });
    }
    let html = page.selector_index.inner_html(resolved.source_index)?;
    Ok(LocatorHtml {
        matched: resolved.matched,
        html,
    })
}

fn execute_get_attribute_by_locator(
    session: &Session,
    locator: &Locator,
    name: String,
) -> Result<LocatorAttribute, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    if element.attribute_is_sensitive(&name) {
        return Err(LocatorOperationError::SensitiveAttribute { name });
    }
    Ok(LocatorAttribute {
        matched: resolved.matched,
        value: element.attribute(&name).map(str::to_owned),
        name,
    })
}

fn execute_get_checked_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorChecked, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = resolved.interactive_index.ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Checked,
            "matched element has no implemented checked state",
        )
    })?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    let checked = element.checked().ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Checked,
            &format!(
                "checked-state inspection for role {} is not implemented",
                element.role()
            ),
        )
    })?;
    Ok(LocatorChecked {
        matched: resolved.matched,
        checked,
    })
}

fn execute_get_enabled_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorEnabled, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    let enabled = element.enabled().ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Enabled,
            "matched element has no implemented native enabled state",
        )
    })?;
    Ok(LocatorEnabled {
        matched: resolved.matched,
        enabled,
    })
}

fn execute_get_visible_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorVisible, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    let visible = element
        .visible()
        .map_err(|reason| LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Visible,
            reason: reason.into(),
        })?;
    Ok(LocatorVisible {
        matched: resolved.matched,
        visible,
    })
}

fn unsupported_locator_inspection(
    resolved: &ResolvedLocator,
    inspection: LocatorInspection,
    fallback: &str,
) -> LocatorOperationError {
    let reason = resolved.matched.role.as_ref().map_or_else(
        || fallback.into(),
        |role| format!("{inspection} inspection for role {role} is not implemented"),
    );
    LocatorOperationError::InspectionBlocked { inspection, reason }
}

fn execute_click_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<ClickByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Click)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    require_locator_visible(element, LocatorAction::Click)?;
    require_locator_enabled(element, LocatorAction::Click)?;
    let action = element.action.clone();
    match action {
        InteractiveAction::Navigate { href } => {
            let current_url = session
                .current_page
                .as_ref()
                .expect("resolved locator requires a current page")
                .url
                .clone();
            let target = resolve_navigation_url(&current_url, &href)
                .map_err(LocatorOperationError::Navigation)?;
            let page = session
                .open_page(target)
                .map_err(LocatorOperationError::Navigation)?;
            Ok(ClickByLocatorResult::Navigated {
                matched: resolved.matched,
                page,
            })
        }
        InteractiveAction::Unsupported { reason } => {
            Err(LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Click,
                reason,
            })
        }
    }
}

fn execute_fill_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: String,
) -> Result<FillByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Fill)?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &mut page.interactive_elements[index];
    require_locator_visible(element, LocatorAction::Fill)?;
    match &mut element.control_state {
        ControlState::Text(TextValueState::Editable { value }) => {
            *value = replacement;
            Ok(FillByLocatorResult {
                matched: resolved.matched,
                value: value.clone(),
            })
        }
        ControlState::Text(TextValueState::NonEditable { reason, .. }) => {
            Err(LocatorOperationError::ActionBlocked {
                action: LocatorAction::Fill,
                check: ActionabilityCheck::Editable,
                reason: reason.clone(),
            })
        }
        ControlState::Text(TextValueState::Unavailable)
        | ControlState::Checkbox(_)
        | ControlState::Select(_)
        | ControlState::Unavailable => Err(LocatorOperationError::UnsupportedAction {
            action: LocatorAction::Fill,
            reason: format!(
                "fill execution for role {} is not implemented",
                element.role()
            ),
        }),
    }
}

fn execute_select_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: String,
) -> Result<SelectByLocatorResult, LocatorOperationError> {
    let result = execute_select_options_by_locator(
        session,
        locator,
        NonEmpty::one(SelectOptionTarget::Value(replacement)),
    )?;
    Ok(SelectByLocatorResult {
        matched: result.matched,
        value: result.selected[0].clone(),
    })
}

fn execute_select_options_by_locator(
    session: &mut Session,
    locator: &Locator,
    options: NonEmpty<SelectOptionTarget>,
) -> Result<SelectOptionsByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Select)?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &mut page.interactive_elements[index];
    require_locator_visible(element, LocatorAction::Select)?;
    match element.select_options(&options) {
        Ok(selected) => Ok(SelectOptionsByLocatorResult {
            matched: resolved.matched,
            selected,
        }),
        Err(SelectValueError::Blocked { reason }) => Err(LocatorOperationError::ActionBlocked {
            action: LocatorAction::Select,
            check: ActionabilityCheck::Enabled,
            reason,
        }),
        Err(SelectValueError::Unsupported { reason }) => {
            Err(LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Select,
                reason,
            })
        }
        Err(SelectValueError::OptionNotFound { target }) => {
            Err(LocatorOperationError::SelectOptionNotFound { target })
        }
        Err(SelectValueError::OptionDisabled { target }) => {
            Err(LocatorOperationError::SelectOptionDisabled { target })
        }
    }
}

fn execute_set_checked_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: bool,
) -> Result<SetCheckedByLocatorResult, LocatorOperationError> {
    let action = if replacement {
        LocatorAction::Check
    } else {
        LocatorAction::Uncheck
    };
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, action)?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &mut page.interactive_elements[index];
    require_locator_visible(element, action)?;
    match &mut element.control_state {
        ControlState::Checkbox(CheckedState::Editable { checked }) => {
            *checked = replacement;
            Ok(SetCheckedByLocatorResult {
                matched: resolved.matched,
                checked: *checked,
            })
        }
        ControlState::Checkbox(CheckedState::NonEditable { reason, .. }) => {
            Err(LocatorOperationError::ActionBlocked {
                action,
                check: ActionabilityCheck::Enabled,
                reason: reason.clone(),
            })
        }
        ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => {
            Err(LocatorOperationError::UnsupportedAction {
                action,
                reason: format!(
                    "checked-state mutation for role {} is not implemented",
                    element.role()
                ),
            })
        }
    }
}

fn execute_hover_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<HoverByLocatorResult, LocatorOperationError> {
    session.locator_match_for(locator)?;
    Err(LocatorOperationError::UnsupportedAction {
        action: LocatorAction::Hover,
        reason: "hover state and pointer event dispatch are not implemented".into(),
    })
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
            | ControlState::Select(_)
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectElement {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectResult {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptions {
    pub reference: InteractiveElementRef,
    pub options: NonEmpty<SelectOptionTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsResult {
    pub reference: InteractiveElementRef,
    pub selected: NonEmpty<String>,
}

impl private::Sealed for SelectElement {}

impl SessionRequest for SelectElement {
    type Reply = SelectResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &mut session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match element.select_value(&self.value) {
            Ok(value) => Ok(SelectResult {
                reference: self.reference,
                value: value.into(),
            }),
            Err(
                SelectValueError::Blocked { reason } | SelectValueError::Unsupported { reason },
            ) => Err(SessionError::UnsupportedSelect {
                reference: self.reference,
                reason,
            }),
            Err(SelectValueError::OptionNotFound { target }) => {
                Err(reference_option_not_found(self.reference, target))
            }
            Err(SelectValueError::OptionDisabled { target }) => {
                Err(reference_option_disabled(self.reference, target))
            }
        }
    }
}

impl private::Sealed for SelectOptions {}

impl SessionRequest for SelectOptions {
    type Reply = SelectOptionsResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &mut session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match element.select_options(&self.options) {
            Ok(selected) => Ok(SelectOptionsResult {
                reference: self.reference,
                selected,
            }),
            Err(
                SelectValueError::Blocked { reason } | SelectValueError::Unsupported { reason },
            ) => Err(SessionError::UnsupportedSelect {
                reference: self.reference,
                reason,
            }),
            Err(SelectValueError::OptionNotFound { target }) => {
                Err(reference_option_not_found(self.reference, target))
            }
            Err(SelectValueError::OptionDisabled { target }) => {
                Err(reference_option_disabled(self.reference, target))
            }
        }
    }
}

fn reference_option_not_found(
    reference: InteractiveElementRef,
    target: SelectOptionTarget,
) -> SessionError {
    match target {
        SelectOptionTarget::Value(value) => SessionError::SelectOptionNotFound { reference, value },
        target => SessionError::SelectOptionTargetNotFound { reference, target },
    }
}

fn reference_option_disabled(
    reference: InteractiveElementRef,
    target: SelectOptionTarget,
) -> SessionError {
    match target {
        SelectOptionTarget::Value(value) => SessionError::SelectOptionDisabled { reference, value },
        target => SessionError::SelectOptionTargetDisabled { reference, target },
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
        if let Some(value) = element.value() {
            return Ok(ElementValue {
                reference: self.reference,
                value: value.into(),
            });
        }
        let reason = format!(
            "value inspection for role {} is not implemented",
            element.role()
        );
        Err(SessionError::UnsupportedValue {
            reference: self.reference,
            reason,
        })
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementHtml {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementHtml {
    pub reference: InteractiveElementRef,
    pub html: String,
}

impl private::Sealed for GetElementHtml {}

impl SessionRequest for GetElementHtml {
    type Reply = ElementHtml;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page");
        let source_index = page
            .locator_elements
            .iter()
            .position(|element| element.interactive_index == Some(interactive_index))
            .expect("every interactive element has one locator source");
        let contains_sensitive_value = page
            .selector_index
            .inner_html_contains_sensitive_value(source_index)
            .map_err(|error| SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: error.to_string(),
            })?;
        if contains_sensitive_value {
            return Err(SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: "inner HTML contains a password value attribute".into(),
            });
        }
        let html = page
            .selector_index
            .inner_html(source_index)
            .map_err(|error| SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: error.to_string(),
            })?;
        Ok(ElementHtml {
            reference: self.reference,
            html,
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
        let name = normalize_attribute_name(self.name)?;
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

fn normalize_attribute_name(name: String) -> Result<String, SessionError> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(SessionError::InvalidAttributeName { name });
    }
    Ok(name.to_ascii_lowercase())
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
pub struct GetElementVisible {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementVisible {
    pub reference: InteractiveElementRef,
    pub visible: bool,
}

impl private::Sealed for GetElementVisible {}

impl SessionRequest for GetElementVisible {
    type Reply = ElementVisible;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        let visible = element
            .visible()
            .map_err(|reason| SessionError::UnsupportedVisibility {
                reference: self.reference,
                reason: reason.into(),
            })?;
        Ok(ElementVisible {
            reference: self.reference,
            visible,
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
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => {
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
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => {
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
        Ok(install_layout_result(session, layout))
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
        Ok(install_layout_result(session, layout))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyMutations {
    pub mutations: Vec<LayoutMutation>,
}

impl private::Sealed for ApplyMutations {}

impl SessionRequest for ApplyMutations {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.apply_mutations(self.mutations)?;
        Ok(install_layout_result(session, layout))
    }
}

fn install_layout_result(session: &mut Session, layout: LayoutSnapshot) -> RuleResult {
    let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
    let snapshot = Snapshot::from_layout(snapshot_id, layout);
    let result = evaluate_horizontal_overflow(&snapshot);
    session.last_snapshot = Some(snapshot);
    result
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
    RoleLocatorNotFound {
        locator: RoleLocator,
    },
    RoleLocatorAmbiguous {
        locator: RoleLocator,
        match_count: usize,
    },
    LocatorNotFound {
        locator: Locator,
    },
    LocatorAmbiguous {
        locator: Locator,
        match_count: usize,
    },
    LocatorQuery {
        locator: Locator,
        reason: String,
    },
    UnsupportedLocatorInspection {
        locator: Locator,
        inspection: LocatorInspection,
        reason: String,
    },
    SensitiveLocatorAttribute {
        locator: Locator,
        name: String,
    },
    LocatorSelectOptionNotFound {
        locator: Locator,
        value: String,
    },
    LocatorSelectOptionDisabled {
        locator: Locator,
        value: String,
    },
    LocatorSelectOptionTargetNotFound {
        locator: Locator,
        target: SelectOptionTarget,
    },
    LocatorSelectOptionTargetDisabled {
        locator: Locator,
        target: SelectOptionTarget,
    },
    RoleNavigation {
        locator: RoleLocator,
        error: LoadError,
    },
    LocatorNavigation {
        locator: Locator,
        error: LoadError,
    },
    RoleActionBlocked {
        locator: RoleLocator,
        action: RoleAction,
        check: ActionabilityCheck,
        reason: String,
    },
    LocatorActionBlocked {
        locator: Locator,
        action: LocatorAction,
        check: ActionabilityCheck,
        reason: String,
    },
    UnsupportedRoleAction {
        locator: RoleLocator,
        action: RoleAction,
        reason: String,
    },
    UnsupportedLocatorAction {
        locator: Locator,
        action: LocatorAction,
        reason: String,
    },
    UnsupportedClick {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedFill {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedSelect {
        reference: InteractiveElementRef,
        reason: String,
    },
    SelectOptionNotFound {
        reference: InteractiveElementRef,
        value: String,
    },
    SelectOptionDisabled {
        reference: InteractiveElementRef,
        value: String,
    },
    SelectOptionTargetNotFound {
        reference: InteractiveElementRef,
        target: SelectOptionTarget,
    },
    SelectOptionTargetDisabled {
        reference: InteractiveElementRef,
        target: SelectOptionTarget,
    },
    UnsupportedValue {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedHtml {
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
    UnsupportedVisibility {
        reference: InteractiveElementRef,
        reason: String,
    },
}

fn locator_session_error(locator: Locator, error: LocatorOperationError) -> SessionError {
    match error {
        LocatorOperationError::NoPage => SessionError::NoPage,
        LocatorOperationError::NotFound => SessionError::LocatorNotFound { locator },
        LocatorOperationError::Ambiguous { match_count } => SessionError::LocatorAmbiguous {
            locator,
            match_count,
        },
        LocatorOperationError::Query { reason } => SessionError::LocatorQuery { locator, reason },
        LocatorOperationError::InspectionBlocked { inspection, reason } => {
            SessionError::UnsupportedLocatorInspection {
                locator,
                inspection,
                reason,
            }
        }
        LocatorOperationError::SensitiveAttribute { name } => {
            SessionError::SensitiveLocatorAttribute { locator, name }
        }
        LocatorOperationError::SelectOptionNotFound { target } => match target {
            SelectOptionTarget::Value(value) => {
                SessionError::LocatorSelectOptionNotFound { locator, value }
            }
            target => SessionError::LocatorSelectOptionTargetNotFound { locator, target },
        },
        LocatorOperationError::SelectOptionDisabled { target } => match target {
            SelectOptionTarget::Value(value) => {
                SessionError::LocatorSelectOptionDisabled { locator, value }
            }
            target => SessionError::LocatorSelectOptionTargetDisabled { locator, target },
        },
        LocatorOperationError::Navigation(error) => {
            SessionError::LocatorNavigation { locator, error }
        }
        LocatorOperationError::ActionBlocked {
            action,
            check,
            reason,
        } => SessionError::LocatorActionBlocked {
            locator,
            action,
            check,
            reason,
        },
        LocatorOperationError::UnsupportedAction { action, reason } => {
            SessionError::UnsupportedLocatorAction {
                locator,
                action,
                reason,
            }
        }
    }
}

fn role_session_error(locator: RoleLocator, error: LocatorOperationError) -> SessionError {
    match error {
        LocatorOperationError::NoPage => SessionError::NoPage,
        LocatorOperationError::NotFound => SessionError::RoleLocatorNotFound { locator },
        LocatorOperationError::Ambiguous { match_count } => SessionError::RoleLocatorAmbiguous {
            locator,
            match_count,
        },
        LocatorOperationError::Query { .. } => {
            unreachable!("role locators do not execute document selector queries")
        }
        LocatorOperationError::InspectionBlocked { .. }
        | LocatorOperationError::SensitiveAttribute { .. }
        | LocatorOperationError::SelectOptionNotFound { .. }
        | LocatorOperationError::SelectOptionDisabled { .. } => {
            unreachable!("role requests do not execute generic locator reads or selection")
        }
        LocatorOperationError::Navigation(error) => SessionError::RoleNavigation { locator, error },
        LocatorOperationError::ActionBlocked {
            action,
            check,
            reason,
        } => SessionError::RoleActionBlocked {
            locator,
            action,
            check,
            reason,
        },
        LocatorOperationError::UnsupportedAction { action, reason } => {
            SessionError::UnsupportedRoleAction {
                locator,
                action,
                reason,
            }
        }
    }
}

impl From<SelectorQueryError> for LocatorOperationError {
    fn from(error: SelectorQueryError) -> Self {
        Self::Query {
            reason: error.to_string(),
        }
    }
}

fn require_locator_visible(
    element: &InteractiveElementSource,
    action: LocatorAction,
) -> Result<(), LocatorOperationError> {
    match element.visible() {
        Ok(true) => Ok(()),
        Ok(false) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Visible,
            reason: "element is hidden or has an empty box".into(),
        }),
        Err(reason) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Visible,
            reason: reason.into(),
        }),
    }
}

fn require_locator_enabled(
    element: &InteractiveElementSource,
    action: LocatorAction,
) -> Result<(), LocatorOperationError> {
    match element.enabled() {
        Some(true) => Ok(()),
        Some(false) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Enabled,
            reason: "element is disabled".into(),
        }),
        None => Err(LocatorOperationError::UnsupportedAction {
            action,
            reason: format!(
                "enabled-state evidence for role {} is not implemented",
                element.role()
            ),
        }),
    }
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
