use std::io::{BufRead, Write};

use crate::cli::{ExitStatus, combine_status, write_line, write_session_error};
use crate::{
    CaptureInteractiveSnapshot, ClickElement, ClickResult, FillElement, GetElementAttribute,
    GetElementChecked, GetElementEnabled, GetElementText, GetElementValue, GetElementVisible,
    GetPageTitle, GetPageUrl, InteractiveElementRef, InteractiveElementState, InteractiveSnapshot,
    OpenPage, ReloadPage, SelectElement, Session, SetElementChecked,
};

const SESSION_HELP: &str = "session commands:
  open <url>
  reload
  snapshot --interactive
  click <ref>
  fill <ref> <text>
  select <ref> <value>
  check <ref>
  uncheck <ref>
  is checked <ref>
  is enabled <ref>
  is visible <ref>
  get attr <ref> <name>
  get text <ref>
  get value <ref>
  get url
  get title
  help
  exit
";

#[derive(Debug)]
struct CliSession {
    engine: Session,
    current_references: Vec<InteractiveElementRef>,
}

impl CliSession {
    fn new() -> Self {
        Self {
            engine: Session::new(),
            current_references: Vec::new(),
        }
    }

    fn run_command(
        &mut self,
        command: SessionCommand<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match command {
            SessionCommand::Page(command) => self.run_page_command(command, output, errors),
            SessionCommand::Element(command) => self.run_element_command(command, output, errors),
            SessionCommand::Help => {
                let status = if output.write_all(SESSION_HELP.as_bytes()).is_ok() {
                    ExitStatus::Success
                } else {
                    ExitStatus::Unavailable
                };
                SessionStep::Continue(status)
            }
            SessionCommand::Exit => SessionStep::Exit,
            SessionCommand::Empty => SessionStep::Continue(ExitStatus::Success),
        }
    }

    fn run_page_command(
        &mut self,
        command: PageCommand<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match command {
            PageCommand::Open(url) => self.open(url, output, errors),
            PageCommand::Reload => self.reload(output, errors),
            PageCommand::SnapshotInteractive => self.snapshot(output, errors),
            PageCommand::GetUrl => self.get_url(output, errors),
            PageCommand::GetTitle => self.get_title(output, errors),
        }
    }

    fn run_element_command(
        &mut self,
        command: ElementCommand<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match command {
            ElementCommand::Click(reference) => self.click(reference, output, errors),
            ElementCommand::Fill(reference, value) => self.fill(reference, value, output, errors),
            ElementCommand::Select(reference, value) => {
                self.select(reference, value, output, errors)
            }
            ElementCommand::Check(reference) => self.set_checked(reference, true, output, errors),
            ElementCommand::Uncheck(reference) => {
                self.set_checked(reference, false, output, errors)
            }
            ElementCommand::IsChecked(reference) => self.is_checked(reference, output, errors),
            ElementCommand::IsEnabled(reference) => self.is_enabled(reference, output, errors),
            ElementCommand::IsVisible(reference) => self.is_visible(reference, output, errors),
            ElementCommand::GetAttribute(reference, name) => {
                self.get_attribute(reference, name, output, errors)
            }
            ElementCommand::GetText(reference) => self.get_text(reference, output, errors),
            ElementCommand::GetValue(reference) => self.get_value(reference, output, errors),
        }
    }

