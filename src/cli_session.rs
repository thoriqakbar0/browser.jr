use std::io::{BufRead, Write};

use crate::cli::{ExitStatus, combine_status, write_line, write_session_error};
use crate::{
    CaptureInteractiveSnapshot, ClickByRole, ClickByRoleResult, ClickElement, ClickResult,
    FillByRole, FillElement, FindByRole, GetElementAttribute, GetElementChecked, GetElementEnabled,
    GetElementText, GetElementValue, GetElementVisible, GetPageTitle, GetPageUrl, HoverByRole,
    HoverByRoleResult, InteractiveElementRef, InteractiveElementState, InteractiveSnapshot,
    OpenPage, ReloadPage, RoleLocator, SelectElement, Session, SetCheckedByRole, SetElementChecked,
};

const SESSION_HELP: &str = "session commands:
  open <url>
  reload
  snapshot --interactive
  find role <role> [click|fill <text>|check|uncheck|hover|text] [--name <accessible-name>] [--exact]
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
            ElementCommand::FindRole {
                role,
                name,
                exact,
                action,
            } => self.find_role(role, name, exact, action, output, errors),
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

    fn find_role(
        &mut self,
        role: &str,
        name: Option<&str>,
        exact: bool,
        action: FindRoleAction<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match RoleLocator::new(role) {
            Ok(locator) => match name {
                Some(name) if exact => locator.with_exact_name(name),
                Some(name) => locator.with_name(name),
                None => locator,
            },
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: invalid role locator: {error}"),
                    ExitStatus::InvalidInput,
                ));
            }
        };
        match action {
            FindRoleAction::Click => match self.engine.execute(ClickByRole { locator }) {
                Ok(ClickByRoleResult::Navigated { matched, page }) => {
                    self.current_references.clear();
                    SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "navigated role={:?} name={:?} element={:?} url={} elements={}",
                            matched.role,
                            matched.name,
                            matched.element,
                            page.url,
                            page.interactive_element_count
                        ),
                        ExitStatus::Success,
                    ))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Fill(value) => match self.engine.execute(FillByRole {
                locator,
                value: value.into(),
            }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "filled role={:?} name={:?} element={:?} characters={}",
                        result.matched.role,
                        result.matched.name,
                        result.matched.element,
                        result.value.chars().count()
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Check | FindRoleAction::Uncheck => {
                let checked = action == FindRoleAction::Check;
                match self.engine.execute(SetCheckedByRole { locator, checked }) {
                    Ok(result) => SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "checked role={:?} name={:?} element={:?} checked={}",
                            result.matched.role,
                            result.matched.name,
                            result.matched.element,
                            result.checked
                        ),
                        ExitStatus::Success,
                    )),
                    Err(error) => SessionStep::Continue(write_session_error(errors, error)),
                }
            }
            FindRoleAction::Hover => match self.engine.execute(HoverByRole { locator }) {
                Ok(HoverByRoleResult { matched }) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "hovered role={:?} name={:?} element={:?}",
                        matched.role, matched.name, matched.element
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Text => match self.engine.execute(FindByRole { locator }) {
                Ok(element) => {
                    SessionStep::Continue(write_line(output, &element.text, ExitStatus::Success))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
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
    FindRole {
        role: &'a str,
        name: Option<&'a str>,
        exact: bool,
        action: FindRoleAction<'a>,
    },
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FindRoleAction<'a> {
    Click,
    Fill(&'a str),
    Check,
    Uncheck,
    Hover,
    Text,
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
    if let Some(rest) = line.strip_prefix("find")
        && (rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace())
    {
        return parse_find_command(rest);
    }
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

fn parse_find_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: find requires role <role> [click|fill <text>|check|uncheck|hover|text] [--name <accessible-name>] [--exact]";
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let rest = strip_option(rest, "role").ok_or(ERROR)?;
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let (role, rest) = split_first_token(rest).ok_or(ERROR)?;
    let (action, options) = parse_find_action(rest).ok_or(ERROR)?;
    let (name, exact) = parse_find_options(options).ok_or(ERROR)?;
    Ok(SessionCommand::Element(ElementCommand::FindRole {
        role,
        name,
        exact,
        action,
    }))
}

fn parse_find_action(rest: &str) -> Option<(FindRoleAction<'_>, &str)> {
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let Some((token, remaining)) = split_first_token(rest) else {
        return Some((FindRoleAction::Click, rest));
    };
    match token {
        "click" => Some((FindRoleAction::Click, remaining)),
        "check" => Some((FindRoleAction::Check, remaining)),
        "uncheck" => Some((FindRoleAction::Uncheck, remaining)),
        "hover" => Some((FindRoleAction::Hover, remaining)),
        "text" => Some((FindRoleAction::Text, remaining)),
        "fill" => {
            let input = remaining.trim_start_matches(|value: char| value.is_ascii_whitespace());
            let (value, options) = split_find_fill_options(input);
            (!value.is_empty()).then_some((FindRoleAction::Fill(value), options))
        }
        _ => Some((FindRoleAction::Click, rest)),
    }
}

fn split_find_fill_options(input: &str) -> (&str, &str) {
    let boundary = [find_token(input, "--name"), find_token(input, "--exact")]
        .into_iter()
        .flatten()
        .min();
    match boundary {
        Some(boundary) => (input[..boundary].trim_end(), &input[boundary..]),
        None => (input, ""),
    }
}

fn find_token(value: &str, token: &str) -> Option<usize> {
    value.match_indices(token).find_map(|(index, _)| {
        let before = index == 0 || value.as_bytes()[index - 1].is_ascii_whitespace();
        let after_index = index + token.len();
        let after =
            after_index == value.len() || value.as_bytes()[after_index].is_ascii_whitespace();
        (before && after).then_some(index)
    })
}

fn parse_find_options(options: &str) -> Option<(Option<&str>, bool)> {
    let options = options.trim_start_matches(|value: char| value.is_ascii_whitespace());
    if options.is_empty() {
        return Some((None, false));
    }
    if let Some(rest) = strip_option(options, "--exact") {
        let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
        return Some((Some(parse_name_option(rest)?), true));
    }
    let name = parse_name_option(options)?;
    let (name, exact) = strip_trailing_exact(name);
    (!name.is_empty()).then_some((Some(name), exact))
}

fn strip_trailing_exact(name: &str) -> (&str, bool) {
    let name = name.trim_end_matches(|value: char| value.is_ascii_whitespace());
    let Some(prefix) = name.strip_suffix("--exact") else {
        return (name, false);
    };
    if prefix.is_empty() {
        return ("", true);
    }
    if !prefix
        .chars()
        .next_back()
        .is_some_and(|value| value.is_ascii_whitespace())
    {
        return (name, false);
    }
    (
        prefix.trim_end_matches(|value: char| value.is_ascii_whitespace()),
        true,
    )
}

fn split_first_token(value: &str) -> Option<(&str, &str)> {
    if value.is_empty() {
        return None;
    }
    match value.find(|character: char| character.is_ascii_whitespace()) {
        Some(boundary) => Some((&value[..boundary], &value[boundary..])),
        None => Some((value, "")),
    }
}

fn strip_option<'a>(value: &'a str, option: &str) -> Option<&'a str> {
    let rest = value.strip_prefix(option)?;
    (rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace()).then_some(rest)
}

fn parse_name_option(value: &str) -> Option<&str> {
    let rest = strip_option(value, "--name")?;
    if rest.is_empty() {
        return None;
    }
    let name = rest.trim_start_matches(|character: char| character.is_ascii_whitespace());
    (!name.is_empty()).then_some(name)
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
        ElementCommand, ExitStatus, FindRoleAction, PageCommand, SessionCommand, parse_command,
        run_session,
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
        assert!(output.contains("find role <role>"));
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

    #[test]
    fn find_role_parses_name_and_exact_variants() {
        assert!(matches!(
            parse_command("find role button"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                name: None,
                exact: false,
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --name Save changes"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                name: Some("Save changes"),
                exact: false,
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --name Save changes --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                name: Some("Save changes"),
                exact: true,
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --exact --name Save changes"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                name: Some("Save changes"),
                exact: true,
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role heading text --name Skills"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "heading",
                name: Some("Skills"),
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find role textbox fill hello world --name Email address --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "textbox",
                name: Some("Email address"),
                exact: true,
                action: FindRoleAction::Fill("hello world"),
            }))
        ));
        assert!(matches!(
            parse_command("find role checkbox check --name Terms"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "checkbox",
                name: Some("Terms"),
                exact: false,
                action: FindRoleAction::Check,
            }))
        ));
        assert!(matches!(
            parse_command("find role checkbox uncheck --name Terms"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "checkbox",
                name: Some("Terms"),
                exact: false,
                action: FindRoleAction::Uncheck,
            }))
        ));
        assert!(matches!(
            parse_command("find role button hover --name Menu"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                name: Some("Menu"),
                exact: false,
                action: FindRoleAction::Hover,
            }))
        ));
        assert!(parse_command("find role").is_err());
        assert!(parse_command("find role button --name").is_err());
        assert!(parse_command("find role button --name --exact").is_err());
        assert!(parse_command("find role button --exact").is_err());
        assert!(parse_command("find role textbox fill --name Email").is_err());
        assert!(parse_command("find text Save").is_err());
    }
}
