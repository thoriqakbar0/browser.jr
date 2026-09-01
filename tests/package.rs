use browser_jr::{
    AccessibilitySnapshotOptions, ActionabilityCheck, AltLocator, ApplyMutation, ApplyMutations,
    BoundingBox, CaptureAccessibilitySnapshot, CaptureAccessibilitySnapshotWithin,
    CaptureInteractiveSnapshot, CaptureInteractiveSnapshotWithin, CaptureTarget, CheckElementWidth,
    ClickByLocator, ClickByLocatorResult, ClickByRole, ClickByRoleResult, ClickElement,
    ClickResult, Comparison, CountByLocator, CssLocator, DomEventType, ElementBoundingBox,
    ElementInput, ElementVisible, FillByLocator, FillByRole, FillElement, FillResult,
    FindAllByLocator, FindByLocator, FindByRole, FocusByLocator, FocusElement, FocusResult,
    GetAttributeByLocator, GetBoundingBoxByLocator, GetCheckedByLocator, GetEditableByLocator,
    GetElementAttribute, GetElementBoundingBox, GetElementChecked, GetElementEditable,
    GetElementEnabled, GetElementFocused, GetElementHovered, GetElementHtml, GetElementText,
    GetElementValue, GetElementVisible, GetEnabledByLocator, GetFocusedByLocator,
    GetHoveredByLocator, GetHtmlByLocator, GetPageText, GetPageTitle, GetPageUrl,
    GetValueByLocator, GetViewportSize, GetVisibleByLocator, GoBack, GoForward,
    HistoryNavigationResult, HoverByLocator, HoverByRole, HoverElement, InteractiveElementState,
    KeyDown, KeyUp, KeyboardEventKey, KeyboardInsertText, KeyboardKey, KeyboardModifier,
    KeyboardTextEffect, KeyboardType, LabelLocator, LayoutInput, LayoutMutation, LintLayout,
    Locator, LocatorAction, LocatorInspection, NonEmpty, OnDemandRasterProcess, OpenPage,
    PageScroll, PageText, PlaceholderLocator, PrepareScreenshot, PressByLocator, PressKey,
    PressResult, ReloadPage, RoleAction, RoleLocator, RuleConstraint, RuleResult, ScrollDirection,
    ScrollElementIntoView, ScrollIntoViewByLocator, ScrollPage, SelectByLocator, SelectElement,
    SelectOptionTarget, SelectOptions, SelectOptionsByLocator, SelectOptionsResult, SelectResult,
    Session, SessionError, SetCheckedByLocator, SetCheckedByRole, SetElementChecked,
    SetViewportSize, SoftwareRasterProcessFactory, TakeDomEvents, TestIdLocator, TextLocator,
    TextPressEffect, TitleLocator, TypeByLocator, TypeElement, TypeResult, ViewportSize,
    XPathLocator,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

static NETWORK_TEST: Mutex<()> = Mutex::new(());

fn value_targets(values: &[&str]) -> NonEmpty<SelectOptionTarget> {
    NonEmpty::from_vec(
        values
            .iter()
            .map(|value| SelectOptionTarget::Value((*value).into()))
            .collect(),
    )
    .expect("test option targets are non-empty")
}

fn text_press(result: &PressResult) -> &TextPressEffect {
    result.text().expect("expected a text press effect")
}

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
            read_request_headers(&stream);
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

fn read_request_headers(stream: &std::net::TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
            return;
        }
    }
}

fn serve_pages_recording_requests(bodies: Vec<&'static str>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::with_capacity(bodies.len());
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_line = String::new();
            {
                let mut reader = BufReader::new(&mut stream);
                reader.read_line(&mut request_line).unwrap();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }
            }
            requests.push(request_line.trim_end().into());
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    (format!("http://{address}/"), handle)
}

fn serve_redirect_page(expected_requests: usize) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::with_capacity(expected_requests);
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_line = String::new();
            {
                let mut reader = BufReader::new(&mut stream);
                reader.read_line(&mut request_line).unwrap();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }
            }
            let path = request_line.split_whitespace().nth(1).unwrap_or("/");
            requests.push(path.to_owned());
            if path == "/start" {
                write!(
                    stream,
                    "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            } else {
                let body = "<h1>Redirected</h1>";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        }
        requests
    });
    (format!("http://{address}/start"), handle)
}

#[test]
fn redirected_navigation_records_the_committed_final_url_in_history() {
    let network_guard = network_test_guard();
    let (redirect_url, redirect_server) = serve_redirect_page(3);
    let (other_url, other_server) = serve_page("<p>Other</p>");
    let mut session = Session::new();

    let opened = session.execute(OpenPage { url: redirect_url }).unwrap();
    assert!(opened.url.ends_with("/final"));
    session.execute(OpenPage { url: other_url }).unwrap();
    other_server.join().unwrap();

    let back = session.execute(GoBack).unwrap();
    let requests = redirect_server.join().unwrap();
    drop(network_guard);

    assert!(matches!(back, HistoryNavigationResult::Navigated(_)));
    assert_eq!(requests, vec!["/start", "/final", "/final"]);
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
fn accessibility_snapshot_exposes_typed_tree_nodes_and_resolvable_heading_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Outside</button><main id="content"><h1>Hello <em>there</em></h1><a href="/docs">Docs</a></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let snapshot = session
        .execute(CaptureAccessibilitySnapshotWithin {
            locator: Locator::from(CssLocator::new("#content").unwrap()),
            options: AccessibilitySnapshotOptions::default(),
        })
        .unwrap();
    let heading = snapshot
        .nodes
        .iter()
        .find(|node| node.role() == "heading")
        .unwrap();
    let heading_reference = heading.reference.unwrap();
    let text = session
        .execute(GetElementText {
            reference: heading_reference,
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(snapshot.nodes[0].role(), "main");
    assert_eq!(snapshot.nodes[0].depth, 0);
    assert_eq!(heading.depth, 1);
    assert_eq!(heading.name(), "Hello there");
    assert_eq!(heading_reference.to_string(), "@e2");
    assert!(
        snapshot
            .nodes
            .iter()
            .any(|node| { node.role() == "emphasis" && node.name() == "there" && node.depth == 2 })
    );
    assert_eq!(text.text, "Hello there");
}

#[test]
fn accessibility_snapshot_emits_list_markers_only_for_unscoped_non_compact_trees() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="content"><ul><li>Alpha</li><li hidden>Hidden</li><li>Beta<ul><li>Nested</li></ul></li></ul><ol start="3"><li>Third</li><li value="7">Seventh</li><li>Eighth</li></ol></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let full = session
        .execute(CaptureAccessibilitySnapshot {
            options: AccessibilitySnapshotOptions::default(),
        })
        .unwrap();
    let compact = session
        .execute(CaptureAccessibilitySnapshot {
            options: AccessibilitySnapshotOptions {
                compact: true,
                max_depth: None,
            },
        })
        .unwrap();
    let scoped = session
        .execute(CaptureAccessibilitySnapshotWithin {
            locator: Locator::from(CssLocator::new("#content").unwrap()),
            options: AccessibilitySnapshotOptions::default(),
        })
        .unwrap();
    drop(network_guard);

    let markers = full
        .nodes
        .iter()
        .filter(|node| node.role() == "ListMarker")
        .collect::<Vec<_>>();
    assert_eq!(
        markers.iter().map(|node| node.name()).collect::<Vec<_>>(),
        ["• ", "• ", "• ", "1. ", "2. ", "3. "]
    );
    assert!(markers.iter().all(|node| node.depth == 0));
    assert!(markers.iter().all(|node| node.reference.is_none()));
    assert!(compact.nodes.iter().all(|node| node.role() != "ListMarker"));
    assert!(scoped.nodes.iter().all(|node| node.role() != "ListMarker"));
}

#[test]
fn accessibility_snapshot_requires_an_open_page() {
    let result = Session::new().execute(CaptureAccessibilitySnapshot {
        options: AccessibilitySnapshotOptions::default(),
    });

    assert_eq!(result, Err(SessionError::NoPage));
}

#[test]
fn interactive_snapshots_resolve_link_target_urls() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<a href="guide/next?q=1#details">Next</a><button>Save</button><a href="action" role="button">Open</a>"#,
    );
    let mut session = Session::new();

    session.execute(OpenPage { url: url.clone() }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert_eq!(
        snapshot.elements[0].target_url(),
        Some(format!("{url}guide/next?q=1#details").as_str())
    );
    assert_eq!(snapshot.elements[1].target_url(), None);
    assert_eq!(snapshot.elements[2].target_url(), None);
}

#[test]
fn package_prepares_scroll_aware_locator_and_full_page_screenshots() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0">
                <div style="width:100px;height:150px"></div>
                <main id="target" style="width:20px;height:10px;background-color:#ff0000"></main>
            </body>
        "#,
    );
    let mut session = Session::new();
    session
        .execute(SetViewportSize {
            width: 100,
            height: 100,
        })
        .unwrap();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let locator = Locator::from(CssLocator::new("#target").unwrap());
    let element = session
        .execute(PrepareScreenshot {
            target: CaptureTarget::Element(locator.clone()),
        })
        .unwrap();
    let scroll = session
        .execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 0,
        })
        .unwrap();
    let full_page = session
        .execute(PrepareScreenshot {
            target: CaptureTarget::FullPage,
        })
        .unwrap();
    let mut raster = OnDemandRasterProcess::new(SoftwareRasterProcessFactory);
    let image = raster.render(&element).unwrap();
    drop(network_guard);

    assert_eq!(element.target, CaptureTarget::Element(locator));
    assert_eq!(element.scene.capture_bounds.x(), 0);
    assert_eq!(element.scene.capture_bounds.y(), 150);
    assert_eq!(element.scene.capture_bounds.width(), 20);
    assert_eq!(element.scene.capture_bounds.height(), 10);
    assert_eq!(scroll.y, 60);
    assert_eq!(full_page.scene.capture_bounds.width(), 100);
    assert_eq!(full_page.scene.capture_bounds.height(), 160);
    assert_eq!(image.width(), 20);
    assert_eq!(image.height(), 10);
    assert!(
        image
            .rgba()
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 0, 0, 255])
    );
}

#[test]
fn package_blocks_screenshots_when_visible_paint_is_unsupported() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<main style="width:100px;height:20px;background-color:#fff">hello</main>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let result = session.execute(PrepareScreenshot {
        target: CaptureTarget::Viewport,
    });
    drop(network_guard);

    assert!(matches!(
        result,
        Err(SessionError::UnsupportedScreenshot { reason, .. }) if reason.contains("text paint")
    ));
}

#[test]
fn scoped_snapshots_keep_only_descendants_and_map_refs_to_source_elements() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="outside">Outside</button>
            <section id="scope">
                <input aria-label="Email">
                <button>Inside</button>
            </section>
            <section id="empty"><p>Nothing interactive</p></section>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let full = session.execute(CaptureInteractiveSnapshot).unwrap();
    let scoped = session
        .execute(CaptureInteractiveSnapshotWithin {
            locator: Locator::from(CssLocator::new("#scope").unwrap()),
        })
        .unwrap();
    let inside = session
        .execute(GetElementText {
            reference: scoped.elements[1].reference,
        })
        .unwrap();
    let missing_locator = Locator::from(CssLocator::new(".missing").unwrap());
    let missing = session.execute(CaptureInteractiveSnapshotWithin {
        locator: missing_locator.clone(),
    });
    let inside_after_failed_scope = session
        .execute(GetElementText {
            reference: scoped.elements[1].reference,
        })
        .unwrap();
    let stale = session.execute(GetElementText {
        reference: full.elements[0].reference,
    });
    let empty = session
        .execute(CaptureInteractiveSnapshotWithin {
            locator: Locator::from(CssLocator::new("#empty").unwrap()),
        })
        .unwrap();
    let target = session
        .execute(CaptureInteractiveSnapshotWithin {
            locator: Locator::from(CssLocator::new("#outside").unwrap()),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        scoped
            .elements
            .iter()
            .map(|element| (element.reference.to_string(), element.name.as_str()))
            .collect::<Vec<_>>(),
        vec![("@e2".into(), "Email"), ("@e3".into(), "Inside")]
    );
    assert_eq!(inside.text, "Inside");
    assert_eq!(inside_after_failed_scope.text, "Inside");
    assert_eq!(
        missing,
        Err(SessionError::LocatorNotFound {
            locator: missing_locator,
        })
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: full.elements[0].reference,
        })
    );
    assert!(empty.elements.is_empty());
    assert_eq!(target.elements.len(), 1);
    assert_eq!(target.elements[0].name, "Outside");
}

#[test]
fn role_locators_resolve_without_a_prior_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Save Draft</button><button>Publish</button><input aria-label="Email address">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let found = session
        .execute(FindByRole {
            locator: RoleLocator::new("BUTTON").unwrap().with_name("draft"),
        })
        .unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert_eq!(snapshot.id.get(), 1);
    assert_eq!(found.element, "button[1]");
    assert_eq!(found.role, "button");
    assert_eq!(found.name, "Save Draft");
    assert_eq!(found.text, "Save Draft");
}

#[test]
fn role_locators_use_descendant_image_alt_text_in_accessible_names() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save"><span><img alt="Save image"></span></button>
            <button id="presentational"><img role="presentation" alt="Ignored"></button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let found = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Save image"),
        })
        .unwrap();
    let ignored = session.execute(FindByRole {
        locator: RoleLocator::new("button")
            .unwrap()
            .with_exact_name("Ignored"),
    });
    drop(network_guard);

    assert_eq!(found.element, "save");
    assert_eq!(found.name, "Save image");
    assert_eq!(found.text, "");
    assert!(matches!(
        ignored,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
}

#[test]
fn role_locator_resolution_is_strict_and_transactional() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Save Draft</button><button>Save Changes</button><input aria-label="Email address">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let current = snapshot.elements[2].reference;

    let ambiguous_locator = RoleLocator::new("button").unwrap().with_name("save");
    let ambiguous = session.execute(FindByRole {
        locator: ambiguous_locator.clone(),
    });
    let missing_locator = RoleLocator::new("button")
        .unwrap()
        .with_exact_name("save draft");
    let missing = session.execute(FindByRole {
        locator: missing_locator.clone(),
    });
    let preserved = session.execute(GetElementValue { reference: current });
    drop(network_guard);

    assert_eq!(
        ambiguous,
        Err(SessionError::RoleLocatorAmbiguous {
            locator: ambiguous_locator,
            match_count: 2,
        })
    );
    assert_eq!(
        missing,
        Err(SessionError::RoleLocatorNotFound {
            locator: missing_locator,
        })
    );
    assert_eq!(preserved.unwrap().value, "");
}

#[test]
fn successful_role_resolution_preserves_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button><button>Publish</button>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let old_reference = snapshot.elements[0].reference;

    let found = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Publish"),
        })
        .unwrap();
    let preserved = session.execute(GetElementText {
        reference: old_reference,
    });
    drop(network_guard);

    assert_eq!(found.element, "button[2]");
    assert_eq!(found.text, "Publish");
    assert_eq!(preserved.unwrap().text, "Save");
}

#[test]
fn role_locators_resolve_structural_roles_and_role_specific_names() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <header aria-label="Site header"><h1>Home</h1></header>
            <main>
                <nav aria-labelledby="nav-name"><span id="nav-name">Primary</span><a href="/docs">Docs</a></nav>
                <h2>Skills</h2>
                <ul><li>Rust</li><li>Go</li></ul>
            </main>
            <footer aria-label="Site footer">Legal</footer>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let heading = session
        .execute(FindByRole {
            locator: RoleLocator::new("heading")
                .unwrap()
                .with_exact_name("Skills"),
        })
        .unwrap();
    let navigation = session
        .execute(FindByRole {
            locator: RoleLocator::new("navigation")
                .unwrap()
                .with_exact_name("Primary"),
        })
        .unwrap();
    let list = session
        .execute(FindByRole {
            locator: RoleLocator::new("list").unwrap(),
        })
        .unwrap();
    let banner_named_from_contents = session.execute(FindByRole {
        locator: RoleLocator::new("banner").unwrap().with_exact_name("Home"),
    });
    drop(network_guard);

    assert_eq!(heading.text, "Skills");
    assert_eq!(navigation.text, "PrimaryDocs");
    assert_eq!(list.name, "");
    assert_eq!(list.text, "Rust Go");
    assert!(matches!(
        banner_named_from_contents,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
}

#[test]
fn role_locators_cover_current_implicit_html_role_mappings() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <map><area href="/map" alt="Map target"></map>
            <table role="grid"><caption>Data</caption><tr><td>Cell</td></tr></table>
            <code>code</code><datalist id="cities"></datalist><dd>definition</dd>
            <del>old</del><dfn>term</dfn><em>emphasis</em><ins>new</ins><mark>mark</mark>
            <math></math><meter value="1" max="2"></meter><optgroup label="group"></optgroup>
            <search></search><strong>strong</strong><sub>sub</sub><sup>sup</sup><time>now</time>
            <input list="cities" aria-label="City"><input type="file" aria-label="Upload">
            <img alt=""><img alt="" title="Chart">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let expected_roles = [
        "link",
        "grid",
        "caption",
        "gridcell",
        "code",
        "listbox",
        "definition",
        "deletion",
        "term",
        "emphasis",
        "insertion",
        "mark",
        "math",
        "meter",
        "group",
        "search",
        "strong",
        "subscript",
        "superscript",
        "time",
        "combobox",
        "button",
        "presentation",
        "img",
    ];
    for role in expected_roles {
        let matched = session
            .execute(FindByRole {
                locator: RoleLocator::new(role).unwrap(),
            })
            .unwrap();
        assert_eq!(matched.role, role);
    }
    let area = session
        .execute(FindByRole {
            locator: RoleLocator::new("link")
                .unwrap()
                .with_exact_name("Map target"),
        })
        .unwrap();
    let titled_image = session
        .execute(FindByRole {
            locator: RoleLocator::new("img").unwrap().with_exact_name("Chart"),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(area.name, "Map target");
    assert_eq!(titled_image.name, "Chart");
}

#[test]
fn presentational_roles_yield_to_focus_and_global_aria_conflicts() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button role="presentation">Focusable</button>
            <h2 role="none" tabindex="-1">Heading</h2>
            <img role="presentation" alt="Busy chart" aria-busy="false">
            <code role="presentation" aria-label="Prohibited label">code</code>
            <button role="presentation" disabled>Disabled</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let button = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Focusable"),
        })
        .unwrap();
    let heading = session
        .execute(FindByRole {
            locator: RoleLocator::new("heading")
                .unwrap()
                .with_exact_name("Heading"),
        })
        .unwrap();
    let image = session
        .execute(FindByRole {
            locator: RoleLocator::new("img")
                .unwrap()
                .with_exact_name("Busy chart"),
        })
        .unwrap();
    let presentations = session
        .execute(CountByLocator {
            locator: Locator::from(RoleLocator::new("presentation").unwrap()),
        })
        .unwrap();
    let prohibited_name = session.execute(FindByRole {
        locator: RoleLocator::new("presentation")
            .unwrap()
            .with_exact_name("Prohibited label"),
    });
    drop(network_guard);

    assert_eq!(button.name, "Focusable");
    assert_eq!(heading.name, "Heading");
    assert_eq!(image.name, "Busy chart");
    assert_eq!(presentations.count, 2);
    assert!(matches!(
        prohibited_name,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
}

#[test]
fn role_locators_match_accessible_descriptions_in_precedence_order() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <p id="first" hidden>First hint</p>
            <p id="second" aria-label="Second hint">ignored</p>
            <button aria-describedby="first second" aria-description="lower priority" title="lowest priority">Save</button>
            <button aria-description="Opens settings">Settings</button>
            <button aria-label="Same" title="Same">ignored name</button>
            <button title="Title only"></button>
            <button aria-describedby="" aria-description="ignored fallback">Empty reference</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let described_by = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Save")
                .with_exact_description("First hint Second hint"),
        })
        .unwrap();
    let aria_description = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_description("OPENS SETTINGS"),
        })
        .unwrap();
    let title_not_used_for_name = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Same")
                .with_exact_description("Same"),
        })
        .unwrap();
    let title_used_for_name = session.execute(FindByRole {
        locator: RoleLocator::new("button")
            .unwrap()
            .with_exact_name("Title only")
            .with_exact_description("Title only"),
    });
    let empty_described_by_wins = session.execute(FindByRole {
        locator: RoleLocator::new("button")
            .unwrap()
            .with_exact_name("Empty reference")
            .with_exact_description("ignored fallback"),
    });
    drop(network_guard);

    assert_eq!(described_by.name, "Save");
    assert_eq!(aria_description.name, "Settings");
    assert_eq!(title_not_used_for_name.name, "Same");
    assert!(matches!(
        title_used_for_name,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
    assert!(matches!(
        empty_described_by_wins,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
}

#[test]
fn role_locators_filter_accessibility_state_and_current_control_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <h1>Overview</h1><h2 aria-level="1">Overview</h2>
            <label><input type="checkbox" checked>Terms</label>
            <button disabled>Save</button>
            <button aria-expanded="true">Menu</button>
            <button aria-expanded="invalid">Closed</button>
            <button>Pin</button>
            <div role="tab" aria-selected="TRUE">Details</div>
            <div role="application" aria-label="Editor" aria-expanded="true"></div>
            <div role="group" aria-disabled="true">
                <button aria-disabled="false">Override</button><button>Inherited</button>
            </div>
            <fieldset disabled>
                <legend><button>Legend</button></legend><button>Blocked</button>
            </fieldset>
            <div aria-hidden="true"><button>Ghost</button></div>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let heading = session
        .execute(FindByRole {
            locator: RoleLocator::new("heading")
                .unwrap()
                .with_exact_name("Overview")
                .with_level(2)
                .unwrap(),
        })
        .unwrap();
    let checked_locator = RoleLocator::new("checkbox")
        .unwrap()
        .with_exact_name("Terms")
        .with_checked(true)
        .unwrap();
    session
        .execute(SetCheckedByRole {
            locator: checked_locator,
            checked: false,
        })
        .unwrap();
    let unchecked = session
        .execute(FindByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Terms")
                .with_checked(false)
                .unwrap(),
        })
        .unwrap();
    let disabled = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Save")
                .with_disabled(true),
        })
        .unwrap();
    let expanded = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Menu")
                .with_expanded(true)
                .unwrap(),
        })
        .unwrap();
    let collapsed = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Closed")
                .with_expanded(false)
                .unwrap(),
        })
        .unwrap();
    let pressed = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Pin")
                .with_pressed(false)
                .unwrap(),
        })
        .unwrap();
    let selected = session
        .execute(FindByRole {
            locator: RoleLocator::new("tab")
                .unwrap()
                .with_exact_name("Details")
                .with_selected(true)
                .unwrap(),
        })
        .unwrap();
    let application = session
        .execute(FindByRole {
            locator: RoleLocator::new("application")
                .unwrap()
                .with_exact_name("Editor")
                .with_expanded(true)
                .unwrap(),
        })
        .unwrap();
    let disabled_override = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Override")
                .with_disabled(false),
        })
        .unwrap();
    let inherited_disabled = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Inherited")
                .with_disabled(true),
        })
        .unwrap();
    let legend_enabled = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Legend")
                .with_disabled(false),
        })
        .unwrap();
    let fieldset_disabled = session
        .execute(FindByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Blocked")
                .with_disabled(true),
        })
        .unwrap();
    let hidden_locator = RoleLocator::new("button").unwrap().with_exact_name("Ghost");
    let hidden = session.execute(FindByRole {
        locator: hidden_locator.clone(),
    });
    let included = session
        .execute(FindByRole {
            locator: hidden_locator.with_include_hidden(true),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(heading.role, "heading");
    assert_eq!(unchecked.name, "Terms");
    assert_eq!(disabled.name, "Save");
    assert_eq!(expanded.name, "Menu");
    assert_eq!(collapsed.name, "Closed");
    assert_eq!(pressed.name, "Pin");
    assert_eq!(selected.name, "Details");
    assert_eq!(application.name, "Editor");
    assert_eq!(disabled_override.name, "Override");
    assert_eq!(inherited_disabled.name, "Inherited");
    assert_eq!(legend_enabled.name, "Legend");
    assert_eq!(fieldset_disabled.name, "Blocked");
    assert!(matches!(
        hidden,
        Err(SessionError::RoleLocatorNotFound { .. })
    ));
    assert_eq!(included.name, "Ghost");
}

#[test]
fn role_locators_report_unknown_stylesheet_visibility() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<link rel="stylesheet" href="theme.css"><button>Save</button>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let locator = RoleLocator::new("button").unwrap().with_exact_name("Save");

    let unavailable = session.execute(FindByRole {
        locator: locator.clone(),
    });
    let included = session
        .execute(FindByRole {
            locator: locator.with_include_hidden(true),
        })
        .unwrap();
    drop(network_guard);

    assert!(matches!(
        unavailable,
        Err(SessionError::LocatorQuery { reason, .. })
            if reason.contains("linked stylesheet loading is not implemented")
    ));
    assert_eq!(included.name, "Save");
}

