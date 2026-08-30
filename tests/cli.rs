use std::io::Write;
use std::net::TcpListener;
use std::process::Command;
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
fn interactive_snapshot_reports_stable_agent_refs() {
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
fn page_load_failure_is_not_a_pass() {
    let network_guard = network_test_guard();
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", "http://localhost:3000"])
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

    assert!(output.status.success());
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

    assert!(output.status.success());
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
