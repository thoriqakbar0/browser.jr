use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

static NETWORK_TEST: Mutex<()> = Mutex::new(());
static SCREENSHOT_TEST_ID: AtomicU64 = AtomicU64::new(1);

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

fn run_json_session_script(script: &str, trailing_flag: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_browser-jr"));
    if trailing_flag {
        command.args(["session", "--json"]);
    } else {
        command.args(["--json", "session"]);
    }
    let mut child = command
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

fn screenshot_test_path(name: &str) -> PathBuf {
    let id = SCREENSHOT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("browser-jr-{name}-{}-{id}.png", std::process::id()))
}

#[test]
fn help_runs_through_the_binary_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "unexpected stdout: {}\nunexpected stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("browser.jr lint <url>"));
    assert!(stdout.contains("browser.jr read <url>"));
    assert!(stdout.contains("browser.jr --json session"));
    assert!(stdout.contains("one envelope for each lifecycle event and input command"));
    assert!(stdout.contains("native action events through the events command"));
    assert!(stdout.contains("solid-box PNG screenshots"));
    assert!(output.stderr.is_empty());
}

#[test]
fn read_command_returns_normalized_static_page_text() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<title>Docs</title><main>Hello <span>world</span><script>ignore()</script> <button>Save</button><input value="secret"></main>"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["read", &url])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Hello world Save\n"
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
fn interactive_snapshot_options_match_agent_browser_output() {
    let network_guard = network_test_guard();
    let body = r#"<a href="guide/next?q=1#details">Next</a><button>Save</button>"#;
    let (url, server) = serve_pages(vec![body, body]);
    let human = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args([
            "snapshot",
            &url,
            "--interactive",
            "--urls",
            "--compact",
            "--depth",
            "0",
        ])
        .output()
        .unwrap();
    let json = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["--json", "snapshot", &url, "-i", "-u", "-c", "-d", "1"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(human.status.success());
    let expected_url = format!("{url}guide/next?q=1#details");
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains(&format!(r#"- link "Next" [ref=@e1, url={expected_url}]"#)));
    assert!(stdout.contains(r#"- button "Save" [ref=@e2]"#));
    assert!(human.stderr.is_empty());

    assert!(json.status.success());
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert!(
        value["data"]["snapshot"]
            .as_str()
            .unwrap()
            .contains(&format!(r#"- link "Next" [ref=e1, url={expected_url}]"#))
    );
    assert_eq!(value["data"]["refs"]["e1"]["role"], "link");
    assert!(json.stderr.is_empty());
}

#[test]
fn session_mode_uses_snapshot_compatibility_options() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<a href="next#details">Next</a><button>Save</button>"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i --urls --compact --depth 0\nget text @e2\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        r#"- link "Next" [ref=@e1, url={url}next#details]"#
    )));
    assert!(stdout.contains(r#"text ref=@e2 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_writes_viewport_and_full_page_png_screenshots() {
    let network_guard = network_test_guard();
    let viewport_path = screenshot_test_path("viewport");
    let full_path = screenshot_test_path("full");
    let element_path = screenshot_test_path("element");
    let (url, server) = serve_page(
        r#"
            <body style="margin-left:0;margin-right:0;margin-top:0;margin-bottom:0;background-color:#102030">
                <main id="target" style="width:40px;height:60px;background-color:#ff0000"></main>
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nset viewport 40 30\nscreenshot {}\nscreenshot --full {}\nscreenshot #target {}\nexit\n",
        viewport_path.display(),
        full_path.display(),
        element_path.display()
    ));
    server.join().unwrap();
    let viewport_png = std::fs::read(&viewport_path).unwrap();
    let full_png = std::fs::read(&full_path).unwrap();
    let element_png = std::fs::read(&element_path).unwrap();
    std::fs::remove_file(&viewport_path).unwrap();
    std::fs::remove_file(&full_path).unwrap();
    std::fs::remove_file(&element_path).unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("width=40 height=30"));
    assert!(stdout.contains("width=40 height=60"));
    assert_eq!(&viewport_png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&full_png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&element_png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_blocks_incomplete_screenshot_paint_without_writing_a_file() {
    let network_guard = network_test_guard();
    let path = screenshot_test_path("blocked");
    let (url, server) =
        serve_page(r#"<main style="width:100px;height:20px;background-color:#fff">hello</main>"#);
    let output = run_session_script(&format!(
        "open {url}\nscreenshot {}\nexit\n",
        path.display()
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    assert!(!path.exists());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("text paint")
    );
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
fn json_session_streams_identified_results_and_recovers_after_errors() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<title>Docs</title><input id="secret"><button>Save</button>"#);
    let output = run_json_session_script(
        &format!(
            "open {url}\nsnapshot -i\nfill #secret private-value\nevents\nwat\nget title\nexit\nget url\n"
        ),
        false,
    );
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("private-value"));
    let values = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 9);
    assert_eq!(values[0]["data"]["event"], "ready");
    assert_eq!(values[1]["data"]["sequence"], 1);
    assert!(
        values[1]["data"]["output"]
            .as_str()
            .unwrap()
            .contains("opened")
    );
    assert_eq!(values[2]["data"]["sequence"], 2);
    assert!(
        values[2]["data"]["output"]
            .as_str()
            .unwrap()
            .contains(r#"- button "Save" [ref=@e2]"#)
    );
    assert_eq!(values[3]["success"], true);
    assert_eq!(values[3]["data"]["sequence"], 3);
    assert_eq!(values[4]["success"], true);
    assert_eq!(values[4]["data"]["sequence"], 4);
    assert!(
        values[4]["data"]["output"]
            .as_str()
            .unwrap()
            .contains("events=2\nevent type=beforeinput")
    );
    assert_eq!(values[5]["success"], false);
    assert_eq!(values[5]["data"]["sequence"], 5);
    assert!(
        values[5]["error"]
            .as_str()
            .unwrap()
            .contains("invalid session command")
    );
    assert_eq!(values[6]["success"], true);
    assert_eq!(values[6]["data"]["output"], r#"title="Docs""#);
    assert_eq!(values[7]["data"]["output"], "");
    assert_eq!(values[8]["data"]["event"], "closed");
}

#[test]
fn json_session_accepts_the_trailing_flag_form() {
    let output = run_json_session_script("get viewport\n", true);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let values = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        values[1]["data"]["output"],
        "viewport width=1280 height=720"
    );
    assert_eq!(values[2]["data"]["event"], "closed");
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
    assert!(stdout.contains(r#"- textbox "Email" [ref=@e2]"#));
    assert!(stdout.contains(r#"- button "Inside" [ref=@e3]"#));
    assert!(!stdout.contains("Outside"));
    assert!(output.stderr.is_empty());
}

#[test]
fn full_snapshot_preserves_ordered_text_depth_urls_and_document_refs() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="main"><h1>Title <em>now</em></h1><p>Before <strong>bold</strong> after.</p><nav aria-label="Primary"><a href="/docs">Docs</a></nav><section><h2>Section</h2><button>Save</button></section></main>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot --urls\nget text @e1\nsnapshot --interactive --urls\nget text @e1\nsnapshot -s section --depth 1\nget text @e5\nexit\n"
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
    assert!(stdout.contains("mode=full nodes=14"));
    assert!(stdout.contains(r#"  - heading "Title now" [ref=e1]"#));
    assert!(stdout.contains(r#"    - StaticText "Title""#));
    assert!(stdout.contains(r#"    - emphasis "now""#));
    assert!(stdout.contains(r#"    - StaticText "Before""#));
    assert!(stdout.contains(r#"    - strong "bold""#));
    assert!(stdout.contains(r#"    - StaticText "after.""#));
    assert!(stdout.contains(&format!(r#"    - link "Docs" [ref=e3, url={url}docs]"#)));
    assert!(stdout.contains(r#"text ref=@e1 "Title now""#));
    assert!(stdout.contains("mode=interactive elements=5"));
    assert!(stdout.contains(r#"- navigation "Primary" [ref=@e2]"#));
    assert!(stdout.contains(&format!(r#"  - link "Docs" [ref=@e3, url={url}docs]"#)));
    assert!(stdout.contains("mode=full nodes=3"));
    assert!(stdout.contains(r#"- region"#));
    assert!(stdout.contains(r#"  - heading "Section" [ref=e4]"#));
    assert!(stdout.contains(r#"  - button "Save" [ref=e5]"#));
    assert!(stdout.contains(r#"text ref=@e5 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn full_snapshot_json_uses_the_tree_text_and_ref_map() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<main><h1>Hello</h1><a href="/docs">Docs</a><section></section></main>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", &url, "--json", "--urls", "--compact"])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], true);
    assert_eq!(value["data"]["refs"]["e1"]["role"], "heading");
    assert_eq!(value["data"]["refs"]["e2"]["name"], "Docs");
    let snapshot = value["data"]["snapshot"].as_str().unwrap();
    assert!(snapshot.contains(r#"- main"#));
    assert!(snapshot.contains(r#"  - heading "Hello" [ref=e1]"#));
    assert!(snapshot.contains(&format!(r#"  - link "Docs" [ref=e2, url={url}docs]"#)));
    assert!(!snapshot.contains("region"));
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
fn session_mode_reports_native_click_effects_and_preserves_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" type="button">Save</button>
            <label><input id="terms" type="checkbox">Accept terms</label>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nclick @e1\nis focused @e1\nfind role checkbox click --name Accept terms --exact\nis checked @e2\nis focused @e2\nclick #terms\nis checked @e2\nget text @e1\nexit\n"
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
    assert!(stdout.contains("clicked ref=@e1 focused=true"));
    assert!(stdout.contains(
        r#"clicked role="checkbox" name="Accept terms" element="terms" focused=true checked=true"#
    ));
    assert!(stdout.contains(
        r#"clicked role="checkbox" name="Accept terms" element="terms" focused=true checked=false"#
    ));
    assert!(stdout.contains("focused ref=@e1 value=true"));
    assert!(stdout.contains("checked ref=@e2 value=true"));
    assert!(stdout.contains("focused ref=@e2 value=true"));
    assert!(stdout.contains("checked ref=@e2 value=false"));
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
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
fn session_mode_finds_icon_only_buttons_by_accessible_name() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<button id="save"><span><img alt="Save image"></span></button>"#);
    let output = run_session_script(&format!(
        "open {url}\nfind role button text --name Save image --exact\nsnapshot -i\nexit\n"
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
    assert!(stdout.contains(r#"- button "Save image" [ref=@e1]"#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_filters_role_accessibility_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <h1>Overview</h1><h2>Overview</h2>
            <label><input type="checkbox" checked>Terms</label>
            <button aria-hidden="true">Ghost</button>
            <p id="help">Opens account settings</p><button aria-describedby="help">Settings</button>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind role heading text --level 2 --name Overview --exact\nfind role checkbox uncheck --checked true --name Terms --exact\nfind role checkbox check --checked false --name Terms --exact\nfind role button text --include-hidden --name Ghost --exact\nfind role button text --description OPENS ACCOUNT SETTINGS --name Settings\nexit\n"
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
    assert!(stdout.contains("\nOverview\n"));
    assert!(stdout.contains(r#"checked role="checkbox" name="Terms""#));
    assert!(stdout.contains("checked=false"));
    assert!(stdout.contains("checked=true"));
    assert!(stdout.contains("\nGhost\n"));
    assert!(stdout.contains("\nSettings\n"));
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
        "open {url}\nsnapshot -i -s \"main > section\"\nget text @e3\nexit\n"
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
    assert!(stdout.contains(r#"text ref=@e3 "Inside""#));
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
fn session_mode_selects_exclusive_radio_groups_and_updates_tab_order() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form>
                <label><input id="basic" type="radio" name="plan" checked>Basic</label>
                <label><input id="pro" type="radio" name="plan">Pro</label>
            </form>
            <form><label><input id="second" type="radio" name="plan">Second form</label></form>
            <label><input id="solo" type="radio">Solo</label>
            <button id="after" type="button">After</button>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role radio check --name Pro --exact\nis checked @e1\nis checked @e2\nclick #basic\nfind role radio press Space --name Pro --exact\nis checked @e1\nis checked @e2\npress Tab\nfind role radio focused --name Second form --exact\nget text @e5\nexit\n"
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
    assert!(stdout.contains(r#"- radio "Basic" [ref=@e1] [checked=true]"#));
    assert!(stdout.contains(r#"checked role="radio" name="Pro" element="pro" checked=true"#));
    assert!(stdout.contains(
        r#"clicked role="radio" name="Basic" element="basic" focused=true checked=true"#
    ));
    assert!(
        stdout
            .contains(r#"pressed role="radio" name="Pro" element="pro" key="Space" checked=true"#)
    );
    assert!(stdout.contains("checked ref=@e1 value=false"));
    assert!(stdout.contains("checked ref=@e2 value=true"));
    assert!(stdout.contains(
        r#"pressed key="Tab" focus="second" focus-role="radio" focus-name="Second form" previous="pro""#
    ));
    assert!(
        stdout.contains(r#"focused role="radio" name="Second form" element="second" value=true"#)
    );
    assert!(stdout.contains(r#"text ref=@e5 "After""#));
    assert!(stdout.ends_with("session closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_moves_radio_groups_with_arrow_keys() {
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
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role radio press ArrowRight --name A --exact\nis checked @e1\nis checked @e4\npress ArrowRight\npress ArrowLeft\npress ArrowDown\npress ArrowUp\nis focused @e4\nget text @e5\nexit\n"
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
    assert!(stdout.contains(
        r#"pressed role="radio" name="A" element="a" key="ArrowRight" checked=true focus="c" focus-role="radio" focus-name="C""#
    ));
    assert!(stdout.contains(r#"pressed key="ArrowRight" element="a" checked=true"#));
    assert!(stdout.contains(r#"pressed key="ArrowLeft" element="c" checked=true"#));
    assert!(stdout.contains(r#"pressed key="ArrowDown" element="a" checked=true"#));
    assert!(stdout.contains(r#"pressed key="ArrowUp" element="c" checked=true"#));
    assert!(stdout.contains("checked ref=@e1 value=false"));
    assert!(stdout.contains("checked ref=@e4 value=true"));
    assert!(stdout.contains("focused ref=@e4 value=true"));
    assert!(stdout.contains(r#"text ref=@e5 """#));
    assert!(stdout.ends_with("session closed\n"));
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
        r#"navigated role="link" name="Next" element="a[1]" url={url}next elements=1"#
    )));
    assert!(stdout.contains("\nArrived\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_submits_a_get_form_by_click_and_re_resolves_the_document() {
    let network_guard = network_test_guard();
    let form = r#"
        <form action="/search"><input id="query" name="q" value="old">
            <button name="commit" value="save">Search</button></form>
    "#;
    let (url, server) = serve_pages(vec![form, r#"<h1>Results</h1>"#]);
    let output = run_session_script(&format!(
        "open {url}\nfill #query hello world\nfind role button --exact --name Search\nget url\nfind role heading text --exact --name Results\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = format!("{url}search?q=hello+world&commit=save");
    assert!(stdout.contains(&format!(
        r#"navigated role="button" name="Search" element="button[3]" url={expected} elements=1"#
    )));
    assert!(stdout.contains(&format!("url={expected}")));
    assert!(stdout.contains("\nResults\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_submits_a_get_form_with_enter() {
    let network_guard = network_test_guard();
    let form = r#"
        <form action="/find"><input name="q" value="one">
            <button name="commit" value="go">Go</button></form>
    "#;
    let (url, server) = serve_pages(vec![form, r#"<h1>Found</h1>"#]);
    let output = run_session_script(&format!(
        "open {url}\nfind role button press Enter --name Go --exact\nfind role heading text --name Found --exact\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        r#"pressed role="button" name="Go" element="button[3]" key="Enter" url={url}find?q=one&commit=go elements=1"#
    )));
    assert!(stdout.contains("\nFound\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_implicitly_submits_a_filled_text_control_with_enter() {
    let network_guard = network_test_guard();
    let form = r#"
        <form action="/search" method="get">
            <label for="query">Query</label>
            <input id="query" name="q" value="old">
            <button name="commit" value="go">Go</button>
        </form>
    "#;
    let (url, server) = serve_pages(vec![form, r#"<h1>Results</h1>"#]);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfill @e1 rust browser\npress Enter\nfind role heading text --name Results --exact\nexit\n"
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
    assert!(stdout.contains("filled ref=@e1 characters=12"));
    assert!(stdout.contains(&format!(
        r#"pressed key="Enter" element="query" url={url}search?q=rust+browser&commit=go"#
    )));
    assert!(stdout.contains("\nResults\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reports_ignored_implicit_enter_without_navigation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <form action="/search" method="get">
                <input id="first" name="first" value="one">
                <input name="second" value="two">
            </form>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfind css #first press Enter\nget url\nget value #first\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout
            .contains(r#"pressed role="textbox" name="" element="first" key="Enter" ignored=true"#)
    );
    assert!(stdout.contains(&format!("url={url}")));
    assert!(stdout.contains("\none\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn hover_commands_track_the_current_target_and_preserve_snapshot_references() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<button id="save">Save</button><div id="card">Card</div><button id="disabled" disabled>Disabled</button>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nhover @e1\nis hovered @e1\nhover #card\nis hovered @e1\nis hovered #card\nfind role button hover --name Disabled --exact\nfind role button hovered --name Disabled --exact\nis hovered #disabled\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hovered ref=@e1"));
    assert!(stdout.contains("hovered ref=@e1 value=true"));
    assert!(stdout.contains(r#"hovered role="" name="" element="card""#));
    assert!(stdout.contains("hovered ref=@e1 value=false"));
    assert!(stdout.contains(r#"hovered role="button" name="Disabled" element="disabled""#));
    assert!(
        stdout.contains(r#"hovered role="button" name="Disabled" element="disabled" value=true"#)
    );
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_finds_structural_role_text() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<header aria-label="Site header"><h1>Home</h1></header><main><h2>Skills</h2><ul><li>Rust</li><li>Go</li></ul><search>Docs</search><button>Save</button></main>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role heading text --exact --name Skills\nfind role list text\nfind role search text\nfind role banner text --exact --name Site header\nget text @e3\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\nSkills\n"));
    assert!(stdout.contains("\nRust Go\n"));
    assert!(stdout.contains("\nDocs\n"));
    assert!(stdout.contains("\nHome\n"));
    assert!(stdout.contains(r#"text ref=@e3 "Save""#));
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
fn session_mode_moves_back_and_forward_without_losing_no_entry_state() {
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
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nback\nclick @e1\nsnapshot -i\nback\nget title\nforward\nget title\nback\nopen {branch_url}\nforward\nget title\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("back url={url} navigated=false")));
    assert!(stdout.contains(&format!("back url={url} elements=1 navigated=true")));
    assert!(stdout.contains(&format!(
        "forward url={second_url} elements=1 navigated=true"
    )));
    assert!(stdout.contains("title=\"One return\""));
    assert!(stdout.contains("title=\"Two return\""));
    assert!(stdout.contains(&format!("forward url={branch_url} navigated=false")));
    assert!(stdout.contains("title=\"Branch\""));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_current_page_and_an_optional_url() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<main>First page <button>Keep</button></main>"#,
        r#"<main>Second <strong>page</strong></main>"#,
    ]);
    let second_url = format!("{url}two");
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nread\nget text @e1\nread {second_url}\nread\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("First page Keep\n"));
    assert!(stdout.contains(r#"text ref=@e1 "Keep""#));
    assert_eq!(stdout.matches("Second page\n").count(), 2);
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_preserves_refs_after_an_unsupported_click() {
    let network_guard = network_test_guard();
    let (url, server) = serve_pages(vec![
        r#"<div role="button">Save</div><a href="/next">Next</a>"#,
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
fn session_mode_blocks_pointer_actions_without_static_stability_evidence() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" style="position:fixed;left:0;top:0;width:100px;height:40px;animation-name:pulse">Save</button>
            <input id="terms" type="checkbox" aria-label="Terms" style="position:fixed;left:0;top:50px;width:20px;height:20px;animation-name:pulse">
            <div id="card" style="position:fixed;left:0;top:80px;width:100px;height:40px;animation-name:pulse">Card</div>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nclick @e1\ncheck @e2\nhover #card\nis checked @e2\nis hovered #card\nevents\nget text @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("checked ref=@e2 value=false"));
    assert!(stdout.contains("\nfalse\n"));
    assert!(stdout.contains("events=0"));
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    assert!(
        stderr.contains(&format!(
            "browser.jr: cannot click @e1: stable check failed: {}",
            "inline animation-name stability is not implemented for save"
        )),
        "unexpected stderr: {stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "browser.jr: cannot change checked state on @e2: stable check failed: {}",
            "inline animation-name stability is not implemented for terms"
        )),
        "unexpected stderr: {stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "browser.jr: cannot hover CSS selector \"#card\": stable check blocked: {}",
            "inline animation-name stability is not implemented for card"
        )),
        "unexpected stderr: {stderr}"
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
fn session_mode_drains_native_dom_events_without_echoing_input_values() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <main id="root">
                <input id="name">
                <input id="terms" type="checkbox">
                <select id="size"><option value="s">Small</option><option value="l">Large</option></select>
                <button id="save" type="button">Save</button>
            </main>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nfill #name private-value\nkeyboard inserttext private-tail\nkeyboard type aé\npress x\nkeydown q\nkeyup q\ncheck #terms\ncheck #terms\nselect #size l\nselect #size l\nclick #save\nevents\nevents\nexit\n"
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
    assert!(stdout.contains("events=29"));
    assert!(stdout.contains("event type=beforeinput document=1 target=\"name\""));
    assert!(stdout.contains("event type=change document=1 target=\"terms\""));
    assert!(stdout.contains("event type=change document=1 target=\"size\""));
    assert!(stdout.contains("event type=click document=1 target=\"save\""));
    assert!(stdout.contains("path=\"name > root\""));
    assert!(stdout.contains("event type=keydown document=1 target=\"name\""));
    assert!(stdout.contains("event type=keypress document=1 target=\"name\""));
    assert!(stdout.contains("event type=keyup document=1 target=\"name\""));
    assert_eq!(stdout.matches("event type=").count(), 29);
    assert!(stdout.contains("events=0"));
    assert!(!stdout.contains("private-value"));
    assert!(!stdout.contains("private-tail"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_types_text_without_clearing_the_current_value() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email</label><input id="email" value="old"><textarea id="note" aria-label="Note">draft</textarea>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\ntype @e1  plus\ntype #note  more\nget value @e1\nget value #note\nsnapshot -i\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("typed ref=@e1 characters=4"));
    assert!(stdout.contains(r#"typed role="textbox" name="Note" element="note" characters=4"#));
    assert!(stdout.contains(r#"value ref=@e1 "oldplus""#));
    assert!(stdout.contains("draftmore"));
    assert!(stdout.contains(r#"- textbox "Email" [ref=@e1]: "oldplus""#));
    assert!(stdout.contains(r#"- textbox "Note" [ref=@e2]: "draftmore""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_focuses_then_presses_the_bounded_key_subset() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<label for="email">Email</label><input id="email" value="old"><textarea id="note" aria-label="Note">draft</textarea>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role textbox focus --name Email --exact\npress Z\nfocus @e2\npress Enter\npress X\nget value #email\nget value #note\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"focused role="textbox" name="Email" element="email""#));
    assert!(stdout.contains(r#"pressed key="Z" element="email" characters=4"#));
    assert!(stdout.contains(r#"focused ref=@e2 element="note""#));
    assert!(stdout.contains(r#"pressed key="Enter" element="note" characters=6"#));
    assert!(stdout.contains("Zold"));
    assert!(stdout.contains("\nXdraft"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_activates_focused_native_controls_with_keys() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="save" type="button">Save</button>
            <label><input id="terms" type="checkbox">Accept terms</label>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfocus @e1\npress Space\npress Enter\nfind role checkbox press Space --name Accept terms --exact\nis checked @e2\npress Space\nis checked @e2\nget text @e1\nexit\n"
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
    assert!(stdout.contains(r#"pressed key="Space" element="save" activated=true"#));
    assert!(stdout.contains(r#"pressed key="Enter" element="save" activated=true"#));
    assert!(stdout.contains(
        r#"pressed role="checkbox" name="Accept terms" element="terms" key="Space" checked=true"#
    ));
    assert!(stdout.contains(r#"pressed key="Space" element="terms" checked=false"#));
    assert!(stdout.contains("checked ref=@e2 value=true"));
    assert!(stdout.contains("checked ref=@e2 value=false"));
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_navigates_when_enter_activates_a_native_link() {
    let network_guard = network_test_guard();
    let first = r#"<a id="next" href="/next">Next</a>"#;
    let destination = r#"<title>Arrived</title><h1>Arrived</h1>"#;
    let (url, server) = serve_pages(vec![first, destination, first, destination]);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfind role link press Enter --name Next --exact\nget url\nget title\nback\nsnapshot -i\nfocus @e1\npress Enter\nget url\nget title\nexit\n"
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
    assert!(stdout.contains(&format!(
        r#"pressed role="link" name="Next" element="next" key="Enter" url={url}next elements=1"#
    )));
    assert!(stdout.contains(&format!(
        r#"pressed key="Enter" element="next" url={url}next elements=1"#
    )));
    assert_eq!(stdout.matches(&format!("url={url}next")).count(), 4);
    assert_eq!(stdout.matches(r#"title="Arrived""#).count(), 2);
    assert!(stdout.ends_with("session closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_focused_state_through_refs_selectors_and_semantic_locators() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <button id="first">First</button>
                <label for="second">Second</label><input id="second">
                <div id="plain">Plain</div>
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nis focused @e1\nis focused body\nfocus @e2\nis focused @e1\nis focused @e2\nis focused '#plain'\nis focused body\nfind role textbox focused --name Second --exact\npress Tab\nis focused body\nfind role textbox focused --name Second --exact\nfocus @e1\nis focused @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("focused ref=@e1 value=false"));
    assert!(stdout.contains("focused ref=@e2 value=true"));
    assert!(stdout.contains(r#"focused role="textbox" name="Second" element="second" value=true"#));
    assert!(
        stdout.contains(r#"focused role="textbox" name="Second" element="second" value=false"#)
    );
    assert!(stdout.contains(r#"pressed key="Tab" focus="body" previous="second""#));
    assert!(stdout.contains("focused ref=@e1 value=true"));
    assert!(stdout.lines().filter(|line| *line == "true").count() >= 2);
    assert!(stdout.lines().filter(|line| *line == "false").count() >= 2);
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reports_editing_key_selection_and_mutation() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<input id="text" value="abc">"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfocus @e1\npress ArrowRight\npress Shift+ArrowRight\npress X\npress ControlOrMeta+A\npress Z\nget value @e1\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(
        r#"pressed key="ArrowRight" element="text" characters=3 selection=1:1 changed=false"#
    ));
    assert!(stdout.contains(
        r#"pressed key="Shift+ArrowRight" element="text" characters=3 selection=1:2 changed=false"#
    ));
    assert!(
        stdout
            .contains(r#"pressed key="X" element="text" characters=3 selection=2:2 changed=true"#)
    );
    assert!(stdout.contains(r#"value ref=@e1 "Z""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_inserts_and_types_text_at_the_focused_selection() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<input id="text" value="abc"><input id="locked" value="fixed" readonly><button id="save">Save</button>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nkeyboard type ignored\nfocus @e1\npress ArrowRight\npress Shift+ArrowRight\nkeyboard inserttext X\nkeyboard type 😀\nget value @e1\nfocus @e2\nkeyboard type Q\nget value @e2\nfocus @e3\nkeyboard inserttext !\nexit\n"
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
    assert!(stdout.contains(r#"keyboard type element="body" characters=7 changed=false"#));
    assert!(stdout.contains(
        r#"keyboard inserttext element="text" characters=1 value_characters=3 selection=2:2 changed=true"#
    ));
    assert!(stdout.contains(
        r#"keyboard type element="text" characters=1 value_characters=4 selection=4:4 changed=true"#
    ));
    assert!(stdout.contains(r#"value ref=@e1 "aX😀c""#));
    assert!(stdout.contains(
        r#"keyboard type element="locked" characters=1 value_characters=5 selection=0:0 changed=false"#
    ));
    assert!(stdout.contains(r#"value ref=@e2 "fixed""#));
    assert!(stdout.contains(r#"keyboard inserttext element="save" characters=1 changed=false"#));
    assert!(!stdout.contains("ignored"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_holds_and_releases_keyboard_keys() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<input id="text" value="abc">"#);
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nfocus @e1\nkeydown ShiftLeft\npress ArrowRight\nkeydown x\nkeydown x\nkeyup x\nkeyup x\nkeyup Shift\nkeydown ControlOrMeta\npress a\nkeyup ControlOrMeta\nkeydown Backspace\nkeyup Backspace\nget value @e1\nexit\n"
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
    assert!(stdout.contains(r#"keydown key="Shift" repeat=false modifier=true"#));
    assert!(stdout.contains(
        r#"pressed key="Shift+ArrowRight" element="text" characters=3 selection=0:1 changed=false"#
    ));
    assert!(stdout.contains(
        r#"keydown key="x" repeat=false pressed key="X" element="text" characters=3 selection=1:1 changed=true"#
    ));
    assert!(stdout.contains(
        r#"keydown key="x" repeat=true pressed key="X" element="text" characters=4 selection=2:2 changed=true"#
    ));
    assert!(stdout.contains(r#"keyup key="x" was-pressed=true"#));
    assert!(stdout.contains(r#"keyup key="x" was-pressed=false"#));
    assert!(stdout.contains(
        r#"pressed key="ControlOrMeta+A" element="text" characters=4 selection=0:4 changed=false"#
    ));
    assert!(stdout.contains(
        r#"keydown key="Backspace" repeat=false pressed key="Backspace" element="text" characters=0 selection=0:0 changed=true"#
    ));
    assert!(stdout.contains(r#"value ref=@e1 """#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_defers_checkbox_space_until_key_up() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<input id="terms" type="checkbox">"#);
    let output = run_session_script(&format!(
        "open {url}\nfocus #terms\nkeydown Space\nis checked #terms\nkeyup Space\nis checked #terms\nevents\nexit\n"
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
    assert!(stdout.contains(r#"keydown key="Space" repeat=false deferred=true"#));
    assert!(stdout.contains(
        r#"keyup key="Space" was-pressed=true pressed key="Space" element="terms" checked=true"#
    ));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| matches!(*line, "false" | "true"))
            .collect::<Vec<_>>(),
        vec!["false", "true"]
    );
    assert_eq!(
        stdout
            .lines()
            .filter_map(|line| line.strip_prefix("event type="))
            .filter_map(|line| line.split_once(' ').map(|(event, _)| event))
            .collect::<Vec<_>>(),
        vec!["keydown", "keypress", "keyup", "click", "input", "change"]
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_finds_then_presses_without_prior_focus() {
    let network_guard = network_test_guard();
    let (url, server) =
        serve_page(r#"<label for="email">Email</label><input id="email" value="abc">"#);
    let output = run_session_script(&format!(
        "open {url}\nfind role textbox press End --name Email --exact\nfind css #email press Backspace\nget value #email\nexit\n"
    ));
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(
        r#"pressed role="textbox" name="Email" element="email" key="End" characters=3 selection=3:3 changed=false"#
    ));
    assert!(stdout.contains(
        r#"pressed role="textbox" name="Email" element="email" key="Backspace" characters=2 selection=2:2 changed=true"#
    ));
    assert!(stdout.lines().any(|line| line == "ab"));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_traverses_focus_with_tab_and_locator_press() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r##"
            <button id="natural1">one</button>
            <input id="positive2" tabindex="2">
            <a id="link" href="#x">link</a>
            <input id="positive1" tabindex="1">
        "##,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\npress Tab\npress Shift+Tab\nfind role button press Tab --name one --exact\npress Shift+Tab\nfocus @e1\nexit\n"
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
    assert!(stdout.contains(
        r#"pressed key="Tab" focus="positive1" focus-role="textbox" focus-name="" previous="body""#
    ));
    assert!(stdout.contains(r#"pressed key="Shift+Tab" focus="body" previous="positive1""#));
    assert!(stdout.contains(
        r#"pressed role="button" name="one" element="natural1" key="Tab" focus="link" focus-role="link" focus-name="link" previous="natural1""#
    ));
    assert!(stdout.contains(
        r#"pressed key="Shift+Tab" focus="natural1" focus-role="button" focus-name="one" previous="link""#
    ));
    assert!(stdout.contains(r#"focused ref=@e1 element="natural1""#));
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
fn session_mode_reads_complete_bounding_boxes_and_hidden_null() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <button id="fixed" style="position:fixed;left:20px;top:30px;width:120px;height:40px">Save</button>
            <button id="hidden" hidden style="position:fixed;left:1px;top:2px;width:3px;height:4px">Hidden</button>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nget box @e1\nget box '#fixed'\nget box \"//button[@id='fixed']\"\nget box '#hidden'\nget text @e1\nexit\n"
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
    assert_eq!(
        stdout.lines().filter(|line| *line == "x:      20").count(),
        3
    );
    assert_eq!(
        stdout.lines().filter(|line| *line == "y:      30").count(),
        3
    );
    assert_eq!(
        stdout.lines().filter(|line| *line == "width:  120").count(),
        3
    );
    assert_eq!(
        stdout.lines().filter(|line| *line == "height: 40").count(),
        3
    );
    assert!(stdout.contains("height: 40\nnull\n"));
    assert!(stdout.contains(r#"text ref=@e1 "Save""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_reads_normal_flow_boxes_through_references_and_selectors() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <main id="shell" style="padding-top:4px">
                    <section id="first" style="height:20px"></section>
                    <button id="action" style="display:block;box-sizing:border-box;width:100px;height:24px">Act</button>
                    <section id="second" style="height:30px"></section>
                </main>
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nis visible @e1\nis visible '#second'\nget box @e1\nget box '#second'\nget box body\nget text @e1\nexit\n"
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
    assert!(stdout.contains("visible ref=@e1 value=true\ntrue\n"));
    assert!(stdout.contains("x:      8\ny:      32\nwidth:  100\nheight: 24"));
    assert!(stdout.contains("x:      8\ny:      56\nwidth:  1264\nheight: 30"));
    assert!(stdout.contains("x:      8\ny:      8\nwidth:  1264\nheight: 78"));
    assert!(stdout.contains(r#"text ref=@e1 "Act""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_scrolls_pages_and_elements_through_refs_selectors_and_roles() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <div style="width:1800px;height:1310px"></div>
                <button id="target" style="display:block;box-sizing:border-box;width:100px;height:40px">Target</button>
                <button id="fixed" style="position:fixed;left:20px;top:30px;width:100px;height:40px">Fixed</button>
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nscroll down\nget box @e1\nget box #fixed\nscroll right 1000\nscrollintoview @e1\nget box @e1\nscrollinto #fixed\nfind role button scroll --name Target --exact\nexit\n"
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
    assert!(stdout.contains("scrolled x=0 y=300 moved=true"));
    assert!(stdout.contains("x:      8\ny:      1018\nwidth:  100\nheight: 40"));
    assert!(stdout.contains("x:      20\ny:      30\nwidth:  100\nheight: 40"));
    assert!(stdout.contains("scrolled x=528 y=300 moved=true"));
    assert!(stdout.contains("scrolled into view ref=@e1 x=8 y=638 moved=true"));
    assert!(stdout.contains("x:      0\ny:      680\nwidth:  100\nheight: 40"));
    assert!(stdout.contains(r#"scrolled into view element="fixed" x=8 y=638 moved=false"#));
    assert!(stdout.contains(
        r#"scrolled into view role="button" name="Target" element="target" x=8 y=638 moved=false"#
    ));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_auto_scrolls_supported_pointer_action_targets() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body>
                <div style="height:800px"></div>
                <button id="click" style="display:block;box-sizing:border-box;width:100px;height:40px">Click</button>
                <input id="check" type="checkbox" style="display:block;box-sizing:border-box;width:20px;height:20px">
                <div id="hover" style="display:block;box-sizing:border-box;width:100px;height:40px">Hover</div>
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "set viewport 640 300\nopen {url}\nsnapshot -i\nget box @e1\nclick @e1\nget box @e1\nscroll up 9999\ncheck #check\nget box #check\nscroll up 9999\ncheck #check\nget box @e1\nhover #hover\nget box #hover\nexit\n"
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
    assert!(stdout.contains("x:      8\ny:      808\nwidth:  100\nheight: 40"));
    assert!(stdout.contains("clicked ref=@e1 focused=true"));
    assert!(stdout.contains("x:      8\ny:      260\nwidth:  100\nheight: 40"));
    assert!(stdout.contains(r#"checked role="checkbox" name="" element="check" checked=true"#));
    assert!(stdout.contains("x:      8\ny:      280\nwidth:  20\nheight: 20"));
    assert!(stdout.contains(r#"hovered role="" name="" element="hover""#));
    assert!(output.stderr.is_empty());
}

#[test]
fn session_mode_sets_viewport_before_open_and_reflows_current_state() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <body style="height:900px">
                <input id="email" aria-label="Email" value="before">
            </body>
        "#,
    );
    let output = run_session_script(&format!(
        "get viewport\nset viewport 640 480\nopen {url}\nsnapshot -i\nfill @e1 changed\nscroll down 1000\nset viewport 800 600\nget viewport\nget box body\nget value @e1\nis focused @e1\nset viewport 800 600\nexit\n"
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
    assert!(stdout.contains("viewport width=1280 height=720"));
    assert!(
        stdout.contains(
            "viewport width=640 height=480 resized=true scroll-x=0 scroll-y=0 moved=false"
        )
    );
    assert!(stdout.contains("scrolled x=0 y=428 moved=true"));
    assert!(
        stdout.contains(
            "viewport width=800 height=600 resized=true scroll-x=0 scroll-y=308 moved=true"
        )
    );
    assert!(stdout.contains("viewport width=800 height=600\n"));
    assert!(stdout.contains("x:      8\ny:      -300\nwidth:  784\nheight: 900"));
    assert!(stdout.contains(r#"value ref=@e1 "changed""#));
    assert!(stdout.contains("focused ref=@e1 value=true"));
    assert!(stdout.contains(
        "viewport width=800 height=600 resized=false scroll-x=0 scroll-y=308 moved=false"
    ));
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
fn session_mode_reads_editable_state_through_references_and_selectors() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"
            <input id="text">
            <input id="readonly" readonly>
            <select id="select"><option>One</option></select>
            <input id="checkbox" type="checkbox">
            <div contenteditable><span id="editable-child">Inherited</span></div>
        "#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot -i\nis editable @e1\nis editable @e2\nis editable #select\nis editable #checkbox\nis editable #editable-child\nget text @e1\nexit\n"
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
    assert!(stdout.contains("editable ref=@e1 value=true"));
    assert!(stdout.contains("editable ref=@e2 value=false"));
    assert_eq!(stdout.lines().filter(|line| *line == "true").count(), 3);
    assert!(stdout.contains(r#"text ref=@e1 """#));
    assert!(stdout.ends_with("session closed\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn snapshot_defaults_to_the_full_accessibility_tree() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(r#"<main><h1>Hello</h1><button>Save</button></main>"#);
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["snapshot", &url])
        .output()
        .unwrap();
    server.join().unwrap();
    drop(network_guard);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mode=full"));
    assert!(stdout.contains(r#"- main"#));
    assert!(stdout.contains(r#"  - heading "Hello" [ref=e1]"#));
    assert!(stdout.contains(r#"  - button "Save" [ref=e2]"#));
    assert!(output.stderr.is_empty());
}

#[test]
fn full_snapshot_matches_document_list_marker_projection() {
    let network_guard = network_test_guard();
    let (url, server) = serve_page(
        r#"<main id="main"><ul><li>Alpha</li><li hidden>Hidden</li><li>Beta<ul><li>Nested</li></ul></li></ul><ol start="3"><li>Third</li><li value="7">Seventh</li><li>Eighth</li></ol></main>"#,
    );
    let output = run_session_script(&format!(
        "open {url}\nsnapshot\nsnapshot --compact\nsnapshot --depth 0\nsnapshot --selector #main\nexit\n"
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
    let captures = stdout.split("snapshot=").skip(1).collect::<Vec<_>>();
    assert_eq!(captures.len(), 4);
    assert!(captures[0].starts_with('1'));
    assert_eq!(captures[0].matches("- ListMarker \"• \"").count(), 3);
    assert!(
        captures[0].contains("- ListMarker \"1. \"\n- ListMarker \"2. \"\n- ListMarker \"3. \"")
    );
    assert!(!captures[1].contains("ListMarker"));
    assert!(!captures[2].contains("  - list"));
    assert_eq!(captures[2].matches("- ListMarker").count(), 6);
    assert!(!captures[3].contains("ListMarker"));
    assert!(output.stderr.is_empty());
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
    assert!(stdout.contains("events=2"));
    assert!(stdout.contains(
        r#"event type=beforeinput document=1 target="direct" ordinal=2 bubbles=true composed=true path="direct > app""#
    ));
    assert!(stdout.contains(
        r#"event type=input document=1 target="direct" ordinal=2 bubbles=true composed=true path="direct > app""#
    ));
}