#[test]
fn role_actions_fill_and_check_without_capturing_a_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label>Email<input value="old"></label><label><input type="checkbox">Terms</label>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();
    let textbox_reference = before.elements[0].reference;
    let checkbox_reference = before.elements[1].reference;

    let filled = session
        .execute(FillByRole {
            locator: RoleLocator::new("textbox")
                .unwrap()
                .with_exact_name("Email"),
            value: "new value".into(),
        })
        .unwrap();
    let checked = session
        .execute(SetCheckedByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Terms"),
            checked: true,
        })
        .unwrap();
    let checked_again = session
        .execute(SetCheckedByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Terms"),
            checked: true,
        })
        .unwrap();
    let unchecked = session
        .execute(SetCheckedByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Terms"),
            checked: false,
        })
        .unwrap();
    let unchecked_again = session
        .execute(SetCheckedByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Terms"),
            checked: false,
        })
        .unwrap();
    let current_value = session
        .execute(GetElementValue {
            reference: textbox_reference,
        })
        .unwrap();
    let current_checked = session
        .execute(GetElementChecked {
            reference: checkbox_reference,
        })
        .unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    drop(network_guard);

    assert_eq!(filled.matched.element, "input[2]");
    assert_eq!(filled.value, "new value");
    assert_eq!(checked.matched.element, "input[4]");
    assert!(checked.checked);
    assert!(checked_again.checked);
    assert!(!unchecked.checked);
    assert!(!unchecked_again.checked);
    assert_eq!(current_value.value, "new value");
    assert!(!current_checked.checked);
    assert_eq!(before.id.get(), 1);
    assert_eq!(snapshot.id.get(), 2);
    assert_eq!(
        snapshot.elements[0].state,
        InteractiveElementState::Value("new value".into())
    );
    assert_eq!(
        snapshot.elements[1].state,
        InteractiveElementState::Checked(false)
    );
}

#[test]
fn role_actions_resolve_strictly_before_mutation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input aria-label="Email" value="first"><input aria-label="Email address" value="second">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;
    let locator = RoleLocator::new("textbox").unwrap().with_name("email");

    let result = session.execute(FillByRole {
        locator: locator.clone(),
        value: "changed".into(),
    });
    let preserved = session.execute(GetElementValue { reference: first });
    drop(network_guard);

    assert_eq!(
        result,
        Err(SessionError::RoleLocatorAmbiguous {
            locator,
            match_count: 2,
        })
    );
    assert_eq!(preserved.unwrap().value, "first");
}

#[test]
fn role_actions_report_actionability_and_unsupported_behavior() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input aria-label="Hidden" hidden><input aria-label="Disabled" disabled><button>Save</button><h1>Title</h1>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let hidden_locator = RoleLocator::new("textbox")
        .unwrap()
        .with_exact_name("Hidden")
        .with_include_hidden(true);
    let hidden = session.execute(FillByRole {
        locator: hidden_locator.clone(),
        value: "no".into(),
    });
    let disabled_locator = RoleLocator::new("textbox")
        .unwrap()
        .with_exact_name("Disabled");
    let disabled = session.execute(FillByRole {
        locator: disabled_locator.clone(),
        value: "no".into(),
    });
    let heading_locator = RoleLocator::new("heading")
        .unwrap()
        .with_exact_name("Title");
    let heading = session.execute(ClickByRole {
        locator: heading_locator.clone(),
    });
    let hover_locator = RoleLocator::new("button").unwrap().with_exact_name("Save");
    let hover = session.execute(HoverByRole {
        locator: hover_locator.clone(),
    });
    drop(network_guard);

    assert!(matches!(
        hidden,
        Err(SessionError::RoleActionBlocked {
            locator,
            action: RoleAction::Fill,
            check: ActionabilityCheck::Visible,
            ..
        }) if locator == hidden_locator
    ));
    assert!(matches!(
        disabled,
        Err(SessionError::RoleActionBlocked {
            locator,
            action: RoleAction::Fill,
            check: ActionabilityCheck::Editable,
            ..
        }) if locator == disabled_locator
    ));
    assert!(matches!(
        heading,
        Err(SessionError::UnsupportedRoleAction {
            locator,
            action: RoleAction::Click,
            ..
        }) if locator == heading_locator
    ));
    assert_eq!(hover.unwrap().matched.element, "button[3]");
}

#[test]
fn pointer_actions_require_supported_static_stability_evidence_before_mutation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="reference-click" style="position:fixed;left:0;top:0;width:100px;height:40px;transition:all 1s">Reference click</button>
            <input id="reference-check" type="checkbox" aria-label="Reference check" style="position:fixed;left:0;top:50px;width:20px;height:20px;transition:all 1s">
            <button id="locator-click" style="position:fixed;left:0;top:80px;width:100px;height:40px;transition:all 1s">Locator click</button>
            <input id="locator-check" type="checkbox" aria-label="Locator check" style="position:fixed;left:0;top:130px;width:20px;height:20px;transition:all 1s">
            <button id="reference-hover" style="position:fixed;left:0;top:160px;width:100px;height:40px;transition:all 1s">Reference hover</button>
            <div id="locator-hover" style="position:fixed;left:0;top:210px;width:100px;height:40px;transition:all 1s">Locator hover</div>
            <div id="moving-parent" style="transition:all 1s">
                <div id="nested-hover">Nested hover</div>
            </div>
            <a id="locator-link" href="/next" style="transition:all 1s">Moving link</a>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference_click = snapshot.elements[0].reference;
    let reference_check = snapshot.elements[1].reference;
    let reference_hover = snapshot.elements[4].reference;

    let click_by_reference = session.execute(ClickElement {
        reference: reference_click,
    });
    let click_locator = Locator::from(CssLocator::new("#locator-click").unwrap());
    let click_by_locator = session.execute(ClickByLocator {
        locator: click_locator.clone(),
    });
    let role_click_locator = RoleLocator::new("button")
        .unwrap()
        .with_exact_name("Locator click");
    let click_by_role = session.execute(ClickByRole {
        locator: role_click_locator.clone(),
    });
    let check_by_reference = session.execute(SetElementChecked {
        reference: reference_check,
        checked: true,
    });
    let check_locator = Locator::from(CssLocator::new("#locator-check").unwrap());
    let check_by_locator = session.execute(SetCheckedByLocator {
        locator: check_locator.clone(),
        checked: true,
    });
    let hover_by_reference = session.execute(HoverElement {
        reference: reference_hover,
    });
    let hover_locator = Locator::from(CssLocator::new("#locator-hover").unwrap());
    let hover_by_locator = session.execute(HoverByLocator {
        locator: hover_locator.clone(),
    });
    let nested_hover_locator = Locator::from(CssLocator::new("#nested-hover").unwrap());
    let nested_hover = session.execute(HoverByLocator {
        locator: nested_hover_locator.clone(),
    });
    let link_locator = Locator::from(CssLocator::new("#locator-link").unwrap());
    let link_click = session.execute(ClickByLocator {
        locator: link_locator.clone(),
    });
    let reference_checked = session
        .execute(GetElementChecked {
            reference: reference_check,
        })
        .unwrap();
    let locator_checked = session
        .execute(GetCheckedByLocator {
            locator: check_locator,
        })
        .unwrap();
    let reference_hovered = session
        .execute(GetElementHovered {
            reference: reference_hover,
        })
        .unwrap();
    let locator_hovered = session
        .execute(GetHoveredByLocator {
            locator: hover_locator.clone(),
        })
        .unwrap();
    let nested_hovered = session
        .execute(GetHoveredByLocator {
            locator: nested_hover_locator.clone(),
        })
        .unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    let current_url = session.execute(GetPageUrl).unwrap();
    drop(network_guard);

    assert!(
        matches!(
            &click_by_reference,
            Err(SessionError::UnsupportedClick { reference, reason })
                if *reference == reference_click
                    && reason == "stable check failed: inline transition stability is not implemented for reference-click"
        ),
        "unexpected click result: {click_by_reference:?}"
    );
    assert!(matches!(
        click_by_locator,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Click,
            check: ActionabilityCheck::Stable,
            reason,
        }) if locator == click_locator
            && reason == "inline transition stability is not implemented for locator-click"
    ));
    assert!(matches!(
        click_by_role,
        Err(SessionError::RoleActionBlocked {
            locator,
            action: RoleAction::Click,
            check: ActionabilityCheck::Stable,
            reason,
        }) if locator == role_click_locator
            && reason == "inline transition stability is not implemented for locator-click"
    ));
    assert!(matches!(
        check_by_reference,
        Err(SessionError::UnsupportedCheck { reference, reason })
            if reference == reference_check
                && reason == "stable check failed: inline transition stability is not implemented for reference-check"
    ));
    assert!(matches!(
        check_by_locator,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Check,
            check: ActionabilityCheck::Stable,
            reason,
            ..
        }) if reason == "inline transition stability is not implemented for locator-check"
    ));
    assert!(matches!(
        hover_by_reference,
        Err(SessionError::UnsupportedHover { reference, reason })
            if reference == reference_hover
                && reason == "stable check failed: inline transition stability is not implemented for reference-hover"
    ));
    assert!(matches!(
        hover_by_locator,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Hover,
            check: ActionabilityCheck::Stable,
            reason,
        }) if locator == hover_locator
            && reason == "inline transition stability is not implemented for locator-hover"
    ));
    assert!(matches!(
        nested_hover,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Hover,
            check: ActionabilityCheck::Stable,
            reason,
        }) if locator == nested_hover_locator
            && reason == "inline transition stability is not implemented for moving-parent"
    ));
    assert!(matches!(
        link_click,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Click,
            check: ActionabilityCheck::Stable,
            reason,
        }) if locator == link_locator
            && reason == "inline transition stability is not implemented for locator-link"
    ));
    assert!(!reference_checked.checked);
    assert!(!locator_checked.checked);
    assert!(!reference_hovered.hovered);
    assert!(!locator_hovered.hovered);
    assert!(!nested_hovered.hovered);
    assert!(events.is_empty());
    assert_eq!(current_url.url, url);
}

#[test]
fn clicking_by_role_navigates_and_invalidates_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![r#"<a href="/next">Next</a>"#, r#"<h1>Arrived</h1>"#]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let old_reference = snapshot.elements[0].reference;
    let locator = RoleLocator::new("link").unwrap().with_exact_name("Next");

    let result = session
        .execute(ClickByRole {
            locator: locator.clone(),
        })
        .unwrap();
    server.join().unwrap();
    let stale = session.execute(GetElementText {
        reference: old_reference,
    });
    drop(network_guard);

    assert!(matches!(
        result,
        ClickByRoleResult::Navigated { matched, page }
            if matched.name == "Next" && page.url == format!("{url}next")
    ));
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: old_reference,
        })
    );
}

#[test]
fn failed_role_navigation_preserves_the_page_and_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<a href="http://example.com/away">Away</a>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = snapshot.elements[0].reference;
    let locator = RoleLocator::new("link").unwrap().with_exact_name("Away");

    let result = session.execute(ClickByRole {
        locator: locator.clone(),
    });
    let preserved = session.execute(GetElementText { reference });
    drop(network_guard);

    assert!(matches!(
        result,
        Err(SessionError::RoleNavigation {
            locator: failed_locator,
            ..
        }) if failed_locator == locator
    ));
    assert_eq!(preserved.unwrap().text, "Away");
}

#[test]
fn role_actions_block_when_visibility_evidence_is_unavailable() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<link rel="stylesheet" href="/style.css"><input aria-label="Styled" value="old">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let locator = RoleLocator::new("textbox")
        .unwrap()
        .with_exact_name("Styled")
        .with_include_hidden(true);

    let result = session.execute(FillByRole {
        locator: locator.clone(),
        value: "changed".into(),
    });
    drop(network_guard);

    assert_eq!(
        result,
        Err(SessionError::RoleActionBlocked {
            locator,
            action: RoleAction::Fill,
            check: ActionabilityCheck::Visible,
            reason: "linked stylesheet loading is not implemented".into(),
        })
    );
}

#[test]
fn text_label_and_placeholder_locators_follow_user_facing_sources() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <label for="email">Email address</label><input id="email">
            <input id="username" aria-label="Username">
            <textarea id="search" placeholder="Search docs"></textarea>
            <div id="greeting">Hello <span id="world">world</span></div>
            <div id="hello">Hello</div>
            <input id="login" type="submit" value="Log in">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let email = session
        .execute(FindByLocator {
            locator: Locator::from(LabelLocator::new("email").unwrap()),
        })
        .unwrap();
    let username = session
        .execute(FindByLocator {
            locator: Locator::from(LabelLocator::new("Username").unwrap().exact()),
        })
        .unwrap();
    let search = session
        .execute(FindByLocator {
            locator: Locator::from(PlaceholderLocator::new("Search docs").unwrap().exact()),
        })
        .unwrap();
    let world = session
        .execute(FindByLocator {
            locator: Locator::from(TextLocator::new("world").unwrap().exact()),
        })
        .unwrap();
    let hello = session
        .execute(FindByLocator {
            locator: Locator::from(TextLocator::new("Hello").unwrap().exact()),
        })
        .unwrap();
    let submit = session
        .execute(FindByLocator {
            locator: Locator::from(TextLocator::new("Log in").unwrap().exact()),
        })
        .unwrap();
    let ambiguous_locator = Locator::from(TextLocator::new("hello").unwrap());
    let ambiguous = session.execute(FindByLocator {
        locator: ambiguous_locator.clone(),
    });
    drop(network_guard);

    assert_eq!(email.element, "email");
    assert_eq!(email.role.as_deref(), Some("textbox"));
    assert_eq!(username.element, "username");
    assert_eq!(search.element, "search");
    assert_eq!(world.element, "world");
    assert_eq!(world.role, None);
    assert_eq!(hello.element, "hello");
    assert_eq!(submit.element, "login");
    assert_eq!(submit.role.as_deref(), Some("button"));
    assert_eq!(
        ambiguous,
        Err(SessionError::LocatorAmbiguous {
            locator: ambiguous_locator,
            match_count: 2,
        })
    );
}

#[test]
fn locator_actions_fill_check_and_navigate_through_one_pipeline() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"
            <label for="email">Email address</label><input id="email" value="old">
            <input id="search" aria-label="Search" placeholder="Search docs">
            <label><input id="terms" type="checkbox">Accept terms</label>
            <a id="next" href="/next">Next page</a>
        "#,
        r#"<h1>Arrived</h1>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email_ref = snapshot.elements[0].reference;
    let search_ref = snapshot.elements[1].reference;
    let terms_ref = snapshot.elements[2].reference;

    let email = session
        .execute(FillByLocator {
            locator: Locator::from(LabelLocator::new("Email address").unwrap().exact()),
            value: "new address".into(),
        })
        .unwrap();
    let search = session
        .execute(FillByLocator {
            locator: Locator::from(PlaceholderLocator::new("search").unwrap()),
            value: "query".into(),
        })
        .unwrap();
    let terms = session
        .execute(SetCheckedByLocator {
            locator: Locator::from(LabelLocator::new("Accept terms").unwrap().exact()),
            checked: true,
        })
        .unwrap();
    let current_email = session.execute(GetElementValue {
        reference: email_ref,
    });
    let current_search = session.execute(GetElementValue {
        reference: search_ref,
    });
    let current_terms = session.execute(GetElementChecked {
        reference: terms_ref,
    });
    let navigation = session
        .execute(ClickByLocator {
            locator: Locator::from(TextLocator::new("Next page").unwrap().exact()),
        })
        .unwrap();
    server.join().unwrap();
    let stale = session.execute(GetElementValue {
        reference: email_ref,
    });
    drop(network_guard);

    assert_eq!(email.value, "new address");
    assert_eq!(email.matched.element, "email");
    assert_eq!(search.value, "query");
    assert!(terms.checked);
    assert_eq!(current_email.unwrap().value, "new address");
    assert_eq!(current_search.unwrap().value, "query");
    assert!(current_terms.unwrap().checked);
    assert!(matches!(
        navigation,
        ClickByLocatorResult::Navigated { matched, page }
            if matched.element == "next" && page.url == format!("{url}next")
    ));
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: email_ref,
        })
    );
}

#[test]
fn locator_actions_resolve_strictly_and_preserve_state_on_failure() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input aria-label="Email" value="first">
            <input aria-label="Email" value="second">
            <input aria-label="Hidden" placeholder="Secret" value="old" hidden>
            <button>Save</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;
    let save = snapshot.elements[3].reference;
    let ambiguous_locator = Locator::from(LabelLocator::new("Email").unwrap().exact());
    let hidden_locator = Locator::from(PlaceholderLocator::new("Secret").unwrap().exact());
    let hover_locator = Locator::from(TextLocator::new("Save").unwrap().exact());

    let ambiguous = session.execute(FillByLocator {
        locator: ambiguous_locator.clone(),
        value: "changed".into(),
    });
    let hidden = session.execute(FillByLocator {
        locator: hidden_locator.clone(),
        value: "changed".into(),
    });
    let hover = session.execute(HoverByLocator {
        locator: hover_locator.clone(),
    });
    let preserved = session.execute(GetElementValue { reference: first });
    let preserved_ref = session.execute(GetElementText { reference: save });
    drop(network_guard);

    assert_eq!(
        ambiguous,
        Err(SessionError::LocatorAmbiguous {
            locator: ambiguous_locator,
            match_count: 2,
        })
    );
    assert!(matches!(
        hidden,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Fill,
            check: ActionabilityCheck::Visible,
            ..
        }) if locator == hidden_locator
    ));
    assert_eq!(hover.unwrap().matched.element, "button[4]");
    assert_eq!(preserved.unwrap().value, "first");
    assert_eq!(preserved_ref.unwrap().text, "Save");
}

