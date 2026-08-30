use browser_jr::{
    ApplyMutation, CaptureInteractiveSnapshot, CheckElementWidth, ClickElement, ClickResult,
    Comparison, ElementInput, FillElement, FillResult, GetElementAttribute, GetElementChecked,
    GetElementEnabled, GetElementText, GetElementValue, GetPageTitle, GetPageUrl,
    InteractiveElementState, LayoutInput, LayoutMutation, LintLayout, OpenPage, ReloadPage,
    RuleConstraint, RuleResult, Session, SessionError, SetElementChecked,
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
    serve_pages(vec![body])
}

fn serve_pages(bodies: Vec<&'static str>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
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
fn package_session_assigns_fresh_refs_to_each_interactive_snapshot() {
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
    assert_ne!(first.elements[0].reference, second.elements[0].reference);
    assert_eq!(first.elements[0].reference.snapshot(), first.id);
    assert_eq!(second.elements[0].reference.snapshot(), second.id);
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
fn clicking_a_link_navigates_and_invalidates_the_previous_ref() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>First page</title><a id="next" href="/next">Next</a>"#,
        r#"<title> Second
            page </title><button id="arrived">Arrived</button>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let before_url = session.execute(GetPageUrl).unwrap();
    let before_title = session.execute(GetPageTitle).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = snapshot.elements[0].reference;

    let result = session.execute(ClickElement { reference }).unwrap();
    server.join().unwrap();
    let current_url = session.execute(GetPageUrl).unwrap();
    let current_title = session.execute(GetPageTitle).unwrap();
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(ClickElement { reference });
    drop(network_guard);

    assert_eq!(
        result,
        ClickResult::Navigated {
            reference,
            page: browser_jr::OpenedPage {
                url: format!("{url}next"),
                interactive_element_count: 1,
            },
        }
    );
    assert_eq!(after.url, format!("{url}next"));
    assert_eq!(before_url.url, url);
    assert_eq!(before_title.title, "First page");
    assert_eq!(current_url.url, format!("{url}next"));
    assert_eq!(current_title.title, "Second page");
    assert_eq!(after.elements[0].name, "Arrived");
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference })
    );
}

#[test]
fn reload_replaces_the_document_and_failed_reload_preserves_it() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>First</title><button>First</button>"#,
        r#"<title>Second</title><button>Second</button>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let first = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first_reference = first.elements[0].reference;

    let reloaded = session.execute(ReloadPage).unwrap();
    server.join().unwrap();
    let title = session.execute(GetPageTitle).unwrap();
    let second = session.execute(CaptureInteractiveSnapshot).unwrap();
    let second_reference = second.elements[0].reference;
    let stale = session.execute(GetElementText {
        reference: first_reference,
    });
    let failed = session.execute(ReloadPage);
    let text_after_failure = session
        .execute(GetElementText {
            reference: second_reference,
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(reloaded.url, url);
    assert_eq!(title.title, "Second");
    assert_eq!(second.elements[0].name, "Second");
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: first_reference,
        })
    );
    assert!(matches!(failed, Err(SessionError::Load(_))));
    assert_eq!(text_after_failure.text, "Second");
}

#[test]
fn a_new_snapshot_invalidates_previous_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let first = session.execute(CaptureInteractiveSnapshot).unwrap();
    let second = session.execute(CaptureInteractiveSnapshot).unwrap();

    let stale = session.execute(ClickElement {
        reference: first.elements[0].reference,
    });
    let unsupported = session.execute(ClickElement {
        reference: second.elements[0].reference,
    });
    drop(network_guard);

    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: first.elements[0].reference,
        })
    );
    assert!(matches!(
        unsupported,
        Err(SessionError::UnsupportedClick { reference, .. })
            if reference == second.elements[0].reference
    ));
}

#[test]
fn unsupported_clicks_preserve_the_current_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Save</button><a href="/new" target="_blank">New</a><a href="/file" download>File</a>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();

    let results = snapshot
        .elements
        .iter()
        .map(|element| {
            session.execute(ClickElement {
                reference: element.reference,
            })
        })
        .collect::<Vec<_>>();
    drop(network_guard);

    assert!(
        results
            .iter()
            .all(|result| matches!(result, Err(SessionError::UnsupportedClick { .. })))
    );
}

