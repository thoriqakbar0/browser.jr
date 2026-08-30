mod cli;
mod layout;
mod loading;
mod non_empty;
mod page;
mod rules;
mod session;
mod snapshot;

pub use cli::{ExitStatus, run_cli};
pub use layout::{ElementInput, LayoutError, LayoutInput, LayoutMutation, SemanticElementId};
pub use loading::LoadError;
pub use non_empty::NonEmpty;
pub use rules::{Comparison, Finding, RuleConstraint, RuleResult, WidthFinding};
pub use session::{
    ApplyMutation, CaptureInteractiveSnapshot, CheckElementWidth, LintLayout, OpenPage, OpenedPage,
    Session, SessionError, SessionRequest,
};
pub use snapshot::{
    EvidenceRef, InteractiveElement, InteractiveElementRef, InteractiveSnapshot, SnapshotId,
};

#[cfg(test)]
mod tests {
    use crate::layout::{ElementInput, LayoutInput};
    use crate::rules::{Comparison, RuleConstraint, RuleResult};
    use crate::session::{ApplyMutation, LintLayout, Session};

    #[test]
    fn typed_request_returns_an_overflow_report() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 280, 80)],
        };
        let mut session = Session::new();

        let result = session.execute(LintLayout { input }).unwrap();

        match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => {
                assert_eq!(findings.len(), 1);
                assert_eq!(findings[0].affected_element.as_str(), "hero");
                assert_eq!(findings[0].observed_width, 80);
                assert_eq!(findings[0].observed_right, 360);
                assert_eq!(findings[0].evidence.len(), 1);
            }
            other => panic!("expected one overflow finding, got {other:?}"),
        }
    }

    #[test]
    fn unavailable_layout_cannot_pass() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::unsupported(
                "grid",
                "CSS Grid is not implemented",
            )],
        };
        let mut session = Session::new();

        let result = session.execute(LintLayout { input }).unwrap();

        assert_eq!(
            result,
            RuleResult::Blocked {
                rule: "horizontal-overflow",
                causes: crate::NonEmpty::one(RuleConstraint::Unsupported {
                    element: "grid".into(),
                    reason: "CSS Grid is not implemented".into(),
                }),
            }
        );
    }

    #[test]
    fn clean_layout_snapshot_is_stable() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![
                ElementInput::supported("first", 0, 100),
                ElementInput::supported("second", 100, 220),
            ],
        };
        let mut first_session = Session::new();
        let mut second_session = Session::new();

        let first = first_session.execute(LintLayout {
            input: input.clone(),
        });
        let second = second_session.execute(LintLayout { input });

        assert_eq!(first, second);
    }

    #[test]
    fn fitting_elements_pass() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("main", 0, 320)],
        };
        let mut session = Session::new();

        let result = session.execute(LintLayout { input }).unwrap();

        assert_eq!(
            result,
            RuleResult::Compared {
                rule: "horizontal-overflow",
                comparison: Comparison::Pass,
            }
        );
    }

    #[test]
    fn duplicate_semantic_identifiers_are_rejected() {
        let input = LayoutInput {
            viewport_width: 320,
            elements: vec![
                ElementInput::supported("main", 0, 100),
                ElementInput::supported("main", 100, 100),
            ],
        };
        let mut session = Session::new();

        let result = session.execute(LintLayout { input });

        assert_eq!(
            result,
            Err(crate::SessionError::Layout(
                crate::LayoutError::DuplicateElementId("main".into())
            ))
        );
    }

    #[test]
    fn width_mutation_matches_a_fresh_clean_layout() {
        let initial = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 280, 40)],
        };
        let changed = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 280, 80)],
        };
        let mut incremental_session = Session::new();
        incremental_session
            .execute(LintLayout { input: initial })
            .unwrap();

        let incremental = incremental_session
            .execute(ApplyMutation {
                mutation: crate::LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 80,
                },
            })
            .unwrap();
        let clean = Session::new()
            .execute(LintLayout { input: changed })
            .unwrap();

        let right_edge = |result: RuleResult| match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => findings[0].observed_right,
            other => panic!("expected overflow, got {other:?}"),
        };
        assert_eq!(right_edge(incremental), right_edge(clean));
    }

    #[test]
    fn repeated_width_mutation_is_idempotent() {
        let initial = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 280, 40)],
        };
        let mutation = crate::LayoutMutation::SetWidth {
            element: "hero".into(),
            width: 80,
        };
        let mut session = Session::new();
        session.execute(LintLayout { input: initial }).unwrap();

        let first = session
            .execute(ApplyMutation {
                mutation: mutation.clone(),
            })
            .unwrap();
        let second = session.execute(ApplyMutation { mutation }).unwrap();

        let observed = |result: RuleResult| match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => (
                findings[0].observed_left,
                findings[0].observed_width,
                findings[0].observed_right,
            ),
            other => panic!("expected overflow, got {other:?}"),
        };
        assert_eq!(observed(first), observed(second));
    }

    #[test]
    fn failed_mutation_preserves_the_previous_layout() {
        let initial = LayoutInput {
            viewport_width: 320,
            elements: vec![ElementInput::supported("hero", 10, 40)],
        };
        let mut session = Session::new();
        session.execute(LintLayout { input: initial }).unwrap();

        let failure = session.execute(ApplyMutation {
            mutation: crate::LayoutMutation::SetWidth {
                element: "hero".into(),
                width: u64::MAX,
            },
        });
        let after_failure = session
            .execute(ApplyMutation {
                mutation: crate::LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 40,
                },
            })
            .unwrap();

        assert_eq!(
            failure,
            Err(crate::SessionError::Layout(
                crate::LayoutError::CoordinateOverflow {
                    element: "hero".into(),
                }
            ))
        );
        assert_eq!(
            after_failure,
            RuleResult::Compared {
                rule: "horizontal-overflow",
                comparison: Comparison::Pass,
            }
        );
    }
}