#[test]
fn hover_tracks_one_visible_target_across_reference_and_locator_paths() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="first">First</button>
            <div id="card">Card</div>
            <button id="disabled" disabled>Disabled</button>
            <button id="hidden" hidden>Hidden</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;
    let disabled = snapshot.elements[1].reference;
    let hidden = snapshot.elements[2].reference;

    let initial = session
        .execute(GetElementHovered { reference: first })
        .unwrap();
    let first_hover = session.execute(HoverElement { reference: first }).unwrap();
    let first_state = session
        .execute(GetElementHovered { reference: first })
        .unwrap();
    let other_state = session
        .execute(GetElementHovered {
            reference: disabled,
        })
        .unwrap();
    let card_locator = Locator::from(CssLocator::new("#card").unwrap());
    let card_hover = session
        .execute(HoverByLocator {
            locator: card_locator.clone(),
        })
        .unwrap();
    let card_state = session
        .execute(GetHoveredByLocator {
            locator: card_locator,
        })
        .unwrap();
    let old_state = session
        .execute(GetElementHovered { reference: first })
        .unwrap();
    let disabled_hover = session.execute(HoverElement {
        reference: disabled,
    });
    let hidden_hover = session.execute(HoverElement { reference: hidden });
    let preserved = session.execute(GetElementHovered {
        reference: disabled,
    });
    drop(network_guard);

    assert!(!initial.hovered);
    assert_eq!(first_hover.reference, first);
    assert!(first_state.hovered);
    assert!(!other_state.hovered);
    assert_eq!(card_hover.matched.element, "card");
    assert!(card_state.hovered);
    assert!(!old_state.hovered);
    assert_eq!(disabled_hover.unwrap().reference, disabled);
    assert!(matches!(
        hidden_hover,
        Err(SessionError::UnsupportedHover { reference, .. }) if reference == hidden
    ));
    assert!(preserved.unwrap().hovered);
}

#[test]
fn failed_locator_hover_preserves_the_previous_target_and_document_replacement_clears_it() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<button id="visible">Visible</button><button id="hidden" hidden>Hidden</button>"#,
        r#"<button id="visible">Visible</button><button id="hidden" hidden>Hidden</button>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    let visible = Locator::from(CssLocator::new("#visible").unwrap());
    let hidden = Locator::from(CssLocator::new("#hidden").unwrap());
    session
        .execute(HoverByLocator {
            locator: visible.clone(),
        })
        .unwrap();
    let blocked = session.execute(HoverByLocator {
        locator: hidden.clone(),
    });
    let preserved = session
        .execute(GetHoveredByLocator {
            locator: visible.clone(),
        })
        .unwrap();
    session.execute(ReloadPage).unwrap();
    server.join().unwrap();
    let cleared = session
        .execute(GetHoveredByLocator { locator: visible })
        .unwrap();
    drop(network_guard);

    assert!(matches!(
        blocked,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Hover,
            check: ActionabilityCheck::Visible,
            ..
        }) if locator == hidden
    ));
    assert!(preserved.hovered);
    assert!(!cleared.hovered);
}

#[test]
fn alt_title_and_test_id_locators_use_static_attributes() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"
            <img id="hero" alt="Product Image">
            <span id="count" title="Issue count">25 issues</span>
            <input id="email" data-testid="email-field" value="old">
            <div data-testid="duplicate">First</div>
            <div data-testid="duplicate">Second</div>
            <a id="next" data-testid="next-link" href="/next">Continue</a>
        "#,
        r#"<h1>Arrived</h1>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email_ref = snapshot.elements[0].reference;

    let image = session
        .execute(FindByLocator {
            locator: Locator::from(AltLocator::new("product image").unwrap()),
        })
        .unwrap();
    let count = session
        .execute(FindByLocator {
            locator: Locator::from(TitleLocator::new("Issue count").unwrap().exact()),
        })
        .unwrap();
    let email = session
        .execute(FillByLocator {
            locator: Locator::from(TestIdLocator::new("email-field").unwrap()),
            value: "new".into(),
        })
        .unwrap();
    let current_email = session.execute(GetElementValue {
        reference: email_ref,
    });
    let duplicate_locator = Locator::from(TestIdLocator::new("duplicate").unwrap());
    let duplicate = session.execute(FindByLocator {
        locator: duplicate_locator.clone(),
    });
    let navigation = session
        .execute(ClickByLocator {
            locator: Locator::from(TestIdLocator::new("next-link").unwrap()),
        })
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(image.element, "hero");
    assert_eq!(image.role.as_deref(), Some("img"));
    assert_eq!(count.element, "count");
    assert_eq!(count.text, "25 issues");
    assert_eq!(email.matched.element, "email");
    assert_eq!(current_email.unwrap().value, "new");
    assert_eq!(
        duplicate,
        Err(SessionError::LocatorAmbiguous {
            locator: duplicate_locator,
            match_count: 2,
        })
    );
    assert!(matches!(
        navigation,
        ClickByLocatorResult::Navigated { matched, page }
            if matched.element == "next" && page.url == format!("{url}next")
    ));
}

#[test]
fn css_position_locators_select_document_order_without_ambiguity() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="first" class="card field" data-kind="item" value="one">
            <input id="second" class="card field" data-kind="item" value="two">
            <input id="third" class="card field" data-kind="item" value="three">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first_ref = snapshot.elements[0].reference;
    let first = session
        .execute(FindByLocator {
            locator: Locator::from(CssLocator::first(".card").unwrap()),
        })
        .unwrap();
    let zero = session
        .execute(FindByLocator {
            locator: Locator::from(CssLocator::nth(0, ".card").unwrap()),
        })
        .unwrap();
    let last = session
        .execute(FindByLocator {
            locator: Locator::from(CssLocator::last("input.card[data-kind=item]").unwrap()),
        })
        .unwrap();
    let second = session
        .execute(FillByLocator {
            locator: Locator::from(CssLocator::nth(1, "input.field[data-kind='item']").unwrap()),
            value: "changed".into(),
        })
        .unwrap();
    let missing_locator = Locator::from(CssLocator::nth(3, ".card").unwrap());
    let missing = session.execute(FindByLocator {
        locator: missing_locator.clone(),
    });
    let preserved = session.execute(GetElementValue {
        reference: first_ref,
    });
    drop(network_guard);

    assert_eq!(first.element, "first");
    assert_eq!(zero.element, "first");
    assert_eq!(last.element, "third");
    assert_eq!(second.matched.element, "second");
    assert_eq!(second.value, "changed");
    assert_eq!(
        missing,
        Err(SessionError::LocatorNotFound {
            locator: missing_locator,
        })
    );
    assert_eq!(preserved.unwrap().value, "one");
}

#[test]
fn document_css_and_xpath_locators_share_strict_resolution_and_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form><input id="email" name="email" value="old"></form>
            <section data-kind="cards">
                <button id="first">One</button>
                <button id="second">Two</button>
            </section>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email_ref = snapshot.elements[0].reference;

    let filled = session
        .execute(FillByLocator {
            locator: Locator::from(
                CssLocator::new("form > input[name='email']:first-child").unwrap(),
            ),
            value: "changed".into(),
        })
        .unwrap();
    let xpath = session
        .execute(FindByLocator {
            locator: Locator::from(
                XPathLocator::new("//section[@data-kind='cards']/button[2]").unwrap(),
            ),
        })
        .unwrap();
    let ambiguous_locator = Locator::from(CssLocator::new("section > button").unwrap());
    let ambiguous = session.execute(FindByLocator {
        locator: ambiguous_locator.clone(),
    });
    let scalar_locator = Locator::from(XPathLocator::new("count(//button)").unwrap());
    let scalar = session.execute(FindByLocator {
        locator: scalar_locator.clone(),
    });
    let preserved = session.execute(GetElementValue {
        reference: email_ref,
    });
    drop(network_guard);

    assert_eq!(filled.matched.element, "email");
    assert_eq!(filled.value, "changed");
    assert_eq!(xpath.element, "second");
    assert_eq!(xpath.text, "Two");
    assert_eq!(
        ambiguous,
        Err(SessionError::LocatorAmbiguous {
            locator: ambiguous_locator,
            match_count: 2,
        })
    );
    assert!(matches!(
        scalar,
        Err(SessionError::LocatorQuery { locator, reason })
            if locator == scalar_locator && reason.contains("did not return only elements")
    ));
    assert_eq!(preserved.unwrap().value, "changed");
}

#[test]
fn locator_collections_return_document_order_and_allow_zero_matches() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <section class="cards">
                <button id="first" class="card">One</button>
                <button id="second" class="card">Two</button>
                <button id="third" class="card">Three</button>
            </section>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let all = session
        .execute(FindAllByLocator {
            locator: Locator::from(CssLocator::new("section.cards > .card").unwrap()),
        })
        .unwrap();
    let xpath_count = session
        .execute(CountByLocator {
            locator: Locator::from(XPathLocator::new("//section/button").unwrap()),
        })
        .unwrap();
    let missing = session
        .execute(CountByLocator {
            locator: Locator::from(CssLocator::new(".missing").unwrap()),
        })
        .unwrap();
    let positioned = session
        .execute(FindAllByLocator {
            locator: Locator::from(CssLocator::nth(1, ".card").unwrap()),
        })
        .unwrap();
    let scalar_locator = Locator::from(XPathLocator::new("count(//button)").unwrap());
    let scalar = session.execute(CountByLocator {
        locator: scalar_locator.clone(),
    });
    drop(network_guard);

    assert_eq!(
        all.matches
            .iter()
            .map(|matched| matched.element.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second", "third"]
    );
    assert_eq!(xpath_count.count, 3);
    assert_eq!(missing.count, 0);
    assert_eq!(positioned.matches.len(), 1);
    assert_eq!(positioned.matches[0].element, "second");
    assert!(matches!(
        scalar,
        Err(SessionError::LocatorQuery { locator, reason })
            if locator == scalar_locator && reason.contains("did not return only elements")
    ));
}

#[test]
fn locator_reads_and_form_actions_share_current_selector_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="email" value="old">
            <input id="terms" type="checkbox">
            <select id="size">
                <option value="small">Small</option>
                <option value="large">Large</option>
                <option value="blocked" disabled>Blocked</option>
            </select>
            <select id="many" multiple>
                <option value="a" selected>A</option>
                <option value="b">B</option>
                <option value="blocked" disabled>Blocked</option>
            </select>
            <select id="locked" disabled><option value="only">Only</option></select>
            <select id="invisible" hidden><option value="only">Only</option></select>
            <button id="disabled" disabled>Save</button>
            <div id="hidden" hidden>Hidden</div>
            <div id="card" data-kind="demo">Card</div>
            <input id="secret" type="password" value="private">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let email_locator = Locator::from(CssLocator::new("#email").unwrap());
    let terms_locator = Locator::from(CssLocator::new("#terms").unwrap());
    let size_locator = Locator::from(CssLocator::new("#size").unwrap());
    let many_locator = Locator::from(CssLocator::new("#many").unwrap());
    let initial_email = session
        .execute(GetValueByLocator {
            locator: email_locator,
        })
        .unwrap();
    let card_attribute = session
        .execute(GetAttributeByLocator {
            locator: Locator::from(CssLocator::new("#card").unwrap()),
            name: "DATA-KIND".into(),
        })
        .unwrap();
    let initial_checked = session
        .execute(GetCheckedByLocator {
            locator: terms_locator.clone(),
        })
        .unwrap();
    let disabled = session
        .execute(GetEnabledByLocator {
            locator: Locator::from(CssLocator::new("#disabled").unwrap()),
        })
        .unwrap();
    let hidden = session
        .execute(GetVisibleByLocator {
            locator: Locator::from(CssLocator::new("#hidden").unwrap()),
        })
        .unwrap();
    session
        .execute(SetCheckedByLocator {
            locator: terms_locator.clone(),
            checked: true,
        })
        .unwrap();
    let selected = session
        .execute(SelectByLocator {
            locator: size_locator.clone(),
            value: "large".into(),
        })
        .unwrap();
    let current_checked = session
        .execute(GetCheckedByLocator {
            locator: terms_locator,
        })
        .unwrap();
    let current_size = session
        .execute(GetValueByLocator {
            locator: size_locator.clone(),
        })
        .unwrap();
    let selected_many = session
        .execute(SelectOptionsByLocator {
            locator: many_locator.clone(),
            options: value_targets(&["b", "a"]),
        })
        .unwrap();
    let current_many = session
        .execute(GetValueByLocator {
            locator: many_locator.clone(),
        })
        .unwrap();
    let blocked_many = session.execute(SelectOptionsByLocator {
        locator: many_locator.clone(),
        options: value_targets(&["a", "blocked"]),
    });
    let current_many_after_failure = session
        .execute(GetValueByLocator {
            locator: many_locator.clone(),
        })
        .unwrap();
    let disabled_option = session.execute(SelectByLocator {
        locator: size_locator,
        value: "blocked".into(),
    });
    let locked_locator = Locator::from(CssLocator::new("#locked").unwrap());
    let locked = session.execute(SelectByLocator {
        locator: locked_locator.clone(),
        value: "only".into(),
    });
    let invisible_locator = Locator::from(CssLocator::new("#invisible").unwrap());
    let invisible = session.execute(SelectByLocator {
        locator: invisible_locator.clone(),
        value: "only".into(),
    });
    let secret_locator = Locator::from(CssLocator::new("#secret").unwrap());
    let secret = session.execute(GetAttributeByLocator {
        locator: secret_locator.clone(),
        name: "value".into(),
    });
    drop(network_guard);

    assert_eq!(initial_email.value, "old");
    assert_eq!(card_attribute.name, "data-kind");
    assert_eq!(card_attribute.value.as_deref(), Some("demo"));
    assert!(!initial_checked.checked);
    assert!(!disabled.enabled);
    assert!(!hidden.visible);
    assert!(current_checked.checked);
    assert_eq!(selected.value, "large");
    assert_eq!(current_size.value, "large");
    assert_eq!(
        selected_many.selected,
        NonEmpty::from_vec(vec!["b".into(), "a".into()]).unwrap()
    );
    assert_eq!(current_many.value, "a");
    assert_eq!(
        blocked_many,
        Err(SessionError::LocatorSelectOptionDisabled {
            locator: many_locator,
            value: "blocked".into(),
        })
    );
    assert_eq!(current_many_after_failure.value, "a");
    assert_eq!(
        disabled_option,
        Err(SessionError::LocatorSelectOptionDisabled {
            locator: Locator::from(CssLocator::new("#size").unwrap()),
            value: "blocked".into(),
        })
    );
    assert!(matches!(
        locked,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Select,
            check: ActionabilityCheck::Enabled,
            ..
        }) if locator == locked_locator
    ));
    assert!(matches!(
        invisible,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Select,
            check: ActionabilityCheck::Visible,
            ..
        }) if locator == invisible_locator
    ));
    assert_eq!(
        secret,
        Err(SessionError::SensitiveLocatorAttribute {
            locator: secret_locator,
            name: "value".into(),
        })
    );
}

#[test]
fn inner_html_reads_share_normalized_dom_and_block_sensitive_descendants() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <section id="card"><span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note --></section>
            <button id="action"><span>Save</span></button>
            <div id="secret" role="button"><input type="password" value="private"></div>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let expected = r#"<span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note -->"#;
    let css = session
        .execute(GetHtmlByLocator {
            locator: Locator::from(CssLocator::new("#card").unwrap()),
        })
        .unwrap();
    let xpath = session
        .execute(GetHtmlByLocator {
            locator: Locator::from(XPathLocator::new("//section[@id='card']").unwrap()),
        })
        .unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = session
        .execute(GetElementHtml {
            reference: snapshot.elements[0].reference,
        })
        .unwrap();
    let secret_locator = Locator::from(CssLocator::new("#secret").unwrap());
    let sensitive_locator = session.execute(GetHtmlByLocator {
        locator: secret_locator.clone(),
    });
    let sensitive_reference = session.execute(GetElementHtml {
        reference: snapshot.elements[1].reference,
    });
    drop(network_guard);

    assert_eq!(css.html, expected);
    assert_eq!(xpath.html, expected);
    assert_eq!(reference.html, "<span>Save</span>");
    assert!(matches!(
        sensitive_locator,
        Err(SessionError::UnsupportedLocatorInspection {
            locator,
            inspection: LocatorInspection::Html,
            reason,
        }) if locator == secret_locator && reason.contains("password value")
    ));
    assert!(matches!(
        sensitive_reference,
        Err(SessionError::UnsupportedHtml { reason, .. })
            if reason.contains("password value")
    ));
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
fn native_actions_record_data_minimized_dom_event_sequences() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"
            <main id="root">
                <label>Name<input id="name" value="old"></label>
                <input id="locked" value="fixed" readonly>
                <label><input id="terms" type="checkbox">Terms</label>
                <select id="size"><option value="s">Small</option><option value="l">Large</option></select>
                <button id="save" type="button">Save</button>
                <input id="starter" type="radio" name="plan" checked>
                <input id="pro" type="radio" name="plan">
                <a id="next" href="/next">Next</a>
            </main>
        "#,
        r#"<h1 id="arrived">Arrived</h1>"#,
    ]);
    let mut session = Session::new();
    assert!(session.execute(TakeDomEvents).unwrap().is_empty());
    session.execute(OpenPage { url }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let name = snapshot.elements[0].reference;
    let terms = snapshot.elements[2].reference;
    let size = snapshot.elements[3].reference;
    let save = snapshot.elements[4].reference;
    let pro = snapshot.elements[6].reference;
    let next = snapshot.elements[7].reference;

    session
        .execute(FillElement {
            reference: name,
            value: "updated".into(),
        })
        .unwrap();
    session
        .execute(FillElement {
            reference: name,
            value: "updated".into(),
        })
        .unwrap();
    let blocked_fill = session.execute(FillByLocator {
        locator: Locator::from(CssLocator::new("#locked").unwrap()),
        value: "do not record".into(),
    });
    assert!(blocked_fill.is_err());
    session
        .execute(SetElementChecked {
            reference: terms,
            checked: true,
        })
        .unwrap();
    session
        .execute(SetElementChecked {
            reference: terms,
            checked: true,
        })
        .unwrap();
    session
        .execute(SelectElement {
            reference: size,
            value: "l".into(),
        })
        .unwrap();
    session
        .execute(SelectElement {
            reference: size,
            value: "l".into(),
        })
        .unwrap();
    session.execute(ClickElement { reference: save }).unwrap();
    session.execute(ClickElement { reference: pro }).unwrap();
    session.execute(ClickElement { reference: pro }).unwrap();
    session.execute(ClickElement { reference: next }).unwrap();
    server.join().unwrap();

    let events = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    DomEventType::BeforeInput
                        | DomEventType::Change
                        | DomEventType::Click
                        | DomEventType::Input
                )
            })
            .map(|event| (event.event_type, event.target.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (DomEventType::BeforeInput, "name"),
            (DomEventType::Input, "name"),
            (DomEventType::BeforeInput, "name"),
            (DomEventType::Input, "name"),
            (DomEventType::Click, "terms"),
            (DomEventType::Input, "terms"),
            (DomEventType::Change, "terms"),
            (DomEventType::Input, "size"),
            (DomEventType::Change, "size"),
            (DomEventType::Input, "size"),
            (DomEventType::Change, "size"),
            (DomEventType::Click, "save"),
            (DomEventType::Click, "pro"),
            (DomEventType::Input, "pro"),
            (DomEventType::Change, "pro"),
            (DomEventType::Click, "pro"),
            (DomEventType::Click, "next"),
        ]
    );
    let document_epoch = events[0].document_epoch;
    assert!(events.iter().all(|event| {
        event.document_epoch == document_epoch
            && event.bubbles == event.event_type.bubbles()
            && event.composed == event.event_type.composed()
            && event.target_ordinal > 0
            && event.path.first() == Some(&event.target)
            && event.path.iter().any(|element| element == "root")
    }));
    assert!(session.execute(TakeDomEvents).unwrap().is_empty());
}

#[test]
fn native_clicks_focus_buttons_and_toggle_checkboxes_without_invalidating_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" type="button">Save</button>
            <label><input id="terms" type="checkbox">Accept terms</label>
            <input id="locked" type="checkbox" aria-label="Locked" disabled>
            <button id="hidden" type="button" hidden>Hidden</button>
            <form><button id="reset" type="reset">Reset</button></form>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let save = snapshot.elements[0].reference;
    let terms = snapshot.elements[1].reference;
    let locked = snapshot.elements[2].reference;
    let hidden = snapshot.elements[3].reference;

    let activated = session.execute(ClickElement { reference: save }).unwrap();
    let save_focused = session
        .execute(GetElementFocused { reference: save })
        .unwrap();
    let located_activation = session
        .execute(ClickByLocator {
            locator: Locator::from(CssLocator::new("#save").unwrap()),
        })
        .unwrap();
    let role_activation = session
        .execute(ClickByRole {
            locator: RoleLocator::new("button").unwrap().with_exact_name("Save"),
        })
        .unwrap();
    let checked = session
        .execute(ClickByRole {
            locator: RoleLocator::new("checkbox")
                .unwrap()
                .with_exact_name("Accept terms"),
        })
        .unwrap();
    let terms_focused = session
        .execute(GetElementFocused { reference: terms })
        .unwrap();
    let save_after_checkbox = session
        .execute(GetElementFocused { reference: save })
        .unwrap();
    let unchecked = session
        .execute(ClickByLocator {
            locator: Locator::from(CssLocator::new("#terms").unwrap()),
        })
        .unwrap();
    let current = session
        .execute(GetElementChecked { reference: terms })
        .unwrap();
    let disabled = session.execute(ClickElement { reference: locked });
    let invisible = session.execute(ClickElement { reference: hidden });
    let reset_locator = RoleLocator::new("button").unwrap().with_exact_name("Reset");
    let reset = session.execute(ClickByRole {
        locator: reset_locator.clone(),
    });
    let preserved = session.execute(GetElementText { reference: save }).unwrap();
    drop(network_guard);

    assert_eq!(activated, ClickResult::Activated { reference: save });
    assert!(save_focused.focused);
    assert!(matches!(
        located_activation,
        ClickByLocatorResult::Activated { matched } if matched.element == "save"
    ));
    assert!(matches!(
        role_activation,
        ClickByRoleResult::Activated { matched } if matched.name == "Save"
    ));
    assert!(matches!(
        checked,
        ClickByRoleResult::Checked { matched, checked: true }
            if matched.name == "Accept terms"
    ));
    assert!(terms_focused.focused);
    assert!(!save_after_checkbox.focused);
    assert!(matches!(
        unchecked,
        ClickByLocatorResult::Checked { matched, checked: false }
            if matched.element == "terms"
    ));
    assert!(!current.checked);
    assert!(matches!(
        disabled,
        Err(SessionError::UnsupportedClick { reference, .. }) if reference == locked
    ));
    assert!(matches!(
        invisible,
        Err(SessionError::UnsupportedClick { reference, .. }) if reference == hidden
    ));
    assert!(matches!(
        reset,
        Err(SessionError::UnsupportedRoleAction {
            locator,
            action: RoleAction::Click,
            ..
        }) if locator == reset_locator
    ));
    assert_eq!(preserved.text, "Save");
}

