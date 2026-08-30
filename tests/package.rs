use browser_jr::{
    ApplyMutation, CaptureInteractiveSnapshot, CheckElementWidth, Comparison, ElementInput,
    LayoutInput, LayoutMutation, LintLayout, OpenPage, RuleConstraint, RuleResult, Session,
    SessionError,
};
use std::io::Write;
use std::net::TcpListener;
use std::sync::{Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

static NETWORK_TEST: Mutex<()> = Mutex::new(());

fn network_test_guard() -> MutexGuard<'static, ()> {
    NETWORK_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn serve_page(body: &'static str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    (format!("http://{address}/"), handle)
}

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
fn package_session_opens_a_page_and_keeps_interactive_refs_stable() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email</label><input id="email"><button id="save">Save</button>"#,
    );
    let mut session = Session::new();

    let opened = session.execute(OpenPage { url: url.clone() }).unwrap();
    server.join().unwrap();
    let first = session.execute(CaptureInteractiveSnapshot).unwrap();
    let second = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert_eq!(opened.url, url);
    assert_eq!(opened.interactive_element_count, 2);
    assert_eq!(first.id.get(), 1);
    assert_eq!(second.id.get(), 2);
    assert_eq!(first.elements[0].reference, second.elements[0].reference);
    assert_eq!(first.elements[0].reference.to_string(), "@e1");
    assert_eq!(first.elements[0].role, "textbox");
    assert_eq!(first.elements[0].name, "Email");
    assert_eq!(first.elements[1].reference.to_string(), "@e2");
    assert_eq!(first.elements[1].role, "button");
    assert_eq!(first.elements[1].name, "Save");
}

#[test]
fn opening_a_new_document_replaces_interactive_refs() {
    let network_guard = network_test_guard();
    let (first_url, first_server) = serve_page(r#"<button>First</button>"#);
    let (second_url, second_server) = serve_page(r#"<button>Second</button>"#);
    let mut session = Session::new();

    session.execute(OpenPage { url: first_url }).unwrap();
    first_server.join().unwrap();
    let first = session.execute(CaptureInteractiveSnapshot).unwrap();
    session.execute(OpenPage { url: second_url }).unwrap();
    second_server.join().unwrap();
    let second = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert_ne!(first.elements[0].reference, second.elements[0].reference);
    assert_eq!(first.elements[0].reference.to_string(), "@e1");
    assert_eq!(second.elements[0].reference.to_string(), "@e1");
    assert_eq!(second.elements[0].name, "Second");
}

#[test]
fn interactive_snapshot_requires_an_open_page() {
    let result = Session::new().execute(CaptureInteractiveSnapshot);

    assert_eq!(result, Err(SessionError::NoPage));
}

#[test]
fn opening_a_page_invalidates_previous_layout_evidence() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button>"#);
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("article", 0, 320)],
            },
        })
        .unwrap();

    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let result = session.execute(CheckElementWidth {
        element: "article".into(),
        maximum_width: 320,
    });
    drop(network_guard);

    assert_eq!(result, Err(SessionError::NoSnapshot));
}

#[test]
fn failed_open_preserves_the_current_page() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();

    let failure = session.execute(OpenPage {
        url: "http://example.com".into(),
    });
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert!(matches!(failure, Err(SessionError::Load(_))));
    assert_eq!(before.elements[0].reference, after.elements[0].reference);
    assert_eq!(after.elements[0].name, "Save");
}

#[test]
fn package_caller_can_apply_a_project_width_limit() {
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 1280,
                elements: vec![ElementInput::supported("article", 200, 880)],
            },
        })
        .unwrap();

    let result = session
        .execute(CheckElementWidth {
            element: "article".into(),
            maximum_width: 720,
        })
        .unwrap();

    match result {
        RuleResult::Compared {
            comparison: Comparison::Fail(findings),
            ..
        } => {
            let finding = &findings[0];
            assert_eq!(finding.affected_element.as_str(), "article");
            assert_eq!(finding.maximum_width, 720);
            assert_eq!(finding.observed_width, 880);
            assert_eq!(finding.evidence[0].element.as_str(), "article");
        }
        other => panic!("expected a width finding, got {other:?}"),
    }
}

#[test]
fn project_width_limit_passes_at_the_boundary() {
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 1280,
                elements: vec![ElementInput::supported("article", 280, 720)],
            },
        })
        .unwrap();

    let result = session
        .execute(CheckElementWidth {
            element: "article".into(),
            maximum_width: 720,
        })
        .unwrap();

    assert_eq!(
        result,
        RuleResult::Compared {
            rule: "max-element-width",
            comparison: Comparison::Pass,
        }
    );
}

#[test]
fn project_width_limit_blocks_when_the_element_is_missing() {
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 1280,
                elements: vec![ElementInput::supported("main", 0, 1280)],
            },
        })
        .unwrap();

    let result = session
        .execute(CheckElementWidth {
            element: "article".into(),
            maximum_width: 720,
        })
        .unwrap();

    assert_eq!(
        result,
        RuleResult::Blocked {
            rule: "max-element-width",
            causes: browser_jr::NonEmpty::one(RuleConstraint::MissingElement {
                element: "article".into(),
            }),
        }
    );
}

#[test]
fn project_width_limit_blocks_when_target_geometry_is_unsupported() {
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 1280,
                elements: vec![ElementInput::unsupported(
                    "article",
                    "CSS Grid is not implemented",
                )],
            },
        })
        .unwrap();

    let result = session
        .execute(CheckElementWidth {
            element: "article".into(),
            maximum_width: 720,
        })
        .unwrap();

    assert_eq!(
        result,
        RuleResult::Blocked {
            rule: "max-element-width",
            causes: browser_jr::NonEmpty::one(RuleConstraint::Unsupported {
                element: "article".into(),
                reason: "CSS Grid is not implemented".into(),
            }),
        }
    );
}

#[test]
fn project_width_limit_requires_a_snapshot() {
    let result = Session::new().execute(CheckElementWidth {
        element: "article".into(),
        maximum_width: 720,
    });

    assert_eq!(result, Err(SessionError::NoSnapshot));
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
