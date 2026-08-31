use std::ffi::OsString;
use std::io::{BufRead, Write};

use crate::cli_session::{run_session, write_interactive_snapshot};
use crate::loading::{LoadError, load_local_html};
use crate::page::layout_input_from_html;
use crate::{
    CaptureInteractiveSnapshot, CheckElementWidth, Comparison, LintLayout, OpenPage,
    RuleConstraint, RuleResult, Session, SessionError, WidthFinding,
};

const DEFAULT_VIEWPORT_WIDTH: u64 = 1280;

const HELP: &str = "browser.jr
A browser engine package for programmable interface verification.

Usage:
  browser.jr lint <url> [--viewport <css-px>] [--max-width <element> <css-px>]
  browser.jr snapshot <url> --interactive
  browser.jr session
  browser.jr help

Options:
  -h, --help     Show this help
  -V, --version  Show the version
  --viewport     Set the viewport width; default: 1280 CSS px
  --max-width    Require one semantic element to stay within a project limit
  -i, --interactive  Include interactive semantic elements in the snapshot

Current implementation:
  Static HTML design lint is available for loopback HTTP pages.
  Interactive snapshots include a stated native HTML and ARIA role subset.
  Session mode supports semantic, attribute, and positioned CSS locators through stdin.
  Parent-aware block flow and fixed pixel geometry form the layout subset.
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitStatus {
    Success,
    Findings,
    InvalidInput,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ElementWidthLimit {
    element: String,
    maximum_width: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LintOptions {
    url: OsString,
    viewport_width: u64,
    width_limit: Option<ElementWidthLimit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotOptions {
    url: OsString,
}

impl ExitStatus {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Findings => 1,
            Self::InvalidInput => 2,
            Self::Unavailable => 3,
        }
    }
}

pub fn run_cli<I, O, E>(args: I, output: &mut O, errors: &mut E) -> ExitStatus
where
    I: IntoIterator<Item = OsString>,
    O: Write,
    E: Write,
{
    run_cli_with_input(args, &mut std::io::empty(), output, errors)
}

pub fn run_cli_with_input<I, R, O, E>(
    args: I,
    input: &mut R,
    output: &mut O,
    errors: &mut E,
) -> ExitStatus
where
    I: IntoIterator<Item = OsString>,
    R: BufRead,
    O: Write,
    E: Write,
{
    let args: Vec<OsString> = args.into_iter().collect();
    match args.as_slice() {
        [] => write_help(output),
        [arg] if arg == "help" || arg == "-h" || arg == "--help" => write_help(output),
        [arg] if arg == "-V" || arg == "--version" => write_line(
            output,
            concat!("browser.jr ", env!("CARGO_PKG_VERSION")),
            ExitStatus::Success,
        ),
        [command, rest @ ..] if command == "lint" => match parse_lint_options(rest) {
            Ok(options) => run_lint(options, output, errors),
            Err(message) => write_line(errors, &message, ExitStatus::InvalidInput),
        },
        [command, rest @ ..] if command == "snapshot" => match parse_snapshot_options(rest) {
            Ok(options) => run_snapshot(options, output, errors),
            Err(message) => write_line(errors, &message, ExitStatus::InvalidInput),
        },
        [command] if command == "session" => run_session(input, output, errors),
        _ => write_line(
            errors,
            "browser.jr: invalid arguments; run browser.jr help",
            ExitStatus::InvalidInput,
        ),
    }
}

fn parse_snapshot_options(args: &[OsString]) -> Result<SnapshotOptions, String> {
    match args {
        [] => Err("browser.jr: snapshot requires a URL and --interactive".into()),
        [_] => Err("browser.jr: snapshot requires --interactive".into()),
        [url, flag] if flag == "-i" || flag == "--interactive" => {
            Ok(SnapshotOptions { url: url.clone() })
        }
        _ => Err("browser.jr: invalid snapshot arguments; run browser.jr help".into()),
    }
}

fn parse_lint_options(args: &[OsString]) -> Result<LintOptions, String> {
    let Some(url) = args.first() else {
        return Err("browser.jr: lint requires a URL".into());
    };
    let mut options = LintOptions {
        url: url.clone(),
        viewport_width: DEFAULT_VIEWPORT_WIDTH,
        width_limit: None,
    };
    let mut viewport_was_set = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].to_str() {
            Some("--viewport") if index + 1 < args.len() => {
                if viewport_was_set {
                    return Err("browser.jr: --viewport cannot be repeated".into());
                }
                options.viewport_width = parse_positive_width(&args[index + 1], "--viewport")?;
                viewport_was_set = true;
                index += 2;
            }
            Some("--max-width") if index + 2 < args.len() => {
                if options.width_limit.is_some() {
                    return Err("browser.jr: --max-width cannot be repeated".into());
                }
                let Some(element) = args[index + 1].to_str() else {
                    return Err("browser.jr: --max-width element must be valid UTF-8".into());
                };
                if element.trim().is_empty() {
                    return Err("browser.jr: --max-width requires a non-empty element".into());
                }
                let maximum_width = parse_width(&args[index + 2], "--max-width")?;
                options.width_limit = Some(ElementWidthLimit {
                    element: element.into(),
                    maximum_width,
                });
                index += 3;
            }
            Some("--viewport") => {
                return Err(
                    "browser.jr: --viewport requires a positive integer CSS pixel width".into(),
                );
            }
            Some("--max-width") => {
                return Err(
                    "browser.jr: --max-width requires an element and integer CSS pixel width"
                        .into(),
                );
            }
            _ => return Err("browser.jr: invalid arguments; run browser.jr help".into()),
        }
    }
    Ok(options)
}