#[test]
fn get_form_click_serializes_current_successful_controls_and_navigates() {
    let network_guard = network_test_guard();
    let form = r#"
        <form action="/search?discard=old" method="get">
            <input id="query" name="q" value="old">
            <textarea id="note" name="note">old note</textarea>
            <input type="hidden" name="token" value="a b">
            <label><input id="rust" type="checkbox" name="tag" value="rust" checked>Rust</label>
            <label><input id="go" type="checkbox" name="tag" value="go">Go</label>
            <select name="size"><option>small</option><option value="large" selected>Large</option></select>
            <select name="multi" multiple>
                <option value="a" selected>A</option>
                <option value="skip" selected disabled>Skip</option>
                <option value="b" selected>B</option>
            </select>
            <input name="ignored" value="x" disabled>
            <input value="no-name">
            <button name="commit" value="save">Search</button>
        </form>
    "#;
    let (url, server) = serve_pages_recording_requests(vec![form, "<h1>Results</h1>"]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let old_reference = snapshot.elements[0].reference;
    session
        .execute(FillByLocator {
            locator: Locator::from(CssLocator::new("#query").unwrap()),
            value: "rust é".into(),
        })
        .unwrap();
    session
        .execute(FillByLocator {
            locator: Locator::from(CssLocator::new("#note").unwrap()),
            value: "line one\nline two".into(),
        })
        .unwrap();
    session
        .execute(SetCheckedByLocator {
            locator: Locator::from(CssLocator::new("#go").unwrap()),
            checked: true,
        })
        .unwrap();

    let result = session
        .execute(ClickByRole {
            locator: RoleLocator::new("button")
                .unwrap()
                .with_exact_name("Search"),
        })
        .unwrap();
    let requests = server.join().unwrap();
    let stale = session.execute(GetElementValue {
        reference: old_reference,
    });
    drop(network_guard);

    let query = "discard=old&q=rust+%C3%A9&note=line+one%0D%0Aline+two&token=a+b&tag=rust&tag=go&size=large&multi=a&multi=b&commit=save";
    let expected_url = format!("{url}search?{query}");
    assert!(matches!(
        result,
        ClickByRoleResult::Navigated { matched, page }
            if matched.name == "Search" && page.url == expected_url
    ));
    assert_eq!(requests[0], "GET / HTTP/1.1");
    assert_eq!(requests[1], format!("GET /search?{query} HTTP/1.1"));
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: old_reference,
        })
    );
}

#[test]
fn form_activation_keys_use_submitter_overrides_and_external_form_controls() {
    let network_guard = network_test_guard();
    let form = r#"
        <form id="search" action="/ignored" method="post">
            <input name="q" value="one">
            <button id="submit" name="commit" value="go"
                formaction="/find?discard=old" formmethod="get">Go</button>
        </form>
        <input form="search" name="outside" value="two">
    "#;
    let (url, server) =
        serve_pages_recording_requests(vec![form, "<h1>Found</h1>", form, "<h1>Found</h1>"]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();

    let enter = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#submit").unwrap()),
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    session.execute(GoBack).unwrap();
    let space = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#submit").unwrap()),
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let requests = server.join().unwrap();
    drop(network_guard);

    let expected_url = format!("{url}find?discard=old&q=one&commit=go&outside=two");
    assert_eq!(enter.matched.element, "submit");
    assert_eq!(enter.press.navigated().unwrap().url, expected_url);
    assert_eq!(space.press.navigated().unwrap().url, expected_url);
    assert_eq!(
        requests[1],
        "GET /find?discard=old&q=one&commit=go&outside=two HTTP/1.1"
    );
    assert_eq!(requests[3], requests[1]);
}

#[test]
fn implicit_enter_uses_the_first_default_submitter_and_current_values() {
    let network_guard = network_test_guard();
    let form = r#"
        <button id="external" form="search" name="commit" value="external"
            formaction="/external">External</button>
        <form id="search" action="/fallback" method="get">
            <label for="query">Query</label>
            <input id="query" name="q" value="old">
            <button id="inside" name="commit" value="inside"
                formaction="/inside">Inside</button>
        </form>
    "#;
    let (url, server) = serve_pages_recording_requests(vec![form, "<h1>Results</h1>"]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let old_query = snapshot.elements[1].reference;
    session
        .execute(FillByLocator {
            locator: Locator::from(CssLocator::new("#query").unwrap()),
            value: "rust browser".into(),
        })
        .unwrap();

    let result = session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let stale = session.execute(GetElementValue {
        reference: old_query,
    });
    let requests = server.join().unwrap();
    drop(network_guard);

    let expected_url = format!("{url}external?commit=external&q=rust+browser");
    assert_eq!(result.navigated().unwrap().element.element, "query");
    assert_eq!(result.navigated().unwrap().url, expected_url);
    assert_eq!(requests[0], "GET / HTTP/1.1");
    assert_eq!(
        requests[1],
        "GET /external?commit=external&q=rust+browser HTTP/1.1"
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference {
            reference: old_query,
        })
    );
}

#[test]
fn implicit_enter_without_a_submitter_obeys_the_blocking_field_count() {
    let network_guard = network_test_guard();
    let form = r#"
        <form id="multi" action="/multi" method="get">
            <input id="first" name="first" value="one">
            <input id="second" name="second" value="two">
        </form>
        <form id="solo" action="/solo" method="get">
            <input id="solo-field" name="q" value="one">
            <textarea name="note">line one</textarea>
            <input type="hidden" name="token" value="a b">
        </form>
    "#;
    let (url, server) = serve_pages_recording_requests(vec![form, "<h1>Solo</h1>"]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;

    let ignored = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#first").unwrap()),
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let current_url = session.execute(GetPageUrl).unwrap();
    let preserved = session
        .execute(GetElementValue { reference: first })
        .unwrap();
    let navigated = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#solo-field").unwrap()),
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let requests = server.join().unwrap();
    drop(network_guard);

    assert_eq!(ignored.press.ignored().unwrap().element, "first");
    assert_eq!(current_url.url, url);
    assert_eq!(preserved.value, "one");
    assert_eq!(
        navigated.press.navigated().unwrap().url,
        format!("{url}solo?q=one&note=line+one&token=a+b")
    );
    assert_eq!(requests[0], "GET / HTTP/1.1");
    assert_eq!(
        requests[1],
        "GET /solo?q=one&note=line+one&token=a+b HTTP/1.1"
    );
}

#[test]
fn implicit_enter_preserves_state_for_disabled_defaults_and_unsupported_methods() {
    let network_guard = network_test_guard();
    let form = r#"
        <form id="disabled-default" action="/disabled" method="get">
            <input id="disabled-field" name="q" value="one">
            <button disabled name="commit" value="blocked">Blocked</button>
            <button name="commit" value="enabled">Enabled</button>
        </form>
        <form id="post" action="/post" method="post">
            <input id="post-field" name="q" value="two">
        </form>
        <input id="outside" value="three">
    "#;
    let (url, server) = serve_page(form);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let disabled_field = snapshot.elements[0].reference;
    let post_field = snapshot.elements[3].reference;
    let outside = snapshot.elements[4].reference;

    let ignored = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#disabled-field").unwrap()),
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let unsupported = session.execute(PressByLocator {
        locator: Locator::from(CssLocator::new("#post-field").unwrap()),
        key: KeyboardKey::new("Enter").unwrap(),
    });
    session
        .execute(FillElement {
            reference: outside,
            value: "updated".into(),
        })
        .unwrap();
    let outside_ignored = session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let first_value = session
        .execute(GetElementValue {
            reference: disabled_field,
        })
        .unwrap();
    let second_value = session
        .execute(GetElementValue {
            reference: post_field,
        })
        .unwrap();
    let current_url = session.execute(GetPageUrl).unwrap();
    drop(network_guard);

    assert_eq!(ignored.press.ignored().unwrap().element, "disabled-field");
    assert!(matches!(
        unsupported,
        Err(SessionError::UnsupportedLocatorAction {
            action: LocatorAction::Press,
            reason,
            ..
        }) if reason == "form method \"post\" is not implemented"
    ));
    assert_eq!(outside_ignored.ignored().unwrap().element, "outside");
    assert_eq!(first_value.value, "one");
    assert_eq!(second_value.value, "two");
    assert_eq!(current_url.url, url);
}

#[test]
fn unsupported_form_submission_modes_preserve_the_page_and_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form method="post"><button>Post</button></form>
            <form><input type="file" name="upload"><button>Upload</button></form>
            <form action="http://example.com/away"><button>Remote</button></form>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let post_reference = snapshot.elements[0].reference;

    let post = session.execute(ClickByRole {
        locator: RoleLocator::new("button").unwrap().with_exact_name("Post"),
    });
    let upload = session.execute(ClickByRole {
        locator: RoleLocator::new("button")
            .unwrap()
            .with_exact_name("Upload"),
    });
    let remote = session.execute(ClickByRole {
        locator: RoleLocator::new("button")
            .unwrap()
            .with_exact_name("Remote"),
    });
    let preserved = session.execute(GetElementText {
        reference: post_reference,
    });
    drop(network_guard);

    assert!(matches!(
        post,
        Err(SessionError::UnsupportedRoleAction { reason, .. })
            if reason == "form method \"post\" is not implemented"
    ));
    assert!(matches!(
        upload,
        Err(SessionError::UnsupportedRoleAction { reason, .. })
            if reason == "file input form submission is not implemented"
    ));
    assert!(matches!(remote, Err(SessionError::RoleNavigation { .. })));
    assert_eq!(preserved.unwrap().text, "Post");
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
fn history_navigation_moves_transactionally_and_truncates_forward_entries() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>One</title><a href="/two">Next</a>"#,
        r#"<title>Two</title><button>Two</button>"#,
        r#"<title>One return</title><a href="/two">Next</a>"#,
        r#"<title>Two return</title><button>Two</button>"#,
        r#"<title>One again</title><a href="/two">Next</a>"#,
        r#"<title>Branch</title><button>Branch</button>"#,
    ]);
    let second_url = format!("{url}two");
    let branch_url = format!("{url}branch");
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let first = session.execute(CaptureInteractiveSnapshot).unwrap();
    let link = first.elements[0].reference;

    let no_previous = session.execute(GoBack).unwrap();
    let preserved = session.execute(GetElementText { reference: link }).unwrap();
    session.execute(ClickElement { reference: link }).unwrap();
    let stale = session.execute(GetElementText { reference: link });
    let back = session.execute(GoBack).unwrap();
    let back_title = session.execute(GetPageTitle).unwrap();
    let forward = session.execute(GoForward).unwrap();
    let forward_title = session.execute(GetPageTitle).unwrap();
    session.execute(GoBack).unwrap();
    session
        .execute(OpenPage {
            url: branch_url.clone(),
        })
        .unwrap();
    let no_forward = session.execute(GoForward).unwrap();
    let branch_title = session.execute(GetPageTitle).unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(
        no_previous,
        HistoryNavigationResult::NoEntry {
            current_url: url.clone(),
        }
    );
    assert_eq!(preserved.text, "Next");
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: link })
    );
    assert_eq!(
        back,
        HistoryNavigationResult::Navigated(browser_jr::OpenedPage {
            url: url.clone(),
            interactive_element_count: 1,
        })
    );
    assert_eq!(back_title.title, "One return");
    assert_eq!(
        forward,
        HistoryNavigationResult::Navigated(browser_jr::OpenedPage {
            url: second_url,
            interactive_element_count: 1,
        })
    );
    assert_eq!(forward_title.title, "Two return");
    assert_eq!(
        no_forward,
        HistoryNavigationResult::NoEntry {
            current_url: branch_url,
        }
    );
    assert_eq!(branch_title.title, "Branch");
}

#[test]
fn failed_history_load_preserves_the_page_reference_and_history_position() {
    let network_guard = network_test_guard();
    let (first_url, first_server) = serve_page(r#"<button>First</button>"#);
    let mut session = Session::new();
    session
        .execute(OpenPage {
            url: first_url.clone(),
        })
        .unwrap();
    first_server.join().unwrap();
    let (second_url, second_server) = serve_page(r#"<button>Second</button>"#);
    session
        .execute(OpenPage {
            url: second_url.clone(),
        })
        .unwrap();
    second_server.join().unwrap();
    let second = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = second.elements[0].reference;

    let failed = session.execute(GoBack);
    let current_url = session.execute(GetPageUrl).unwrap();
    let current_text = session.execute(GetElementText { reference }).unwrap();
    drop(network_guard);

    assert!(matches!(failed, Err(SessionError::Load(_))));
    assert_eq!(current_url.url, second_url);
    assert_eq!(current_text.text, "Second");
}

#[test]
fn reload_replaces_the_document_without_adding_a_history_entry() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>First</title><button>First</button>"#,
        r#"<title>Reloaded</title><button>Reloaded</button>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    session.execute(ReloadPage).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let reference = snapshot.elements[0].reference;

    let back = session.execute(GoBack).unwrap();
    let current = session.execute(GetElementText { reference }).unwrap();
    drop(network_guard);

    assert_eq!(back, HistoryNavigationResult::NoEntry { current_url: url });
    assert_eq!(current.text, "Reloaded");
}

#[test]
fn a_new_snapshot_invalidates_previous_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<div role="button">Save</div>"#);
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
        r#"<div role="button">Save</div><a href="/new" target="_blank">New</a><a href="/file" download>File</a>"#,
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
    let note_focused = session
        .execute(GetElementFocused { reference: note })
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
    let note_after_failed_fill = session
        .execute(GetElementFocused { reference: note })
        .unwrap();
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
    assert!(note_focused.focused);
    assert!(note_after_failed_fill.focused);
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
fn type_actions_append_text_and_preserve_failure_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <label for="email">Email</label><input id="email" value="old">
            <textarea id="note" aria-label="Note">draft</textarea>
            <input id="locked" aria-label="Locked" value="fixed" readonly>
            <button>Save</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email = before.elements[0].reference;
    let note = before.elements[1].reference;
    let locked = before.elements[2].reference;
    let button = before.elements[3].reference;

    let typed = session
        .execute(TypeElement {
            reference: email,
            text: " plus".into(),
        })
        .unwrap();
    let typed_by_locator = session
        .execute(TypeByLocator {
            locator: Locator::from(CssLocator::new("#email").unwrap()),
            text: " more".into(),
        })
        .unwrap();
    let empty = session
        .execute(TypeElement {
            reference: note,
            text: String::new(),
        })
        .unwrap();
    let locked_type = session.execute(TypeElement {
        reference: locked,
        text: " changed".into(),
    });
    let locked_locator = Locator::from(CssLocator::new("#locked").unwrap());
    let locked_type_by_locator = session.execute(TypeByLocator {
        locator: locked_locator.clone(),
        text: " changed".into(),
    });
    let unsupported = session.execute(TypeElement {
        reference: button,
        text: "ignored".into(),
    });
    let current_locked = session
        .execute(GetElementValue { reference: locked })
        .unwrap();
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(TypeElement {
        reference: email,
        text: " stale".into(),
    });
    drop(network_guard);

    assert_eq!(
        typed,
        TypeResult {
            reference: email,
            value: "old plus".into(),
        }
    );
    assert_eq!(typed_by_locator.value, "old plus more");
    assert_eq!(empty.value, "draft");
    assert_eq!(current_locked.value, "fixed");
    assert!(matches!(
        locked_type,
        Err(SessionError::UnsupportedType { reference, .. }) if reference == locked
    ));
    assert!(matches!(
        locked_type_by_locator,
        Err(SessionError::LocatorActionBlocked {
            locator,
            action: LocatorAction::Type,
            check: ActionabilityCheck::Editable,
            ..
        }) if locator == locked_locator
    ));
    assert!(matches!(
        unsupported,
        Err(SessionError::UnsupportedType { reference, .. }) if reference == button
    ));
    assert_eq!(
        after.elements[0].state,
        InteractiveElementState::Value("old plus more".into())
    );
    assert_eq!(
        after.elements[1].state,
        InteractiveElementState::Value("draft".into())
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: email })
    );
}

