use std::io::{BufRead, Write};

use crate::cli::{ExitStatus, combine_status, write_line, write_session_error};
use crate::{
    AltLocator, CaptureInteractiveSnapshot, CaptureInteractiveSnapshotWithin, ClickByLocator,
    ClickByLocatorResult, ClickElement, ClickResult, CountByLocator, CssLocator, FillByLocator,
    FillElement, FindByLocator, GetAttributeByLocator, GetCheckedByLocator, GetElementAttribute,
    GetElementChecked, GetElementEnabled, GetElementHtml, GetElementText, GetElementValue,
    GetElementVisible, GetEnabledByLocator, GetHtmlByLocator, GetPageTitle, GetPageUrl,
    GetValueByLocator, GetVisibleByLocator, HoverByLocator, HoverByLocatorResult,
    InteractiveElementRef, InteractiveElementState, InteractiveSnapshot, LabelLocator, Locator,
    NonEmpty, OpenPage, PlaceholderLocator, ReloadPage, RoleLocator, SelectOptionTarget,
    SelectOptions, SelectOptionsByLocator, SelectOptionsByLocatorResult, SelectOptionsResult,
    Session, SetCheckedByLocator, SetElementChecked, TakeDomEvents, TestIdLocator, TextLocator,
    TitleLocator, XPathLocator,
};

