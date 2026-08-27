use browser_jr::{
    ApplyMutation, Comparison, ElementInput, LayoutInput, LayoutMutation, LintLayout, RuleResult,
    Session,
};

#[test]
fn package_caller_receives_typed_overflow_evidence() {
    let mut session = Session::new();
    let result = session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("hero", 280, 80)],
            },
        })
        .unwrap();

    match result {
        RuleResult::Compared {
            comparison: Comparison::Fail(findings),
            ..
        } => {
            assert_eq!(findings[0].affected_element.as_str(), "hero");
            assert_eq!(findings[0].observed_right, 360);
            assert_eq!(findings[0].evidence.len(), 1);
        }
        other => panic!("expected overflow, got {other:?}"),
    }
}

#[test]
fn package_mutation_uses_the_same_session() {
    let mut session = Session::new();
    let initial = session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("hero", 280, 40)],
            },
        })
        .unwrap();
    let changed = session
        .execute(ApplyMutation {
            mutation: LayoutMutation::SetWidth {
                element: "hero".into(),
                width: 80,
            },
        })
        .unwrap();

    assert!(matches!(
        initial,
        RuleResult::Compared {
            comparison: Comparison::Pass,
            ..
        }
    ));
    assert!(matches!(
        changed,
        RuleResult::Compared {
            comparison: Comparison::Fail(_),
            ..
        }
    ));
}