#[test]
fn focus_and_press_use_page_owned_focus_and_control_owned_selection() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <label for="email">Email</label><input id="email" value="old">
            <textarea id="note" aria-label="Note">draft</textarea>
            <input id="locked" aria-label="Locked" value="fixed" readonly>
            <input id="disabled" aria-label="Disabled" value="fixed" disabled>
            <button>Save</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email = snapshot.elements[0].reference;
    let locked = snapshot.elements[2].reference;
    let disabled = snapshot.elements[3].reference;
    let button = snapshot.elements[4].reference;

    assert_eq!(
        session.execute(PressKey {
            key: KeyboardKey::new("X").unwrap(),
        }),
        Err(SessionError::NoFocusedElement)
    );
    assert_eq!(
        session.execute(FocusElement { reference: email }).unwrap(),
        FocusResult {
            reference: email,
            element: "email".into(),
        }
    );
    let character = session
        .execute(PressKey {
            key: KeyboardKey::new("Z").unwrap(),
        })
        .unwrap();
    let note_focus = session
        .execute(FocusByLocator {
            locator: Locator::from(CssLocator::new("#note").unwrap()),
        })
        .unwrap();
    let enter = session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let unicode = session
        .execute(PressKey {
            key: KeyboardKey::new("é").unwrap(),
        })
        .unwrap();
    session.execute(FocusElement { reference: email }).unwrap();
    let rejected_focus = session.execute(FocusElement {
        reference: disabled,
    });
    let ambiguous_locator = Locator::from(RoleLocator::new("textbox").unwrap());
    let ambiguous_focus = session.execute(FocusByLocator {
        locator: ambiguous_locator.clone(),
    });
    let after_rejected_focus = session
        .execute(PressKey {
            key: KeyboardKey::new("Y").unwrap(),
        })
        .unwrap();
    session.execute(FocusElement { reference: locked }).unwrap();
    let readonly_press = session
        .execute(PressKey {
            key: KeyboardKey::new("Q").unwrap(),
        })
        .unwrap();
    session.execute(FocusElement { reference: button }).unwrap();
    let button_enter = session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    session.execute(FocusElement { reference: email }).unwrap();
    let space = session
        .execute(PressKey {
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let email_value = session
        .execute(GetElementValue { reference: email })
        .unwrap();
    drop(network_guard);

    let character = text_press(&character);
    let enter = text_press(&enter);
    let unicode = text_press(&unicode);
    let after_rejected_focus = text_press(&after_rejected_focus);
    let readonly_press = text_press(&readonly_press);
    let space = text_press(&space);

    assert_eq!(character.value, "Zold");
    assert!(character.changed);
    assert_eq!(
        (character.selection.start(), character.selection.end()),
        (1, 1)
    );
    assert_eq!(note_focus.matched.element, "note");
    assert_eq!(enter.value, "\ndraft");
    assert_eq!(unicode.value, "\nédraft");
    assert!(matches!(
        rejected_focus,
        Err(SessionError::UnsupportedFocus { reference, .. }) if reference == disabled
    ));
    assert!(matches!(
        ambiguous_focus,
        Err(SessionError::LocatorAmbiguous { locator, .. }) if locator == ambiguous_locator
    ));
    assert_eq!(after_rejected_focus.value, "ZYold");
    assert_eq!(readonly_press.value, "fixed");
    assert!(!readonly_press.changed);
    assert_eq!(
        (
            readonly_press.selection.start(),
            readonly_press.selection.end()
        ),
        (0, 0)
    );
    assert!(
        button_enter
            .activated()
            .unwrap()
            .element
            .starts_with("button")
    );
    assert_eq!(space.value, "ZY old");
    assert_eq!(email_value.value, "ZY old");
    assert_eq!(KeyboardKey::new(" "), KeyboardKey::new("Space"));
    assert!(KeyboardKey::new("").is_err());
    assert!(KeyboardKey::new("Backspace").is_ok());
    assert!(KeyboardKey::new("Escape").is_err());
}

#[test]
fn control_activation_keys_share_native_click_state_and_preserve_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" type="button">Save</button>
            <label><input id="terms" type="checkbox">Accept terms</label>
            <form><button id="reset" type="reset">Reset</button></form>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let save = snapshot.elements[0].reference;
    let terms = snapshot.elements[1].reference;

    let button_space = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#save").unwrap()),
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let checkbox_space = session
        .execute(PressByLocator {
            locator: Locator::from(
                RoleLocator::new("checkbox")
                    .unwrap()
                    .with_exact_name("Accept terms"),
            ),
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let checkbox_again = session
        .execute(PressKey {
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let checkbox_enter = session.execute(PressKey {
        key: KeyboardKey::new("Enter").unwrap(),
    });
    let form_button = session.execute(PressByLocator {
        locator: Locator::from(CssLocator::new("#reset").unwrap()),
        key: KeyboardKey::new("Enter").unwrap(),
    });
    let current = session
        .execute(GetElementChecked { reference: terms })
        .unwrap();
    let preserved = session.execute(GetElementText { reference: save }).unwrap();
    drop(network_guard);

    assert_eq!(button_space.matched.element, "save");
    assert_eq!(button_space.press.activated().unwrap().element, "save");
    assert_eq!(checkbox_space.matched.element, "terms");
    assert!(checkbox_space.press.checked().unwrap().1);
    assert!(!checkbox_again.checked().unwrap().1);
    assert!(matches!(
        checkbox_enter,
        Err(SessionError::UnsupportedPress { element, .. }) if element == "terms"
    ));
    assert!(matches!(
        form_button,
        Err(SessionError::UnsupportedLocatorAction {
            action: LocatorAction::Press,
            reason,
            ..
        }) if reason == "form reset is not implemented"
    ));
    assert!(!current.checked);
    assert_eq!(preserved.text, "Save");
}

#[test]
fn link_enter_navigates_through_locator_and_current_focus_paths() {
    let network_guard = network_test_guard();
    let first = r#"<a id="next" href="/next">Next</a>"#;
    let destination = r#"<title>Arrived</title><h1>Arrived</h1>"#;
    let (url, server) = serve_pages(vec![first, destination, first, destination]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let first_snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let old_link = first_snapshot.elements[0].reference;

    let locator_press = session
        .execute(PressByLocator {
            locator: Locator::from(RoleLocator::new("link").unwrap().with_exact_name("Next")),
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let stale = session.execute(GetElementText {
        reference: old_link,
    });
    session.execute(GoBack).unwrap();
    let second_snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let current_link = second_snapshot.elements[0].reference;
    session
        .execute(FocusElement {
            reference: current_link,
        })
        .unwrap();
    let focused_press = session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    let title = session.execute(GetPageTitle).unwrap();
    server.join().unwrap();
    drop(network_guard);

    let locator_navigation = locator_press.press.navigated().unwrap();
    assert_eq!(locator_press.matched.element, "next");
    assert_eq!(locator_navigation.element.element, "next");
    assert_eq!(locator_navigation.url, format!("{url}next"));
    assert_eq!(locator_navigation.interactive_element_count, 1);
    assert!(matches!(
        stale,
        Err(SessionError::StaleElementReference { reference }) if reference == old_link
    ));
    let focused_navigation = focused_press.navigated().unwrap();
    assert_eq!(focused_navigation.element.element, "next");
    assert_eq!(focused_navigation.url, format!("{url}next"));
    assert_eq!(title.title, "Arrived");
}

#[test]
fn failed_link_enter_preserves_the_page_focus_and_reference() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<a id="next" href="/missing">Next</a>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let link = snapshot.elements[0].reference;
    session.execute(FocusElement { reference: link }).unwrap();

    let failed = session.execute(PressKey {
        key: KeyboardKey::new("Enter").unwrap(),
    });
    let current_url = session.execute(GetPageUrl).unwrap();
    let current_text = session.execute(GetElementText { reference: link }).unwrap();
    let current_focus = session
        .execute(GetElementFocused { reference: link })
        .unwrap();
    drop(network_guard);

    assert!(matches!(
        failed,
        Err(SessionError::PressNavigation { key, element, .. })
            if key == KeyboardKey::new("Enter").unwrap() && element == "next"
    ));
    assert_eq!(current_url.url, url);
    assert_eq!(current_text.text, "Next");
    assert!(current_focus.focused);
}

#[test]
fn focused_state_reads_follow_page_focus_and_strict_locators() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <button id="first">First</button>
                <label for="second">Second</label>
                <input id="second" placeholder="Work email" data-testid="second-input" title="Current field">
                <div id="plain">Plain</div>
                <img alt="Avatar">
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;
    let second = snapshot.elements[1].reference;

    let initial_first = session
        .execute(GetElementFocused { reference: first })
        .unwrap();
    let initial_body = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        })
        .unwrap();
    assert!(!initial_first.focused);
    assert!(initial_body.focused);

    session
        .execute(FocusByLocator {
            locator: Locator::from(
                RoleLocator::new("textbox")
                    .unwrap()
                    .with_exact_name("Second"),
            ),
        })
        .unwrap();
    let first_state = session
        .execute(GetElementFocused { reference: first })
        .unwrap();
    let second_state = session
        .execute(GetElementFocused { reference: second })
        .unwrap();
    let semantic_state = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(
                RoleLocator::new("textbox")
                    .unwrap()
                    .with_exact_name("Second"),
            ),
        })
        .unwrap();
    let structural_state = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("#plain").unwrap()),
        })
        .unwrap();
    let body_while_element_focused = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        })
        .unwrap();
    assert!(!first_state.focused);
    assert!(second_state.focused);
    assert_eq!(semantic_state.matched.element, "second");
    assert!(semantic_state.focused);
    assert_eq!(structural_state.matched.element, "plain");
    assert!(!structural_state.focused);
    assert!(!body_while_element_focused.focused);

    let focused_locators = [
        Locator::from(
            RoleLocator::new("textbox")
                .unwrap()
                .with_exact_name("Second"),
        ),
        Locator::from(LabelLocator::new("Second").unwrap().exact()),
        Locator::from(PlaceholderLocator::new("Work email").unwrap().exact()),
        Locator::from(TestIdLocator::new("second-input").unwrap()),
        Locator::from(TitleLocator::new("Current field").unwrap().exact()),
        Locator::from(CssLocator::new("#second").unwrap()),
        Locator::from(XPathLocator::new("//input[@id='second']").unwrap()),
        Locator::from(CssLocator::first("input").unwrap()),
        Locator::from(CssLocator::last("input").unwrap()),
        Locator::from(CssLocator::nth(0, "input").unwrap()),
    ];
    for locator in focused_locators {
        assert!(
            session
                .execute(GetFocusedByLocator { locator })
                .unwrap()
                .focused
        );
    }
    let structural_locators = [
        Locator::from(TextLocator::new("Plain").unwrap().exact()),
        Locator::from(AltLocator::new("Avatar").unwrap().exact()),
    ];
    for locator in structural_locators {
        assert!(
            !session
                .execute(GetFocusedByLocator { locator })
                .unwrap()
                .focused
        );
    }

    session
        .execute(PressKey {
            key: KeyboardKey::new("Tab").unwrap(),
        })
        .unwrap();
    let second_after_tab = session
        .execute(GetElementFocused { reference: second })
        .unwrap();
    let body_after_tab = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        })
        .unwrap();
    assert!(!second_after_tab.focused);
    assert!(body_after_tab.focused);

    session.execute(FocusElement { reference: first }).unwrap();
    let fresh_snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(GetElementFocused { reference: first });
    let fresh = session
        .execute(GetElementFocused {
            reference: fresh_snapshot.elements[0].reference,
        })
        .unwrap();
    drop(network_guard);

    assert!(matches!(
        stale,
        Err(SessionError::StaleElementReference { reference }) if reference == first
    ));
    assert!(fresh.focused);

    let mut empty = Session::new();
    assert_eq!(
        empty.execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        }),
        Err(SessionError::NoPage)
    );
}

#[test]
fn editing_keys_match_playwright_caret_and_selection_boundaries() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="plain" value="abc">
            <input id="unicode" value="a😀b">
            <textarea id="note">ab
cd</textarea>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let unicode = snapshot.elements[1].reference;
    let note = snapshot.elements[2].reference;

    session.execute(FocusElement { reference: plain }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowRight").unwrap(),
        })
        .unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Shift+ArrowRight").unwrap(),
        })
        .unwrap();
    let replacement = session
        .execute(PressKey {
            key: KeyboardKey::new("X").unwrap(),
        })
        .unwrap();
    let replacement = text_press(&replacement);
    assert_eq!(replacement.value, "aXc");
    assert_eq!(
        (replacement.selection.start(), replacement.selection.end()),
        (2, 2)
    );

    session
        .execute(PressKey {
            key: KeyboardKey::new("Home").unwrap(),
        })
        .unwrap();
    let deleted = session
        .execute(PressKey {
            key: KeyboardKey::new("Delete").unwrap(),
        })
        .unwrap();
    let deleted = text_press(&deleted);
    assert_eq!(deleted.value, "Xc");
    session
        .execute(PressKey {
            key: KeyboardKey::new("End").unwrap(),
        })
        .unwrap();
    let backspaced = session
        .execute(PressKey {
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    let backspaced = text_press(&backspaced);
    assert_eq!(backspaced.value, "X");

    session
        .execute(PressKey {
            key: KeyboardKey::new("ControlOrMeta+A").unwrap(),
        })
        .unwrap();
    let selected_all = session
        .execute(PressKey {
            key: KeyboardKey::new("Z").unwrap(),
        })
        .unwrap();
    let selected_all = text_press(&selected_all);
    assert_eq!(selected_all.value, "Z");

    session
        .execute(TypeElement {
            reference: plain,
            text: "tail".into(),
        })
        .unwrap();
    let after_type = session
        .execute(PressKey {
            key: KeyboardKey::new("X").unwrap(),
        })
        .unwrap();
    let after_type = text_press(&after_type);
    assert_eq!(after_type.value, "ZXtail");
    session
        .execute(FillElement {
            reference: plain,
            value: "hi😀".into(),
        })
        .unwrap();
    let after_fill = session
        .execute(PressKey {
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    let after_fill = text_press(&after_fill);
    assert_eq!(after_fill.value, "hi");
    assert_eq!(
        (after_fill.selection.start(), after_fill.selection.end()),
        (2, 2)
    );

    session
        .execute(FocusElement { reference: unicode })
        .unwrap();
    for _ in 0..2 {
        session
            .execute(PressKey {
                key: KeyboardKey::new("ArrowRight").unwrap(),
            })
            .unwrap();
    }
    let unicode_delete = session
        .execute(PressKey {
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    let unicode_delete = text_press(&unicode_delete);
    assert_eq!(unicode_delete.value, "ab");
    assert_eq!(
        (
            unicode_delete.selection.start(),
            unicode_delete.selection.end()
        ),
        (1, 1)
    );

    session.execute(FocusElement { reference: note }).unwrap();
    for _ in 0..4 {
        session
            .execute(PressKey {
                key: KeyboardKey::new("ArrowRight").unwrap(),
            })
            .unwrap();
    }
    let home = session
        .execute(PressKey {
            key: KeyboardKey::new("Home").unwrap(),
        })
        .unwrap();
    let home = text_press(&home);
    assert_eq!((home.selection.start(), home.selection.end()), (3, 3));
    let end = session
        .execute(PressKey {
            key: KeyboardKey::new("End").unwrap(),
        })
        .unwrap();
    let end = text_press(&end);
    assert_eq!((end.selection.start(), end.selection.end()), (5, 5));
    drop(network_guard);
}

#[test]
fn focused_keyboard_text_uses_selection_and_preserves_noneditable_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input id="plain" value="abc"><input id="locked" value="fixed" readonly><button id="save">Save</button>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let locked = snapshot.elements[1].reference;
    let button = snapshot.elements[2].reference;

    let body = session
        .execute(KeyboardType {
            text: "ignored".into(),
        })
        .unwrap();
    assert_eq!(body.effect, KeyboardTextEffect::Ignored { element: None });

    session.execute(FocusElement { reference: plain }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowRight").unwrap(),
        })
        .unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Shift+ArrowRight").unwrap(),
        })
        .unwrap();
    let inserted = session
        .execute(KeyboardInsertText { text: "X".into() })
        .unwrap();
    let inserted = inserted.text().unwrap();
    assert_eq!(inserted.value, "aXc");
    assert_eq!(
        (inserted.selection.start(), inserted.selection.end()),
        (2, 2)
    );
    assert!(inserted.changed);

    let typed = session
        .execute(KeyboardType {
            text: "😀".into()
        })
        .unwrap();
    let typed = typed.text().unwrap();
    assert_eq!(typed.value, "aX😀c");
    assert_eq!((typed.selection.start(), typed.selection.end()), (4, 4));
    assert!(typed.changed);

    let empty = session
        .execute(KeyboardInsertText {
            text: String::new(),
        })
        .unwrap();
    let empty = empty.text().unwrap();
    assert_eq!(empty.value, "aX😀c");
    assert_eq!((empty.selection.start(), empty.selection.end()), (4, 4));
    assert!(!empty.changed);

    session.execute(FocusElement { reference: locked }).unwrap();
    let readonly = session.execute(KeyboardType { text: "Q".into() }).unwrap();
    let readonly = readonly.text().unwrap();
    assert_eq!(readonly.value, "fixed");
    assert_eq!(
        (readonly.selection.start(), readonly.selection.end()),
        (0, 0)
    );
    assert!(!readonly.changed);

    session.execute(FocusElement { reference: button }).unwrap();
    let ignored = session
        .execute(KeyboardInsertText { text: "!".into() })
        .unwrap();
    assert!(matches!(
        ignored.effect,
        KeyboardTextEffect::Ignored { element: Some(element) } if element.element == "save"
    ));
    drop(network_guard);

    assert_eq!(
        Session::new().execute(KeyboardType { text: "x".into() }),
        Err(SessionError::NoPage)
    );
}

#[test]
fn keyboard_insert_text_records_editable_and_readonly_native_events() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="root"><input id="plain"><input id="locked" readonly><button id="save">Save</button></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let locked = snapshot.elements[1].reference;
    let save = snapshot.elements[2].reference;

    session.execute(FocusElement { reference: plain }).unwrap();
    session
        .execute(KeyboardInsertText {
            text: "private".into(),
        })
        .unwrap();
    session
        .execute(KeyboardInsertText {
            text: String::new(),
        })
        .unwrap();
    let editable_events = session.execute(TakeDomEvents).unwrap();
    assert_eq!(
        editable_events
            .iter()
            .map(|event| (event.event_type, event.target.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (DomEventType::BeforeInput, "plain"),
            (DomEventType::Input, "plain")
        ]
    );

    session.execute(FocusElement { reference: locked }).unwrap();
    session
        .execute(KeyboardInsertText {
            text: "blocked".into(),
        })
        .unwrap();
    session.execute(FocusElement { reference: save }).unwrap();
    session
        .execute(KeyboardInsertText {
            text: "ignored".into(),
        })
        .unwrap();
    let readonly_events = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);

    assert_eq!(readonly_events.len(), 1);
    assert_eq!(readonly_events[0].event_type, DomEventType::BeforeInput);
    assert_eq!(readonly_events[0].target, "locked");
    assert_eq!(
        readonly_events[0].path.first().map(String::as_str),
        Some("locked")
    );
    assert!(
        readonly_events[0]
            .path
            .iter()
            .any(|target| target == "root")
    );
}

#[test]
fn keyboard_type_records_portable_per_scalar_native_events() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="root"><input id="plain"><input id="locked" readonly><button id="save">Save</button></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let locked = snapshot.elements[1].reference;
    let save = snapshot.elements[2].reference;

    session.execute(FocusElement { reference: plain }).unwrap();
    let typed = session
        .execute(KeyboardType {
            text: "aé😀".into(),
        })
        .unwrap();
    assert_eq!(typed.text().unwrap().value, "aé😀");
    assert_eq!(
        session
            .execute(TakeDomEvents)
            .unwrap()
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::BeforeInput,
            DomEventType::Input,
        ]
    );

    session.execute(FocusElement { reference: locked }).unwrap();
    let readonly = session.execute(KeyboardType { text: "aé".into() }).unwrap();
    assert!(!readonly.text().unwrap().changed);
    let readonly_events = session.execute(TakeDomEvents).unwrap();
    assert_eq!(
        readonly_events
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
        ]
    );
    assert!(
        readonly_events
            .iter()
            .all(|event| { event.target == "locked" && event.bubbles && event.composed })
    );

    session.execute(FocusElement { reference: save }).unwrap();
    session
        .execute(KeyboardType {
            text: "ignored".into(),
        })
        .unwrap();
    session
        .execute(KeyboardType {
            text: String::new(),
        })
        .unwrap();
    drop(network_guard);
    assert!(session.execute(TakeDomEvents).unwrap().is_empty());
}

#[test]
fn keyboard_type_normalizes_line_breaks_for_text_controls() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<input id="plain"><textarea id="note"></textarea>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let note = snapshot.elements[1].reference;

    session.execute(FocusElement { reference: plain }).unwrap();
    let single_line = session
        .execute(KeyboardType {
            text: "a\r\nb".into(),
        })
        .unwrap();
    assert_eq!(single_line.text().unwrap().value, "ab");
    assert_eq!(
        session
            .execute(TakeDomEvents)
            .unwrap()
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
        ]
    );

    session.execute(FocusElement { reference: note }).unwrap();
    let multiline = session
        .execute(KeyboardType {
            text: "a\r\nb".into(),
        })
        .unwrap();
    assert_eq!(multiline.text().unwrap().value, "a\n\nb");
    let multiline_events = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);
    assert_eq!(multiline_events.len(), 20);
    assert_eq!(
        multiline_events
            .iter()
            .filter(|event| event.event_type == DomEventType::Input)
            .count(),
        4
    );
}

#[test]
fn complete_press_records_portable_text_and_native_control_events() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <main id="root">
                <input id="plain">
                <input id="locked" readonly>
                <button id="save" type="button">Save</button>
                <input id="terms" type="checkbox">
                <input id="selected" type="radio" checked>
            </main>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;
    let locked = snapshot.elements[1].reference;
    let save = snapshot.elements[2].reference;
    let terms = snapshot.elements[3].reference;
    let selected = snapshot.elements[4].reference;

    session.execute(FocusElement { reference: plain }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("x").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
        ]
    );
    session
        .execute(PressKey {
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
        ]
    );
    session
        .execute(PressKey {
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyUp]
    );

    session.execute(FocusElement { reference: locked }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Q").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
        ]
    );

    session.execute(FocusElement { reference: save }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Enter").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::Click,
            DomEventType::KeyUp,
        ]
    );
    session
        .execute(PressKey {
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
            DomEventType::Click,
        ]
    );

    session.execute(FocusElement { reference: terms }).unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
            DomEventType::Click,
            DomEventType::Input,
            DomEventType::Change,
        ]
    );

    session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#plain").unwrap()),
            key: KeyboardKey::new("y").unwrap(),
        })
        .unwrap();
    let locator_events = session.execute(TakeDomEvents).unwrap();
    assert_eq!(locator_events.len(), 5);
    assert!(locator_events.iter().all(|event| event.target == "plain"));

    session
        .execute(PressKey {
            key: KeyboardKey::new("é").unwrap(),
        })
        .unwrap();
    assert!(session.execute(TakeDomEvents).unwrap().is_empty());

    session
        .execute(FocusElement {
            reference: selected,
        })
        .unwrap();
    session
        .execute(PressKey {
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
        ]
    );

    session.execute(FocusElement { reference: plain }).unwrap();
    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("z").unwrap(),
        })
        .unwrap();
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("z").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::BeforeInput,
            DomEventType::Input,
            DomEventType::KeyUp,
        ]
    );

    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Shift").unwrap(),
        })
        .unwrap();
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Shift").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyUp]
    );

    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Tab").unwrap(),
        })
        .unwrap();
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Tab").unwrap(),
        })
        .unwrap();
    let traversal_events = session.execute(TakeDomEvents).unwrap();
    assert_eq!(
        traversal_events
            .iter()
            .map(|event| (event.event_type, event.target.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (DomEventType::KeyDown, "plain"),
            (DomEventType::KeyUp, "locked"),
        ]
    );

    session.execute(FocusElement { reference: save }).unwrap();
    let down = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(down.deferred);
    assert!(down.press.is_none());
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyPress]
    );
    let up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    drop(network_guard);
    assert!(up.was_pressed);
    assert_eq!(up.press.unwrap().activated().unwrap().element, "save");
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyUp, DomEventType::Click]
    );
}

#[test]
fn held_space_defers_native_state_and_cancels_after_focus_change() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" type="button">Save</button>
            <button id="other" type="button">Other</button>
            <input id="terms" type="checkbox">
            <input id="selected" type="radio" name="plan" checked>
            <input id="alternate" type="radio" name="plan">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let save = snapshot.elements[0].reference;
    let other = snapshot.elements[1].reference;
    let terms = snapshot.elements[2].reference;
    let selected = snapshot.elements[3].reference;
    let alternate = snapshot.elements[4].reference;

    session.execute(FocusElement { reference: save }).unwrap();
    let first_down = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    let repeated_down = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(!first_down.repeat);
    assert!(first_down.deferred);
    assert!(repeated_down.repeat);
    assert!(repeated_down.deferred);
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyDown,
            DomEventType::KeyPress,
        ]
    );
    let button_up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert_eq!(
        button_up.press.unwrap().activated().unwrap().element,
        "save"
    );
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyUp, DomEventType::Click]
    );

    session.execute(FocusElement { reference: terms }).unwrap();
    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(
        !session
            .execute(GetElementChecked { reference: terms })
            .unwrap()
            .checked
    );
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyPress]
    );
    let checkbox_up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(checkbox_up.press.unwrap().checked().unwrap().1);
    assert!(
        session
            .execute(GetElementChecked { reference: terms })
            .unwrap()
            .checked
    );
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyUp,
            DomEventType::Click,
            DomEventType::Input,
            DomEventType::Change,
        ]
    );

    session
        .execute(FocusElement {
            reference: alternate,
        })
        .unwrap();
    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(
        !session
            .execute(GetElementChecked {
                reference: alternate,
            })
            .unwrap()
            .checked
    );
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(
        session
            .execute(GetElementChecked {
                reference: alternate,
            })
            .unwrap()
            .checked
    );
    assert!(
        !session
            .execute(GetElementChecked {
                reference: selected,
            })
            .unwrap()
            .checked
    );
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
            DomEventType::Click,
            DomEventType::Input,
            DomEventType::Change,
        ]
    );

    session
        .execute(FocusElement {
            reference: alternate,
        })
        .unwrap();
    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    let selected_up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(selected_up.press.unwrap().checked().unwrap().1);
    assert_eq!(
        take_event_types(&mut session),
        vec![
            DomEventType::KeyDown,
            DomEventType::KeyPress,
            DomEventType::KeyUp,
        ]
    );

    session.execute(FocusElement { reference: save }).unwrap();
    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyPress]
    );
    session.execute(FocusElement { reference: other }).unwrap();
    let canceled = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    drop(network_guard);
    assert!(canceled.press.is_none());
    assert_eq!(
        session
            .execute(TakeDomEvents)
            .unwrap()
            .iter()
            .map(|event| (event.event_type, event.target.as_str()))
            .collect::<Vec<_>>(),
        vec![(DomEventType::KeyUp, "other")]
    );
}

