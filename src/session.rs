use crate::layout::{LayoutError, LayoutInput, LayoutKernel, LayoutMutation, LayoutProgram};
use crate::rules::{RuleResult, evaluate_horizontal_overflow};
use crate::snapshot::{Snapshot, SnapshotId};

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
    last_snapshot: Option<Snapshot>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            layout: LayoutKernel::new(LayoutProgram::initial()),
            next_snapshot_id: 1,
            last_snapshot: None,
        }
    }

    pub fn execute<R>(&mut self, request: R) -> Result<R::Reply, SessionError>
    where
        R: SessionRequest,
    {
        request.execute(self)
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
    Layout(LayoutError),
}

impl From<LayoutError> for SessionError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}