    fn open(&mut self, url: &str, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(OpenPage { url: url.into() }) {
            Ok(page) => {
                self.current_references.clear();
                SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "opened url={} elements={}",
                        page.url, page.interactive_element_count
                    ),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn reload(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(ReloadPage) {
            Ok(page) => {
                self.current_references.clear();
                SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "reloaded url={} elements={}",
                        page.url, page.interactive_element_count
                    ),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn snapshot(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(CaptureInteractiveSnapshot) {
            Ok(snapshot) => {
                self.current_references = snapshot
                    .elements
                    .iter()
                    .map(|element| element.reference)
                    .collect();
                SessionStep::Continue(write_interactive_snapshot(output, &snapshot))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn click(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(ClickElement { reference }) {
            Ok(ClickResult::Navigated { reference, page }) => {
                self.current_references.clear();
                SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "navigated ref={reference} url={} elements={}",
                        page.url, page.interactive_element_count
                    ),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn fill(
        &mut self,
        reference_name: &str,
        value: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(FillElement {
            reference,
            value: value.into(),
        }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "filled ref={} characters={}",
                    result.reference,
                    result.value.chars().count()
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_value(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementValue { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("value ref={} {:?}", result.reference, result.value),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn select(
        &mut self,
        reference_name: &str,
        value: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(SelectElement {
            reference,
            value: value.into(),
        }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("selected ref={} value={:?}", result.reference, result.value),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_text(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementText { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("text ref={} {:?}", result.reference, result.text),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_attribute(
        &mut self,
        reference_name: &str,
        name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementAttribute {
            reference,
            name: name.into(),
        }) {
            Ok(result) => {
                let value = result
                    .value
                    .map(|value| format!("{value:?}"))
                    .unwrap_or_else(|| "null".into());
                SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "attr ref={} name={:?} value={value}",
                        result.reference, result.name
                    ),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn set_checked(
        &mut self,
        reference_name: &str,
        checked: bool,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self
            .engine
            .execute(SetElementChecked { reference, checked })
        {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "set checked ref={} value={}",
                    result.reference, result.checked
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn is_checked(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementChecked { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("checked ref={} value={}", result.reference, result.checked),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn is_enabled(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementEnabled { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("enabled ref={} value={}", result.reference, result.enabled),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn is_visible(
        &mut self,
        reference_name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(reference_name) else {
            return unknown_reference(errors, reference_name);
        };
        match self.engine.execute(GetElementVisible { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("visible ref={} value={}", result.reference, result.visible),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_url(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(GetPageUrl) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("url={}", result.url),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_title(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(GetPageTitle) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("title={:?}", result.title),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn run_line(
        &mut self,
        line: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match parse_command(line) {
            Ok(command) => self.run_command(command, output, errors),
            Err(message) => {
                SessionStep::Continue(write_line(errors, message, ExitStatus::InvalidInput))
            }
        }
    }

    fn resolve_reference(&self, name: &str) -> Option<InteractiveElementRef> {
        let ordinal = name.strip_prefix("@e")?.parse::<usize>().ok()?;
        let reference = *self.current_references.get(ordinal.checked_sub(1)?)?;
        (reference.to_string() == name).then_some(reference)
    }
}

fn unknown_reference(errors: &mut impl Write, reference_name: &str) -> SessionStep {
    SessionStep::Continue(write_line(
        errors,
        &format!("browser.jr: unknown or stale element reference {reference_name}"),
        ExitStatus::InvalidInput,
    ))
}

enum SessionCommand<'a> {
    Page(PageCommand<'a>),
    Element(ElementCommand<'a>),
    Help,
    Exit,
    Empty,
}

enum PageCommand<'a> {
    Open(&'a str),
    Reload,
    SnapshotInteractive,
    GetUrl,
    GetTitle,
}

enum ElementCommand<'a> {
    Click(&'a str),
    Fill(&'a str, &'a str),
    Select(&'a str, &'a str),
    Check(&'a str),
    Uncheck(&'a str),
    IsChecked(&'a str),
    IsEnabled(&'a str),
    IsVisible(&'a str),
    GetAttribute(&'a str, &'a str),
    GetText(&'a str),
    GetValue(&'a str),
}

enum SessionStep {
    Continue(ExitStatus),
    Exit,
}

pub(crate) fn run_session(
    input: &mut impl BufRead,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    if writeln!(output, "session ready").is_err() || output.flush().is_err() {
        return ExitStatus::Unavailable;
    }

    let mut session = CliSession::new();
    let commands_status = run_commands(&mut session, input, output, errors);
    let status = combine_status(
        commands_status,
        write_line(output, "session closed", ExitStatus::Success),
    );
    if flush_streams(output, errors).is_err() {
        ExitStatus::Unavailable
    } else {
        status
    }
}

fn run_commands(
    session: &mut CliSession,
    input: &mut impl BufRead,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let mut status = ExitStatus::Success;
    for line in input.lines() {
        let step = match line {
            Ok(line) => session.run_line(&line, output, errors),
            Err(error) => {
                let read_status = write_line(
                    errors,
                    &format!("browser.jr: session input failed: {error}"),
                    ExitStatus::Unavailable,
                );
                return combine_status(status, read_status);
            }
        };
        match step {
            SessionStep::Continue(command_status) => {
                status = combine_status(status, command_status);
            }
            SessionStep::Exit => break,
        }
        if flush_streams(output, errors).is_err() {
            return ExitStatus::Unavailable;
        }
    }
    status
}

pub(crate) fn write_interactive_snapshot(
    output: &mut impl Write,
    snapshot: &InteractiveSnapshot,
) -> ExitStatus {
    if writeln!(
        output,
        "snapshot={} url={} mode=interactive elements={}",
        snapshot.id.get(),
        snapshot.url,
        snapshot.elements.len()
    )
    .is_err()
    {
        return ExitStatus::Unavailable;
    }
    for element in &snapshot.elements {
        let state = match &element.state {
            InteractiveElementState::Unavailable => String::new(),
            InteractiveElementState::Value(value) => format!(": {value:?}"),
            InteractiveElementState::Checked(checked) => format!(" [checked={checked}]"),
        };
        let written = writeln!(
            output,
            "- {} {:?} [ref={}]{}",
            element.role, element.name, element.reference, state
        );
        if written.is_err() {
            return ExitStatus::Unavailable;
        }
    }
    ExitStatus::Success
}

fn parse_command(line: &str) -> Result<SessionCommand<'_>, &'static str> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if let Some(rest) = line.strip_prefix("fill")
        && (rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace())
    {
        return parse_fill_command(rest);
    }
    if let Some(rest) = line.strip_prefix("select")
        && (rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace())
    {
        return parse_select_command(rest);
    }
    let mut parts = line.split_ascii_whitespace();
    let command = parts.next();
    let arguments = (parts.next(), parts.next(), parts.next());
    match command {
        None if arguments == (None, None, None) => Ok(SessionCommand::Empty),
        Some(command @ ("open" | "reload" | "snapshot")) => parse_page_command(command, arguments),
        Some("get") => parse_get_command(arguments),
        Some(command @ ("click" | "check" | "uncheck" | "is")) => {
            parse_element_command(command, arguments)
        }
        Some(command @ ("help" | "exit")) => parse_lifecycle_command(command, arguments),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

type CommandArguments<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);

fn parse_page_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    match (command, arguments) {
        ("open", (Some(url), None, None)) => Ok(SessionCommand::Page(PageCommand::Open(url))),
        ("reload", (None, None, None)) => Ok(SessionCommand::Page(PageCommand::Reload)),
        ("snapshot", (Some("--interactive" | "-i"), None, None)) => {
            Ok(SessionCommand::Page(PageCommand::SnapshotInteractive))
        }
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_get_command(arguments: CommandArguments<'_>) -> Result<SessionCommand<'_>, &'static str> {
    match arguments {
        (Some("value"), Some(reference), None) => {
            Ok(SessionCommand::Element(ElementCommand::GetValue(reference)))
        }
        (Some("text"), Some(reference), None) => {
            Ok(SessionCommand::Element(ElementCommand::GetText(reference)))
        }
        (Some("attr"), Some(reference), Some(name)) => Ok(SessionCommand::Element(
            ElementCommand::GetAttribute(reference, name),
        )),
        (Some("url"), None, None) => Ok(SessionCommand::Page(PageCommand::GetUrl)),
        (Some("title"), None, None) => Ok(SessionCommand::Page(PageCommand::GetTitle)),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_element_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    let command = match (command, arguments) {
        ("click", (Some(reference), None, None)) => ElementCommand::Click(reference),
        ("check", (Some(reference), None, None)) => ElementCommand::Check(reference),
        ("uncheck", (Some(reference), None, None)) => ElementCommand::Uncheck(reference),
        ("is", (Some("checked"), Some(reference), None)) => ElementCommand::IsChecked(reference),
        ("is", (Some("enabled"), Some(reference), None)) => ElementCommand::IsEnabled(reference),
        ("is", (Some("visible"), Some(reference), None)) => ElementCommand::IsVisible(reference),
        _ => return Err("browser.jr: invalid session command; enter help"),
    };
    Ok(SessionCommand::Element(command))
}

fn parse_lifecycle_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    match (command, arguments) {
        ("help", (None, None, None)) => Ok(SessionCommand::Help),
        ("exit", (None, None, None)) => Ok(SessionCommand::Exit),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_fill_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let (reference, value) =
        parse_reference_and_value(rest, "browser.jr: fill requires a reference and text")?;
    Ok(SessionCommand::Element(ElementCommand::Fill(
        reference, value,
    )))
}

fn parse_select_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let (reference, value) =
        parse_reference_and_value(rest, "browser.jr: select requires a reference and value")?;
    Ok(SessionCommand::Element(ElementCommand::Select(
        reference, value,
    )))
}

fn parse_reference_and_value<'a>(
    rest: &'a str,
    error: &'static str,
) -> Result<(&'a str, &'a str), &'static str> {
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let Some(boundary) = rest.find(|value: char| value.is_ascii_whitespace()) else {
        return Err(error);
    };
    let reference = &rest[..boundary];
    let value = rest[boundary..].trim_start_matches(|value: char| value.is_ascii_whitespace());
    if reference.is_empty() {
        Err(error)
    } else {
        Ok((reference, value))
    }
}

fn flush_streams(output: &mut impl Write, errors: &mut impl Write) -> std::io::Result<()> {
    output.flush()?;
    errors.flush()
}

#[cfg(test)]
mod tests {
    use super::{
        ElementCommand, ExitStatus, PageCommand, SessionCommand, parse_command, run_session,
    };
    use std::io::Cursor;

    #[test]
    fn invalid_command_does_not_end_the_session() {
        let mut input = Cursor::new("wat\nhelp\nexit\n");
        let mut output = Vec::new();
        let mut errors = Vec::new();

        let status = run_session(&mut input, &mut output, &mut errors);

        assert_eq!(status, ExitStatus::InvalidInput);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("session ready"));
        assert!(output.contains("session commands:"));
        assert!(output.contains("session closed"));
        assert_eq!(
            String::from_utf8(errors).unwrap(),
            "browser.jr: invalid session command; enter help\n"
        );
    }

    #[test]
    fn reference_names_require_the_current_snapshot() {
        let mut input = Cursor::new("click @e1\nexit\n");
        let mut output = Vec::new();
        let mut errors = Vec::new();

        let status = run_session(&mut input, &mut output, &mut errors);

        assert_eq!(status, ExitStatus::InvalidInput);
        assert!(
            String::from_utf8(errors)
                .unwrap()
                .contains("unknown or stale element reference @e1")
        );
    }

    #[test]
    fn fill_uses_the_remaining_line_as_text() {
        assert!(matches!(
            parse_command("fill @e1 hello world"),
            Ok(SessionCommand::Element(ElementCommand::Fill(
                "@e1",
                "hello world"
            )))
        ));
        assert!(matches!(
            parse_command("fill @e1 "),
            Ok(SessionCommand::Element(ElementCommand::Fill("@e1", "")))
        ));
        assert!(parse_command("fill @e1").is_err());
        assert!(matches!(
            parse_command("select @e1 large value"),
            Ok(SessionCommand::Element(ElementCommand::Select(
                "@e1",
                "large value"
            )))
        ));
        assert!(matches!(
            parse_command("select @e1 "),
            Ok(SessionCommand::Element(ElementCommand::Select("@e1", "")))
        ));
        assert!(parse_command("select @e1").is_err());
        assert!(matches!(
            parse_command("get value @e1"),
            Ok(SessionCommand::Element(ElementCommand::GetValue("@e1")))
        ));
        assert!(matches!(
            parse_command("get text @e1"),
            Ok(SessionCommand::Element(ElementCommand::GetText("@e1")))
        ));
        assert!(matches!(
            parse_command("get attr @e1 href"),
            Ok(SessionCommand::Element(ElementCommand::GetAttribute(
                "@e1", "href"
            )))
        ));
        assert!(matches!(
            parse_command("get url"),
            Ok(SessionCommand::Page(PageCommand::GetUrl))
        ));
        assert!(matches!(
            parse_command("reload"),
            Ok(SessionCommand::Page(PageCommand::Reload))
        ));
        assert!(matches!(
            parse_command("get title"),
            Ok(SessionCommand::Page(PageCommand::GetTitle))
        ));
        assert!(matches!(
            parse_command("check @e1"),
            Ok(SessionCommand::Element(ElementCommand::Check("@e1")))
        ));
        assert!(matches!(
            parse_command("uncheck @e1"),
            Ok(SessionCommand::Element(ElementCommand::Uncheck("@e1")))
        ));
        assert!(matches!(
            parse_command("is checked @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsChecked("@e1")))
        ));
        assert!(matches!(
            parse_command("is enabled @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsEnabled("@e1")))
        ));
        assert!(matches!(
            parse_command("is visible @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsVisible("@e1")))
        ));
        assert!(parse_command("get value @e1 extra").is_err());
    }
}
