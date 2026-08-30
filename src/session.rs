use crate::layout::{LayoutError, LayoutInput, LayoutKernel, LayoutMutation, LayoutProgram};
use crate::loading::{LoadError, load_local_html};
use crate::page::{InteractiveElementSource, interactive_elements_from_html};
use crate::rules::{
    RuleResult, WidthFinding, evaluate_horizontal_overflow, evaluate_max_element_width,
};
use crate::snapshot::{InteractiveSnapshot, Snapshot, SnapshotId};

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
    next_snapshot_id: u64,
    next_document_epoch: u64,
    last_snapshot: Option<Snapshot>,
    current_page: Option<CurrentPage>,
}

#[derive(Debug)]
struct CurrentPage {
    epoch: u64,
    url: String,
    interactive_elements: Vec<InteractiveElementSource>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            layout: LayoutKernel::new(LayoutProgram::initial()),
            next_snapshot_id: 1,
            next_document_epoch: 1,
            last_snapshot: None,
            current_page: None,
        }
    }

    pub fn execute<R>(&mut self, request: R) -> Result<R::Reply, SessionError>
    where
        R: SessionRequest,
    {
        request.execute(self)
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
        let html = load_local_html(&self.url)?;
        let interactive_elements = interactive_elements_from_html(&html);
        let epoch = session.next_document_epoch;
        session.next_document_epoch = session
            .next_document_epoch
            .checked_add(1)
            .expect("document epoch exhausted");
        let reply = OpenedPage {
            url: self.url.clone(),
            interactive_element_count: interactive_elements.len(),
        };
        session.layout = LayoutKernel::new(LayoutProgram::initial());
        session.last_snapshot = None;
        session.current_page = Some(CurrentPage {
            epoch,
            url: self.url,
            interactive_elements,
        });
        Ok(reply)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureInteractiveSnapshot;

impl private::Sealed for CaptureInteractiveSnapshot {}

impl SessionRequest for CaptureInteractiveSnapshot {
    type Reply = InteractiveSnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let snapshot_id = SnapshotId::next(&mut session.next_snapshot_id);
        Ok(InteractiveSnapshot::from_document(
            snapshot_id,
            page.epoch,
            page.url.clone(),
            &page.interactive_elements,
        ))
    }
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
        let snapshot_id = SnapshotId::next(&mut session.next_snapshot_id);
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
        let snapshot_id = SnapshotId::next(&mut session.next_snapshot_id);
        let snapshot = Snapshot::from_layout(snapshot_id, layout);
        let result = evaluate_horizontal_overflow(&snapshot);
        session.last_snapshot = Some(snapshot);
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    Load(LoadError),
    Layout(LayoutError),
    NoPage,
    NoSnapshot,
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