fn parse_positive_width(value: &OsString, flag: &str) -> Result<u64, String> {
    value
        .to_str()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|width| *width > 0)
        .ok_or_else(|| format!("browser.jr: {flag} requires a positive integer CSS pixel width"))
}

fn parse_width(value: &OsString, flag: &str) -> Result<u64, String> {
    value
        .to_str()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            format!("browser.jr: {flag} requires a non-negative integer CSS pixel width")
        })
}

fn run_lint(options: LintOptions, output: &mut impl Write, errors: &mut impl Write) -> ExitStatus {
    let Some(url) = options.url.to_str() else {
        return write_line(
            errors,
            "browser.jr: URL must be valid UTF-8",
            ExitStatus::InvalidInput,
        );
    };
    let html = match load_local_html(url) {
        Ok(html) => html,
        Err(error) => return write_load_error(errors, error),
    };
    let mut session = Session::new();
    let result = match session.execute(LintLayout {
        input: layout_input_from_html(&html, options.viewport_width),
    }) {
        Ok(result) => result,
        Err(error) => {
            return write_line(
                errors,
                &format!("browser.jr: layout failed: {error:?}"),
                ExitStatus::Unavailable,
            );
        }
    };
    let overflow_status = write_rule_result(output, errors, url, options.viewport_width, result);
    let Some(limit) = options.width_limit else {
        return overflow_status;
    };
    let width_result = session
        .execute(CheckElementWidth {
            element: limit.element,
            maximum_width: limit.maximum_width,
        })
        .expect("lint layout always creates a snapshot");
    let width_status =
        write_width_rule_result(output, errors, url, options.viewport_width, width_result);
    combine_status(overflow_status, width_status)
}

fn run_snapshot(
    options: SnapshotOptions,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let Some(url) = options.url.to_str() else {
        return write_line(
            errors,
            "browser.jr: URL must be valid UTF-8",
            ExitStatus::InvalidInput,
        );
    };
    let mut session = Session::new();
    if let Err(error) = session.execute(OpenPage { url: url.into() }) {
        return write_session_error(errors, error);
    }
    let snapshot = match session.execute(CaptureInteractiveSnapshot) {
        Ok(snapshot) => snapshot,
        Err(error) => return write_session_error(errors, error),
    };
    write_interactive_snapshot(output, &snapshot)
}

