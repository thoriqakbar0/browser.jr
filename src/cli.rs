use std::ffi::OsString;
use std::io::{BufRead, Write};

use crate::cli_output::{
    SnapshotOutputOptions, write_accessibility_snapshot_json, write_error_json, write_snapshot_json,
};
use crate::cli_session::{run_session, write_accessibility_snapshot, write_interactive_snapshot};
use crate::cli_session_json::run_json_session;
use crate::loading::{LoadError, load_html};
use crate::page::layout_input_from_html;
use crate::{
    AccessibilitySnapshotOptions, CaptureAccessibilitySnapshot, CaptureAccessibilitySnapshotWithin,
    CaptureInteractiveSnapshot, CaptureInteractiveSnapshotWithin, CheckElementWidth, Comparison,
    CssLocator, GetPageText, LintLayout, Locator, NetworkAccess, OpenPage, RuleConstraint,
    RuleResult, Session, SessionError, WidthFinding,
};

use crate::DEFAULT_VIEWPORT_WIDTH;

const HELP: &str = "browser.jr
A browser engine package for programmable interface verification.

Usage:
  browser.jr [--allow-loopback] lint <url> [--viewport <css-px>] [--max-width <element> <css-px>]
  browser.jr [--allow-loopback] read <url>
  browser.jr [--allow-loopback] [--json] snapshot <url> [--interactive] [snapshot-options] [--json]
  browser.jr [--allow-loopback] session
  browser.jr [--allow-loopback] --json session
  browser.jr help

Options:
  -h, --help     Show this help
  -V, --version  Show the version
  --viewport     Set the viewport width; default: 1280 CSS px
  --max-width    Require one semantic element to stay within a project limit
  -i, --interactive  Project the tree to agent-oriented reference elements
  -u, --urls          Include resolved URLs for links
  -c, --compact       Remove empty structural leaves from the full tree
  -d, --depth         Limit the full tree to a zero-based depth
  -s, --selector      Limit the snapshot to one strict CSS target
  --json             Emit machine-readable snapshot or session results on stdout
  --allow-loopback   Allow explicit localhost and loopback-IP URLs