const SESSION_HELP: &str = "session commands:
  open <url>
  reload
  snapshot --interactive [-s|--selector <css>]
  find role <role> [click|fill <text>|check|uncheck|hover|text] [--name <accessible-name>] [--exact]
  find text <text> [click|fill <text>|check|uncheck|hover|text] [--exact]
  find label <label> [click|fill <text>|check|uncheck|hover|text] [--exact]
  find placeholder <text> [click|fill <text>|check|uncheck|hover|text] [--exact]
  find alt <text> [click|fill <text>|check|uncheck|hover|text] [--exact]
  find title <text> [click|fill <text>|check|uncheck|hover|text] [--exact]
  find testid <id> [click|fill <text>|check|uncheck|hover|text]
  find css <selector> [click|fill <text>|check|uncheck|hover|text]
  find xpath <expression> [click|fill <text>|check|uncheck|hover|text]
  find first <selector> [click|fill <text>|check|uncheck|hover|text]
  find last <selector> [click|fill <text>|check|uncheck|hover|text]
  find nth <index> <selector> [click|fill <text>|check|uncheck|hover|text]
  click <ref|selector>
  fill <ref|selector> <text>
  select <ref|selector> <value>
  select <ref|selector> \"<value>\" [\"<value>\" ...]
  check <ref|selector>
  uncheck <ref|selector>
  is checked <ref|selector>
  is enabled <ref|selector>
  is visible <ref|selector>
  get attr <ref|selector> <name>
  get count <selector>
  get html <ref|selector>
  get text <ref|selector>
  get value <ref|selector>
  get url
  get title
  events
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
            SessionCommand::Events => self.events(output),
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

    fn events(&mut self, output: &mut impl Write) -> SessionStep {
        let events = self
            .engine
            .execute(TakeDomEvents)
            .expect("taking DOM events has no failure path");
        let mut status = write_line(
            output,
            &format!("events={}", events.len()),
            ExitStatus::Success,
        );
        for event in events {
            status = combine_status(
                status,
                write_line(
                    output,
                    &format!(
                        "event type={} document={} target={:?} bubbles={} path={:?} ordinal={}",
                        event.event_type,
                        event.document_epoch,
                        event.target,
                        event.bubbles,
                        event.path.join(" > "),
                        event.target_ordinal
                    ),
                    ExitStatus::Success,
                ),
            );
        }
        SessionStep::Continue(status)
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
            PageCommand::SnapshotInteractive(selector) => self.snapshot(selector, output, errors),
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
            ElementCommand::FindLocator {
                kind,
                value,
                exact,
                action,
            } => self.find_typed_locator(kind, value, exact, action, output, errors),
            ElementCommand::Fill(reference, value) => self.fill(reference, value, output, errors),
            ElementCommand::Select(reference, values) => {
                self.select(reference, values, output, errors)
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
            ElementCommand::GetCount(selector) => self.get_count(selector, output, errors),
            ElementCommand::GetHtml(reference) => self.get_html(reference, output, errors),
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

    fn snapshot(
        &mut self,
        selector: Option<&str>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let snapshot = match selector {
            Some(selector) => {
                let locator = match CssLocator::new(selector) {
                    Ok(locator) => Locator::from(locator),
                    Err(error) => return invalid_locator(errors, error.to_string()),
                };
                self.engine
                    .execute(CaptureInteractiveSnapshotWithin { locator })
            }
            None => self.engine.execute(CaptureInteractiveSnapshot),
        };
        match snapshot {
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
        self.find_locator(locator.into(), action, output, errors)
    }

    fn find_typed_locator(
        &mut self,
        kind: FindLocatorKind,
        value: &str,
        exact: bool,
        action: FindRoleAction<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match build_locator(kind, value, exact) {
            Ok(locator) => locator,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: invalid locator: {error}"),
                    ExitStatus::InvalidInput,
                ));
            }
        };
        self.find_locator(locator, action, output, errors)
    }

    fn find_locator(
        &mut self,
        locator: Locator,
        action: FindRoleAction<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match action {
            FindRoleAction::Click => match self.engine.execute(ClickByLocator { locator }) {
                Ok(ClickByLocatorResult::Navigated { matched, page }) => {
                    self.current_references.clear();
                    SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "navigated role={:?} name={:?} element={:?} url={} elements={}",
                            matched.role.as_deref().unwrap_or(""),
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
            FindRoleAction::Fill(value) => match self.engine.execute(FillByLocator {
                locator,
                value: value.into(),
            }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "filled role={:?} name={:?} element={:?} characters={}",
                        result.matched.role.as_deref().unwrap_or(""),
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
                match self
                    .engine
                    .execute(SetCheckedByLocator { locator, checked })
                {
                    Ok(result) => SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "checked role={:?} name={:?} element={:?} checked={}",
                            result.matched.role.as_deref().unwrap_or(""),
                            result.matched.name,
                            result.matched.element,
                            result.checked
                        ),
                        ExitStatus::Success,
                    )),
                    Err(error) => SessionStep::Continue(write_session_error(errors, error)),
                }
            }
            FindRoleAction::Hover => match self.engine.execute(HoverByLocator { locator }) {
                Ok(HoverByLocatorResult { matched }) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "hovered role={:?} name={:?} element={:?}",
                        matched.role.as_deref().unwrap_or(""),
                        matched.name,
                        matched.element
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Text => match self.engine.execute(FindByLocator { locator }) {
                Ok(element) => {
                    SessionStep::Continue(write_line(output, &element.text, ExitStatus::Success))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
        }
    }

    fn click(
        &mut self,
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            return self.run_direct_locator(target, FindRoleAction::Click, output, errors);
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
        target: &str,
        value: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            return self.run_direct_locator(target, FindRoleAction::Fill(value), output, errors);
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
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetValueByLocator { locator }) {
                Ok(result) => {
                    SessionStep::Continue(write_line(output, &result.value, ExitStatus::Success))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
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
        target: &str,
        values: SelectCommandValues<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let list_output = matches!(values, SelectCommandValues::List(_));
        let options = match values {
            SelectCommandValues::Single(value) => {
                NonEmpty::one(SelectOptionTarget::Value(value.into()))
            }
            SelectCommandValues::List(values) => NonEmpty::from_vec(
                values
                    .into_iter()
                    .map(str::to_owned)
                    .map(SelectOptionTarget::Value)
                    .collect(),
            )
            .expect("the select parser requires at least one quoted value"),
        };
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self
                .engine
                .execute(SelectOptionsByLocator { locator, options })
            {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format_locator_selection(&result, list_output),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(SelectOptions { reference, options }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format_reference_selection(&result, list_output),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_text(
        &mut self,
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            return self.run_direct_locator(target, FindRoleAction::Text, output, errors);
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

    fn get_html(
        &mut self,
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetHtmlByLocator { locator }) {
                Ok(result) => {
                    SessionStep::Continue(write_line(output, &result.html, ExitStatus::Success))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(GetElementHtml { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("html ref={} {:?}", result.reference, result.html),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn get_attribute(
        &mut self,
        target: &str,
        name: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetAttributeByLocator {
                locator,
                name: name.into(),
            }) {
                Ok(result) => {
                    let value = result.value.unwrap_or_else(|| "null".into());
                    SessionStep::Continue(write_line(output, &value, ExitStatus::Success))
                }
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
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

    fn get_count(
        &mut self,
        selector: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match build_direct_locator(selector) {
            Ok(locator) => locator,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: invalid locator: {error}"),
                    ExitStatus::InvalidInput,
                ));
            }
        };
        match self.engine.execute(CountByLocator { locator }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &result.count.to_string(),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn set_checked(
        &mut self,
        target: &str,
        checked: bool,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let action = if checked {
                FindRoleAction::Check
            } else {
                FindRoleAction::Uncheck
            };
            return self.run_direct_locator(target, action, output, errors);
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
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetCheckedByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.checked.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
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
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetEnabledByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.enabled.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
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
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(GetVisibleByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.visible.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
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

    fn run_direct_locator(
        &mut self,
        target: &str,
        action: FindRoleAction<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match build_direct_locator(target) {
            Ok(locator) => locator,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: invalid locator: {error}"),
                    ExitStatus::InvalidInput,
                ));
            }
        };
        self.find_locator(locator, action, output, errors)
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

fn build_locator(kind: FindLocatorKind, value: &str, exact: bool) -> Result<Locator, String> {
    match kind {
        FindLocatorKind::Text => TextLocator::new(value)
            .map(|locator| Locator::from(if exact { locator.exact() } else { locator }))
            .map_err(|error| error.to_string()),
        FindLocatorKind::Label => LabelLocator::new(value)
            .map(|locator| Locator::from(if exact { locator.exact() } else { locator }))
            .map_err(|error| error.to_string()),
        FindLocatorKind::Placeholder => PlaceholderLocator::new(value)
            .map(|locator| Locator::from(if exact { locator.exact() } else { locator }))
            .map_err(|error| error.to_string()),
        FindLocatorKind::Alt => AltLocator::new(value)
            .map(|locator| Locator::from(if exact { locator.exact() } else { locator }))
            .map_err(|error| error.to_string()),
        FindLocatorKind::Title => TitleLocator::new(value)
            .map(|locator| Locator::from(if exact { locator.exact() } else { locator }))
            .map_err(|error| error.to_string()),
        FindLocatorKind::TestId => TestIdLocator::new(value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
        FindLocatorKind::Css => CssLocator::new(value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
        FindLocatorKind::XPath => XPathLocator::new(value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
        FindLocatorKind::First => CssLocator::first(value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
        FindLocatorKind::Last => CssLocator::last(value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
        FindLocatorKind::Nth(index) => CssLocator::nth(index, value)
            .map(Locator::from)
            .map_err(|error| error.to_string()),
    }
}

fn build_direct_locator(target: &str) -> Result<Locator, String> {
    if let Some(expression) = target.strip_prefix("xpath=") {
        return XPathLocator::new(expression)
            .map(Locator::from)
            .map_err(|error| error.to_string());
    }
    if target.starts_with("//") || target.starts_with("..") {
        return XPathLocator::new(target)
            .map(Locator::from)
            .map_err(|error| error.to_string());
    }
    let selector = target.strip_prefix("css=").unwrap_or(target);
    CssLocator::new(selector)
        .map(Locator::from)
        .map_err(|error| error.to_string())
}

fn invalid_locator(errors: &mut impl Write, error: String) -> SessionStep {
    SessionStep::Continue(write_line(
        errors,
        &format!("browser.jr: invalid locator: {error}"),
        ExitStatus::InvalidInput,
    ))
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
    Events,
    Help,
    Exit,
    Empty,
}

enum PageCommand<'a> {
    Open(&'a str),
    Reload,
    SnapshotInteractive(Option<&'a str>),
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
    FindLocator {
        kind: FindLocatorKind,
        value: &'a str,
        exact: bool,
        action: FindRoleAction<'a>,
    },
    Click(&'a str),
    Fill(&'a str, &'a str),
    Select(&'a str, SelectCommandValues<'a>),
    Check(&'a str),
    Uncheck(&'a str),
    IsChecked(&'a str),
    IsEnabled(&'a str),
    IsVisible(&'a str),
    GetAttribute(&'a str, &'a str),
    GetCount(&'a str),
    GetHtml(&'a str),
    GetText(&'a str),
    GetValue(&'a str),
}

enum SelectCommandValues<'a> {
    Single(&'a str),
    List(Vec<&'a str>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FindLocatorKind {
    Text,
    Label,
    Placeholder,
    Alt,
    Title,
    TestId,
    Css,
    XPath,
    First,
    Last,
    Nth(usize),
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

fn format_locator_selection(result: &SelectOptionsByLocatorResult, list_output: bool) -> String {
    let selection = format_selection(&result.selected, list_output);
    format!(
        "selected role={:?} name={:?} element={:?} {selection}",
        result.matched.role.as_deref().unwrap_or(""),
        result.matched.name,
        result.matched.element,
    )
}

fn format_reference_selection(result: &SelectOptionsResult, list_output: bool) -> String {
    let selection = format_selection(&result.selected, list_output);
    format!("selected ref={} {selection}", result.reference)
}

fn format_selection(selected: &NonEmpty<String>, list_output: bool) -> String {
    if list_output {
        format!("values={:?}", selected.iter().collect::<Vec<_>>())
    } else {
        format!("value={:?}", selected[0])
    }
}

fn parse_command(line: &str) -> Result<SessionCommand<'_>, &'static str> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if let Some(rest) = command_rest(line, "find") {
        return parse_find_command(rest);
    }
    if let Some(rest) = command_rest(line, "snapshot") {
        return parse_snapshot_session_command(rest);
    }
    if let Some(rest) = command_rest(line, "fill") {
        return parse_fill_command(rest);
    }
    if let Some(rest) = command_rest(line, "select") {
        return parse_select_command(rest);
    }
    if let Some(rest) = command_rest(line, "get attr") {
        return parse_target_and_token(
            rest,
            "browser.jr: get attr requires a reference or selector and attribute name",
        )
        .map(|(target, name)| SessionCommand::Element(ElementCommand::GetAttribute(target, name)));
    }
    if let Some(command) = parse_simple_target_command(line) {
        return command;
    }
    let mut parts = line.split_ascii_whitespace();
    let command = parts.next();
    let arguments = (parts.next(), parts.next(), parts.next());
    match command {
        None if arguments == (None, None, None) => Ok(SessionCommand::Empty),
        Some(command @ ("open" | "reload")) => parse_page_command(command, arguments),
        Some("get") => parse_get_command(arguments),
        Some(command @ ("events" | "help" | "exit")) => parse_lifecycle_command(command, arguments),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

#[derive(Clone, Copy)]
enum SimpleTargetCommand {
    Click,
    GetText,
    GetCount,
    GetHtml,
    GetValue,
    Check,
    Uncheck,
    IsChecked,
    IsEnabled,
    IsVisible,
}

impl SimpleTargetCommand {
    fn build(self, target: &str) -> SessionCommand<'_> {
        let command = match self {
            Self::Click => ElementCommand::Click(target),
            Self::GetText => ElementCommand::GetText(target),
            Self::GetCount => ElementCommand::GetCount(target),
            Self::GetHtml => ElementCommand::GetHtml(target),
            Self::GetValue => ElementCommand::GetValue(target),
            Self::Check => ElementCommand::Check(target),
            Self::Uncheck => ElementCommand::Uncheck(target),
            Self::IsChecked => ElementCommand::IsChecked(target),
            Self::IsEnabled => ElementCommand::IsEnabled(target),
            Self::IsVisible => ElementCommand::IsVisible(target),
        };
        SessionCommand::Element(command)
    }
}

fn parse_simple_target_command(line: &str) -> Option<Result<SessionCommand<'_>, &'static str>> {
    const COMMANDS: &[(&str, &str, SimpleTargetCommand)] = &[
        (
            "click",
            "browser.jr: click requires a reference or selector",
            SimpleTargetCommand::Click,
        ),
        (
            "get text",
            "browser.jr: get text requires a reference or selector",
            SimpleTargetCommand::GetText,
        ),
        (
            "get count",
            "browser.jr: get count requires a selector",
            SimpleTargetCommand::GetCount,
        ),
        (
            "get html",
            "browser.jr: get html requires a reference or selector",
            SimpleTargetCommand::GetHtml,
        ),
        (
            "get value",
            "browser.jr: get value requires a reference or selector",
            SimpleTargetCommand::GetValue,
        ),
        (
            "check",
            "browser.jr: check requires a reference or selector",
            SimpleTargetCommand::Check,
        ),
        (
            "uncheck",
            "browser.jr: uncheck requires a reference or selector",
            SimpleTargetCommand::Uncheck,
        ),
        (
            "is checked",
            "browser.jr: is checked requires a reference or selector",
            SimpleTargetCommand::IsChecked,
        ),
        (
            "is enabled",
            "browser.jr: is enabled requires a reference or selector",
            SimpleTargetCommand::IsEnabled,
        ),
        (
            "is visible",
            "browser.jr: is visible requires a reference or selector",
            SimpleTargetCommand::IsVisible,
        ),
    ];
    COMMANDS.iter().find_map(|(name, error, command)| {
        command_rest(line, name)
            .map(|rest| parse_target_command(rest, error).map(|target| command.build(target)))
    })
}

fn command_rest<'a>(line: &'a str, command: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(command)?;
    (rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace()).then_some(rest)
}

fn parse_find_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: find requires role|text|label|placeholder|alt|title|testid|css|xpath|first|last|nth and a valid value";
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let (kind, rest) = split_first_token(rest).ok_or(ERROR)?;
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    if kind != "role" {
        let kind = match kind {
            "text" => FindLocatorKind::Text,
            "label" => FindLocatorKind::Label,
            "placeholder" => FindLocatorKind::Placeholder,
            "alt" => FindLocatorKind::Alt,
            "title" => FindLocatorKind::Title,
            "testid" => FindLocatorKind::TestId,
            "css" => FindLocatorKind::Css,
            "xpath" => FindLocatorKind::XPath,
            "first" => FindLocatorKind::First,
            "last" => FindLocatorKind::Last,
            "nth" => {
                let (index, remaining) = split_first_token(rest).ok_or(ERROR)?;
                let index = index.parse::<usize>().map_err(|_| ERROR)?;
                let remaining =
                    remaining.trim_start_matches(|value: char| value.is_ascii_whitespace());
                return parse_non_role_locator(FindLocatorKind::Nth(index), remaining, ERROR);
            }
            _ => return Err(ERROR),
        };
        return parse_non_role_locator(kind, rest, ERROR);
    }
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

fn parse_non_role_locator<'a>(
    kind: FindLocatorKind,
    rest: &'a str,
    error: &'static str,
) -> Result<SessionCommand<'a>, &'static str> {
    let (value, rest) = split_locator_value(rest).ok_or(error)?;
    let (action, options) = parse_find_action(rest).ok_or(error)?;
    let exact = parse_exact_option(options).ok_or(error)?;
    let supports_exact = matches!(
        kind,
        FindLocatorKind::Text
            | FindLocatorKind::Label
            | FindLocatorKind::Placeholder
            | FindLocatorKind::Alt
            | FindLocatorKind::Title
    );
    if exact && !supports_exact {
        return Err(error);
    }
    Ok(SessionCommand::Element(ElementCommand::FindLocator {
        kind,
        value,
        exact,
        action,
    }))
}

fn split_locator_value(value: &str) -> Option<(&str, &str)> {
    let value = value.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let quote = value.as_bytes().first().copied();
    if matches!(quote, Some(b'\'' | b'"')) {
        let quote = quote.expect("matched quote must exist");
        let end = value.as_bytes()[1..]
            .iter()
            .position(|candidate| *candidate == quote)?
            + 1;
        let locator_value = &value[1..end];
        let remaining = &value[end + 1..];
        if locator_value.is_empty()
            || (!remaining.is_empty() && !remaining.as_bytes()[0].is_ascii_whitespace())
        {
            return None;
        }
        return Some((locator_value, remaining));
    }
    split_first_token(value)
}

fn parse_exact_option(options: &str) -> Option<bool> {
    let options = options.trim_matches(|value: char| value.is_ascii_whitespace());
    match options {
        "" => Some(false),
        "--exact" => Some(true),
        _ => None,
    }
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
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_snapshot_session_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: snapshot requires --interactive and an optional CSS selector";
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let (projection, rest) = split_first_token(rest).ok_or(ERROR)?;
    if !matches!(projection, "-i" | "--interactive") {
        return Err(ERROR);
    }
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    if rest.is_empty() {
        return Ok(SessionCommand::Page(PageCommand::SnapshotInteractive(None)));
    }
    let (option, rest) = split_first_token(rest).ok_or(ERROR)?;
    if !matches!(option, "-s" | "--selector") {
        return Err(ERROR);
    }
    let (selector, trailing) = split_locator_value(rest).ok_or(ERROR)?;
    if !trailing
        .trim_matches(|value: char| value.is_ascii_whitespace())
        .is_empty()
    {
        return Err(ERROR);
    }
    Ok(SessionCommand::Page(PageCommand::SnapshotInteractive(
        Some(selector),
    )))
}

fn parse_get_command(arguments: CommandArguments<'_>) -> Result<SessionCommand<'_>, &'static str> {
    match arguments {
        (Some("url"), None, None) => Ok(SessionCommand::Page(PageCommand::GetUrl)),
        (Some("title"), None, None) => Ok(SessionCommand::Page(PageCommand::GetTitle)),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_lifecycle_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    match (command, arguments) {
        ("help", (None, None, None)) => Ok(SessionCommand::Help),
        ("events", (None, None, None)) => Ok(SessionCommand::Events),
        ("exit", (None, None, None)) => Ok(SessionCommand::Exit),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_fill_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let (reference, value) = parse_target_and_value(
        rest,
        "browser.jr: fill requires a reference or selector and text",
    )?;
    Ok(SessionCommand::Element(ElementCommand::Fill(
        reference, value,
    )))
}

fn parse_select_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let (reference, value) =
        parse_target_and_value(rest, "browser.jr: select requires a reference and value")?;
    let values = parse_select_values(value)
        .ok_or("browser.jr: select quoted values must be complete and whitespace-separated")?;
    Ok(SessionCommand::Element(ElementCommand::Select(
        reference, values,
    )))
}

fn parse_select_values(value: &str) -> Option<SelectCommandValues<'_>> {
    if !matches!(value.as_bytes().first(), Some(b'\'' | b'"')) {
        return Some(SelectCommandValues::Single(value));
    }

    let mut values = Vec::new();
    let mut remaining = value;
    loop {
        remaining = remaining.trim_start_matches(|character: char| character.is_ascii_whitespace());
        if remaining.is_empty() {
            break;
        }
        let (value, rest) = split_quoted_select_value(remaining)?;
        values.push(value);
        remaining = rest;
    }
    (!values.is_empty()).then_some(SelectCommandValues::List(values))
}

fn split_quoted_select_value(value: &str) -> Option<(&str, &str)> {
    let quote = *value.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let end = value.as_bytes()[1..]
        .iter()
        .position(|candidate| *candidate == quote)?
        + 1;
    let remaining = &value[end + 1..];
    if !remaining.is_empty() && !remaining.as_bytes()[0].is_ascii_whitespace() {
        return None;
    }
    Some((&value[1..end], remaining))
}

fn parse_target_and_value<'a>(
    rest: &'a str,
    error: &'static str,
) -> Result<(&'a str, &'a str), &'static str> {
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let Some((target, remaining)) = split_locator_value(rest) else {
        return Err(error);
    };
    if remaining.is_empty() {
        return Err(error);
    }
    let value = remaining.trim_start_matches(|value: char| value.is_ascii_whitespace());
    Ok((target, value))
}

fn parse_target_command<'a>(rest: &'a str, error: &'static str) -> Result<&'a str, &'static str> {
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let (target, remaining) = split_locator_value(rest).ok_or(error)?;
    remaining
        .trim_matches(|value: char| value.is_ascii_whitespace())
        .is_empty()
        .then_some(target)
        .ok_or(error)
}

fn parse_target_and_token<'a>(
    rest: &'a str,
    error: &'static str,
) -> Result<(&'a str, &'a str), &'static str> {
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    let (target, remaining) = split_locator_value(rest).ok_or(error)?;
    let remaining = remaining.trim_matches(|value: char| value.is_ascii_whitespace());
    let (token, trailing) = split_first_token(remaining).ok_or(error)?;
    trailing
        .trim_matches(|value: char| value.is_ascii_whitespace())
        .is_empty()
        .then_some((target, token))
        .ok_or(error)
}

fn flush_streams(output: &mut impl Write, errors: &mut impl Write) -> std::io::Result<()> {
    output.flush()?;
    errors.flush()
}

#[cfg(test)]
mod tests {
    use super::{
        ElementCommand, ExitStatus, FindLocatorKind, FindRoleAction, PageCommand,
        SelectCommandValues, SessionCommand, build_direct_locator, parse_command, run_session,
    };
    use crate::Locator;
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
        assert!(output.contains("find label <label>"));
        assert!(output.contains("find testid <id>"));
        assert!(output.contains("find nth <index> <selector>"));
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
                SelectCommandValues::Single("large value")
            )))
        ));
        assert!(matches!(
            parse_command("select @e1 "),
            Ok(SessionCommand::Element(ElementCommand::Select(
                "@e1",
                SelectCommandValues::Single("")
            )))
        ));
        match parse_command(r#"select @e1 "a" "large value" """#) {
            Ok(SessionCommand::Element(ElementCommand::Select(
                "@e1",
                SelectCommandValues::List(values),
            ))) => assert_eq!(values, vec!["a", "large value", ""]),
            _ => panic!("unexpected select list parse result"),
        }
        assert!(parse_command(r#"select @e1 "a" trailing"#).is_err());
        assert!(parse_command(r#"select @e1 "a"#).is_err());
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
            parse_command("get html @e1"),
            Ok(SessionCommand::Element(ElementCommand::GetHtml("@e1")))
        ));
        assert!(matches!(
            parse_command("get count \"section > .card\""),
            Ok(SessionCommand::Element(ElementCommand::GetCount(
                "section > .card"
            )))
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
            parse_command("snapshot -i"),
            Ok(SessionCommand::Page(PageCommand::SnapshotInteractive(None)))
        ));
        assert!(matches!(
            parse_command("snapshot --interactive -s \"main > section\""),
            Ok(SessionCommand::Page(PageCommand::SnapshotInteractive(
                Some("main > section")
            )))
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
        assert!(parse_command("get html @e1 extra").is_err());
        assert!(parse_command("snapshot -s main").is_err());
        assert!(parse_command("snapshot -i -s").is_err());
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
    }

    #[test]
    fn semantic_and_position_locators_parse_values_and_actions() {
        assert!(matches!(
            parse_command("find text Save"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Text,
                value: "Save",
                exact: false,
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find text \"Save draft\" text --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Text,
                value: "Save draft",
                exact: true,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find label 'Email address' fill hello world --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Label,
                value: "Email address",
                exact: true,
                action: FindRoleAction::Fill("hello world"),
            }))
        ));
        assert!(matches!(
            parse_command("find placeholder Search fill query"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Placeholder,
                value: "Search",
                exact: false,
                action: FindRoleAction::Fill("query"),
            }))
        ));
        assert!(matches!(
            parse_command("find alt 'Product image' text --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Alt,
                value: "Product image",
                exact: true,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find title Issues text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Title,
                value: "Issues",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find testid save-card text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::TestId,
                value: "save-card",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find css \"form > input:first-child\" fill hello"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Css,
                value: "form > input:first-child",
                exact: false,
                action: FindRoleAction::Fill("hello"),
            }))
        ));
        assert!(matches!(
            parse_command("find xpath \"//section/button[2]\" text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::XPath,
                value: "//section/button[2]",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find first .card text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::First,
                value: ".card",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find first \"input[title='hello world']\" text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::First,
                value: "input[title='hello world']",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find last div.card text"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Last,
                value: "div.card",
                exact: false,
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find nth 2 '[data-kind=item]' hover"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Nth(2),
                value: "[data-kind=item]",
                exact: false,
                action: FindRoleAction::Hover,
            }))
        ));
        assert!(parse_command("find text Save changes").is_err());
        assert!(parse_command("find label \"\" fill value").is_err());
        assert!(parse_command("find placeholder Search --name Query").is_err());
        assert!(parse_command("find testid save --exact").is_err());
        assert!(parse_command("find first .card --exact").is_err());
        assert!(parse_command("find nth -1 .card text").is_err());
        assert!(parse_command("find nth nope .card text").is_err());
    }

    #[test]
    fn direct_selectors_parse_quoted_targets() {
        assert!(matches!(
            parse_command("click \"main > a.next\""),
            Ok(SessionCommand::Element(ElementCommand::Click(
                "main > a.next"
            )))
        ));
        assert!(matches!(
            parse_command("fill \"form > input\" hello world"),
            Ok(SessionCommand::Element(ElementCommand::Fill(
                "form > input",
                "hello world"
            )))
        ));
        assert!(matches!(
            parse_command("get text \"//section/button[2]\""),
            Ok(SessionCommand::Element(ElementCommand::GetText(
                "//section/button[2]"
            )))
        ));
        assert!(matches!(
            parse_command("get value \"form > input\""),
            Ok(SessionCommand::Element(ElementCommand::GetValue(
                "form > input"
            )))
        ));
        assert!(matches!(
            parse_command("get attr \"section > article\" data-kind"),
            Ok(SessionCommand::Element(ElementCommand::GetAttribute(
                "section > article",
                "data-kind"
            )))
        ));
        assert!(matches!(
            parse_command("check \"form > input\""),
            Ok(SessionCommand::Element(ElementCommand::Check(
                "form > input"
            )))
        ));
        assert!(matches!(
            parse_command("uncheck \"form > input\""),
            Ok(SessionCommand::Element(ElementCommand::Uncheck(
                "form > input"
            )))
        ));
        assert!(matches!(
            parse_command("is checked \"form > input\""),
            Ok(SessionCommand::Element(ElementCommand::IsChecked(
                "form > input"
            )))
        ));
        assert!(matches!(
            parse_command("is enabled \"form > button\""),
            Ok(SessionCommand::Element(ElementCommand::IsEnabled(
                "form > button"
            )))
        ));
        assert!(matches!(
            parse_command("is visible \"section > article\""),
            Ok(SessionCommand::Element(ElementCommand::IsVisible(
                "section > article"
            )))
        ));
        assert!(matches!(
            parse_command("select \"form > select\" large value"),
            Ok(SessionCommand::Element(ElementCommand::Select(
                "form > select",
                SelectCommandValues::Single("large value")
            )))
        ));
        assert!(matches!(
            build_direct_locator("css=main > button"),
            Ok(Locator::Css(_))
        ));
        assert!(matches!(
            build_direct_locator("xpath=//main/button"),
            Ok(Locator::XPath(_))
        ));
        assert!(matches!(
            build_direct_locator("//main/button"),
            Ok(Locator::XPath(_))
        ));
    }
}
