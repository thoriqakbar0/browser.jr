use crate::layout::{LayoutObservation, LayoutSnapshot, SemanticElementId};
use crate::non_empty::NonEmpty;

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