#[test]
fn text_value_actions_update_and_read_current_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email</label><input id="email" value="old"><textarea aria-label="Note">draft</textarea><input aria-label="Locked" value="fixed" readonly><button>Save</button>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();

    let email = before.elements[0].reference;
    let note = before.elements[1].reference;
    let locked = before.elements[2].reference;
    let button = before.elements[3].reference;
    let unsupported = session.execute(FillElement {
        reference: button,
        value: "ignored".into(),
    });
    let email_result = session
        .execute(FillElement {
            reference: email,
            value: "hello@example.com".into(),
        })
        .unwrap();
    let note_result = session
        .execute(FillElement {
            reference: note,
            value: "hello world".into(),
        })
        .unwrap();
    let current_email = session
        .execute(GetElementValue { reference: email })
        .unwrap();
    let current_locked = session
        .execute(GetElementValue { reference: locked })
        .unwrap();
    let locked_fill = session.execute(FillElement {
        reference: locked,
        value: "changed".into(),
    });
    let button_value = session.execute(GetElementValue { reference: button });
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(FillElement {
        reference: email,
        value: "stale".into(),
    });
    let stale_value = session.execute(GetElementValue { reference: email });
    drop(network_guard);

    assert_eq!(
        before.elements[0].state,
        InteractiveElementState::Value("old".into())
    );
    assert_eq!(
        before.elements[1].state,
        InteractiveElementState::Value("draft".into())
    );
    assert_eq!(
        before.elements[2].state,
        InteractiveElementState::Value("fixed".into())
    );
    assert_eq!(
        before.elements[3].state,
        InteractiveElementState::Unavailable
    );
    assert_eq!(
        email_result,
        FillResult {
            reference: email,
            value: "hello@example.com".into(),
        }
    );
    assert_eq!(note_result.value, "hello world");
    assert_eq!(current_email.value, "hello@example.com");
    assert_eq!(current_locked.value, "fixed");
    assert!(matches!(
        unsupported,
        Err(SessionError::UnsupportedFill { reference, .. }) if reference == button
    ));
    assert!(matches!(
        locked_fill,
        Err(SessionError::UnsupportedFill { reference, .. }) if reference == locked
    ));
    assert!(matches!(
        button_value,
        Err(SessionError::UnsupportedValue { reference, .. }) if reference == button
    ));
    assert_eq!(
        after.elements[0].state,
        InteractiveElementState::Value("hello@example.com".into())
    );
    assert_eq!(
        after.elements[1].state,
        InteractiveElementState::Value("hello world".into())
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: email })
    );
    assert_eq!(
        stale_value,
        Err(SessionError::StaleElementReference { reference: email })
    );
}

#[test]
fn checkbox_actions_update_and_read_current_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label><input type="checkbox"> Updates</label><input type="checkbox" checked disabled aria-label="Locked"><button>Save</button><div role="switch" aria-label="Custom"></div>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();
    let updates = before.elements[0].reference;
    let locked = before.elements[1].reference;
    let button = before.elements[2].reference;
    let custom = before.elements[3].reference;

    let initial = session
        .execute(GetElementChecked { reference: updates })
        .unwrap();
    let first = session
        .execute(SetElementChecked {
            reference: updates,
            checked: true,
        })
        .unwrap();
    let repeated = session
        .execute(SetElementChecked {
            reference: updates,
            checked: true,
        })
        .unwrap();
    let current = session
        .execute(GetElementChecked { reference: updates })
        .unwrap();
    let locked_state = session
        .execute(GetElementChecked { reference: locked })
        .unwrap();
    let locked_change = session.execute(SetElementChecked {
        reference: locked,
        checked: false,
    });
    let button_state = session.execute(GetElementChecked { reference: button });
    let updates_enabled = session
        .execute(GetElementEnabled { reference: updates })
        .unwrap();
    let locked_enabled = session
        .execute(GetElementEnabled { reference: locked })
        .unwrap();
    let button_enabled = session
        .execute(GetElementEnabled { reference: button })
        .unwrap();
    let custom_enabled = session.execute(GetElementEnabled { reference: custom });
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(GetElementChecked { reference: updates });
    drop(network_guard);

    assert_eq!(
        before.elements[0].state,
        InteractiveElementState::Checked(false)
    );
    assert_eq!(
        before.elements[1].state,
        InteractiveElementState::Checked(true)
    );
    assert_eq!(
        before.elements[2].state,
        InteractiveElementState::Unavailable
    );
    assert!(!initial.checked);
    assert!(first.checked);
    assert_eq!(first, repeated);
    assert!(current.checked);
    assert!(locked_state.checked);
    assert!(matches!(
        locked_change,
        Err(SessionError::UnsupportedCheck { reference, .. }) if reference == locked
    ));
    assert!(matches!(
        button_state,
        Err(SessionError::UnsupportedCheckedState { reference, .. }) if reference == button
    ));
    assert!(updates_enabled.enabled);
    assert!(!locked_enabled.enabled);
    assert!(button_enabled.enabled);
    assert!(matches!(
        custom_enabled,
        Err(SessionError::UnsupportedEnabledState { reference, .. }) if reference == custom
    ));
    assert_eq!(
        after.elements[0].state,
        InteractiveElementState::Checked(true)
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: updates })
    );
}

