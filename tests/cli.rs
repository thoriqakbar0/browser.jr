use std::io::Write;
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
