use crate::layout::{LayoutObservation, LayoutSnapshot, SemanticElementId};
use crate::non_empty::NonEmpty;
use crate::page::InteractiveElementSource;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotId(u64);

impl SnapshotId {
    pub(crate) fn next(current: &mut u64) -> Self {
        let id = Self(*current);
        *current = current
            .checked_add(1)
            .expect("snapshot identifier exhausted");
        id
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRef {
    pub snapshot: SnapshotId,
    pub element: SemanticElementId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InteractiveElementRef {
    document_epoch: u64,
    snapshot: SnapshotId,
    ordinal: u64,
}

impl InteractiveElementRef {
    pub(crate) const fn new(document_epoch: u64, snapshot: SnapshotId, ordinal: u64) -> Self {
        Self {
            document_epoch,
            snapshot,
            ordinal,
        }
    }

    pub(crate) const fn document_epoch(self) -> u64 {
        self.document_epoch
    }

    pub const fn snapshot(self) -> SnapshotId {
        self.snapshot
    }

    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }
}

impl std::fmt::Display for InteractiveElementRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "@e{}", self.ordinal)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveElement {
    pub reference: InteractiveElementRef,
    pub element: String,
    pub role: String,
    pub name: String,
    pub state: InteractiveElementState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InteractiveElementState {
    Unavailable,
    Value(String),
    Checked(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveSnapshot {
    pub id: SnapshotId,
    pub url: String,
    pub elements: Vec<InteractiveElement>,
}

impl InteractiveSnapshot {
    pub(crate) fn from_document(
        id: SnapshotId,
        document_epoch: u64,
        url: String,
        elements: &[InteractiveElementSource],
    ) -> Self {
        let elements = elements
            .iter()
            .enumerate()
            .map(|(index, source)| InteractiveElement {
                reference: InteractiveElementRef::new(
                    document_epoch,
                    id,
                    u64::try_from(index + 1).expect("interactive element count exceeds u64"),
                ),
                element: source.element().into(),
                role: source.role().into(),
                name: source.name().into(),
                state: source
                    .value()
                    .map(|value| InteractiveElementState::Value(value.into()))
                    .or_else(|| source.checked().map(InteractiveElementState::Checked))
                    .unwrap_or(InteractiveElementState::Unavailable),
            })
            .collect();
        Self { id, url, elements }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Evidenced<T> {
    pub(crate) value: T,
    pub(crate) evidence: NonEmpty<EvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ObservationCell<T> {
    Available(Evidenced<T>),
    Unsupported {
        element: SemanticElementId,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Geometry {
    pub(crate) element: SemanticElementId,
    pub(crate) x: i64,
    pub(crate) width: u64,
    pub(crate) right: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) viewport_width: u64,
    pub(crate) geometry: Vec<ObservationCell<Geometry>>,
}

impl Snapshot {
    pub(crate) fn from_layout(id: SnapshotId, layout: LayoutSnapshot) -> Self {
        let geometry = layout
            .observations
            .into_iter()
            .map(|observation| match observation {
                LayoutObservation::Available(fragment) => {
                    let element = fragment.element;
                    ObservationCell::Available(Evidenced {
                        value: Geometry {
                            element: element.clone(),
                            x: fragment.x,
                            width: fragment.width,
                            right: fragment.right,
                        },
                        evidence: NonEmpty::one(EvidenceRef {
                            snapshot: id,
                            element,
                        }),
                    })
                }
                LayoutObservation::Unsupported { element, reason } => {
                    ObservationCell::Unsupported { element, reason }
                }
            })
            .collect();

        Self {
            viewport_width: layout.viewport_width,
            geometry,
        }
    }
}