#[test]
fn element_text_reads_descendants_without_using_the_accessible_name() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button aria-label="Save changes"> Hello <span>world</span> </button><input aria-label="Email">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let button = snapshot.elements[0].reference;
    let input = snapshot.elements[1].reference;

    let button_text = session
        .execute(GetElementText { reference: button })
        .unwrap();
    let input_text = session
        .execute(GetElementText { reference: input })
        .unwrap();
    let current_again = session
        .execute(GetElementText { reference: button })
        .unwrap();
    drop(network_guard);

    assert_eq!(button_text.text, "Hello world");
    assert_eq!(input_text.text, "");
    assert_eq!(button_text, current_again);
}

#[test]
fn element_attribute_reads_preserve_missing_and_sensitive_states() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<a href="/next" data-kind="primary">Next</a><input type="password" value="secret" aria-label="Password">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let link = snapshot.elements[0].reference;
    let password = snapshot.elements[1].reference;

    let href = session
        .execute(GetElementAttribute {
            reference: link,
            name: "href".into(),
        })
        .unwrap();
    let normalized = session
        .execute(GetElementAttribute {
            reference: link,
            name: "DATA-KIND".into(),
        })
        .unwrap();
    let missing = session
        .execute(GetElementAttribute {
            reference: link,
            name: "title".into(),
        })
        .unwrap();
    let password_type = session
        .execute(GetElementAttribute {
            reference: password,
            name: "type".into(),
        })
        .unwrap();
    let sensitive = session.execute(GetElementAttribute {
        reference: password,
        name: "value".into(),
    });
    let invalid = session.execute(GetElementAttribute {
        reference: link,
        name: "data kind".into(),
    });
    drop(network_guard);

    assert_eq!(href.value.as_deref(), Some("/next"));
    assert_eq!(normalized.name, "data-kind");
    assert_eq!(normalized.value.as_deref(), Some("primary"));
    assert_eq!(missing.value, None);
    assert_eq!(password_type.value.as_deref(), Some("password"));
    assert_eq!(
        sensitive,
        Err(SessionError::SensitiveAttribute {
            reference: password,
            name: "value".into(),
        })
    );
    assert_eq!(
        invalid,
        Err(SessionError::InvalidAttributeName {
            name: "data kind".into(),
        })
    );
}

#[test]
fn failed_link_navigation_preserves_the_page_and_ref() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<a href="http://example.com/away">Away</a>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = snapshot.elements[0].reference;

    let first = session.execute(ClickElement { reference });
    let second = session.execute(ClickElement { reference });
    drop(network_guard);

    assert!(matches!(
        first,
        Err(SessionError::Navigation {
            reference: failed_ref,
            ..
        }) if failed_ref == reference
    ));
    assert!(matches!(
        second,
        Err(SessionError::Navigation {
            reference: failed_ref,
            ..
        }) if failed_ref == reference
    ));
}

#[test]
fn interactive_snapshot_requires_an_open_page() {
    let result = Session::new().execute(CaptureInteractiveSnapshot);

    assert_eq!(result, Err(SessionError::NoPage));
}

#[test]
fn page_url_requires_an_open_page() {
    assert_eq!(
        Session::new().execute(GetPageUrl),
        Err(SessionError::NoPage)
    );
}

#[test]
fn page_title_requires_an_open_page() {
    assert_eq!(
        Session::new().execute(GetPageTitle),
        Err(SessionError::NoPage)
    );
}

#[test]
fn reload_requires_an_open_page() {
    assert_eq!(
        Session::new().execute(ReloadPage),
        Err(SessionError::NoPage)
    );
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
    assert_eq!(before.url, after.url);
    assert_ne!(before.elements[0].reference, after.elements[0].reference);
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
