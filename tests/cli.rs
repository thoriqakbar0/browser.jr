use std::process::Command;

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
fn unavailable_lint_is_not_a_successful_process() {
    let output = Command::new(env!("CARGO_BIN_EXE_browser-jr"))
        .args(["lint", "http://localhost:3000"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "browser.jr: lint is unavailable because page loading is not implemented\n"
    );
}
