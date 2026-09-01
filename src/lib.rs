mod cli;
mod cli_output;
mod cli_session;
mod cli_session_json;
mod keyboard;
mod layout;
mod loading;
mod locator;
mod non_empty;
mod page;
mod rules;
mod screenshot;
mod selection;
mod session;
mod snapshot;

pub const DEFAULT_VIEWPORT_HEIGHT: u64 = 720;
pub const DEFAULT_VIEWPORT_WIDTH: u64 = 1280;

pub use cli::{ExitStatus, run_cli, run_cli_with_input};
pub use keyboard::{
    FocusTraversalEffect, FocusedElement, KeyboardEventKey, KeyboardKey, KeyboardKeyError,
    KeyboardModifier, KeyboardTextEffect, NavigationPressEffect, PressEffect, TextPressEffect,
    TextSelection,
};
pub use layout::{
    BoundingBox, ElementInput, LayoutError, LayoutInput, LayoutMutation, SemanticElementId,
};
pub use loading::{LoadError, NetworkAccess};
pub use locator::{
    AltLocator, CssLocator, CssLocatorError, LabelLocator, Locator, LocatorMatch, LocatorPosition,
    LocatorValueError, PlaceholderLocator, RoleLocator, RoleLocatorError, RoleMatch, TestIdLocator,
    TextLocator, TitleLocator, XPathLocator, XPathLocatorError,
};
pub use non_empty::NonEmpty;
pub use rules::{Comparison, Finding, RuleConstraint, RuleResult, WidthFinding};
pub use screenshot::{
    CaptureRect, CaptureRectError, CaptureTarget, MAX_SCREENSHOT_PAINT_PIXELS,
    MAX_SCREENSHOT_PIXELS, OnDemandRasterProcess, PaintCommand, PaintScene, PngEncodingError,
    PreparedScreenshot, RasterImage, RasterImageError, RasterProcess, RasterProcessError,
    RasterProcessFactory, Rgba8, SoftwareRasterProcess, SoftwareRasterProcessFactory, encode_png,
};
pub use selection::SelectOptionTarget;
pub use session::{
    ActionabilityCheck, ApplyMutation, ApplyMutations, CaptureAccessibilitySnapshot,
    CaptureAccessibilitySnapshotWithin, CaptureInteractiveSnapshot,
    CaptureInteractiveSnapshotWithin, CheckElementWidth, ClickByLocator, ClickByLocatorResult,
    ClickByRole, ClickByRoleResult, ClickElement, ClickResult, CountByLocator, DomEvent,
    DomEventTargetIdentity, DomEventType, ElementAttribute, ElementBoundingBox, ElementChecked,
    ElementEditable, ElementEnabled, ElementFocused, ElementHovered, ElementHtml, ElementScroll,
    ElementText, ElementValue, ElementVisible, FillByLocator, FillByLocatorResult, FillByRole,
    FillByRoleResult, FillElement, FillResult, FindAllByLocator, FindByLocator, FindByRole,
    FocusByLocator, FocusByLocatorResult, FocusElement, FocusResult, GetAttributeByLocator,
    GetBoundingBoxByLocator, GetCheckedByLocator, GetEditableByLocator, GetElementAttribute,
    GetElementBoundingBox, GetElementChecked, GetElementEditable, GetElementEnabled,
    GetElementFocused, GetElementHovered, GetElementHtml, GetElementText, GetElementValue,
    GetElementVisible, GetEnabledByLocator, GetFocusedByLocator, GetHoveredByLocator,
    GetHtmlByLocator, GetPageText, GetPageTitle, GetPageUrl, GetValueByLocator, GetViewportSize,
    GetVisibleByLocator, GoBack, GoForward, HistoryNavigationResult, HoverByLocator,
    HoverByLocatorResult, HoverByRole, HoverByRoleResult, HoverElement, HoverResult, KeyDown,
    KeyDownResult, KeyUp, KeyUpResult, KeyboardInsertText, KeyboardTextResult, KeyboardType,
    LintLayout, LocatorAction, LocatorAttribute, LocatorBoundingBox, LocatorChecked, LocatorCount,
    LocatorEditable, LocatorEnabled, LocatorFocused, LocatorHovered, LocatorHtml,
    LocatorInspection, LocatorMatches, LocatorScroll, LocatorValue, LocatorVisible, OpenPage,
    OpenedPage, PageScroll, PageText, PageTitle, PageUrl, PrepareScreenshot, PressByLocator,
    PressByLocatorResult, PressKey, PressResult, ReloadPage, RoleAction, ScrollDirection,
    ScrollElementIntoView, ScrollIntoViewByLocator, ScrollPage, SelectByLocator,
    SelectByLocatorResult, SelectElement, SelectOptions, SelectOptionsByLocator,
    SelectOptionsByLocatorResult, SelectOptionsResult, SelectResult, Session, SessionError,
    SessionRequest, SetCheckedByLocator, SetCheckedByLocatorResult, SetCheckedByRole,
    SetCheckedByRoleResult, SetCheckedResult, SetElementChecked, SetViewportSize, TakeDomEvents,
    TypeByLocator, TypeByLocatorResult, TypeElement, TypeResult, ViewportResize, ViewportSize,
};
pub use snapshot::{
    AccessibilitySnapshot, AccessibilitySnapshotNode, AccessibilitySnapshotOptions,
    AccessibilitySnapshotSourceInfo, EvidenceRef, InteractiveElement, InteractiveElementRef,
    InteractiveElementSourceInfo, InteractiveElementState, InteractiveSnapshot, SnapshotId,
};

#[cfg(test)]
mod tests {
    use crate::layout::{ElementInput, LayoutInput};
    use crate::rules::{Comparison, RuleConstraint, RuleResult};
    use crate::session::{ApplyMutation, ApplyMutations, LintLayout, Session};

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
    fn batched_position_and_width_mutations_match_a_fresh_clean_layout() {
        let initial = LayoutInput {
            viewport_width: 320,
            elements: vec![
                ElementInput::supported("header", 0, 320),
                ElementInput::supported("hero", 20, 40),
            ],
        };
        let changed = LayoutInput {
            viewport_width: 320,
            elements: vec![
                ElementInput::supported("header", 0, 320),
                ElementInput::supported("hero", 280, 80),
            ],
        };
        let mut incremental_session = Session::new();
        incremental_session
            .execute(LintLayout { input: initial })
            .unwrap();

        let incremental = incremental_session
            .execute(ApplyMutations {
                mutations: vec![
                    crate::LayoutMutation::SetWidth {
                        element: "hero".into(),
                        width: 80,
                    },
                    crate::LayoutMutation::SetX {
                        element: "hero".into(),
                        x: 280,
                    },
                ],
            })
            .unwrap();
        let clean = Session::new()
            .execute(LintLayout { input: changed })
            .unwrap();

        let geometry = |result: RuleResult| match result {
            RuleResult::Compared {
                comparison: Comparison::Fail(findings),
                ..
            } => (
                findings[0].affected_element.as_str().to_owned(),
                findings[0].observed_left,
                findings[0].observed_width,
                findings[0].observed_right,
            ),
            other => panic!("expected overflow, got {other:?}"),
        };
        assert_eq!(geometry(incremental), geometry(clean));
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