#[test]
fn held_space_defers_submit_navigation_until_key_up() {
    let network_guard = network_test_guard();
    let form = r#"
        <form action="/sent" method="get">
            <button id="submit" name="commit" value="yes">Send</button>
        </form>
    "#;
    let (url, server) = serve_pages_recording_requests(vec![form, "<h1>Sent</h1>"]);
    let mut session = Session::new();
    session.execute(OpenPage { url: url.clone() }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let submit = snapshot.elements[0].reference;
    session.execute(FocusElement { reference: submit }).unwrap();

    let down = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    assert!(down.deferred);
    assert!(down.press.is_none());
    assert_eq!(session.execute(GetPageUrl).unwrap().url, url);
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyDown, DomEventType::KeyPress]
    );

    let up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Space").unwrap(),
        })
        .unwrap();
    let requests = server.join().unwrap();
    drop(network_guard);
    assert_eq!(
        up.press.unwrap().navigated().unwrap().url,
        format!("{url}sent?commit=yes")
    );
    assert_eq!(requests[1], "GET /sent?commit=yes HTTP/1.1");
    assert_eq!(
        take_event_types(&mut session),
        vec![DomEventType::KeyUp, DomEventType::Click]
    );
}

fn take_event_types(session: &mut Session) -> Vec<DomEventType> {
    session
        .execute(TakeDomEvents)
        .unwrap()
        .into_iter()
        .map(|event| event.event_type)
        .collect()
}

#[test]
fn held_keyboard_keys_apply_modifiers_and_report_repeat_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input id="plain" value="abc"><label for="other">Other</label><input id="other">"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let plain = snapshot.elements[0].reference;

    let unfocused = session.execute(KeyDown {
        key: KeyboardEventKey::new("x").unwrap(),
    });
    assert_eq!(unfocused, Err(SessionError::NoFocusedElement));
    let unfocused_up = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("x").unwrap(),
        })
        .unwrap();
    assert!(!unfocused_up.was_pressed);

    session.execute(FocusElement { reference: plain }).unwrap();
    let shift = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("ShiftLeft").unwrap(),
        })
        .unwrap();
    assert_eq!(shift.key.to_string(), "Shift");
    assert_eq!(shift.key.modifier(), Some(KeyboardModifier::Shift));
    assert!(!shift.repeat);
    assert!(shift.press.is_none());

    let selected = session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowRight").unwrap(),
        })
        .unwrap();
    assert_eq!(selected.key.to_string(), "Shift+ArrowRight");
    let selected = text_press(&selected);
    assert_eq!(
        (selected.selection.start(), selected.selection.end()),
        (0, 1)
    );

    let first = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("x").unwrap(),
        })
        .unwrap();
    assert!(!first.repeat);
    assert_eq!(first.press.as_ref().unwrap().key.to_string(), "X");
    assert_eq!(text_press(first.press.as_ref().unwrap()).value, "Xbc");

    let repeated = session
        .execute(KeyDown {
            key: KeyboardEventKey::new("x").unwrap(),
        })
        .unwrap();
    assert!(repeated.repeat);
    assert_eq!(text_press(repeated.press.as_ref().unwrap()).value, "XXbc");

    let released = session
        .execute(KeyUp {
            key: KeyboardEventKey::new("x").unwrap(),
        })
        .unwrap();
    assert!(released.was_pressed);
    assert!(
        !session
            .execute(KeyUp {
                key: KeyboardEventKey::new("x").unwrap(),
            })
            .unwrap()
            .was_pressed
    );
    assert!(
        session
            .execute(KeyUp {
                key: KeyboardEventKey::new("Shift").unwrap(),
            })
            .unwrap()
            .was_pressed
    );

    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("ControlOrMeta").unwrap(),
        })
        .unwrap();
    let select_all = session
        .execute(PressKey {
            key: KeyboardKey::new("a").unwrap(),
        })
        .unwrap();
    assert_eq!(select_all.key.to_string(), "ControlOrMeta+A");
    let select_all = text_press(&select_all);
    assert_eq!(
        (select_all.selection.start(), select_all.selection.end()),
        (0, 4)
    );
    let unsupported_modified = session.execute(KeyDown {
        key: KeyboardEventKey::new("b").unwrap(),
    });
    assert!(matches!(
        unsupported_modified,
        Err(SessionError::UnsupportedPress { key, element, reason })
            if key == KeyboardKey::new("b").unwrap()
                && element == "plain"
                && reason.contains("held modifier combination")
    ));
    assert!(
        !session
            .execute(KeyUp {
                key: KeyboardEventKey::new("b").unwrap(),
            })
            .unwrap()
            .was_pressed
    );
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("ControlOrMeta").unwrap(),
        })
        .unwrap();

    session
        .execute(KeyDown {
            key: KeyboardEventKey::new("Shift").unwrap(),
        })
        .unwrap();
    let locator_press = session
        .execute(PressByLocator {
            locator: Locator::from(LabelLocator::new("Other").unwrap()),
            key: KeyboardKey::new("a").unwrap(),
        })
        .unwrap();
    assert_eq!(locator_press.press.key.to_string(), "A");
    assert_eq!(text_press(&locator_press.press).value, "A");
    session
        .execute(KeyUp {
            key: KeyboardEventKey::new("Shift").unwrap(),
        })
        .unwrap();
    drop(network_guard);

    assert!(KeyboardEventKey::new("Shift+ArrowLeft").is_err());
    assert_eq!(
        Session::new().execute(KeyDown {
            key: KeyboardEventKey::new("Shift").unwrap(),
        }),
        Err(SessionError::NoPage)
    );
}

#[test]
fn press_by_locator_focuses_one_strict_target_before_editing() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <label for="first">First</label><input id="first" value="one">
            <label for="second">Second</label><input id="second" value="two">
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let first = snapshot.elements[0].reference;
    session.execute(FocusElement { reference: first }).unwrap();
    let second = Locator::from(LabelLocator::new("Second").unwrap().exact());

    let end = session
        .execute(PressByLocator {
            locator: second.clone(),
            key: KeyboardKey::new("End").unwrap(),
        })
        .unwrap();
    let backspace = session
        .execute(PressByLocator {
            locator: second.clone(),
            key: KeyboardKey::new("Backspace").unwrap(),
        })
        .unwrap();
    let missing_locator = Locator::from(LabelLocator::new("Missing").unwrap().exact());
    let missing = session.execute(PressByLocator {
        locator: missing_locator.clone(),
        key: KeyboardKey::new("X").unwrap(),
    });
    let after_missing = session
        .execute(PressKey {
            key: KeyboardKey::new("Y").unwrap(),
        })
        .unwrap();
    drop(network_guard);

    let end_effect = text_press(&end.press);
    let backspace_effect = text_press(&backspace.press);
    let after_missing = text_press(&after_missing);

    assert_eq!(end.matched.element, "second");
    assert_eq!(
        (end_effect.selection.start(), end_effect.selection.end()),
        (3, 3)
    );
    assert_eq!(backspace_effect.value, "tw");
    assert_eq!(
        missing,
        Err(SessionError::LocatorNotFound {
            locator: missing_locator,
        })
    );
    assert_eq!(after_missing.element.element, "second");
    assert_eq!(after_missing.value, "twY");
}

#[test]
fn tab_and_shift_tab_match_chromium_sequential_focus_order() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r##"
            <button id="natural1">one</button>
            <input id="negative" tabindex="-1">
            <input id="positive2" tabindex="2">
            <a id="link" href="#x">link</a>
            <input id="disabled" disabled>
            <input id="positive1" tabindex="1">
            <textarea id="natural2"></textarea>
            <div id="zero" role="button" tabindex="0">zero</div>
            <input id="hidden" hidden>
        "##,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let mut forward = Vec::new();
    for _ in 0..8 {
        let result = session
            .execute(PressKey {
                key: KeyboardKey::new("Tab").unwrap(),
            })
            .unwrap();
        forward.push(
            result
                .focus_traversal()
                .unwrap()
                .current
                .as_ref()
                .map(|element| element.element.as_str())
                .unwrap_or("body")
                .to_owned(),
        );
    }
    assert_eq!(
        forward,
        [
            "positive1",
            "positive2",
            "natural1",
            "link",
            "natural2",
            "zero",
            "body",
            "positive1",
        ]
    );

    let reverse_boundary = session
        .execute(PressKey {
            key: KeyboardKey::new("Shift+Tab").unwrap(),
        })
        .unwrap();
    assert!(
        reverse_boundary
            .focus_traversal()
            .unwrap()
            .current
            .is_none()
    );
    let reverse_from_body = session
        .execute(PressKey {
            key: KeyboardKey::new("Shift+Tab").unwrap(),
        })
        .unwrap();
    assert_eq!(
        reverse_from_body
            .focus_traversal()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .element,
        "zero"
    );

    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let locator_tab = session
        .execute(PressByLocator {
            locator: Locator::from(RoleLocator::new("button").unwrap().with_exact_name("one")),
            key: KeyboardKey::new("Tab").unwrap(),
        })
        .unwrap();
    assert_eq!(locator_tab.matched.element, "natural1");
    let locator_traversal = locator_tab.press.focus_traversal().unwrap();
    assert_eq!(
        locator_traversal.previous.as_ref().unwrap().element,
        "natural1"
    );
    assert_eq!(locator_traversal.current.as_ref().unwrap().element, "link");
    session
        .execute(FocusElement {
            reference: snapshot.elements[0].reference,
        })
        .unwrap();

    let negative = snapshot.elements[1].reference;
    session
        .execute(FocusElement {
            reference: negative,
        })
        .unwrap();
    let from_negative = session
        .execute(PressKey {
            key: KeyboardKey::new("Tab").unwrap(),
        })
        .unwrap();
    assert_eq!(
        from_negative
            .focus_traversal()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .element,
        "positive2"
    );
    session
        .execute(FocusElement {
            reference: negative,
        })
        .unwrap();
    let reverse_from_negative = session
        .execute(PressKey {
            key: KeyboardKey::new("Shift+Tab").unwrap(),
        })
        .unwrap();
    assert_eq!(
        reverse_from_negative
            .focus_traversal()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .element,
        "natural1"
    );
    drop(network_guard);

    assert!(KeyboardKey::new("Tab").is_ok());
    assert!(KeyboardKey::new("Shift+Tab").is_ok());
}

#[test]
fn tab_blocks_when_the_document_has_an_unrepresented_focus_target() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<button>Save</button><div contenteditable="true">Draft</div>"#);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let result = session.execute(PressKey {
        key: KeyboardKey::new("Tab").unwrap(),
    });
    drop(network_guard);

    assert!(matches!(
        result,
        Err(SessionError::UnsupportedPress { key, reason, .. })
            if key == KeyboardKey::new("Tab").unwrap()
                && reason.contains("without a supported interactive role")
    ));
}

#[test]
fn document_replacement_clears_focus_before_the_next_press() {
    let network_guard = network_test_guard();
    let body = r#"<input id="email" value="old">"#;
    let (url, server) = serve_pages(vec![body, body]);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    session
        .execute(FocusElement {
            reference: snapshot.elements[0].reference,
        })
        .unwrap();
    session.execute(ReloadPage).unwrap();
    server.join().unwrap();
    let press = session.execute(PressKey {
        key: KeyboardKey::new("X").unwrap(),
    });
    drop(network_guard);

    assert_eq!(press, Err(SessionError::NoFocusedElement));
}

#[test]
fn select_actions_update_native_selects_and_preserve_failure_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <select aria-label="Size">
                <option value="s">Small</option>
                <option value="large value">Large</option>
                <option value="disabled" disabled>Disabled</option>
                <optgroup disabled><option value="group">Group</option></optgroup>
            </select>
            <select aria-label="Locked" disabled><option value="fixed">Fixed</option></select>
            <select aria-label="Many" multiple>
                <option value="a" selected>A</option>
                <option value="b" selected>B</option>
                <option value="disabled" disabled>Disabled</option>
            </select>
            <button>Save</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session.execute(CaptureInteractiveSnapshot).unwrap();
    let size = before.elements[0].reference;
    let locked = before.elements[1].reference;
    let many = before.elements[2].reference;
    let button = before.elements[3].reference;

    let selected = session
        .execute(SelectElement {
            reference: size,
            value: "large value".into(),
        })
        .unwrap();
    let repeated = session
        .execute(SelectElement {
            reference: size,
            value: "large value".into(),
        })
        .unwrap();
    let single_from_many_values = session
        .execute(SelectOptions {
            reference: size,
            options: value_targets(&["large value", "s"]),
        })
        .unwrap();
    session
        .execute(SelectElement {
            reference: size,
            value: "large value".into(),
        })
        .unwrap();
    let disabled = session.execute(SelectElement {
        reference: size,
        value: "disabled".into(),
    });
    let disabled_group = session.execute(SelectElement {
        reference: size,
        value: "group".into(),
    });
    let missing = session.execute(SelectElement {
        reference: size,
        value: "missing".into(),
    });
    let current = session
        .execute(GetElementValue { reference: size })
        .unwrap();
    let locked_value = session
        .execute(GetElementValue { reference: locked })
        .unwrap();
    let locked_select = session.execute(SelectElement {
        reference: locked,
        value: "fixed".into(),
    });
    let initial_multiple_value = session
        .execute(GetElementValue { reference: many })
        .unwrap();
    let single_multiple_select = session
        .execute(SelectElement {
            reference: many,
            value: "b".into(),
        })
        .unwrap();
    let multiple_select = session
        .execute(SelectOptions {
            reference: many,
            options: value_targets(&["b", "a"]),
        })
        .unwrap();
    let disabled_multiple = session.execute(SelectOptions {
        reference: many,
        options: value_targets(&["b", "disabled"]),
    });
    let missing_multiple = session.execute(SelectOptions {
        reference: many,
        options: value_targets(&["b", "missing"]),
    });
    let final_multiple_value = session
        .execute(GetElementValue { reference: many })
        .unwrap();
    let wrong_role = session.execute(SelectElement {
        reference: button,
        value: "anything".into(),
    });
    let after = session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(SelectElement {
        reference: size,
        value: "s".into(),
    });
    drop(network_guard);

    assert_eq!(
        before.elements[0].state,
        InteractiveElementState::Value("s".into())
    );
    assert_eq!(
        before.elements[1].state,
        InteractiveElementState::Value("fixed".into())
    );
    assert_eq!(
        before.elements[2].state,
        InteractiveElementState::Value("a".into())
    );
    assert_eq!(
        selected,
        SelectResult {
            reference: size,
            value: "large value".into(),
        }
    );
    assert_eq!(repeated, selected);
    assert_eq!(single_from_many_values.selected, NonEmpty::one("s".into()));
    assert_eq!(current.value, "large value");
    assert_eq!(locked_value.value, "fixed");
    assert_eq!(
        disabled,
        Err(SessionError::SelectOptionDisabled {
            reference: size,
            value: "disabled".into(),
        })
    );
    assert_eq!(
        disabled_group,
        Err(SessionError::SelectOptionDisabled {
            reference: size,
            value: "group".into(),
        })
    );
    assert_eq!(
        missing,
        Err(SessionError::SelectOptionNotFound {
            reference: size,
            value: "missing".into(),
        })
    );
    assert!(matches!(
        locked_select,
        Err(SessionError::UnsupportedSelect { reference, .. }) if reference == locked
    ));
    assert_eq!(initial_multiple_value.value, "a");
    assert_eq!(single_multiple_select.value, "b");
    assert_eq!(
        multiple_select,
        SelectOptionsResult {
            reference: many,
            selected: NonEmpty::from_vec(vec!["b".into(), "a".into()]).unwrap(),
        }
    );
    assert_eq!(
        disabled_multiple,
        Err(SessionError::SelectOptionDisabled {
            reference: many,
            value: "disabled".into(),
        })
    );
    assert_eq!(
        missing_multiple,
        Err(SessionError::SelectOptionNotFound {
            reference: many,
            value: "missing".into(),
        })
    );
    assert_eq!(final_multiple_value.value, "a");
    assert!(matches!(
        wrong_role,
        Err(SessionError::UnsupportedSelect { reference, .. }) if reference == button
    ));
    assert_eq!(
        after.elements[0].state,
        InteractiveElementState::Value("large value".into())
    );
    assert_eq!(
        after.elements[2].state,
        InteractiveElementState::Value("a".into())
    );
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: size })
    );
}

#[test]
fn select_options_match_labels_and_indexes_transactionally() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <select id="many" aria-label="Many" multiple>
                <option value="a" label="Alpha label" selected>Alpha text</option>
                <option value="b">Bravo text</option>
                <option value="c" label="Charlie label" disabled>Charlie text</option>
            </select>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let many = snapshot.elements[0].reference;
    let locator = Locator::from(CssLocator::new("#many").unwrap());

    let selected = session
        .execute(SelectOptions {
            reference: many,
            options: NonEmpty::from_vec(vec![
                SelectOptionTarget::Label("Bravo text".into()),
                SelectOptionTarget::Index(0),
            ])
            .unwrap(),
        })
        .unwrap();
    let selected_by_locator = session
        .execute(SelectOptionsByLocator {
            locator: locator.clone(),
            options: NonEmpty::from_vec(vec![
                SelectOptionTarget::Label("Alpha label".into()),
                SelectOptionTarget::Index(1),
            ])
            .unwrap(),
        })
        .unwrap();
    let missing_label = session.execute(SelectOptions {
        reference: many,
        options: NonEmpty::from_vec(vec![
            SelectOptionTarget::Index(1),
            SelectOptionTarget::Label("missing".into()),
        ])
        .unwrap(),
    });
    let disabled_index = session.execute(SelectOptions {
        reference: many,
        options: NonEmpty::one(SelectOptionTarget::Index(2)),
    });
    let missing_index = session.execute(SelectOptionsByLocator {
        locator: locator.clone(),
        options: NonEmpty::one(SelectOptionTarget::Index(3)),
    });
    let current = session
        .execute(GetElementValue { reference: many })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        selected.selected,
        NonEmpty::from_vec(vec!["b".into(), "a".into()]).unwrap()
    );
    assert_eq!(
        selected_by_locator.selected,
        NonEmpty::from_vec(vec!["a".into(), "b".into()]).unwrap()
    );
    assert_eq!(
        missing_label,
        Err(SessionError::SelectOptionTargetNotFound {
            reference: many,
            target: SelectOptionTarget::Label("missing".into()),
        })
    );
    assert_eq!(
        disabled_index,
        Err(SessionError::SelectOptionTargetDisabled {
            reference: many,
            target: SelectOptionTarget::Index(2),
        })
    );
    assert_eq!(
        missing_index,
        Err(SessionError::LocatorSelectOptionTargetNotFound {
            locator,
            target: SelectOptionTarget::Index(3),
        })
    );
    assert_eq!(current.value, "a");
}

#[test]
fn editable_reads_match_native_and_contenteditable_boundaries() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="text">
            <input id="readonly" readonly>
            <input id="disabled" disabled>
            <textarea id="textarea"></textarea>
            <select id="select"><option>One</option></select>
            <input id="checkbox" type="checkbox">
            <input id="input-button" type="button" value="Button">
            <button id="button">Button</button>
            <div contenteditable><span id="editable-child">Inherited</span></div>
            <div id="editable-false" contenteditable="false">Not editable</div>
            <fieldset disabled><input id="fieldset-input"></fieldset>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let text = snapshot.elements[0].reference;
    let readonly = snapshot.elements[1].reference;
    let button = snapshot.elements[7].reference;

    let text_reference = session
        .execute(GetElementEditable { reference: text })
        .unwrap();
    let readonly_reference = session
        .execute(GetElementEditable {
            reference: readonly,
        })
        .unwrap();
    let button_reference = session.execute(GetElementEditable { reference: button });
    let mut selector_results = Vec::new();
    for selector in [
        "#text",
        "#readonly",
        "#disabled",
        "#textarea",
        "#select",
        "#checkbox",
        "#input-button",
        "#editable-child",
    ] {
        selector_results.push(
            session
                .execute(GetEditableByLocator {
                    locator: Locator::from(CssLocator::new(selector).unwrap()),
                })
                .unwrap()
                .editable,
        );
    }
    let false_contenteditable = session.execute(GetEditableByLocator {
        locator: Locator::from(CssLocator::new("#editable-false").unwrap()),
    });
    let fieldset = session.execute(GetEditableByLocator {
        locator: Locator::from(CssLocator::new("#fieldset-input").unwrap()),
    });
    drop(network_guard);

    assert!(text_reference.editable);
    assert!(!readonly_reference.editable);
    assert!(matches!(
        button_reference,
        Err(SessionError::UnsupportedEditableState { reference, .. }) if reference == button
    ));
    assert_eq!(
        selector_results,
        vec![true, false, false, true, true, true, true, true]
    );
    assert!(matches!(
        false_contenteditable,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::Editable,
            ..
        })
    ));
    assert!(matches!(
        fieldset,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::Editable,
            reason,
            ..
        }) if reason == "disabled fieldset editable state is not implemented"
    ));
}

