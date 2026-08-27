use std::ffi::OsString;
use std::io::Write;

const HELP: &str = "browser.jr
A small browser engine for programmable interface verification.

Usage:
  browser.jr lint <url>
  browser.jr help

Options:
  -h, --help     Show this help
  -V, --version  Show the version

Current implementation:
  CLI discovery is available.
  Page loading and design lint execution are not implemented yet.
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitStatus {
    Success,
    InvalidInput,
    Unavailable,
}

impl ExitStatus {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
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
    let args: Vec<OsString> = args.into_iter().collect();
    match args.as_slice() {
        [] => write_help(output),
        [arg] if arg == "help" || arg == "-h" || arg == "--help" => write_help(output),
        [arg] if arg == "-V" || arg == "--version" => write_line(
            output,
            concat!("browser.jr ", env!("CARGO_PKG_VERSION")),
            ExitStatus::Success,
        ),
        [command, _url] if command == "lint" => write_line(
            errors,
            "browser.jr: lint is unavailable because page loading is not implemented",
            ExitStatus::Unavailable,
        ),
        [command] if command == "lint" => write_line(
            errors,
            "browser.jr: lint requires a URL",
            ExitStatus::InvalidInput,
        ),
        _ => write_line(
            errors,
            "browser.jr: invalid arguments; run browser.jr help",
            ExitStatus::InvalidInput,
        ),
    }
}

fn write_help(output: &mut impl Write) -> ExitStatus {
    if output.write_all(HELP.as_bytes()).is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::Unavailable
    }
}

fn write_line(output: &mut impl Write, value: &str, success: ExitStatus) -> ExitStatus {
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
        assert!(output.contains("not implemented yet"));
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
    fn unavailable_lint_does_not_claim_a_pass() {
        let (status, output, errors) = run(&["lint", "http://localhost:3000"]);

        assert_eq!(status, ExitStatus::Unavailable);
        assert!(output.is_empty());
        assert!(errors.contains("unavailable"));
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
