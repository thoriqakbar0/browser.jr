use crate::layout::SemanticElementId;
use crate::non_empty::NonEmpty;
use crate::snapshot::{EvidenceRef, ObservationCell, Snapshot};

pub(crate) const HORIZONTAL_OVERFLOW: &str = "horizontal-overflow";

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Comparison {
    Pass,
    Fail(NonEmpty<Finding>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleResult {
    Compared {
        rule: &'static str,
        comparison: Comparison,
    },
    Blocked {
        rule: &'static str,
        causes: NonEmpty<RuleConstraint>,
    },
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