#[test]
fn visibility_reads_require_supported_static_box_evidence() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button>Visible</button>
            <button hidden>Hidden</button>
            <div style="display:none"><button>Ancestor hidden</button></div>
            <div role="button" aria-label="Empty"></div>
            <button style="width:0">Unknown box</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let visible = snapshot.elements[0].reference;
    let hidden = snapshot.elements[1].reference;
    let ancestor_hidden = snapshot.elements[2].reference;
    let empty = snapshot.elements[3].reference;
    let unknown = snapshot.elements[4].reference;

    let visible_result = session.execute(GetElementVisible { reference: visible });
    let hidden_result = session.execute(GetElementVisible { reference: hidden });
    let ancestor_result = session.execute(GetElementVisible {
        reference: ancestor_hidden,
    });
    let empty_result = session.execute(GetElementVisible { reference: empty });
    let unknown_result = session.execute(GetElementVisible { reference: unknown });
    let still_enabled = session.execute(GetElementEnabled { reference: visible });
    session.execute(CaptureInteractiveSnapshot).unwrap();
    let stale = session.execute(GetElementVisible { reference: visible });
    drop(network_guard);

    assert_eq!(
        visible_result,
        Ok(ElementVisible {
            reference: visible,
            visible: true,
        })
    );
    assert_eq!(
        hidden_result,
        Ok(ElementVisible {
            reference: hidden,
            visible: false,
        })
    );
    assert_eq!(
        ancestor_result,
        Ok(ElementVisible {
            reference: ancestor_hidden,
            visible: false,
        })
    );
    assert_eq!(
        empty_result,
        Ok(ElementVisible {
            reference: empty,
            visible: false,
        })
    );
    assert!(matches!(
        unknown_result,
        Err(SessionError::UnsupportedVisibility { reference, .. }) if reference == unknown
    ));
    assert!(still_enabled.unwrap().enabled);
    assert_eq!(
        stale,
        Err(SessionError::StaleElementReference { reference: visible })
    );
}

#[test]
fn bounding_box_reads_share_complete_geometry_and_preserve_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="fixed" style="position:fixed;left:20px;top:30px;width:120px;height:40px;padding-left:5px;padding-right:7px;padding-top:3px;padding-bottom:4px;border-left-width:2px;border-left-style:solid;border-right-width:3px;border-right-style:solid;border-top-width:1px;border-top-style:solid;border-bottom-width:2px;border-bottom-style:solid">Save</button>
            <button id="hidden" hidden style="position:fixed;left:1px;top:2px;width:3px;height:4px">Hidden</button>
            <button id="border-box" style="position:fixed;left:-8px;top:-9px;width:50px;height:30px;box-sizing:border-box;padding-left:5px;padding-right:5px;padding-top:3px;padding-bottom:3px;border-left-width:2px;border-left-style:solid;border-right-width:2px;border-right-style:solid;border-top-width:1px;border-top-style:solid;border-bottom-width:1px;border-bottom-style:solid">Sized</button>
            <button id="zero" style="position:fixed;left:4px;top:5px;width:0;height:8px">Zero</button>
            <button id="normal">Normal</button>
            <button id="fixed-margin" style="position:fixed;left:10px;top:10px;width:20px;height:20px;margin-left:2px">Margin</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let fixed = snapshot.elements[0].reference;
    let hidden = snapshot.elements[1].reference;

    let reference_box = session
        .execute(GetElementBoundingBox { reference: fixed })
        .unwrap();
    let role_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(RoleLocator::new("button").unwrap().with_exact_name("Save")),
        })
        .unwrap();
    let xpath_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(XPathLocator::new("//button[@id='fixed']").unwrap()),
        })
        .unwrap();
    let hidden_reference = session
        .execute(GetElementBoundingBox { reference: hidden })
        .unwrap();
    let hidden_locator = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#hidden").unwrap()),
        })
        .unwrap();
    let border_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#border-box").unwrap()),
        })
        .unwrap();
    let zero_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#zero").unwrap()),
        })
        .unwrap();
    let unsupported = session.execute(GetBoundingBoxByLocator {
        locator: Locator::from(CssLocator::new("#normal").unwrap()),
    });
    let unsupported_fixed_margin = session.execute(GetBoundingBoxByLocator {
        locator: Locator::from(CssLocator::new("#fixed-margin").unwrap()),
    });
    let preserved = session
        .execute(GetElementText { reference: fixed })
        .unwrap();
    drop(network_guard);

    let expected = BoundingBox {
        x: 20,
        y: 30,
        width: 137,
        height: 50,
    };
    assert_eq!(
        reference_box,
        ElementBoundingBox {
            reference: fixed,
            value: Some(expected),
        }
    );
    assert_eq!(role_box.value, Some(expected));
    assert_eq!(role_box.matched.element, "fixed");
    assert_eq!(xpath_box.value, Some(expected));
    assert_eq!(hidden_reference.value, None);
    assert_eq!(hidden_locator.value, None);
    assert_eq!(
        border_box.value,
        Some(BoundingBox {
            x: -8,
            y: -9,
            width: 50,
            height: 30,
        })
    );
    assert_eq!(zero_box.value, None);
    assert!(matches!(
        unsupported,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::BoundingBox,
            reason,
            ..
        }) if reason == "normal-flow button layout is not implemented"
    ));
    assert!(matches!(
        unsupported_fixed_margin,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::BoundingBox,
            reason,
            ..
        }) if reason == "fixed-position margin-left geometry is not implemented"
    ));
    assert_eq!(preserved.text, "Save");
}

#[test]
fn normal_flow_bounding_boxes_stack_blocks_and_size_auto_parents() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <main id="shell" style="width:300px;padding-left:10px;padding-top:5px;border-left-width:2px;border-left-style:solid;border-top-width:3px;border-top-style:solid">
                    <section id="first" style="height:20px"></section>
                    <button id="action" style="display:block;box-sizing:border-box;width:100px;height:24px">Act</button>
                    <div id="empty"></div>
                    <section id="hidden" hidden style="height:100px"></section>
                    <aside id="overlay" style="position:fixed;left:400px;top:40px;width:50px;height:60px"></aside>
                    <section id="invisible" style="visibility:hidden;height:10px"></section>
                    <section id="second" style="height:30px"></section>
                </main>
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let action = snapshot.elements[0].reference;

    let read = |session: &mut Session, selector: &str| {
        session
            .execute(GetBoundingBoxByLocator {
                locator: Locator::from(CssLocator::new(selector).unwrap()),
            })
            .unwrap()
            .value
    };
    let body = read(&mut session, "body");
    let shell = read(&mut session, "#shell");
    let first = read(&mut session, "#first");
    let action_box = session
        .execute(GetElementBoundingBox { reference: action })
        .unwrap();
    let empty = read(&mut session, "#empty");
    let hidden = read(&mut session, "#hidden");
    let overlay = read(&mut session, "#overlay");
    let invisible = read(&mut session, "#invisible");
    let second = read(&mut session, "#second");
    let second_visible = session
        .execute(GetVisibleByLocator {
            locator: Locator::from(CssLocator::new("#second").unwrap()),
        })
        .unwrap();
    let empty_visible = session
        .execute(GetVisibleByLocator {
            locator: Locator::from(CssLocator::new("#empty").unwrap()),
        })
        .unwrap();
    let invisible_visible = session
        .execute(GetVisibleByLocator {
            locator: Locator::from(CssLocator::new("#invisible").unwrap()),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        body,
        Some(BoundingBox {
            x: 8,
            y: 8,
            width: 1264,
            height: 92,
        })
    );
    assert_eq!(
        shell,
        Some(BoundingBox {
            x: 8,
            y: 8,
            width: 312,
            height: 92,
        })
    );
    assert_eq!(
        first,
        Some(BoundingBox {
            x: 20,
            y: 16,
            width: 300,
            height: 20,
        })
    );
    assert_eq!(
        action_box,
        ElementBoundingBox {
            reference: action,
            value: Some(BoundingBox {
                x: 20,
                y: 36,
                width: 100,
                height: 24,
            }),
        }
    );
    assert_eq!(empty, None);
    assert_eq!(hidden, None);
    assert_eq!(
        overlay,
        Some(BoundingBox {
            x: 400,
            y: 40,
            width: 50,
            height: 60,
        })
    );
    assert_eq!(invisible, None);
    assert_eq!(
        second,
        Some(BoundingBox {
            x: 20,
            y: 70,
            width: 300,
            height: 30,
        })
    );
    assert!(second_visible.visible);
    assert!(!empty_visible.visible);
    assert!(!invisible_visible.visible);
}

#[test]
fn normal_flow_bounding_boxes_reject_intrinsic_text_and_collapsing_margins() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <main id="sized" style="height:40px">
                <div id="text">Needs line layout</div>
            </main>
            <section id="margined" style="height:10px;margin-top:5px"></section>
            <aside id="after" style="height:10px"></aside>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    let sized = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#sized").unwrap()),
        })
        .unwrap();
    let text = session.execute(GetBoundingBoxByLocator {
        locator: Locator::from(CssLocator::new("#text").unwrap()),
    });
    let margined = session.execute(GetBoundingBoxByLocator {
        locator: Locator::from(CssLocator::new("#margined").unwrap()),
    });
    let after = session.execute(GetBoundingBoxByLocator {
        locator: Locator::from(CssLocator::new("#after").unwrap()),
    });
    drop(network_guard);

    assert_eq!(
        sized.value,
        Some(BoundingBox {
            x: 0,
            y: 0,
            width: 1280,
            height: 40,
        })
    );
    assert!(matches!(
        text,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::BoundingBox,
            reason,
            ..
        }) if reason == "intrinsic text height is not implemented for text"
    ));
    assert!(matches!(
        margined,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::BoundingBox,
            reason,
            ..
        }) if reason == "vertical margin collapsing is not implemented"
    ));
    assert!(matches!(
        after,
        Err(SessionError::UnsupportedLocatorInspection {
            inspection: LocatorInspection::BoundingBox,
            reason,
            ..
        }) if reason.starts_with("previous normal-flow sibling geometry is unsupported")
    ));
}

#[test]
fn page_scroll_moves_document_boxes_keeps_fixed_boxes_and_resets_on_reload() {
    let network_guard = network_test_guard();
    let body = r#"
        <body>
            <div id="wide" style="width:1800px;height:10px"></div>
            <div id="spacer" style="height:1300px"></div>
            <button id="target" style="display:block;box-sizing:border-box;width:100px;height:40px">Target</button>
            <button id="fixed" style="position:fixed;left:20px;top:30px;width:100px;height:40px">Fixed</button>
            <button id="hidden" hidden style="position:fixed;left:0;top:0;width:100px;height:40px">Hidden</button>
        </body>
    "#;
    let (url, server) = serve_pages(vec![body, body]);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let target = snapshot.elements[0].reference;
    let fixed = snapshot.elements[1].reference;

    let down = session
        .execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 300,
        })
        .unwrap();
    let target_after_down = session
        .execute(GetElementBoundingBox { reference: target })
        .unwrap();
    let fixed_after_down = session
        .execute(GetElementBoundingBox { reference: fixed })
        .unwrap();
    let right = session
        .execute(ScrollPage {
            direction: ScrollDirection::Right,
            distance: 1_000,
        })
        .unwrap();
    let target_after_right = session
        .execute(GetElementBoundingBox { reference: target })
        .unwrap();
    let capped = session
        .execute(ScrollPage {
            direction: ScrollDirection::Right,
            distance: 1,
        })
        .unwrap();
    let down_capped = session
        .execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 1_000,
        })
        .unwrap();
    let up = session
        .execute(ScrollPage {
            direction: ScrollDirection::Up,
            distance: 200,
        })
        .unwrap();
    let left = session
        .execute(ScrollPage {
            direction: ScrollDirection::Left,
            distance: 28,
        })
        .unwrap();
    session.execute(ReloadPage).unwrap();
    server.join().unwrap();
    let reset = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#target").unwrap()),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        down,
        PageScroll {
            x: 0,
            y: 300,
            moved: true,
        }
    );
    assert_eq!(target_after_down.value.unwrap().y, 1_018);
    assert_eq!(fixed_after_down.value.unwrap().y, 30);
    assert_eq!(
        right,
        PageScroll {
            x: 528,
            y: 300,
            moved: true,
        }
    );
    assert_eq!(target_after_right.value.unwrap().x, -520);
    assert_eq!(
        capped,
        PageScroll {
            x: 528,
            y: 300,
            moved: false,
        }
    );
    assert_eq!(
        down_capped,
        PageScroll {
            x: 528,
            y: 638,
            moved: true,
        }
    );
    assert_eq!(
        up,
        PageScroll {
            x: 528,
            y: 438,
            moved: true,
        }
    );
    assert_eq!(
        left,
        PageScroll {
            x: 500,
            y: 438,
            moved: true,
        }
    );
    assert_eq!(reset.value.unwrap().x, 8);
    assert_eq!(reset.value.unwrap().y, 1_318);
}

#[test]
fn descendants_of_fixed_boxes_keep_viewport_coordinates_during_scroll() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <div style="width:1800px;height:1000px"></div>
                <div id="fixed" style="position:fixed;left:20px;top:30px;width:100px;height:40px">
                    <div id="child" style="display:block;width:40px;height:20px">Child</div>
                </div>
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let before = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#child").unwrap()),
        })
        .unwrap();
    session
        .execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 300,
        })
        .unwrap();
    session
        .execute(ScrollPage {
            direction: ScrollDirection::Right,
            distance: 300,
        })
        .unwrap();
    let after = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#child").unwrap()),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(before.value, after.value);
    assert_eq!(
        after.value,
        Some(BoundingBox {
            x: 20,
            y: 30,
            width: 40,
            height: 20,
        })
    );
}

#[test]
fn scroll_into_view_resolves_references_and_locators_without_replacing_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <div style="width:1800px;height:1310px"></div>
                <button id="target" style="display:block;box-sizing:border-box;width:100px;height:40px">Target</button>
                <button id="fixed" style="position:fixed;left:20px;top:30px;width:100px;height:40px">Fixed</button>
                <button id="hidden" hidden style="position:fixed;left:0;top:0;width:100px;height:40px">Hidden</button>
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let target = snapshot.elements[0].reference;
    let fixed = snapshot.elements[1].reference;
    let hidden = snapshot.elements[2].reference;
    session
        .execute(ScrollPage {
            direction: ScrollDirection::Right,
            distance: 600,
        })
        .unwrap();

    let reference_scroll = session
        .execute(ScrollElementIntoView { reference: target })
        .unwrap();
    let target_box = session
        .execute(GetElementBoundingBox { reference: target })
        .unwrap();
    let fixed_scroll = session
        .execute(ScrollIntoViewByLocator {
            locator: Locator::from(CssLocator::new("#fixed").unwrap()),
        })
        .unwrap();
    let hidden_error = session.execute(ScrollElementIntoView { reference: hidden });
    let hidden_locator_error = session.execute(ScrollIntoViewByLocator {
        locator: Locator::from(CssLocator::new("#hidden").unwrap()),
    });
    let preserved = session
        .execute(GetElementText { reference: fixed })
        .unwrap();
    drop(network_guard);

    assert_eq!(reference_scroll.reference, target);
    assert_eq!(reference_scroll.scroll.x, 8);
    assert_eq!(reference_scroll.scroll.y, 638);
    assert!(reference_scroll.scroll.moved);
    assert_eq!(
        target_box.value,
        Some(BoundingBox {
            x: 0,
            y: 680,
            width: 100,
            height: 40,
        })
    );
    assert_eq!(fixed_scroll.matched.element, "fixed");
    assert_eq!(fixed_scroll.scroll.x, 8);
    assert_eq!(fixed_scroll.scroll.y, 638);
    assert!(!fixed_scroll.scroll.moved);
    assert!(
        matches!(
            &hidden_error,
            Err(SessionError::UnsupportedScrollIntoView { reference, reason })
                if *reference == hidden && reason == "element is hidden or has an empty box"
        ),
        "unexpected hidden scroll result: {hidden_error:?}"
    );
    assert!(matches!(
        hidden_locator_error,
        Err(SessionError::UnsupportedLocatorAction {
            action: LocatorAction::ScrollIntoView,
            reason,
            ..
        }) if reason == "element is hidden or has an empty box"
    ));
    assert_eq!(preserved.text, "Fixed");
}

#[test]
fn pointer_actions_auto_scroll_after_checks_and_keep_failed_actions_transactional() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <div style="height:800px"></div>
                <button id="click" style="display:block;box-sizing:border-box;width:100px;height:40px">Click</button>
                <input id="check" type="checkbox" style="display:block;box-sizing:border-box;width:20px;height:20px">
                <div id="hover" style="display:block;box-sizing:border-box;width:100px;height:40px">Hover</div>
                <input id="locked" type="checkbox" disabled style="display:block;box-sizing:border-box;width:20px;height:20px">
            </body>
        "#,
    );
    let mut session = Session::new();
    session
        .execute(SetViewportSize {
            width: 640,
            height: 300,
        })
        .unwrap();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let click = snapshot.elements[0].reference;
    let initial_click_box = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap()
        .value
        .unwrap();

    session.execute(ClickElement { reference: click }).unwrap();
    let clicked_box = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap()
        .value
        .unwrap();
    session
        .execute(ScrollPage {
            direction: ScrollDirection::Up,
            distance: u64::MAX,
        })
        .unwrap();

    let checked = session
        .execute(SetCheckedByLocator {
            locator: Locator::from(CssLocator::new("#check").unwrap()),
            checked: true,
        })
        .unwrap();
    let checked_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#check").unwrap()),
        })
        .unwrap()
        .value
        .unwrap();
    session
        .execute(ScrollPage {
            direction: ScrollDirection::Up,
            distance: u64::MAX,
        })
        .unwrap();
    let repeated = session
        .execute(SetCheckedByLocator {
            locator: Locator::from(CssLocator::new("#check").unwrap()),
            checked: true,
        })
        .unwrap();
    let click_after_no_op = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap()
        .value
        .unwrap();

    session
        .execute(HoverByLocator {
            locator: Locator::from(CssLocator::new("#hover").unwrap()),
        })
        .unwrap();
    let hovered_box = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("#hover").unwrap()),
        })
        .unwrap()
        .value
        .unwrap();
    let before_failure = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap();
    let failed = session.execute(SetCheckedByLocator {
        locator: Locator::from(CssLocator::new("#locked").unwrap()),
        checked: true,
    });
    let after_failure = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap();
    drop(network_guard);

    let is_in_view = |bounding_box: &BoundingBox| {
        let height = i64::try_from(bounding_box.height).unwrap();
        bounding_box.y >= 0 && bounding_box.y + height <= 300
    };

    assert!(initial_click_box.y >= 300);
    assert!(is_in_view(&clicked_box));
    assert!(checked.checked);
    assert!(is_in_view(&checked_box));
    assert!(repeated.checked);
    assert_eq!(click_after_no_op, initial_click_box);
    assert!(is_in_view(&hovered_box));
    assert!(matches!(
        failed,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Check,
            check: ActionabilityCheck::Enabled,
            ..
        })
    ));
    assert_eq!(after_failure, before_failure);
}

#[test]
fn pointer_actions_block_when_a_fixed_element_intercepts_the_action_point() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0">
                <div id="cover" style="position:fixed;left:0;top:0;width:120px;height:100px">Cover</div>
                <div style="height:200px"></div>
                <button id="click" style="display:block;box-sizing:border-box;width:120px;height:40px">Click</button>
                <input id="check" type="checkbox" style="display:block;box-sizing:border-box;width:20px;height:20px">
                <button id="hover" style="display:block;box-sizing:border-box;width:120px;height:40px">Hover</button>
            </body>
        "#,
    );
    let mut session = Session::new();
    session
        .execute(SetViewportSize {
            width: 640,
            height: 100,
        })
        .unwrap();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let click = snapshot.elements[0].reference;
    let check = snapshot.elements[1].reference;
    let hover = snapshot.elements[2].reference;
    let before = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap();

    let reference_click = session.execute(ClickElement { reference: click });
    let locator_click = session.execute(ClickByLocator {
        locator: Locator::from(CssLocator::new("#click").unwrap()),
    });
    let reference_check = session.execute(SetElementChecked {
        reference: check,
        checked: true,
    });
    let locator_check = session.execute(SetCheckedByLocator {
        locator: Locator::from(CssLocator::new("#check").unwrap()),
        checked: true,
    });
    let reference_hover = session.execute(HoverElement { reference: hover });
    let locator_hover = session.execute(HoverByLocator {
        locator: Locator::from(CssLocator::new("#hover").unwrap()),
    });
    let focused = session
        .execute(GetElementFocused { reference: click })
        .unwrap();
    let checked = session
        .execute(GetElementChecked { reference: check })
        .unwrap();
    let hovered = session
        .execute(GetElementHovered { reference: hover })
        .unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    let after = session
        .execute(GetElementBoundingBox { reference: click })
        .unwrap();
    drop(network_guard);

    assert!(matches!(
        reference_click,
        Err(SessionError::UnsupportedClick { reason, .. })
            if reason == "receives events check failed: cover intercepts pointer events at (60, 80)"
    ));
    assert!(matches!(
        locator_click,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Click,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
            ..
        }) if reason == "cover intercepts pointer events at (60, 80)"
    ));
    assert!(matches!(
        reference_check,
        Err(SessionError::UnsupportedCheck { reason, .. })
            if reason == "receives events check failed: cover intercepts pointer events at (10, 90)"
    ));
    assert!(matches!(
        locator_check,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Check,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
            ..
        }) if reason == "cover intercepts pointer events at (10, 90)"
    ));
    assert!(matches!(
        reference_hover,
        Err(SessionError::UnsupportedHover { reason, .. })
            if reason == "receives events check failed: cover intercepts pointer events at (60, 80)"
    ));
    assert!(matches!(
        locator_hover,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Hover,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
            ..
        }) if reason == "cover intercepts pointer events at (60, 80)"
    ));
    assert!(!focused.focused);
    assert!(!checked.checked);
    assert!(!hovered.hovered);
    assert!(events.is_empty());
    assert_eq!(after, before);
}

