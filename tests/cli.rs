use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
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

fn run_session_script(script: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .arg("session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn help_runs_through_the_binary_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("browser.jr lint <url>")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn interactive_snapshot_reports_ordered_agent_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email address</label><input id="email"><button id="save">Save</button>"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", &url, "--interactive"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("snapshot=1"));
    assert!(stdout.contains("mode=interactive elements=2"));
    assert!(stdout.contains(r#"- textbox "Email address" [ref=@e1]"#));
    assert!(stdout.contains(r#"- button "Save" [ref=@e2]"#));
    assert!(output.stderr.is_empty());
}

#[test]
fn interactive_snapshot_json_matches_the_agent_envelope() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email address</label><input id="email"><button>Save</button>"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["--json", "snapshot", &url, "--interactive"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], true);
    assert_eq!(value["data"]["origin"], url);
    assert_eq!(value["data"]["refs"]["e1"]["role"], "textbox");
    assert_eq!(value["data"]["refs"]["e1"]["name"], "Email address");
    assert_eq!(value["data"]["refs"]["e2"]["role"], "button");
    assert_eq!(
        value["data"]["snapshot"],
        "- textbox \"Email address\" [ref=e1]: \"\"\n- button \"Save\" [ref=e2]"
    );
    assert!(value["error"].is_null());
    assert!(output.stderr.is_empty());
}

#[test]
fn snapshot_json_reports_load_failures_on_stdout() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", "http://example.com", "--interactive", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], false);
    assert!(value["data"].is_null());
    assert!(value["error"].as_str().unwrap().contains("loopback"));
    assert!(output.stderr.is_empty());
}

#[test]
fn snapshot_json_reports_scope_failures_on_stdout() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args([
            "snapshot",
            &url,
            "--interactive",
            "--selector",
            "#missing",
            "--json",
        ])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], false);
    assert!(value["data"].is_null());
    assert!(
        value["error"]
            .as_str()
            .unwrap()
            .contains("no element matches")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn one_shot_snapshot_scopes_interactive_elements_with_css() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button>Outside</button>
            <main><section><input aria-label="Email"><button>Inside</button></section></main>
        "#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", &url, "-i", "-s", "main > section"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mode=interactive elements=2"));
    assert!(stdout.contains(r#"- textbox "Email" [ref=@e1]"#));
    assert!(stdout.contains(r#"- button "Inside" [ref=@e2]"#));
    assert!(!stdout.contains("Outside"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_keeps_snapshot_refs_for_link_navigation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>First &quot;page&quot;</title><a href="/next">Next</a>"#,
        r#"<title>Second page</title><button>Arrived</button>"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nget url\nget title\nsnapshot --interactive\nget attr @e1 href\nget attr @e1 title\nget text @e1\nclick @e1\nget url\nget title\nsnapshot --interactive\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("session ready\n"));
    assert!(stdout.contains(&format!("opened url={url} elements=1")));
    assert!(stdout.contains(&format!("\nurl={url}\n")));
    assert!(stdout.contains("\ntitle=\"First \\\"page\\\"\"\n"));
    assert!(stdout.contains(r#"- link "Next" [ref=@e1]"#));
    assert!(stdout.contains(r#"attr ref=@e1 name="href" value="/next""#));
    assert!(stdout.contains(r#"attr ref=@e1 name="title" value=null"#));
    assert!(stdout.contains(r#"text ref=@e1 "Next""#));
    assert!(stdout.contains(&format!("navigated ref=@e1 url={url}next elements=1")));
    assert!(stdout.contains(&format!("\nurl={url}next\n")));
    assert!(stdout.contains("\ntitle=\"Second page\"\n"));
    assert!(stdout.contains("snapshot=2"));
    assert!(stdout.contains(r#"- button "Arrived" [ref=@e1]"#));
    assert!(stdout.contains(r#"text ref=@e1 "Arrived""#));
    assert!(stdout.ends_with("session closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_uses_role_text_and_fill_without_a_prior_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Save Draft</button><button>Publish</button><label>Email address<input value="old"></label>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind role BUTTON text --name draft\nfind role textbox fill new value --exact --name Email address\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\nSave Draft\n"));
    assert!(
        stdout.contains(
            r#"filled role="textbox" name="Email address" element="input[4]" characters=9"#
        )
    );
    assert!(stdout.contains(r#"- textbox "Email address" [ref=@e3]: "new value""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_uses_label_placeholder_and_text_locators() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <label for="email">Email address</label><input id="email" value="old">
            <input id="search" aria-label="Search" placeholder="Search docs">
            <button>Save <span>draft</span></button>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind label \"Email address\" fill new value --exact\nfind placeholder \"Search docs\" fill query --exact\nfind text \"Save draft\" text --exact\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout
            .contains(r#"filled role="textbox" name="Email address" element="email" characters=9"#)
    );
    assert!(
        stdout.contains(r#"filled role="textbox" name="Search" element="search" characters=5"#)
    );
    assert!(stdout.contains("\nSave draft\n"));
    assert!(stdout.contains(r#"- textbox "Email address" [ref=@e1]: "new value""#));
    assert!(stdout.contains(r#"- textbox "Search" [ref=@e2]: "query""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_clicks_by_text_and_re_resolves_a_label() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<a href="/next">Next page</a>"#,
        r#"<label for="note">Note</label><input id="note">"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nfind text \"Next page\"\nfind label Note fill arrived --exact\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        r#"navigated role="link" name="Next page" element="a[1]" url={url}next elements=1"#
    )));
    assert!(stdout.contains(r#"filled role="textbox" name="Note" element="note" characters=7"#));
    assert!(stdout.contains(r#"- textbox "Note" [ref=@e1]: "arrived""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn failed_label_locator_preserves_current_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input aria-label="Email" value="first"><input aria-label="Email" value="second"><button>Save</button>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind label Email fill changed --exact\nget value @e1\nget text @e3\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"value ref=@e1 "first""#));
    assert!(stdout.contains(r#"text ref=@e3 "Save""#));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(r#"2 elements match exact label "Email"; locator must be unique"#));
    assert!(!stderr.contains("unknown or stale element reference"));
}

#[test]
fn session_mode_supports_alt_title_test_id_and_css_positions() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <img alt="Product image">
            <span title="Issue count">25 issues</span>
            <input id="email" data-testid="email-field" value="old">
            <div class="card" data-kind="item">Alpha</div>
            <div class="card" data-kind="item">Beta</div>
            <div class="card" data-kind="item">Gamma</div>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind alt \"Product image\" text --exact\nfind title \"Issue count\" text --exact\nfind testid email-field fill new\nfind first .card text\nfind last div.card text\nfind nth 1 '[data-kind=item]' text\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\n25 issues\n"));
    assert!(stdout.contains(r#"filled role="textbox" name="" element="email" characters=3"#));
    assert!(stdout.contains("\nAlpha\n"));
    assert!(stdout.contains("\nBeta\n"));
    assert!(stdout.contains("\nGamma\n"));
    assert!(stdout.contains(r#"- textbox "" [ref=@e1]: "new""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_css_position_selector_preserves_current_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<input value="old"><div class="card">Card</div>"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind first \"div[\" text\nget value @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"value ref=@e1 "old""#));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid locator: CSS selector is invalid"));
    assert!(!stderr.contains("unknown or stale element reference"));
}

#[test]
fn session_mode_uses_direct_css_and_xpath_selectors() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form><input id="email" name="email" value="old"></form>
            <section class="cards"><button>One</button><button>Two</button></section>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfill \"form > input[name=email]\" changed\nget text \"//section[@class='cards']/button[2]\"\nfind css \"section.cards > button:first-child\" text\nfind xpath \"//form/input\" text\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"filled role="textbox" name="" element="email" characters=7"#));
    assert!(stdout.contains("\nTwo\n"));
    assert!(stdout.contains("\nOne\n"));
    assert!(stdout.contains(r#"- textbox "" [ref=@e1]: "changed""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_counts_direct_css_and_xpath_matches() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <section class="cards">
                <button class="card">One</button>
                <button class="card">Two</button>
            </section>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nget count \"section.cards > .card\"\nget count \"//section/button\"\nget count .missing\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\n2\n2\n0\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_and_changes_state_through_direct_selectors() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="email" value="old">
            <input id="terms" type="checkbox">
            <select id="size">
                <option value="small">Small</option>
                <option value="large">Large</option>
            </select>
            <button id="disabled" disabled>Save</button>
            <div id="hidden" hidden>Hidden</div>
            <div id="card" data-kind="demo">Card</div>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nget value '#email'\nget attr '#card' data-kind\nget attr '#card' missing\nis checked '#terms'\nis enabled '#disabled'\nis visible '#hidden'\ncheck '#terms'\nis checked '#terms'\nselect '#size' large\nget value '#size'\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\nold\ndemo\nnull\nfalse\nfalse\nfalse\n"));
    assert!(stdout.contains(r#"checked role="checkbox" name="" element="terms" checked=true"#));
    assert!(stdout.contains("\ntrue\n"));
    assert!(stdout.contains(r#"selected role="combobox" name="" element="size" value="large""#));
    assert!(stdout.contains("\nlarge\nsession closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_normalized_inner_html_by_selector_and_reference() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <section id="card"><span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note --></section>
            <button id="action"><span>Save</span></button>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nget html '#card'\nget html \"//section[@id='card']\"\nsnapshot -i\nget html @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = r#"<span data-x="a&amp;b">Hello &amp; <b>world</b></span><!-- note -->"#;
    assert_eq!(stdout.matches(expected).count(), 2);
    assert!(stdout.contains(r#"html ref=@e1 "<span>Save</span>""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_scopes_snapshots_and_resolves_compact_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button>Outside</button>
            <main><section><input aria-label="Email"><button>Inside</button></section></main>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i -s \"main > section\"\nget text @e2\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mode=interactive elements=2"));
    assert!(!stdout.contains(r#"button "Outside""#));
    assert!(stdout.contains(r#"text ref=@e2 "Inside""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn direct_css_click_navigates_without_a_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<main><a class="next" href="/next">Next</a></main>"#,
        r#"<h1>Arrived</h1>"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nclick \"main > a.next\"\nfind role heading text --name Arrived\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("url={}next", url)));
    assert!(stdout.contains("\nArrived\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_checks_and_unchecks_by_role_without_a_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<label><input type="checkbox">Terms</label>"#);
    let output = run_session_script(&format!(
        "open {url}\nfind role checkbox check --name Terms\nfind role checkbox uncheck --name Terms\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(r#"checked role="checkbox" name="Terms" element="input[2]" checked=true"#)
    );
    assert!(
        stdout.contains(r#"checked role="checkbox" name="Terms" element="input[2]" checked=false"#)
    );
    assert!(stdout.contains(r#"- checkbox "Terms" [ref=@e1] [checked=false]"#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_clicks_a_link_by_role_and_re_resolves_the_new_document() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![r#"<a href="/next">Next</a>"#, r#"<h1>Arrived</h1>"#]);
    let output = run_session_script(&format!(
        "open {url}\nfind role link --exact --name Next\nfind role heading text --exact --name Arrived\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        r#"navigated role="link" name="Next" element="a[1]" url={url}next elements=0"#
    )));
    assert!(stdout.contains("\nArrived\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn unsupported_role_hover_preserves_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button>"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role button hover --name Save\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(
        r#"cannot hover role "button" with accessible name containing "Save": hover state and pointer event dispatch are not implemented"#
    ));
}

#[test]
fn session_mode_finds_structural_role_text() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<header aria-label="Site header"><h1>Home</h1></header><main><h2>Skills</h2><ul><li>Rust</li><li>Go</li></ul><button>Save</button></main>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role heading text --exact --name Skills\nfind role list text\nfind role banner text --exact --name Site header\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\nSkills\n"));
    assert!(stdout.contains("\nRust Go\n"));
    assert!(stdout.contains("\nHome\n"));
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn failed_role_find_preserves_the_current_reference() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<button>Save</button><button>Save Changes</button>"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role button --name missing\nget text @e1\nfind role button --name save\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(
            r#"no element matches role "button" with accessible name containing "missing""#
        )
    );
    assert!(
        stderr.contains(r#"2 elements match role "button" with accessible name containing "save""#)
    );
    assert!(!stderr.contains("unknown or stale element reference"));
}

#[test]
fn session_mode_rejects_refs_after_navigation_until_a_new_snapshot() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<a href="/next">Next</a>"#,
        r#"<a href="/again">Again</a>"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nclick @e1\nclick @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("navigated ref=@e1 url={url}next elements=1")));
    assert!(stdout.ends_with("session closed\n"));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unknown or stale element reference @e1")
    );
}

#[test]
fn session_mode_reloads_the_current_page() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<title>First</title><button>First</button>"#,
        r#"<title>Second</title><button>Second</button>"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nget title\nsnapshot -i\nreload\nget title\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("title=\"First\""));
    assert!(stdout.contains(r#"- button "First" [ref=@e1]"#));
    assert!(stdout.contains(&format!("reloaded url={url} elements=1")));
    assert!(stdout.contains("title=\"Second\""));
    assert!(stdout.contains(r#"- button "Second" [ref=@e1]"#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_preserves_refs_after_an_unsupported_click() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<button>Save</button><a href="/next">Next</a>"#,
        r#"<button>Arrived</button>"#,
    ]);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nclick @e1\nclick @e2\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("navigated ref=@e2 url={url}next elements=1")));
    assert!(stdout.contains(r#"- button "Arrived" [ref=@e1]"#));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("cannot click @e1: click execution for role button is not implemented")
    );
}

#[test]
fn session_mode_fills_text_and_reports_the_new_value() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<label for="email">Email</label><input id="email" value="old">"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfill @e1 hello world\nget value @e1\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"- textbox "Email" [ref=@e1]: "old""#));
    assert!(stdout.contains("filled ref=@e1 characters=11"));
    assert!(stdout.contains(r#"value ref=@e1 "hello world""#));
    assert!(stdout.contains(r#"- textbox "Email" [ref=@e1]: "hello world""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_selects_an_exact_option_value() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<select aria-label="Size"><option value="s">Small</option><option value="large value">Large</option></select>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nselect @e1 large value\nget value @e1\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"- combobox "Size" [ref=@e1]: "s""#));
    assert!(stdout.contains(r#"selected ref=@e1 value="large value""#));
    assert!(stdout.contains(r#"value ref=@e1 "large value""#));
    assert!(stdout.contains(r#"- combobox "Size" [ref=@e1]: "large value""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_selects_multiple_quoted_option_values() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<select aria-label="Many" multiple><option value="a" selected>A</option><option value="b">B</option></select>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nselect @e1 \"b\" \"a\"\nget value @e1\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"- listbox "Many" [ref=@e1]: "a""#));
    assert!(stdout.contains(r#"selected ref=@e1 values=["b", "a"]"#));
    assert!(stdout.contains(r#"value ref=@e1 "a""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_supported_static_visibility() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button>Visible</button><button hidden>Hidden</button><div style="display:none"><button>Ancestor hidden</button></div>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nis visible @e1\nis visible @e2\nis visible @e3\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("visible ref=@e1 value=true"));
    assert!(stdout.contains("visible ref=@e2 value=false"));
    assert!(stdout.contains("visible ref=@e3 value=false"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_changes_and_reads_checkbox_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label><input type="checkbox"> Updates</label><input type="checkbox" disabled aria-label="Locked">"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nis enabled @e1\nis enabled @e2\nis checked @e1\ncheck @e1\nis checked @e1\nuncheck @e1\nis checked @e1\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"- checkbox "Updates" [ref=@e1] [checked=false]"#));
    assert!(stdout.contains("enabled ref=@e1 value=true"));
    assert!(stdout.contains("enabled ref=@e2 value=false"));
    assert!(stdout.contains("checked ref=@e1 value=false"));
    assert!(stdout.contains("set checked ref=@e1 value=true"));
    assert!(stdout.contains("checked ref=@e1 value=true"));
    assert!(stdout.contains("set checked ref=@e1 value=false"));
    assert!(stdout.ends_with("session closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn snapshot_requires_interactive_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", "http://localhost:3000"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("requires --interactive")
    );
}

#[test]
fn invalid_snapshot_selector_fails_before_loading() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args([
            "snapshot",
            "http://127.0.0.1:1",
            "--interactive",
            "--selector",
            "div[",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid snapshot selector")
    );
}

#[test]
fn empty_interactive_snapshot_succeeds() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<p>Nothing actionable</p>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", &url, "--interactive"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("mode=interactive elements=0")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn page_load_failure_is_not_a_pass() {
    let network_guard = network_test_guard();
    let unavailable_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", unavailable_listener.local_addr().unwrap());
    drop(unavailable_listener);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", &url])
        .output()
        .unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("page request failed")
    );
}

#[test]
fn local_page_overflow_reports_structured_evidence() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<div id="hero" style="position:fixed;left:280px;width:80px"></div>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", &url, "--viewport", "320"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(
        output.status.code(),
        Some(1),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("finding rule=horizontal-overflow"));
    assert!(stdout.contains("severity=error element=hero viewport=320"));
    assert!(stdout.contains("expectation=inside-viewport"));
    assert!(stdout.contains("observed=left:280,width:80,right:360"));
    assert!(stdout.contains("evidence=snapshot:1#hero"));
    assert!(stdout.contains("fail rule=horizontal-overflow"));
    assert!(output.stderr.is_empty());
}

#[test]
fn fitting_local_page_passes() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<main id="content" style="position:fixed;left:0;width:320px"></main>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", &url, "--viewport", "320"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("pass rule=horizontal-overflow")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn normal_block_local_page_passes() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<main id="content"></main>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", &url, "--viewport", "320"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("pass rule=horizontal-overflow")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn unsupported_page_layout_is_blocked() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<span id="content"></span>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", &url])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("blocked rule=horizontal-overflow element=content"));
    assert!(!stderr.contains("pass"));
}

#[test]
fn project_width_limit_reports_observed_width() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<main id="content" style="width:880px"></main>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args([
            "lint",
            &url,
            "--viewport",
            "1280",
            "--max-width",
            "content",
            "720",
        ])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("finding rule=max-element-width"));
    assert!(stdout.contains("element=content viewport=1280"));
    assert!(stdout.contains("expectation=width<=720 observed=width:880"));
    assert!(stdout.contains("evidence=snapshot:1#content"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reports_dom_events_with_normalized_ancestry() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="app"><input id="direct"><section><input id="deep"></section></main>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind first \"html body main > input\" fill direct value\nevents\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("events=1"));
    assert!(stdout.contains(
        r#"event type=input document=1 target="direct" bubbles=true path="direct > app""#
    ));
}
