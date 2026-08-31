use crate::layout::{LayoutObservation, LayoutSnapshot, SemanticElementId};
use crate::loading::resolve_url_reference;
use crate::non_empty::NonEmpty;
use crate::page::{AccessibilityNodeSource, InteractiveElementSource};

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
    pub source: InteractiveElementSourceInfo,
    pub role: String,
    pub name: String,
    pub state: InteractiveElementState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveElementSourceInfo {
    pub element: String,
    pub target_url: Option<String>,
    pub depth: u64,
}

impl InteractiveElement {
    pub fn element(&self) -> &str {
        &self.source.element
    }

    pub fn target_url(&self) -> Option<&str> {
        self.source.target_url.as_deref()
    }

    pub const fn depth(&self) -> u64 {
        self.source.depth
    }
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
    pub(crate) fn from_document_indices(
        identity: SnapshotCaptureIdentity,
        elements: &[InteractiveElementSource],
        element_indices: &[usize],
        element_depths: &[u64],
    ) -> Self {
        assert_eq!(
            element_indices.len(),
            element_depths.len(),
            "snapshot reference indices and depths must stay aligned"
        );
        let elements = element_indices
            .iter()
            .zip(element_depths)
            .map(|(index, depth)| {
                interactive_snapshot_element(&identity, &elements[*index], *index, *depth)
            })
            .collect();
        Self {
            id: identity.id,
            url: identity.url,
            elements,
        }
    }
}

fn interactive_snapshot_element(
    identity: &SnapshotCaptureIdentity,
    source: &InteractiveElementSource,
    index: usize,
    depth: u64,
) -> InteractiveElement {
    InteractiveElement {
        reference: InteractiveElementRef::new(
            identity.document_epoch,
            identity.id,
            u64::try_from(index + 1).expect("interactive element count exceeds u64"),
        ),
        source: InteractiveElementSourceInfo {
            element: source.element().into(),
            target_url: (source.role() == "link")
                .then(|| source.attribute("href"))
                .flatten()
                .and_then(|href| resolve_url_reference(&identity.url, href).ok()),
            depth,
        },
        role: source.role().into(),
        name: source.name().into(),
        state: snapshot_element_state(source).unwrap_or(InteractiveElementState::Unavailable),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotCaptureIdentity {
    pub(crate) id: SnapshotId,
    pub(crate) document_epoch: u64,
    pub(crate) url: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AccessibilitySnapshotOptions {
    pub compact: bool,
    pub max_depth: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilitySnapshot {
    pub id: SnapshotId,
    pub url: String,
    pub nodes: Vec<AccessibilitySnapshotNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilitySnapshotNode {
    pub depth: u64,
    pub reference: Option<InteractiveElementRef>,
    pub source: AccessibilitySnapshotSourceInfo,
    pub state: InteractiveElementState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilitySnapshotSourceInfo {
    pub role: String,
    pub name: String,
    pub target_url: Option<String>,
}

impl AccessibilitySnapshotNode {
    pub fn role(&self) -> &str {
        &self.source.role
    }

    pub fn name(&self) -> &str {
        &self.source.name
    }

    pub fn target_url(&self) -> Option<&str> {
        self.source.target_url.as_deref()
    }
}

impl AccessibilitySnapshot {
    pub(crate) fn from_nodes(
        identity: SnapshotCaptureIdentity,
        source_nodes: &[AccessibilityNodeSource],
        interactive_elements: &[InteractiveElementSource],
        options: AccessibilitySnapshotOptions,
    ) -> Self {
        let base_depth = source_nodes.first().map_or(0, |node| node.depth);
        let nodes = source_nodes
            .iter()
            .enumerate()
            .filter(|(index, node)| {
                !options.compact || !is_empty_compact_structure(*index, source_nodes, node)
            })
            .filter_map(|(_, node)| {
                accessibility_snapshot_node(
                    &identity,
                    node,
                    base_depth,
                    interactive_elements,
                    options.max_depth,
                )
            })
            .collect();
        Self {
            id: identity.id,
            url: identity.url,
            nodes,
        }
    }
}

fn accessibility_snapshot_node(
    identity: &SnapshotCaptureIdentity,
    node: &AccessibilityNodeSource,
    base_depth: u64,
    interactive_elements: &[InteractiveElementSource],
    max_depth: Option<u64>,
) -> Option<AccessibilitySnapshotNode> {
    let depth = node.depth.saturating_sub(base_depth);
    if max_depth.is_some_and(|maximum| depth > maximum) {
        return None;
    }
    let interactive = node
        .origin
        .reference_source_index()
        .and_then(|source_index| {
            interactive_elements
                .iter()
                .enumerate()
                .find(|(_, element)| element.source_index == source_index)
        });
    Some(AccessibilitySnapshotNode {
        depth,
        reference: snapshot_reference(identity, interactive),
        source: AccessibilitySnapshotSourceInfo {
            role: node.role.clone(),
            name: node.name.clone(),
            target_url: snapshot_target_url(&identity.url, interactive),
        },
        state: interactive
            .map(|(_, element)| element)
            .and_then(snapshot_element_state)
            .unwrap_or(InteractiveElementState::Unavailable),
    })
}

fn snapshot_reference(
    identity: &SnapshotCaptureIdentity,
    interactive: Option<(usize, &InteractiveElementSource)>,
) -> Option<InteractiveElementRef> {
    interactive.map(|(index, _)| {
        InteractiveElementRef::new(
            identity.document_epoch,
            identity.id,
            u64::try_from(index + 1).expect("reference element count exceeds u64"),
        )
    })
}

fn snapshot_target_url(
    page_url: &str,
    interactive: Option<(usize, &InteractiveElementSource)>,
) -> Option<String> {
    interactive
        .map(|(_, element)| element)
        .filter(|element| element.role() == "link")
        .and_then(|element| element.attribute("href"))
        .and_then(|href| resolve_url_reference(page_url, href).ok())
}

fn snapshot_element_state(source: &InteractiveElementSource) -> Option<InteractiveElementState> {
    source
        .value()
        .map(|value| InteractiveElementState::Value(value.into()))
        .or_else(|| source.checked().map(InteractiveElementState::Checked))
}

fn is_empty_compact_structure(
    index: usize,
    nodes: &[AccessibilityNodeSource],
    node: &AccessibilityNodeSource,
) -> bool {
    let has_child = nodes
        .get(index + 1)
        .is_some_and(|next| next.depth > node.depth);
    node.role == "ListMarker"
        || (!has_child
            && node.name.is_empty()
            && matches!(
                node.role.as_str(),
                "generic" | "group" | "listitem" | "region" | "row" | "rowgroup"
            ))
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