#[test]
fn hit_testing_accepts_target_descendants_and_ignores_pointer_events_none() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0">
                <button id="target" style="display:block;box-sizing:border-box;width:120px;height:40px;padding-left:0;padding-right:0;padding-top:0;padding-bottom:0">
                    <div style="display:block;width:120px;height:40px">Child</div>
                </button>
                <div id="cover" style="position:fixed;left:0;top:0;width:120px;height:40px;pointer-events:none">Cover</div>
                <div hidden style="position:fixed;left:0;top:0;width:120px;height:40px;pointer-events:painted">Hidden</div>
                <button id="ignored" style="position:fixed;left:200px;top:0;width:120px;height:40px;pointer-events:none">Ignored</button>
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let clicked = session
        .execute(ClickByLocator {
            locator: Locator::from(CssLocator::new("#target").unwrap()),
        })
        .unwrap();
    let focused = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("#target").unwrap()),
        })
        .unwrap();
    let ignored = session.execute(ClickByLocator {
        locator: Locator::from(CssLocator::new("#ignored").unwrap()),
    });
    drop(network_guard);

    assert!(matches!(clicked, ClickByLocatorResult::Activated { .. }));
    assert!(focused.focused);
    assert!(matches!(
        ignored,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Click,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
            ..
        }) if reason == "body[1] intercepts pointer events at (260, 20)"
    ));
}

#[test]
fn overlapping_unsupported_hit_test_evidence_blocks_pointer_actions() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0">
                <button id="target" style="display:block;box-sizing:border-box;width:120px;height:40px">Target</button>
                <div id="far" style="position:fixed;left:0;top:100px;width:120px;height:40px;opacity:0.5">Far</div>
                <div id="overlay" style="position:fixed;left:0;top:0;width:120px;height:40px;opacity:0.5">Overlay</div>
            </body>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let result = session.execute(ClickByLocator {
        locator: Locator::from(CssLocator::new("#target").unwrap()),
    });
    drop(network_guard);

    assert!(matches!(
        result,
        Err(SessionError::LocatorActionBlocked {
            action: LocatorAction::Click,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
            ..
        }) if reason == "hit-test evidence for overlay is not implemented: stacking-context hit testing is not implemented for overlay"
    ));
}

#[test]
fn scrolling_requires_an_open_page() {
    assert_eq!(
        Session::new().execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 300,
        }),
        Err(SessionError::NoPage)
    );
}

#[test]
fn viewport_resize_reflows_geometry_and_preserves_live_page_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="height:900px">
                <input id="email" aria-label="Email" value="before">
            </body>
        "#,
    );
    let mut session = Session::new();

    assert_eq!(
        session.execute(GetViewportSize),
        Ok(ViewportSize {
            width: 1_280,
            height: 720,
        })
    );
    let configured = session
        .execute(SetViewportSize {
            width: 640,
            height: 480,
        })
        .unwrap();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let email = snapshot.elements[0].reference;
    session
        .execute(FillElement {
            reference: email,
            value: "changed".into(),
        })
        .unwrap();
    let before = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        })
        .unwrap();
    let scrolled = session
        .execute(ScrollPage {
            direction: ScrollDirection::Down,
            distance: 1_000,
        })
        .unwrap();
    let resized = session
        .execute(SetViewportSize {
            width: 800,
            height: 600,
        })
        .unwrap();
    let after = session
        .execute(GetBoundingBoxByLocator {
            locator: Locator::from(CssLocator::new("body").unwrap()),
        })
        .unwrap();
    let value = session
        .execute(GetElementValue { reference: email })
        .unwrap();
    let focused = session
        .execute(GetElementFocused { reference: email })
        .unwrap();
    let repeated = session
        .execute(SetViewportSize {
            width: 800,
            height: 600,
        })
        .unwrap();
    let invalid = session.execute(SetViewportSize {
        width: 0,
        height: 600,
    });
    let preserved_viewport = session.execute(GetViewportSize).unwrap();
    drop(network_guard);

    assert!(configured.resized);
    assert_eq!(configured.viewport.width, 640);
    assert_eq!(configured.viewport.height, 480);
    assert_eq!(before.value.unwrap().width, 624);
    assert_eq!(scrolled.y, 428);
    assert!(resized.resized);
    assert_eq!(resized.scroll.y, 308);
    assert!(resized.scroll.moved);
    assert_eq!(
        after.value,
        Some(BoundingBox {
            x: 8,
            y: -300,
            width: 784,
            height: 900,
        })
    );
    assert_eq!(value.value, "changed");
    assert!(focused.focused);
    assert!(!repeated.resized);
    assert_eq!(repeated.scroll.y, 308);
    assert!(!repeated.scroll.moved);
    assert_eq!(
        invalid,
        Err(SessionError::InvalidViewportSize {
            width: 0,
            height: 600,
        })
    );
    assert_eq!(
        preserved_viewport,
        ViewportSize {
            width: 800,
            height: 600,
        }
    );
}

#[test]
fn bounding_box_locator_requires_an_open_page() {
    let locator = Locator::from(CssLocator::new("#fixed").unwrap());

    assert_eq!(
        Session::new().execute(GetBoundingBoxByLocator { locator }),
        Err(SessionError::NoPage)
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
    let locked_no_op = session
        .execute(SetElementChecked {
            reference: locked,
            checked: true,
        })
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
    assert!(locked_no_op.checked);
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
fn radio_groups_share_exclusive_state_across_check_click_press_and_tab() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form id="plans">
                <label><input id="basic" type="radio" name="plan" checked>Basic</label>
                <label><input id="pro" type="radio" name="plan">Pro</label>
                <label><input id="locked" type="radio" name="plan" disabled>Locked</label>
            </form>
            <label><input id="external" type="radio" name="plan" form="plans">External</label>
            <form><label><input id="second" type="radio" name="plan">Second form</label></form>
            <label><input id="solo" type="radio">Solo</label>
            <button id="after" type="button">After</button>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let basic = snapshot.elements[0].reference;
    let pro = snapshot.elements[1].reference;
    let locked = snapshot.elements[2].reference;
    let external = snapshot.elements[3].reference;
    let after = snapshot.elements[6].reference;

    let first_tab = session
        .execute(PressKey {
            key: KeyboardKey::new("Tab").unwrap(),
        })
        .unwrap();
    let clicked = session.execute(ClickElement { reference: pro }).unwrap();
    let clicked_again = session.execute(ClickElement { reference: pro }).unwrap();
    let checked_basic = session
        .execute(SetCheckedByRole {
            locator: RoleLocator::new("radio").unwrap().with_exact_name("Basic"),
            checked: true,
        })
        .unwrap();
    let checked_external = session
        .execute(ClickByLocator {
            locator: Locator::from(CssLocator::new("#external").unwrap()),
        })
        .unwrap();
    let unchanged_false = session
        .execute(SetElementChecked {
            reference: basic,
            checked: false,
        })
        .unwrap();
    let rejected_uncheck = session.execute(SetElementChecked {
        reference: external,
        checked: false,
    });
    let locked_locator = RoleLocator::new("radio").unwrap().with_exact_name("Locked");
    let rejected_locked = session.execute(SetCheckedByRole {
        locator: locked_locator.clone(),
        checked: true,
    });
    let selected_by_space = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#basic").unwrap()),
            key: KeyboardKey::new("Space").unwrap(),
        })
        .unwrap();
    let next_group = session
        .execute(PressKey {
            key: KeyboardKey::new("Tab").unwrap(),
        })
        .unwrap();
    let basic_state = session
        .execute(GetElementChecked { reference: basic })
        .unwrap();
    let pro_state = session
        .execute(GetElementChecked { reference: pro })
        .unwrap();
    let external_state = session
        .execute(GetElementChecked {
            reference: external,
        })
        .unwrap();
    let locked_state = session
        .execute(GetElementChecked { reference: locked })
        .unwrap();
    let preserved = session
        .execute(GetElementText { reference: after })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        first_tab
            .focus_traversal()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .element,
        "basic"
    );
    assert_eq!(
        clicked,
        ClickResult::Checked {
            reference: pro,
            checked: true,
        }
    );
    assert_eq!(clicked, clicked_again);
    assert!(checked_basic.checked);
    assert!(matches!(
        checked_external,
        ClickByLocatorResult::Checked { matched, checked: true }
            if matched.element == "external"
    ));
    assert!(!unchanged_false.checked);
    assert!(matches!(
        rejected_uncheck,
        Err(SessionError::UnsupportedCheck { reference, reason })
            if reference == external && reason == "checked radios cannot be unchecked by activation"
    ));
    assert!(matches!(
        rejected_locked,
        Err(SessionError::RoleActionBlocked {
            locator,
            action: RoleAction::Check,
            check: ActionabilityCheck::Enabled,
            ..
        }) if locator == locked_locator
    ));
    assert!(selected_by_space.press.checked().unwrap().1);
    assert_eq!(
        next_group
            .focus_traversal()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .element,
        "second"
    );
    assert!(basic_state.checked);
    assert!(!pro_state.checked);
    assert!(!external_state.checked);
    assert!(!locked_state.checked);
    assert_eq!(preserved.text, "After");
}

#[test]
fn radio_arrow_keys_wrap_and_skip_disabled_or_hidden_group_members() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form>
                <label><input id="a" type="radio" name="plan" checked>A</label>
                <label><input id="b" type="radio" name="plan" disabled>B</label>
                <label hidden><input id="hidden" type="radio" name="plan">Hidden</label>
                <label><input id="c" type="radio" name="plan">C</label>
            </form>
            <form><label><input id="other" type="radio" name="plan">Other</label></form>
        "#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let a = snapshot.elements[0].reference;
    let b = snapshot.elements[1].reference;
    let hidden = snapshot.elements[2].reference;
    let c = snapshot.elements[3].reference;
    let other = snapshot.elements[4].reference;

    let right = session
        .execute(PressByLocator {
            locator: Locator::from(CssLocator::new("#a").unwrap()),
            key: KeyboardKey::new("ArrowRight").unwrap(),
        })
        .unwrap();
    let wrapped_right = session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowRight").unwrap(),
        })
        .unwrap();
    let wrapped_left = session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowLeft").unwrap(),
        })
        .unwrap();
    let down = session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowDown").unwrap(),
        })
        .unwrap();
    let up = session
        .execute(PressKey {
            key: KeyboardKey::new("ArrowUp").unwrap(),
        })
        .unwrap();
    let states = [a, b, hidden, c, other].map(|reference| {
        session
            .execute(GetElementChecked { reference })
            .unwrap()
            .checked
    });
    let focus = session.execute(GetElementFocused { reference: c }).unwrap();
    drop(network_guard);

    assert_eq!(right.press.checked().unwrap().0.element, "c");
    assert_eq!(wrapped_right.checked().unwrap().0.element, "a");
    assert_eq!(wrapped_left.checked().unwrap().0.element, "c");
    assert_eq!(down.checked().unwrap().0.element, "a");
    assert_eq!(up.checked().unwrap().0.element, "c");
    assert_eq!(states, [false, false, false, true, false]);
    assert!(focus.focused);
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
fn page_text_reads_normalized_static_content_across_navigation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>One</title><main>Hello <span>world</span><script>ignore()</script> <a href="/two">Next</a></main>"#,
        r#"<title>Two</title><main>Second <strong>page</strong><input value="secret"></main>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    let link = snapshot.elements[0].reference;

    let first = session.execute(GetPageText).unwrap();
    let current_link = session.execute(GetElementText { reference: link }).unwrap();
    session.execute(ClickElement { reference: link }).unwrap();
    let second = session.execute(GetPageText).unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(
        first,
        PageText {
            text: "Hello world Next".into(),
        }
    );
    assert_eq!(current_link.text, "Next");
    assert_eq!(second.text, "Second page");
}

#[test]
fn page_text_requires_an_open_page() {
    assert_eq!(
        Session::new().execute(GetPageText),
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
fn history_navigation_requires_an_open_page() {
    assert_eq!(Session::new().execute(GoBack), Err(SessionError::NoPage));
    assert_eq!(Session::new().execute(GoForward), Err(SessionError::NoPage));
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
        url: "http://192.168.1.1".into(),
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
fn native_actions_record_ordered_dom_events_with_ancestry() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="root"><label>Name<input id="name"></label><label><input id="terms" type="checkbox">Terms</label><select id="size"><option value="s">Small</option><option value="l">Large</option></select></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();

    session
        .execute(FillElement {
            reference: snapshot.elements[0].reference,
            value: "Ada".into(),
        })
        .unwrap();
    session
        .execute(SetElementChecked {
            reference: snapshot.elements[1].reference,
            checked: true,
        })
        .unwrap();
    session
        .execute(SetElementChecked {
            reference: snapshot.elements[1].reference,
            checked: true,
        })
        .unwrap();
    session
        .execute(SelectElement {
            reference: snapshot.elements[2].reference,
            value: "l".into(),
        })
        .unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    let empty = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);

    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    DomEventType::BeforeInput
                        | DomEventType::Change
                        | DomEventType::Click
                        | DomEventType::Input
                )
            })
            .map(|event| (event.event_type, event.target.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (DomEventType::BeforeInput, "name"),
            (DomEventType::Input, "name"),
            (DomEventType::Click, "terms"),
            (DomEventType::Input, "terms"),
            (DomEventType::Change, "terms"),
            (DomEventType::Input, "size"),
            (DomEventType::Change, "size"),
        ]
    );
    assert_eq!(events[0].path, vec!["name", "label[2]", "root"]);
    assert_eq!(events[0].target_ordinal, 3);
    assert!(
        events
            .iter()
            .all(|event| event.bubbles == event.event_type.bubbles())
    );
    assert!(empty.is_empty());
}

#[test]
fn link_click_event_survives_successful_navigation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<main id="root"><a id="next" href="/next">Next</a></main>"#,
        r#"<main>Arrived</main>"#,
    ]);
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    let snapshot = session.execute(CaptureInteractiveSnapshot).unwrap();
    session
        .execute(ClickElement {
            reference: snapshot.elements[0].reference,
        })
        .unwrap();
    server.join().unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);

    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::PointerOver,
            DomEventType::PointerEnter,
            DomEventType::MouseOver,
            DomEventType::MouseEnter,
            DomEventType::PointerMove,
            DomEventType::MouseMove,
            DomEventType::PointerDown,
            DomEventType::MouseDown,
            DomEventType::Focus,
            DomEventType::FocusIn,
            DomEventType::PointerUp,
            DomEventType::MouseUp,
            DomEventType::Click,
        ]
    );
    assert!(events.iter().all(|event| {
        event.target == "next" && event.target_ordinal == 2 && event.path == ["next", "root"]
    }));
}

#[test]
fn pointer_click_records_playwright_target_and_focus_order() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="root"><button id="first">First</button><button id="second">Second</button></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    session
        .execute(FocusByLocator {
            locator: Locator::from(CssLocator::new("#first").unwrap()),
        })
        .unwrap();
    assert!(session.execute(TakeDomEvents).unwrap().is_empty());

    session
        .execute(ClickByLocator {
            locator: Locator::from(CssLocator::new("#second").unwrap()),
        })
        .unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    let focused = session
        .execute(GetFocusedByLocator {
            locator: Locator::from(CssLocator::new("#second").unwrap()),
        })
        .unwrap();
    let hovered = session
        .execute(GetHoveredByLocator {
            locator: Locator::from(CssLocator::new("#second").unwrap()),
        })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        events
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::PointerOver,
            DomEventType::PointerEnter,
            DomEventType::MouseOver,
            DomEventType::MouseEnter,
            DomEventType::PointerMove,
            DomEventType::MouseMove,
            DomEventType::PointerDown,
            DomEventType::MouseDown,
            DomEventType::Blur,
            DomEventType::FocusOut,
            DomEventType::Focus,
            DomEventType::FocusIn,
            DomEventType::PointerUp,
            DomEventType::MouseUp,
            DomEventType::Click,
        ]
    );
    assert!(
        events[..8]
            .iter()
            .all(|event| { event.target == "second" && event.related_target.is_none() })
    );
    assert!(events[8..10].iter().all(|event| {
        event.target == "first"
            && event.related_target.as_ref().is_some_and(|target| {
                target.target == "second" && target.target_ordinal == events[10].target_ordinal
            })
    }));
    assert!(events[10..12].iter().all(|event| {
        event.target == "second"
            && event.related_target.as_ref().is_some_and(|target| {
                target.target == "first" && target.target_ordinal == events[8].target_ordinal
            })
    }));
    assert!(events[12..].iter().all(|event| event.target == "second"));
    assert!(focused.focused);
    assert!(hovered.hovered);
}

#[test]
fn hover_records_playwright_chromium_transition_order() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="root"><button id="first">First</button><button id="second">Second</button></main>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();
    let first = Locator::from(CssLocator::new("#first").unwrap());
    let second = Locator::from(CssLocator::new("#second").unwrap());
    session
        .execute(HoverByLocator {
            locator: first.clone(),
        })
        .unwrap();
    session.execute(TakeDomEvents).unwrap();

    session
        .execute(HoverByLocator {
            locator: second.clone(),
        })
        .unwrap();
    let transition = session.execute(TakeDomEvents).unwrap();
    session
        .execute(HoverByLocator {
            locator: second.clone(),
        })
        .unwrap();
    let repeated = session.execute(TakeDomEvents).unwrap();
    let first_state = session
        .execute(GetHoveredByLocator { locator: first })
        .unwrap();
    let second_state = session
        .execute(GetHoveredByLocator { locator: second })
        .unwrap();
    drop(network_guard);

    assert_eq!(
        transition
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![
            DomEventType::PointerOut,
            DomEventType::PointerLeave,
            DomEventType::PointerOver,
            DomEventType::PointerEnter,
            DomEventType::MouseOut,
            DomEventType::MouseLeave,
            DomEventType::MouseOver,
            DomEventType::MouseEnter,
            DomEventType::PointerMove,
            DomEventType::MouseMove,
        ]
    );
    assert!(transition[..2].iter().all(|event| {
        event.target == "first"
            && event
                .related_target
                .as_ref()
                .is_some_and(|target| target.target == "second")
    }));
    assert!(transition[2..4].iter().all(|event| {
        event.target == "second"
            && event
                .related_target
                .as_ref()
                .is_some_and(|target| target.target == "first")
    }));
    assert!(transition[4..6].iter().all(|event| {
        event.target == "first"
            && event
                .related_target
                .as_ref()
                .is_some_and(|target| target.target == "second")
    }));
    assert!(transition[6..8].iter().all(|event| {
        event.target == "second"
            && event
                .related_target
                .as_ref()
                .is_some_and(|target| target.target == "first")
    }));
    assert!(
        transition[8..]
            .iter()
            .all(|event| { event.target == "second" && event.related_target.is_none() })
    );
    assert_eq!(
        repeated
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        vec![DomEventType::PointerMove, DomEventType::MouseMove]
    );
    assert!(!first_state.hovered);
    assert!(second_state.hovered);
}

#[test]
fn optional_tags_preserve_native_event_ancestry() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<ul id="list"><li id="first"><input id="one" type="checkbox"><li id="second"><input id="two" type="checkbox"></ul>"#,
    );
    let mut session = Session::new();
    session.execute(OpenPage { url }).unwrap();
    server.join().unwrap();

    session
        .execute(SetCheckedByLocator {
            locator: Locator::from(CssLocator::first("#second > #two").unwrap()),
            checked: true,
        })
        .unwrap();
    let events = session.execute(TakeDomEvents).unwrap();
    drop(network_guard);

    assert_eq!(events.len(), 15);
    assert_eq!(events[12].event_type, DomEventType::Click);
    assert_eq!(events[13].event_type, DomEventType::Input);
    assert_eq!(events[14].event_type, DomEventType::Change);
    assert_eq!(events[0].target_ordinal, 5);
    assert_eq!(events[0].path, vec!["two", "second", "list"]);
    assert!(events.iter().all(|event| event.path == events[0].path));
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

#[test]
fn package_mutation_batch_is_atomic_and_clean_equivalent() {
    let initial = LayoutInput {
        viewport_width: 320,
        elements: vec![ElementInput::supported("hero", 20, 40)],
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
        .execute(ApplyMutations {
            mutations: vec![
                LayoutMutation::SetX {
                    element: "hero".into(),
                    x: 280,
                },
                LayoutMutation::SetWidth {
                    element: "hero".into(),
                    width: 80,
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
fn failed_package_mutation_batch_preserves_the_committed_layout() {
    let mut session = Session::new();
    session
        .execute(LintLayout {
            input: LayoutInput {
                viewport_width: 320,
                elements: vec![ElementInput::supported("hero", 20, 40)],
            },
        })
        .unwrap();

    let failure = session.execute(ApplyMutations {
        mutations: vec![
            LayoutMutation::SetX {
                element: "hero".into(),
                x: 300,
            },
            LayoutMutation::SetWidth {
                element: "missing".into(),
                width: 80,
            },
        ],
    });
    let after_failure = session
        .execute(ApplyMutation {
            mutation: LayoutMutation::SetWidth {
                element: "hero".into(),
                width: 40,
            },
        })
        .unwrap();

    assert_eq!(
        failure,
        Err(SessionError::Layout(
            browser_jr::LayoutError::UnknownElement("missing".into())
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