Current implementation:
  Static HTML design lint is available for public HTTP and HTTPS pages.
  Loopback pages require --allow-loopback.
  Full and interactive snapshots expose a stated static accessibility subset.
  Session mode supports semantic, attribute, CSS, and XPath locators through stdin.
  Session mode drains data-minimized native action events through the events command.
  JSON session mode writes one envelope for each lifecycle event and input command.
  Parent-aware block flow and fixed pixel geometry form the layout subset.
  Session mode writes bounded solid-box PNG screenshots.
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
    selector: Option<Locator>,
    projection: SnapshotProjection,
    output: SnapshotOutputOptions,
    output_format: SnapshotOutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotProjection {
    Full(AccessibilitySnapshotOptions),
    Interactive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotOutputFormat {
    Human,
    Json,
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

fn extract_network_access(args: Vec<OsString>) -> Result<(Vec<OsString>, NetworkAccess), String> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut allow_loopback = false;
    for argument in args {
        if argument == "--allow-loopback" {
            if allow_loopback {
                return Err("browser.jr: --allow-loopback cannot be repeated".into());
            }
            allow_loopback = true;
        } else {
            filtered.push(argument);
        }
    }
    let access = if allow_loopback {
        NetworkAccess::PublicAndLoopback
    } else {
        NetworkAccess::PublicOnly
    };
    Ok((filtered, access))
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
    let raw_args: Vec<OsString> = args.into_iter().collect();
    let (args, network_access) = match extract_network_access(raw_args) {
        Ok(parsed) => parsed,
        Err(message) => return write_line(errors, &message, ExitStatus::InvalidInput),
    };
    match args.as_slice() {
        [] => write_help(output),
        [arg] if arg == "help" || arg == "-h" || arg == "--help" => write_help(output),
        [arg] if arg == "-V" || arg == "--version" => write_line(
            output,
            concat!("browser.jr ", env!("CARGO_PKG_VERSION")),
            ExitStatus::Success,
        ),
        [command, rest @ ..] if command == "lint" => match parse_lint_options(rest) {
            Ok(options) => run_lint(options, network_access, output, errors),
            Err(message) => write_line(errors, &message, ExitStatus::InvalidInput),
        },
        [command, url] if command == "read" => run_read(url, network_access, output, errors),
        [flag, command, rest @ ..] if flag == "--json" && command == "snapshot" => {
            run_snapshot_invocation(
                rest,
                SnapshotOutputFormat::Json,
                network_access,
                output,
                errors,
            )
        }
        [flag, command] if flag == "--json" && command == "session" => {
            run_json_session(input, output, errors, network_access)
        }
        [command, flag] if command == "session" && flag == "--json" => {
            run_json_session(input, output, errors, network_access)
        }
        [flag, ..] if flag == "--json" => write_snapshot_error(
            output,
            errors,
            SnapshotOutputFormat::Json,
            "browser.jr: --json supports snapshot and session only",
            ExitStatus::InvalidInput,
        ),
        [command, rest @ ..] if command == "snapshot" => run_snapshot_invocation(
            rest,
            SnapshotOutputFormat::Human,
            network_access,
            output,
            errors,
        ),
        [command] if command == "session" => run_session(input, output, errors, network_access),
        _ => write_line(
            errors,
            "browser.jr: invalid arguments; run browser.jr help",
            ExitStatus::InvalidInput,
        ),
    }
}

fn run_read(
    url: &OsString,
    network_access: NetworkAccess,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let Some(url) = url.to_str() else {
        return write_line(
            errors,
            "browser.jr: URL must be valid UTF-8",
            ExitStatus::InvalidInput,
        );
    };
    let mut session = Session::with_network_access(network_access);
    if let Err(error) = session.execute(OpenPage { url: url.into() }) {
        return write_session_error(errors, error);
    }
    match session.execute(GetPageText) {
        Ok(page) => write_line(output, &page.text, ExitStatus::Success),
        Err(error) => write_session_error(errors, error),
    }
}

fn run_snapshot_invocation(
    args: &[OsString],
    initial_output_format: SnapshotOutputFormat,
    network_access: NetworkAccess,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let output_format = if initial_output_format == SnapshotOutputFormat::Json
        || args.iter().any(|argument| argument == "--json")
    {
        SnapshotOutputFormat::Json
    } else {
        SnapshotOutputFormat::Human
    };
    match parse_snapshot_options(args, initial_output_format) {
        Ok(options) => run_snapshot(options, network_access, output, errors),
        Err(message) => write_snapshot_error(
            output,
            errors,
            output_format,
            &message,
            ExitStatus::InvalidInput,
        ),
    }
}

fn parse_snapshot_options(
    args: &[OsString],
    initial_output_format: SnapshotOutputFormat,
) -> Result<SnapshotOptions, String> {
    let mut url = None;
    let mut interactive = false;
    let mut selector = None;
    let mut output = SnapshotOutputOptions::default();
    let mut compact_was_set = false;
    let mut depth_was_set = false;
    let mut compact = false;
    let mut max_depth = None;
    let mut output_format = initial_output_format;
    let mut json_was_set = initial_output_format == SnapshotOutputFormat::Json;
    let mut index = 0;
    while index < args.len() {
        match args[index].to_str() {
            Some("-i" | "--interactive") if !interactive => {
                interactive = true;
                index += 1;
            }
            Some("-u" | "--urls") if !output.include_urls => {
                output.include_urls = true;
                index += 1;
            }
            Some("-c" | "--compact") if !compact_was_set => {
                compact_was_set = true;
                compact = true;
                index += 1;
            }
            Some("-d" | "--depth") if !depth_was_set && index + 1 < args.len() => {
                let Some(value) = args[index + 1].to_str() else {
                    return Err("browser.jr: snapshot depth must be valid UTF-8".into());
                };
                max_depth = Some(value.parse::<u64>().map_err(|_| {
                    "browser.jr: snapshot depth must be a non-negative integer".to_string()
                })?);
                depth_was_set = true;
                index += 2;
            }
            Some("-s" | "--selector") if selector.is_none() && index + 1 < args.len() => {
                let Some(value) = args[index + 1].to_str() else {
                    return Err("browser.jr: snapshot selector must be valid UTF-8".into());
                };
                let locator = CssLocator::new(value)
                    .map(Locator::from)
                    .map_err(|error| format!("browser.jr: invalid snapshot selector: {error}"))?;
                selector = Some(locator);
                index += 2;
            }
            Some("--json") if !json_was_set => {
                output_format = SnapshotOutputFormat::Json;
                json_was_set = true;
                index += 1;
            }
            Some("--json") => {
                return Err("browser.jr: --json cannot be repeated".into());
            }
            Some(value) if !value.starts_with('-') && url.is_none() => {
                url = Some(args[index].clone());
                index += 1;
            }
            _ => {
                return Err("browser.jr: invalid snapshot arguments; run browser.jr help".into());
            }
        }
    }
    let Some(url) = url else {
        return Err("browser.jr: snapshot requires a URL".into());
    };
    let projection = if interactive {
        SnapshotProjection::Interactive
    } else {
        SnapshotProjection::Full(AccessibilitySnapshotOptions { compact, max_depth })
    };
    Ok(SnapshotOptions {
        url,
        selector,
        projection,
        output,
        output_format,
    })
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

fn run_lint(
    options: LintOptions,
    network_access: NetworkAccess,
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
    let html = match load_html(url, network_access) {
        Ok(loaded) => loaded.html,
        Err(error) => return write_load_error(errors, error),
    };
    let mut session = Session::with_network_access(network_access);
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
    network_access: NetworkAccess,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let Some(url) = options.url.to_str() else {
        return write_snapshot_error(
            output,
            errors,
            options.output_format,
            "browser.jr: URL must be valid UTF-8",
            ExitStatus::InvalidInput,
        );
    };
    let mut session = Session::with_network_access(network_access);
    if let Err(error) = session.execute(OpenPage { url: url.into() }) {
        return write_snapshot_session_error(output, errors, options.output_format, error);
    }
    match options.projection {
        SnapshotProjection::Interactive => {
            let snapshot = match options.selector.clone() {
                Some(locator) => session.execute(CaptureInteractiveSnapshotWithin { locator }),
                None => session.execute(CaptureInteractiveSnapshot),
            };
            match snapshot {
                Ok(snapshot) => write_interactive_snapshot_output(output, &snapshot, options),
                Err(error) => {
                    write_snapshot_session_error(output, errors, options.output_format, error)
                }
            }
        }
        SnapshotProjection::Full(snapshot_options) => {
            let snapshot = match options.selector.clone() {
                Some(locator) => session.execute(CaptureAccessibilitySnapshotWithin {
                    locator,
                    options: snapshot_options,
                }),
                None => session.execute(CaptureAccessibilitySnapshot {
                    options: snapshot_options,
                }),
            };
            match snapshot {
                Ok(snapshot) => write_accessibility_snapshot_output(output, &snapshot, options),
                Err(error) => {
                    write_snapshot_session_error(output, errors, options.output_format, error)
                }
            }
        }
    }
}

fn write_interactive_snapshot_output(
    output: &mut impl Write,
    snapshot: &crate::InteractiveSnapshot,
    options: SnapshotOptions,
) -> ExitStatus {
    match options.output_format {
        SnapshotOutputFormat::Human => write_interactive_snapshot(output, snapshot, options.output),
        SnapshotOutputFormat::Json => {
            write_json_snapshot_result(write_snapshot_json(output, snapshot, options.output))
        }
    }
}

fn write_accessibility_snapshot_output(
    output: &mut impl Write,
    snapshot: &crate::AccessibilitySnapshot,
    options: SnapshotOptions,
) -> ExitStatus {
    match options.output_format {
        SnapshotOutputFormat::Human => {
            write_accessibility_snapshot(output, snapshot, options.output)
        }
        SnapshotOutputFormat::Json => write_json_snapshot_result(
            write_accessibility_snapshot_json(output, snapshot, options.output),
        ),
    }
}

fn write_json_snapshot_result(result: std::io::Result<()>) -> ExitStatus {
    if result.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::Unavailable
    }
}

fn write_snapshot_session_error(
    output: &mut impl Write,
    errors: &mut impl Write,
    output_format: SnapshotOutputFormat,
    error: SessionError,
) -> ExitStatus {
    if output_format == SnapshotOutputFormat::Human {
        return write_session_error(errors, error);
    }

    let mut rendered = Vec::new();
    let status = write_session_error(&mut rendered, error);
    let message = String::from_utf8(rendered)
        .expect("session diagnostics are valid UTF-8")
        .trim_end()
        .to_owned();
    write_snapshot_error(output, errors, output_format, &message, status)
}

fn write_snapshot_error(
    output: &mut impl Write,
    errors: &mut impl Write,
    output_format: SnapshotOutputFormat,
    message: &str,
    status: ExitStatus,
) -> ExitStatus {
    match output_format {
        SnapshotOutputFormat::Human => write_line(errors, message, status),
        SnapshotOutputFormat::Json => {
            if write_error_json(output, message).is_ok() {
                status
            } else {
                ExitStatus::Unavailable
            }
        }
    }
}

pub(crate) fn write_session_error(errors: &mut impl Write, error: SessionError) -> ExitStatus {
    match error {
        SessionError::Load(error) => write_load_error(errors, error),
        SessionError::Navigation { reference, error } => write_line(
            errors,
            &format!("browser.jr: navigation from {reference} failed: {error}"),
            ExitStatus::Unavailable,
        ),
        SessionError::PressNavigation {
            key,
            element,
            error,
        } => write_line(
            errors,
            &format!("browser.jr: navigation from pressing {key} on {element:?} failed: {error}"),
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
        SessionError::InvalidViewportSize { width, height } => write_line(
            errors,
            &format!(
                "browser.jr: viewport width and height must be positive; got {width}x{height}"
            ),
            ExitStatus::InvalidInput,
        ),
        SessionError::UnsupportedScreenshot { target, reason } => write_line(
            errors,
            &format!("browser.jr: cannot capture {target:?}: {reason}"),
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
        | SessionError::LocatorQuery { .. }
        | SessionError::UnsupportedLocatorInspection { .. }
        | SessionError::SensitiveLocatorAttribute { .. }
        | SessionError::LocatorSelectOptionNotFound { .. }
        | SessionError::LocatorSelectOptionDisabled { .. }
        | SessionError::LocatorSelectOptionTargetNotFound { .. }
        | SessionError::LocatorSelectOptionTargetDisabled { .. }
        | SessionError::RoleNavigation { .. }
        | SessionError::LocatorNavigation { .. }
        | SessionError::RoleActionBlocked { .. }
        | SessionError::LocatorActionBlocked { .. }
        | SessionError::UnsupportedRoleAction { .. }
        | SessionError::UnsupportedLocatorAction { .. }) => write_locator_error(errors, error),
        SessionError::ActionFailed(failure) => write_line(
            errors,
            &format!(
                "browser.jr: cannot {action} {:?}: checks={checks:?} blocked_by={blocked_by:?} reason={reason} document={}->{}",
                failure.target.matched.element,
                failure.document_generation.before,
                failure.document_generation.after,
                action = failure.action,
                checks = failure.checks,
                blocked_by = failure.blocked_by,
                reason = failure.reason,
            ),
            ExitStatus::Unavailable,
        ),
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
        SessionError::UnsupportedType { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot type into {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedFocus { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot focus {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedHover { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot hover {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedScrollIntoView { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot scroll {reference} into view: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::NoFocusedElement => write_line(
            errors,
            "browser.jr: no element is focused",
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedPress {
            key,
            element,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot press {key} on {element:?}: {reason}"),
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
        SessionError::SelectOptionTargetNotFound { reference, target } => write_line(
            errors,
            &format!("browser.jr: option {target} was not found on {reference}"),
            ExitStatus::Unavailable,
        ),
        SessionError::SelectOptionTargetDisabled { reference, target } => write_line(
            errors,
            &format!("browser.jr: option {target} is disabled on {reference}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedValue { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read value from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedBoundingBox { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read bounding box from {reference}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedHtml { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read HTML from {reference}: {reason}"),
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
        SessionError::UnsupportedEditableState { reference, reason } => write_line(
            errors,
            &format!("browser.jr: cannot read editable state from {reference}: {reason}"),
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
        SessionError::LocatorQuery { locator, reason } => write_line(
            errors,
            &format!("browser.jr: cannot resolve {locator}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::UnsupportedLocatorInspection {
            locator,
            inspection,
            reason,
        } => write_line(
            errors,
            &format!("browser.jr: cannot read {inspection} from {locator}: {reason}"),
            ExitStatus::Unavailable,
        ),
        SessionError::SensitiveLocatorAttribute { locator, name } => write_line(
            errors,
            &format!("browser.jr: cannot read sensitive attribute {name:?} from {locator}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorSelectOptionNotFound { locator, value } => write_line(
            errors,
            &format!("browser.jr: option value {value:?} was not found on {locator}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorSelectOptionDisabled { locator, value } => write_line(
            errors,
            &format!("browser.jr: option value {value:?} is disabled on {locator}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorSelectOptionTargetNotFound { locator, target } => write_line(
            errors,
            &format!("browser.jr: option {target} was not found on {locator}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorSelectOptionTargetDisabled { locator, target } => write_line(
            errors,
            &format!("browser.jr: option {target} is disabled on {locator}"),
            ExitStatus::Unavailable,
        ),
        SessionError::RoleNavigation { locator, error } => write_line(
            errors,
            &format!("browser.jr: navigation from {locator} failed: {error:?}"),
            ExitStatus::Unavailable,
        ),
        SessionError::LocatorNavigation { locator, error } => write_line(
            errors,
            &format!("browser.jr: navigation from {locator} failed: {error:?}"),
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
        assert!(output.contains("browser.jr [--allow-loopback] lint <url>"));
        assert!(output.contains("browser.jr [--allow-loopback] session"));
        assert!(output.contains("--json"));
        assert!(output.contains("--allow-loopback"));
        assert!(output.contains("Static HTML design lint is available"));
        assert!(errors.is_empty());
    }

    #[test]
    fn json_snapshot_parse_errors_use_the_json_envelope() {
        let (status, output, errors) = run(&["snapshot", "--json"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["success"], false);
        assert!(value["data"].is_null());
        assert_eq!(value["error"], "browser.jr: snapshot requires a URL");
        assert!(errors.is_empty());
    }

    #[test]
    fn snapshot_compatibility_options_reject_invalid_values() {
        for args in [
            ["snapshot", "http://localhost:3000", "-i", "-d", "-1"],
            ["snapshot", "http://localhost:3000", "-i", "-u", "-u"],
            ["snapshot", "http://localhost:3000", "-i", "-c", "-c"],
        ] {
            let (status, output, errors) = run(&args);

            assert_eq!(status, ExitStatus::InvalidInput);
            assert!(output.is_empty());
            assert!(errors.contains("snapshot"));
        }
    }

    #[test]
    fn lint_without_url_is_invalid() {
        let (status, output, errors) = run(&["lint"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert_eq!(errors, "browser.jr: lint requires a URL\n");
    }

    #[test]
    fn private_network_lint_is_rejected_before_loading() {
        let (status, output, errors) = run(&["lint", "http://192.168.1.1"]);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(output.is_empty());
        assert!(errors.contains("private and non-routable"));
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