pub(crate) fn write_session_error(errors: &mut impl Write, error: SessionError) -> ExitStatus {
    match error {
        SessionError::Load(error) => write_load_error(errors, error),
        SessionError::Navigation { reference, error } => write_line(
            errors,
            &format!("browser.jr: navigation from {reference} failed: {error}"),
            ExitStatus::Unavailable,
        ),
        SessionError::NoPage => write_line(
            errors,
            "browser.jr: no page is open",
            ExitStatus::Unavailable,
        ),
        SessionError::Layout(error) => write_line(
            errors,
            &format!("browser.jr: layout failed: {error:?}"),
            ExitStatus::Unavailable,
        ),
        SessionError::NoSnapshot => write_line(
            errors,
            "browser.jr: no snapshot is available",
            ExitStatus::Unavailable,
        ),
        SessionError::StaleElementReference { reference } => write_line(
            errors,
            &format!("browser.jr: stale element reference {reference}"),
            ExitStatus::InvalidInput,
        ),
        error @ (SessionError::RoleLocatorNotFound { .. }
        | SessionError::RoleLocatorAmbiguous { .. }
        | SessionError::LocatorNotFound { .. }
        | SessionError::LocatorAmbiguous { .. }
        | SessionError::RoleNavigation { .. }
        | SessionError::LocatorNavigation { .. }
        | SessionError::RoleActionBlocked { .. }
        | SessionError::LocatorActionBlocked { .. }
        | SessionError::UnsupportedRoleAction { .. }
        | SessionError::UnsupportedLocatorAction { .. }) => write_locator_error(errors, error),
        SessionError::UnsupportedClick { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot click {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedFill { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot fill {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedSelect { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot select on {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::SelectOptionNotFound { reference, value } => write_line(
            errors,
            &format!("browser.jr: option value {value:?} was not found on {reference}"),
            ExitStatus::Unavailable,
        ),
        SessionError::SelectOptionDisabled { reference, value } => write_line(
            errors,
            &format!("browser.jr: option value {value:?} is disabled on {reference}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedValue { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read value from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedCheck { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot change checked state on {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedCheckedState { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read checked state from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::InvalidAttributeName { name } => write_line(
            errors,
            &format!("browser.jr: invalid attribute name {name:?}"),
            ExitStatus::InvalidInput,
        ),
        SessionError::SensitiveAttribute { reference, name } => write_line(
            errors,
            &format!("browser.jr: cannot read sensitive attribute {name:?} from {reference}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedEnabledState { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read enabled state from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedVisibility { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read visibility from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
    }
}

fn write_locator_error(errors: &mut impl Write, error: SessionError) -> ExitStatus {
    match error {
        SessionError::RoleLocatorNotFound { locator } => write_line(
            errors,
            &format!("browser.jr: no element matches {locator}"),
            ExitStatus::InvalidInput,
        ),
        SessionError::RoleLocatorAmbiguous {
            locator,
            match_count,
        } => write_line(
            errors,
            &format!("browser.jr: {match_count} elements match {locator}; locator must be unique"),
            ExitStatus::InvalidInput,
        ),
        SessionError::LocatorNotFound { locator } => write_line(
            errors,
            &format!("browser.jr: no element matches {locator}"),
            ExitStatus::InvalidInput,
        ),
        SessionError::LocatorAmbiguous {
            locator,
            match_count,
        } => write_line(
            errors,
            &format!("browser.jr: {match_count} elements match {locator}; locator must be unique"),
            ExitStatus::InvalidInput,
        ),
        SessionError::RoleNavigation { locator, error } => write_line(
            errors,
            &format!("browser.jr: cannot click {locator}: navigation failed: {error:?}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorNavigation { locator, error } => write_line(
            errors,
            &format!("browser.jr: cannot click {locator}: navigation failed: {error:?}"),
            ExitStatus::Unavailable,
        ),
        SessionError::RoleActionBlocked {
            locator,
            action,
            check,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot {action} {locator}: {check} check blocked: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorActionBlocked {
            locator,
            action,
            check,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot {action} {locator}: {check} check blocked: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedRoleAction {
            locator,
            action,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot {action} {locator}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedLocatorAction {
            locator,
            action,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot {action} {locator}: {reason}"),
            ExitStatus::Unavailable,
        ),
        _ => unreachable!("write_locator_error accepts only locator errors"),
    }
}

fn write_load_error(errors: &mut impl Write, error: LoadError) -> ExitStatus {
    let status = if error.is_invalid_input() {
        ExitStatus::InvalidInput
    } else {
        ExitStatus::Unavailable
    };
    write_line(errors, &format!("browser.jr: {error}"), status)
}

fn write_rule_result(
    output: &mut impl Write,
    errors: &mut impl Write,
    url: &str,
    viewport_width: u64,
    result: RuleResult,
) -> ExitStatus {
    match result {
        RuleResult::Compared {
            rule,
            comparison: Comparison::Pass,
        } => write_line(
            output,
            &format!("pass rule={rule} url={url} viewport={viewport_width} mode=static-html"),
            ExitStatus::Success,
        ),
        RuleResult::Compared {
            rule,
            comparison: Comparison::Fail(findings),
        } => {
            for finding in findings.iter() {
                let evidence = finding
                    .evidence
                    .iter()
                    .map(|item| {
                        format!("snapshot:{}#{}", item.snapshot.get(), item.element.as_str())
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                if writeln!(
                    output,
                    "finding rule={rule} severity=error element={} viewport={} expectation=inside-viewport observed=left:{},width:{},right:{} evidence={evidence}",
                    finding.affected_element.as_str(),
                    finding.viewport_width,
                    finding.observed_left,
                    finding.observed_width,
                    finding.observed_right,
                )
                .is_err()
                {
                    return ExitStatus::Unavailable;
                }
            }
            write_line(
                output,
                &format!(
                    "fail rule={rule} url={url} viewport={viewport_width} findings={} mode=static-html",
                    findings.len()
                ),
                ExitStatus::Findings,
            )
        }
        RuleResult::Blocked { rule, causes } => {
            for cause in causes.iter() {
                if write_constraint(errors, rule, cause).is_err() {
                    return ExitStatus::Unavailable;
                }
            }
            write_line(
                errors,
                &format!(
                    "blocked rule={rule} url={url} viewport={viewport_width} causes={} mode=static-html",
                    causes.len()
                ),
                ExitStatus::Unavailable,
            )
        }
    }
}

fn write_width_rule_result(
    output: &mut impl Write,
    errors: &mut impl Write,
    url: &str,
    viewport_width: u64,
    result: RuleResult<WidthFinding>,
) -> ExitStatus {
    match result {
        RuleResult::Compared {
            rule,
            comparison: Comparison::Pass,
        } => write_line(
            output,
            &format!("pass rule={rule} url={url} viewport={viewport_width} mode=static-html"),
            ExitStatus::Success,
        ),
        RuleResult::Compared {
            rule,
            comparison: Comparison::Fail(findings),
        } => {
            let finding = &findings[0];
            let evidence = finding
                .evidence
                .iter()
                .map(|item| format!("snapshot:{}#{}", item.snapshot.get(), item.element.as_str()))
                .collect::<Vec<_>>()
                .join(",");
            if writeln!(
                output,
                "finding rule={rule} severity=error element={} viewport={} expectation=width<={} observed=width:{} evidence={evidence}",
                finding.affected_element.as_str(),
                finding.viewport_width,
                finding.maximum_width,
                finding.observed_width,
            )
            .is_err()
            {
                return ExitStatus::Unavailable;
            }
            write_line(
                output,
                &format!(
                    "fail rule={rule} url={url} viewport={viewport_width} findings=1 mode=static-html"
                ),
                ExitStatus::Findings,
            )
        }
        RuleResult::Blocked { rule, causes } => {
            for cause in causes.iter() {
                if write_constraint(errors, rule, cause).is_err() {
                    return ExitStatus::Unavailable;
                }
            }
            write_line(
                errors,
                &format!(
                    "blocked rule={rule} url={url} viewport={viewport_width} causes={} mode=static-html",
                    causes.len()
                ),
                ExitStatus::Unavailable,
            )
        }
    }
}

fn write_constraint(
    errors: &mut impl Write,
    rule: &str,
    constraint: &RuleConstraint,
) -> std::io::Result<()> {
    match constraint {
        RuleConstraint::Unsupported { element, reason } => {
            writeln!(
                errors,
                "blocked rule={rule} element={element} reason={reason}"
            )
        }
        RuleConstraint::MissingElement { element } => writeln!(
            errors,
            "blocked rule={rule} element={element} reason=element-was-not-observed"
        ),
    }
}

pub(crate) fn combine_status(left: ExitStatus, right: ExitStatus) -> ExitStatus {
    match (left, right) {
        (ExitStatus::Unavailable, _) | (_, ExitStatus::Unavailable) => ExitStatus::Unavailable,
        (ExitStatus::InvalidInput, _) | (_, ExitStatus::InvalidInput) => ExitStatus::InvalidInput,
        (ExitStatus::Findings, _) | (_, ExitStatus::Findings) => ExitStatus::Findings,
        (ExitStatus::Success, ExitStatus::Success) => ExitStatus::Success,
    }
}

fn write_help(output: &mut impl Write) -> ExitStatus {
    if output.write_all(HELP.as_bytes()).is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::Unavailable
    }
}

pub(crate) fn write_line(output: &mut impl Write, value: &str, success: ExitStatus) -> ExitStatus {
    if writeln!(output, "{value}").is_ok() {
        success
    } else {
        ExitStatus::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::{ExitStatus, run_cli};
    use std::ffi::OsString;

    fn run(args: &[&str]) -> (ExitStatus, String, String) {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let status = run_cli(args.iter().map(OsString::from), &mut output, &mut errors);
        (
            status,
            String::from_utf8(output).unwrap(),
            String::from_utf8(errors).unwrap(),
        )
    }

    #[test]
    fn help_exits_without_starting_lint() {
        let (status, output, errors) = run(&["--help"]);

        assert_eq!(status, ExitStatus::Success);
        assert!(output.contains("browser.jr lint <url>"));
        assert!(output.contains("browser.jr session"));
        assert!(output.contains("Static HTML design lint is available"));
        assert!(errors.is_empty());
    }

    #[test]
    fn lint_without_url_is_invalid() {
        let (status, output, errors) = run(&["lint"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert_eq!(errors, "browser.jr: lint requires a URL\n");
    }

    #[test]
    fn non_loopback_lint_is_rejected_before_loading() {
        let (status, output, errors) = run(&["lint", "http://example.com"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert!(errors.contains("loopback"));
    }

    #[test]
    fn zero_viewport_is_invalid() {
        let (status, output, errors) = run(&["lint", "http://localhost:3000", "--viewport", "0"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert!(errors.contains("positive integer"));
    }

    #[test]
    fn incomplete_project_width_limit_is_invalid() {
        let (status, output, errors) =
            run(&["lint", "http://localhost:3000", "--max-width", "content"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert!(errors.contains("requires an element and integer"));
    }

    #[test]
    fn repeated_viewport_is_invalid() {
        let (status, output, errors) = run(&[
            "lint",
            "http://localhost:3000",
            "--viewport",
            "1280",
            "--viewport",
            "320",
        ]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert!(errors.contains("cannot be repeated"));
    }

    #[test]
    fn version_is_machine_stable() {
        let (status, output, errors) = run(&["--version"]);

        assert_eq!(status, ExitStatus::Success);
        assert_eq!(
            output,
            concat!("browser.jr ", env!("CARGO_PKG_VERSION"), "\n")
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_arguments_are_invalid() {
        let (status, output, errors) = run(&["wat"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert_eq!(
            errors,
            "browser.jr: invalid arguments; run browser.jr help\n"
        );
    }
}
