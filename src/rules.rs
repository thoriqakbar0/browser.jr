use crate::layout::SemanticElementId;
use crate::non_empty::NonEmpty;
use crate::snapshot::{EvidenceRef, ObservationCell, Snapshot};

pub(crate) const HORIZONTAL_OVERFLOW: &str = "horizontal-overflow";
pub(crate) const MAX_ELEMENT_WIDTH: &str = "max-element-width";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub affected_element: SemanticElementId,
    pub viewport_width: u64,
    pub observed_left: i64,
    pub observed_width: u64,
    pub observed_right: i64,
    pub evidence: NonEmpty<EvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleConstraint {
    Unsupported { element: String, reason: String },
    MissingElement { element: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Comparison<F = Finding> {
    Pass,
    Fail(NonEmpty<F>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleResult<F = Finding> {
    Compared {
        rule: &'static str,
        comparison: Comparison<F>,
    },
    Blocked {
        rule: &'static str,
        causes: NonEmpty<RuleConstraint>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WidthFinding {
    pub affected_element: SemanticElementId,
    pub viewport_width: u64,
    pub maximum_width: u64,
    pub observed_width: u64,
    pub evidence: NonEmpty<EvidenceRef>,
}

pub(crate) fn evaluate_horizontal_overflow(snapshot: &Snapshot) -> RuleResult {
    let mut findings = Vec::new();
    let mut causes = Vec::new();
    for observation in &snapshot.geometry {
        match observation {
            ObservationCell::Available(observation) => {
                let geometry = &observation.value;
                let overflows_right = u64::try_from(geometry.right)
                    .map(|right| right > snapshot.viewport_width)
                    .unwrap_or(false);
                if geometry.x < 0 || overflows_right {
                    findings.push(Finding {
                        affected_element: geometry.element.clone(),
                        viewport_width: snapshot.viewport_width,
                        observed_left: geometry.x,
                        observed_width: geometry.width,
                        observed_right: geometry.right,
                        evidence: observation.evidence.clone(),
                    });
                }
            }
            ObservationCell::Unsupported { element, reason } => {
                causes.push(RuleConstraint::Unsupported {
                    element: element.as_str().into(),
                    reason: reason.clone(),
                });
            }
        }
    }

    if let Some(causes) = NonEmpty::from_vec(causes) {
        RuleResult::Blocked {
            rule: HORIZONTAL_OVERFLOW,
            causes,
        }
    } else if let Some(findings) = NonEmpty::from_vec(findings) {
        RuleResult::Compared {
            rule: HORIZONTAL_OVERFLOW,
            comparison: Comparison::Fail(findings),
        }
    } else {
        RuleResult::Compared {
            rule: HORIZONTAL_OVERFLOW,
            comparison: Comparison::Pass,
        }
    }
}

pub(crate) fn evaluate_max_element_width(
    snapshot: &Snapshot,
    element: &str,
    maximum_width: u64,
) -> RuleResult<WidthFinding> {
    let observation = snapshot
        .geometry
        .iter()
        .find(|observation| match observation {
            ObservationCell::Available(observation) => {
                observation.value.element.as_str() == element
            }
            ObservationCell::Unsupported {
                element: observed_element,
                ..
            } => observed_element.as_str() == element,
        });

    match observation {
        Some(ObservationCell::Available(observation)) => {
            let geometry = &observation.value;
            let comparison = if geometry.width > maximum_width {
                Comparison::Fail(NonEmpty::one(WidthFinding {
                    affected_element: geometry.element.clone(),
                    viewport_width: snapshot.viewport_width,
                    maximum_width,
                    observed_width: geometry.width,
                    evidence: observation.evidence.clone(),
                }))
            } else {
                Comparison::Pass
            };
            RuleResult::Compared {
                rule: MAX_ELEMENT_WIDTH,
                comparison,
            }
        }
        Some(ObservationCell::Unsupported { element, reason }) => RuleResult::Blocked {
            rule: MAX_ELEMENT_WIDTH,
            causes: NonEmpty::one(RuleConstraint::Unsupported {
                element: element.as_str().into(),
                reason: reason.clone(),
            }),
        },
        None => RuleResult::Blocked {
            rule: MAX_ELEMENT_WIDTH,
            causes: NonEmpty::one(RuleConstraint::MissingElement {
                element: element.into(),
            }),
        },
    }
}
