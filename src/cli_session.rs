use std::io::{BufRead, Write};

use crate::cli::{ExitStatus, combine_status, write_line, write_session_error};
use crate::cli_output::{
    SnapshotOutputOptions, format_accessibility_snapshot_node, format_snapshot_element,
};
use crate::locator::RoleFilterStates;
use crate::{
    AccessibilitySnapshot, AccessibilitySnapshotOptions, AltLocator, BoundingBox,
    CaptureAccessibilitySnapshot, CaptureAccessibilitySnapshotWithin, CaptureInteractiveSnapshot,
    CaptureInteractiveSnapshotWithin, CaptureTarget, ClickByLocator, ClickByLocatorResult,
    ClickElement, ClickResult, CountByLocator, CssLocator, FillByLocator, FillElement,
    FindByLocator, FocusByLocator, FocusElement, GetAttributeByLocator, GetBoundingBoxByLocator,
    GetCheckedByLocator, GetEditableByLocator, GetElementAttribute, GetElementBoundingBox,
    GetElementChecked, GetElementEditable, GetElementEnabled, GetElementFocused, GetElementHovered,
    GetElementHtml, GetElementText, GetElementValue, GetElementVisible, GetEnabledByLocator,
    GetFocusedByLocator, GetHoveredByLocator, GetHtmlByLocator, GetPageText, GetPageTitle,
    GetPageUrl, GetValueByLocator, GetViewportSize, GetVisibleByLocator, GoBack, GoForward,
    HistoryNavigationResult, HoverByLocator, HoverByLocatorResult, HoverElement,
    InteractiveElementRef, InteractiveSnapshot, KeyDown, KeyDownResult, KeyUp, KeyUpResult,
    KeyboardEventKey, KeyboardInsertText, KeyboardKey, KeyboardTextEffect, KeyboardTextResult,
    KeyboardType, LabelLocator, Locator, NonEmpty, OnDemandRasterProcess, OpenPage,
    PlaceholderLocator, PrepareScreenshot, PressByLocator, PressByLocatorResult, PressEffect,
    PressKey, PressResult, ReloadPage, RoleLocator, ScrollDirection, ScrollElementIntoView,
    ScrollIntoViewByLocator, ScrollPage, SelectOptionTarget, SelectOptions, SelectOptionsByLocator,
    SelectOptionsByLocatorResult, SelectOptionsResult, Session, SetCheckedByLocator,
    SetElementChecked, SetViewportSize, SoftwareRasterProcessFactory, TakeDomEvents, TestIdLocator,
    TextLocator, TitleLocator, TypeByLocator, TypeElement, XPathLocator, encode_png,
};

const SESSION_HELP: &str = "session commands:
  open <url>
  read [url]
  back
  forward
  reload
  set viewport <width> <height>
  scroll <up|down|left|right> [pixels]
  snapshot [--interactive] [-u|--urls] [-c|--compact] [-d|--depth <n>] [-s|--selector <css>]
  screenshot [path.png]
  screenshot --full [path.png]
  screenshot <selector> <path.png>
  find role <role> [action] [role-options]
    actions: click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text
    role-options: --name <name> --description <description> --exact
                  --checked <bool> --disabled <bool> --expanded <bool>
                  --include-hidden --level <n>
                  --pressed <bool> --selected <bool>
  find text <text> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text] [--exact]
  find label <label> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text] [--exact]
  find placeholder <text> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text] [--exact]
  find alt <text> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text] [--exact]
  find title <text> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text] [--exact]
  find testid <id> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  find css <selector> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  find xpath <expression> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  find first <selector> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  find last <selector> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  find nth <index> <selector> [click|fill <text>|focus|focused|hover|hovered|press <key>|check|uncheck|scroll|text]
  click <ref|selector>
  fill <ref|selector> <text>
  type <ref|selector> <text>
  keyboard inserttext <text>
  keyboard type <text>
  keydown <key>
  keyup <key>
  focus <ref|selector>
  hover <ref|selector>
  scrollintoview <ref|selector>
  scrollinto <ref|selector>
  press <key>
  select <ref|selector> <value>
  select <ref|selector> \"<value>\" [\"<value>\" ...]
  check <ref|selector>
  uncheck <ref|selector>
  is checked <ref|selector>
  is editable <ref|selector>
  is enabled <ref|selector>
  is focused <ref|selector>
  is hovered <ref|selector>
  is visible <ref|selector>
  get attr <ref|selector> <name>
  get box <ref|selector>
  get count <selector>
  get html <ref|selector>
  get text <ref|selector>
  get value <ref|selector>
  get url
  get title
  get viewport
  events
  help
  exit
";

#[derive(Debug)]
pub(crate) struct CliSession {
    engine: Session,
    current_references: Vec<Option<InteractiveElementRef>>,
    raster: OnDemandRasterProcess<SoftwareRasterProcessFactory>,
    next_screenshot_id: u64,
}

impl CliSession {
    pub(crate) fn new() -> Self {
        Self {
            engine: Session::new(),
            current_references: Vec::new(),
            raster: OnDemandRasterProcess::new(SoftwareRasterProcessFactory),
            next_screenshot_id: 1,
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
            SessionCommand::Keyboard(command) => self.run_keyboard_command(command, output, errors),
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
                        "event type={} document={} target={:?} ordinal={} bubbles={} composed={} path={:?}",
                        event.event_type,
                        event.document_epoch,
                        event.target,
                        event.target_ordinal,
                        event.bubbles,
                        event.composed,
                        event.path.join(" > ")
                    ),
                    ExitStatus::Success,
                ),
            );
        }
        SessionStep::Continue(status)
    }

    fn run_keyboard_command(
        &mut self,
        command: KeyboardCommand<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match command {
            KeyboardCommand::InsertText(text) => self.run_keyboard_text(
                "inserttext",
                text,
                KeyboardInsertText { text: text.into() },
                output,
                errors,
            ),
            KeyboardCommand::Type(text) => self.run_keyboard_text(
                "type",
                text,
                KeyboardType { text: text.into() },
                output,
                errors,
            ),
            KeyboardCommand::Down(key) => self.key_down(key, output, errors),
            KeyboardCommand::Up(key) => self.key_up(key, output, errors),
        }
    }

    fn run_keyboard_text<R>(
        &mut self,
        operation: &str,
        text: &str,
        request: R,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep
    where
        R: crate::SessionRequest<Reply = KeyboardTextResult>,
    {
        let result = self.engine.execute(request);
        match result {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format_keyboard_text_result(operation, text, &result),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn key_down(
        &mut self,
        key: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let key = match parse_keyboard_event_key(key, errors) {
            Ok(key) => key,
            Err(step) => return step,
        };
        match self.engine.execute(KeyDown { key }) {
            Ok(result) => {
                if result
                    .press
                    .as_ref()
                    .is_some_and(|press| press.navigated().is_some())
                {
                    self.current_references.clear();
                }
                SessionStep::Continue(write_line(
                    output,
                    &format_key_down_result(&result),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn key_up(
        &mut self,
        key: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let key = match parse_keyboard_event_key(key, errors) {
            Ok(key) => key,
            Err(step) => return step,
        };
        match self.engine.execute(KeyUp { key }) {
            Ok(result) => {
                if result
                    .press
                    .as_ref()
                    .is_some_and(|press| press.navigated().is_some())
                {
                    self.current_references.clear();
                }
                SessionStep::Continue(write_line(
                    output,
                    &format_key_up_result(&result),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
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
            PageCommand::Read(url) => self.read(url, output, errors),
            PageCommand::Back => self.back(output, errors),
            PageCommand::Forward => self.forward(output, errors),
            PageCommand::Reload => self.reload(output, errors),
            PageCommand::SetViewport(width, height) => {
                self.set_viewport(width, height, output, errors)
            }
            PageCommand::Scroll(direction, distance) => {
                self.scroll_page(direction, distance, output, errors)
            }
            PageCommand::Snapshot(options) => self.snapshot(options, output, errors),
            PageCommand::Screenshot {
                selector,
                path,
                full_page,
            } => self.screenshot(selector, path, full_page, output, errors),
            PageCommand::GetUrl => self.get_url(output, errors),
            PageCommand::GetTitle => self.get_title(output, errors),
            PageCommand::GetViewport => self.get_viewport(output, errors),
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
                options,
                action,
            } => self.find_role(role, options, action, output, errors),
            ElementCommand::FindLocator {
                kind,
                value,
                exact,
                action,
            } => self.find_typed_locator(kind, value, exact, action, output, errors),
            ElementCommand::Fill(reference, value) => self.fill(reference, value, output, errors),
            ElementCommand::Type(reference, text) => {
                self.type_text(reference, text, output, errors)
            }
            ElementCommand::Focus(reference) => self.focus(reference, output, errors),
            ElementCommand::Hover(reference) => self.hover(reference, output, errors),
            ElementCommand::ScrollIntoView(reference) => {
                self.scroll_into_view(reference, output, errors)
            }
            ElementCommand::Press(key) => self.press(key, output, errors),
            ElementCommand::Select(reference, values) => {
                self.select(reference, values, output, errors)
            }
            ElementCommand::Check(reference) => self.set_checked(reference, true, output, errors),
            ElementCommand::Uncheck(reference) => {
                self.set_checked(reference, false, output, errors)
            }
            ElementCommand::IsChecked(reference) => self.is_checked(reference, output, errors),
            ElementCommand::IsEditable(reference) => self.is_editable(reference, output, errors),
            ElementCommand::IsEnabled(reference) => self.is_enabled(reference, output, errors),
            ElementCommand::IsFocused(reference) => self.is_focused(reference, output, errors),
            ElementCommand::IsHovered(reference) => self.is_hovered(reference, output, errors),
            ElementCommand::IsVisible(reference) => self.is_visible(reference, output, errors),
            ElementCommand::GetAttribute(reference, name) => {
                self.get_attribute(reference, name, output, errors)
            }
            ElementCommand::GetBoundingBox(reference) => {
                self.get_bounding_box(reference, output, errors)
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

    fn read(
        &mut self,
        url: Option<&str>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        if let Some(url) = url {
            if let Err(error) = self.engine.execute(OpenPage { url: url.into() }) {
                return SessionStep::Continue(write_session_error(errors, error));
            }
            self.current_references.clear();
        }
        match self.engine.execute(GetPageText) {
            Ok(page) => SessionStep::Continue(write_line(output, &page.text, ExitStatus::Success)),
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

    fn set_viewport(
        &mut self,
        width: u64,
        height: u64,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match self.engine.execute(SetViewportSize { width, height }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "viewport width={} height={} resized={} scroll-x={} scroll-y={} moved={}",
                    result.viewport.width,
                    result.viewport.height,
                    result.resized,
                    result.scroll.x,
                    result.scroll.y,
                    result.scroll.moved
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn scroll_page(
        &mut self,
        direction: ScrollDirection,
        distance: u64,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match self.engine.execute(ScrollPage {
            direction,
            distance,
        }) {
            Ok(scroll) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "scrolled x={} y={} moved={}",
                    scroll.x, scroll.y, scroll.moved
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn back(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        let result = self.engine.execute(GoBack);
        self.finish_history_navigation("back", result, output, errors)
    }

    fn forward(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        let result = self.engine.execute(GoForward);
        self.finish_history_navigation("forward", result, output, errors)
    }

    fn finish_history_navigation(
        &mut self,
        command: &str,
        result: Result<HistoryNavigationResult, crate::SessionError>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        match result {
            Ok(HistoryNavigationResult::Navigated(page)) => {
                self.current_references.clear();
                SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "{command} url={} elements={} navigated=true",
                        page.url, page.interactive_element_count
                    ),
                    ExitStatus::Success,
                ))
            }
            Ok(HistoryNavigationResult::NoEntry { current_url }) => {
                SessionStep::Continue(write_line(
                    output,
                    &format!("{command} url={current_url} navigated=false"),
                    ExitStatus::Success,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn snapshot(
        &mut self,
        options: SnapshotSessionOptions<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match options.selector.map(CssLocator::new).transpose() {
            Ok(locator) => locator.map(Locator::from),
            Err(error) => return invalid_locator(errors, error.to_string()),
        };
        match options.projection {
            SnapshotSessionProjection::Interactive => {
                self.snapshot_interactive(locator, options.output, output, errors)
            }
            SnapshotSessionProjection::Full(snapshot_options) => self.snapshot_accessibility(
                AccessibilitySnapshotRequest {
                    locator,
                    snapshot_options,
                    output_options: options.output,
                },
                output,
                errors,
            ),
        }
    }

    fn snapshot_interactive(
        &mut self,
        locator: Option<Locator>,
        options: SnapshotOutputOptions,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let snapshot = match locator {
            Some(locator) => self
                .engine
                .execute(CaptureInteractiveSnapshotWithin { locator }),
            None => self.engine.execute(CaptureInteractiveSnapshot),
        };
        match snapshot {
            Ok(snapshot) => {
                self.current_references = references_from_interactive_snapshot(&snapshot);
                SessionStep::Continue(write_interactive_snapshot(output, &snapshot, options))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn snapshot_accessibility(
        &mut self,
        request: AccessibilitySnapshotRequest,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let snapshot = match request.locator {
            Some(locator) => self.engine.execute(CaptureAccessibilitySnapshotWithin {
                locator,
                options: request.snapshot_options,
            }),
            None => self.engine.execute(CaptureAccessibilitySnapshot {
                options: request.snapshot_options,
            }),
        };
        match snapshot {
            Ok(snapshot) => {
                self.current_references = references_from_accessibility_snapshot(&snapshot);
                SessionStep::Continue(write_accessibility_snapshot(
                    output,
                    &snapshot,
                    request.output_options,
                ))
            }
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn find_role(
        &mut self,
        role: &str,
        options: FindRoleOptions<'_>,
        action: FindRoleAction<'_>,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let locator = match options.build_locator(role) {
            Ok(locator) => locator,
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
                Ok(ClickByLocatorResult::Activated { matched }) => {
                    SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "clicked role={:?} name={:?} element={:?} focused=true",
                            matched.role.as_deref().unwrap_or(""),
                            matched.name,
                            matched.element
                        ),
                        ExitStatus::Success,
                    ))
                }
                Ok(ClickByLocatorResult::Checked { matched, checked }) => {
                    SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "clicked role={:?} name={:?} element={:?} focused=true checked={checked}",
                            matched.role.as_deref().unwrap_or(""),
                            matched.name,
                            matched.element
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
            FindRoleAction::Focus => match self.engine.execute(FocusByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "focused role={:?} name={:?} element={:?}",
                        result.matched.role.as_deref().unwrap_or(""),
                        result.matched.name,
                        result.matched.element,
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Focused => match self.engine.execute(GetFocusedByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "focused role={:?} name={:?} element={:?} value={}",
                        result.matched.role.as_deref().unwrap_or(""),
                        result.matched.name,
                        result.matched.element,
                        result.focused
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            },
            FindRoleAction::Press(key) => {
                let key = match KeyboardKey::new(key) {
                    Ok(key) => key,
                    Err(error) => {
                        return SessionStep::Continue(write_line(
                            errors,
                            &format!("browser.jr: {error}"),
                            ExitStatus::InvalidInput,
                        ));
                    }
                };
                match self.engine.execute(PressByLocator { locator, key }) {
                    Ok(result) => {
                        if result.press.navigated().is_some() {
                            self.current_references.clear();
                        }
                        SessionStep::Continue(write_line(
                            output,
                            &format_locator_press_result(&result),
                            ExitStatus::Success,
                        ))
                    }
                    Err(error) => SessionStep::Continue(write_session_error(errors, error)),
                }
            }
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
            FindRoleAction::ScrollIntoView => {
                match self.engine.execute(ScrollIntoViewByLocator { locator }) {
                    Ok(result) => SessionStep::Continue(write_line(
                        output,
                        &format!(
                            "scrolled into view role={:?} name={:?} element={:?} x={} y={} moved={}",
                            result.matched.role.as_deref().unwrap_or(""),
                            result.matched.name,
                            result.matched.element,
                            result.scroll.x,
                            result.scroll.y,
                            result.scroll.moved
                        ),
                        ExitStatus::Success,
                    )),
                    Err(error) => SessionStep::Continue(write_session_error(errors, error)),
                }
            }
            FindRoleAction::Hovered => match self.engine.execute(GetHoveredByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "hovered role={:?} name={:?} element={:?} value={}",
                        result.matched.role.as_deref().unwrap_or(""),
                        result.matched.name,
                        result.matched.element,
                        result.hovered
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
            Ok(ClickResult::Activated { reference }) => SessionStep::Continue(write_line(
                output,
                &format!("clicked ref={reference} focused=true"),
                ExitStatus::Success,
            )),
            Ok(ClickResult::Checked { reference, checked }) => SessionStep::Continue(write_line(
                output,
                &format!("clicked ref={reference} focused=true checked={checked}"),
                ExitStatus::Success,
            )),
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

    fn type_text(
        &mut self,
        target: &str,
        text: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let characters = text.chars().count();
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            let locator = match build_direct_locator(target) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            return match self.engine.execute(TypeByLocator {
                locator,
                text: text.into(),
            }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "typed role={:?} name={:?} element={:?} characters={characters}",
                        result.matched.role.as_deref().unwrap_or(""),
                        result.matched.name,
                        result.matched.element,
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(TypeElement {
            reference,
            text: text.into(),
        }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("typed ref={} characters={characters}", result.reference),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn focus(
        &mut self,
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            return self.run_direct_locator(target, FindRoleAction::Focus, output, errors);
        };
        match self.engine.execute(FocusElement { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "focused ref={} element={:?}",
                    result.reference, result.element
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn hover(
        &mut self,
        target: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let Some(reference) = self.resolve_reference(target) else {
            if target.starts_with('@') {
                return unknown_reference(errors, target);
            }
            return self.run_direct_locator(target, FindRoleAction::Hover, output, errors);
        };
        match self.engine.execute(HoverElement { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("hovered ref={}", result.reference),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn scroll_into_view(
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
            return match self.engine.execute(ScrollIntoViewByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format!(
                        "scrolled into view element={:?} x={} y={} moved={}",
                        result.matched.element,
                        result.scroll.x,
                        result.scroll.y,
                        result.scroll.moved
                    ),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(ScrollElementIntoView { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "scrolled into view ref={} x={} y={} moved={}",
                    result.reference, result.scroll.x, result.scroll.y, result.scroll.moved
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn press(
        &mut self,
        key: &str,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let key = match KeyboardKey::new(key) {
            Ok(key) => key,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: {error}"),
                    ExitStatus::InvalidInput,
                ));
            }
        };
        match self.engine.execute(PressKey { key }) {
            Ok(result) => {
                if result.navigated().is_some() {
                    self.current_references.clear();
                }
                SessionStep::Continue(write_line(
                    output,
                    &format_press_result(&result),
                    ExitStatus::Success,
                ))
            }
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

    fn get_bounding_box(
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
            return match self.engine.execute(GetBoundingBoxByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &format_bounding_box(result.value),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(GetElementBoundingBox { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format_bounding_box(result.value),
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

    fn is_editable(
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
            return match self.engine.execute(GetEditableByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.editable.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(GetElementEditable { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "editable ref={} value={}",
                    result.reference, result.editable
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn is_focused(
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
            return match self.engine.execute(GetFocusedByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.focused.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(GetElementFocused { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("focused ref={} value={}", result.reference, result.focused),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn is_hovered(
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
            return match self.engine.execute(GetHoveredByLocator { locator }) {
                Ok(result) => SessionStep::Continue(write_line(
                    output,
                    &result.hovered.to_string(),
                    ExitStatus::Success,
                )),
                Err(error) => SessionStep::Continue(write_session_error(errors, error)),
            };
        };
        match self.engine.execute(GetElementHovered { reference }) {
            Ok(result) => SessionStep::Continue(write_line(
                output,
                &format!("hovered ref={} value={}", result.reference, result.hovered),
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

    fn get_viewport(&mut self, output: &mut impl Write, errors: &mut impl Write) -> SessionStep {
        match self.engine.execute(GetViewportSize) {
            Ok(viewport) => SessionStep::Continue(write_line(
                output,
                &format!(
                    "viewport width={} height={}",
                    viewport.width, viewport.height
                ),
                ExitStatus::Success,
            )),
            Err(error) => SessionStep::Continue(write_session_error(errors, error)),
        }
    }

    fn screenshot(
        &mut self,
        selector: Option<&str>,
        path: Option<&str>,
        full_page: bool,
        output: &mut impl Write,
        errors: &mut impl Write,
    ) -> SessionStep {
        let path = path.map_or_else(|| self.next_screenshot_path(), std::path::PathBuf::from);
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("png"))
        {
            return SessionStep::Continue(write_line(
                errors,
                "browser.jr: screenshot path must end in .png",
                ExitStatus::InvalidInput,
            ));
        }
        let target = if full_page {
            CaptureTarget::FullPage
        } else if let Some(selector) = selector {
            let locator = match build_direct_locator(selector) {
                Ok(locator) => locator,
                Err(error) => return invalid_locator(errors, error),
            };
            CaptureTarget::Element(locator)
        } else {
            CaptureTarget::Viewport
        };
        let prepared = match self.engine.execute(PrepareScreenshot { target }) {
            Ok(prepared) => prepared,
            Err(error) => return SessionStep::Continue(write_session_error(errors, error)),
        };
        let image = match self.raster.render(&prepared) {
            Ok(image) => image,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: screenshot raster failed: {error}"),
                    ExitStatus::Unavailable,
                ));
            }
        };
        let bytes = match encode_png(&image) {
            Ok(bytes) => bytes,
            Err(error) => {
                return SessionStep::Continue(write_line(
                    errors,
                    &format!("browser.jr: {error}"),
                    ExitStatus::Unavailable,
                ));
            }
        };
        if let Err(error) = std::fs::write(&path, bytes) {
            return SessionStep::Continue(write_line(
                errors,
                &format!("browser.jr: cannot write screenshot to {:?}: {error}", path),
                ExitStatus::Unavailable,
            ));
        }
        SessionStep::Continue(write_line(
            output,
            &format!(
                "screenshot path={:?} width={} height={}",
                path,
                image.width(),
                image.height()
            ),
            ExitStatus::Success,
        ))
    }

    fn next_screenshot_path(&mut self) -> std::path::PathBuf {
        let id = self.next_screenshot_id;
        self.next_screenshot_id = self
            .next_screenshot_id
            .checked_add(1)
            .expect("screenshot identifier exhausted");
        std::env::temp_dir().join(format!("browser-jr-{}-{id}.png", std::process::id()))
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

    pub(crate) fn run_line(
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
        let reference = *self
            .current_references
            .get(ordinal.checked_sub(1)?)?
            .as_ref()?;
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

fn parse_keyboard_event_key(
    key: &str,
    errors: &mut impl Write,
) -> Result<KeyboardEventKey, SessionStep> {
    KeyboardEventKey::new(key).map_err(|error| {
        SessionStep::Continue(write_line(
            errors,
            &format!("browser.jr: {error}"),
            ExitStatus::InvalidInput,
        ))
    })
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
    Keyboard(KeyboardCommand<'a>),
    Events,
    Help,
    Exit,
    Empty,
}

enum KeyboardCommand<'a> {
    Down(&'a str),
    InsertText(&'a str),
    Type(&'a str),
    Up(&'a str),
}

enum PageCommand<'a> {
    Open(&'a str),
    Read(Option<&'a str>),
    Back,
    Forward,
    Reload,
    SetViewport(u64, u64),
    Scroll(ScrollDirection, u64),
    Snapshot(SnapshotSessionOptions<'a>),
    Screenshot {
        selector: Option<&'a str>,
        path: Option<&'a str>,
        full_page: bool,
    },
    GetUrl,
    GetTitle,
    GetViewport,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SnapshotSessionOptions<'a> {
    selector: Option<&'a str>,
    projection: SnapshotSessionProjection,
    output: SnapshotOutputOptions,
}

struct AccessibilitySnapshotRequest {
    locator: Option<Locator>,
    snapshot_options: AccessibilitySnapshotOptions,
    output_options: SnapshotOutputOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotSessionProjection {
    Full(AccessibilitySnapshotOptions),
    Interactive,
}

impl Default for SnapshotSessionProjection {
    fn default() -> Self {
        Self::Full(AccessibilitySnapshotOptions::default())
    }
}

enum ElementCommand<'a> {
    FindRole {
        role: &'a str,
        options: FindRoleOptions<'a>,
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
    Type(&'a str, &'a str),
    Focus(&'a str),
    Hover(&'a str),
    ScrollIntoView(&'a str),
    Press(&'a str),
    Select(&'a str, SelectCommandValues<'a>),
    Check(&'a str),
    Uncheck(&'a str),
    IsChecked(&'a str),
    IsEditable(&'a str),
    IsEnabled(&'a str),
    IsFocused(&'a str),
    IsHovered(&'a str),
    IsVisible(&'a str),
    GetAttribute(&'a str, &'a str),
    GetBoundingBox(&'a str),
    GetCount(&'a str),
    GetHtml(&'a str),
    GetText(&'a str),
    GetValue(&'a str),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FindRoleOptions<'a> {
    name: Option<&'a str>,
    description: Option<&'a str>,
    exact: bool,
    filters: FindRoleFilters,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FindRoleFilters {
    states: RoleFilterStates,
    include_hidden: bool,
    level: Option<u32>,
}

impl FindRoleOptions<'_> {
    fn build_locator(self, role: &str) -> Result<RoleLocator, crate::RoleLocatorError> {
        let mut locator = RoleLocator::new(role)?;
        locator = match self.name {
            Some(name) if self.exact => locator.with_exact_name(name),
            Some(name) => locator.with_name(name),
            None => locator,
        };
        locator = match self.description {
            Some(description) if self.exact => locator.with_exact_description(description),
            Some(description) => locator.with_description(description),
            None => locator,
        };
        let filters = self.filters;
        let states = filters.states;
        if let Some(value) = states.checked {
            locator = locator.with_checked(value)?;
        }
        if let Some(value) = states.disabled {
            locator = locator.with_disabled(value);
        }
        if let Some(value) = states.expanded {
            locator = locator.with_expanded(value)?;
        }
        locator = locator.with_include_hidden(filters.include_hidden);
        if let Some(value) = filters.level {
            locator = locator.with_level(value)?;
        }
        if let Some(value) = states.pressed {
            locator = locator.with_pressed(value)?;
        }
        if let Some(value) = states.selected {
            locator = locator.with_selected(value)?;
        }
        Ok(locator)
    }
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
    Focus,
    Focused,
    Press(&'a str),
    Check,
    Uncheck,
    Hover,
    Hovered,
    ScrollIntoView,
    Text,
}

pub(crate) enum SessionStep {
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
    options: SnapshotOutputOptions,
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
        let reference = element.reference.to_string();
        let written = writeln!(
            output,
            "{}",
            format_snapshot_element(element, &reference, options)
        );
        if written.is_err() {
            return ExitStatus::Unavailable;
        }
    }
    ExitStatus::Success
}

pub(crate) fn write_accessibility_snapshot(
    output: &mut impl Write,
    snapshot: &AccessibilitySnapshot,
    options: SnapshotOutputOptions,
) -> ExitStatus {
    if writeln!(
        output,
        "snapshot={} url={} mode=full nodes={}",
        snapshot.id.get(),
        snapshot.url,
        snapshot.nodes.len()
    )
    .is_err()
    {
        return ExitStatus::Unavailable;
    }
    for node in &snapshot.nodes {
        if writeln!(
            output,
            "{}",
            format_accessibility_snapshot_node(node, options)
        )
        .is_err()
        {
            return ExitStatus::Unavailable;
        }
    }
    ExitStatus::Success
}

fn insert_reference(
    mut references: Vec<Option<InteractiveElementRef>>,
    reference: InteractiveElementRef,
) -> Vec<Option<InteractiveElementRef>> {
    let index =
        usize::try_from(reference.ordinal() - 1).expect("snapshot reference ordinal fits usize");
    if references.len() <= index {
        references.resize(index + 1, None);
    }
    references[index] = Some(reference);
    references
}

fn references_from_interactive_snapshot(
    snapshot: &InteractiveSnapshot,
) -> Vec<Option<InteractiveElementRef>> {
    snapshot
        .elements
        .iter()
        .fold(Vec::new(), |references, element| {
            insert_reference(references, element.reference)
        })
}

fn references_from_accessibility_snapshot(
    snapshot: &AccessibilitySnapshot,
) -> Vec<Option<InteractiveElementRef>> {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| node.reference)
        .fold(Vec::new(), insert_reference)
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

fn format_bounding_box(value: Option<BoundingBox>) -> String {
    value.map_or_else(
        || "null".into(),
        |value| {
            format!(
                "x:      {}\ny:      {}\nwidth:  {}\nheight: {}",
                value.x, value.y, value.width, value.height
            )
        },
    )
}

fn format_press_result(result: &PressResult) -> String {
    match &result.effect {
        PressEffect::Text(effect) => format!(
            "pressed key={:?} element={:?} characters={} selection={}:{} changed={}",
            result.key.to_string(),
            effect.element.element,
            effect.value.chars().count(),
            effect.selection.start(),
            effect.selection.end(),
            effect.changed
        ),
        PressEffect::FocusTraversal(effect) => format!(
            "pressed key={:?} {} previous={:?}",
            result.key.to_string(),
            format_focus_destination(effect.current.as_ref()),
            effect
                .previous
                .as_ref()
                .map(|element| element.element.as_str())
                .unwrap_or("body")
        ),
        PressEffect::Navigated(effect) => format!(
            "pressed key={:?} element={:?} url={} elements={}",
            result.key.to_string(),
            effect.element.element,
            effect.url,
            effect.interactive_element_count
        ),
        PressEffect::Ignored { element } => format!(
            "pressed key={:?} element={:?} ignored=true",
            result.key.to_string(),
            element.element
        ),
        PressEffect::Activated { element } => format!(
            "pressed key={:?} element={:?} activated=true",
            result.key.to_string(),
            element.element
        ),
        PressEffect::Checked { element, checked } => format!(
            "pressed key={:?} element={:?} checked={checked}",
            result.key.to_string(),
            element.element
        ),
    }
}

fn format_keyboard_text_result(
    operation: &str,
    input: &str,
    result: &KeyboardTextResult,
) -> String {
    let characters = input.chars().count();
    match &result.effect {
        KeyboardTextEffect::Text(effect) => format!(
            "keyboard {operation} element={:?} characters={characters} value_characters={} selection={}:{} changed={}",
            effect.element.element,
            effect.value.chars().count(),
            effect.selection.start(),
            effect.selection.end(),
            effect.changed
        ),
        KeyboardTextEffect::Ignored { element } => format!(
            "keyboard {operation} element={:?} characters={characters} changed=false",
            element
                .as_ref()
                .map_or("body", |element| element.element.as_str())
        ),
    }
}

fn format_key_down_result(result: &KeyDownResult) -> String {
    let state = format!(
        "keydown key={:?} repeat={}",
        result.key.to_string(),
        result.repeat
    );
    if result.deferred {
        format!("{state} deferred=true")
    } else {
        result.press.as_ref().map_or_else(
            || format!("{state} modifier=true"),
            |press| format!("{state} {}", format_press_result(press)),
        )
    }
}

fn format_key_up_result(result: &KeyUpResult) -> String {
    let state = format!(
        "keyup key={:?} was-pressed={}",
        result.key.to_string(),
        result.was_pressed
    );
    result.press.as_ref().map_or(state.clone(), |press| {
        format!("{state} {}", format_press_result(press))
    })
}

fn format_locator_press_result(result: &PressByLocatorResult) -> String {
    let identity = format!(
        "role={:?} name={:?} element={:?}",
        result.matched.role.as_deref().unwrap_or(""),
        result.matched.name,
        result.matched.element
    );
    match &result.press.effect {
        PressEffect::Text(effect) => format!(
            "pressed {identity} key={:?} characters={} selection={}:{} changed={}",
            result.press.key.to_string(),
            effect.value.chars().count(),
            effect.selection.start(),
            effect.selection.end(),
            effect.changed
        ),
        PressEffect::FocusTraversal(effect) => format!(
            "pressed {identity} key={:?} {} previous={:?}",
            result.press.key.to_string(),
            format_focus_destination(effect.current.as_ref()),
            effect
                .previous
                .as_ref()
                .map(|element| element.element.as_str())
                .unwrap_or("body")
        ),
        PressEffect::Navigated(effect) => format!(
            "pressed {identity} key={:?} url={} elements={}",
            result.press.key.to_string(),
            effect.url,
            effect.interactive_element_count
        ),
        PressEffect::Ignored { .. } => format!(
            "pressed {identity} key={:?} ignored=true",
            result.press.key.to_string()
        ),
        PressEffect::Activated { .. } => format!(
            "pressed {identity} key={:?} activated=true",
            result.press.key.to_string()
        ),
        PressEffect::Checked { element, checked } => format!(
            "pressed {identity} key={:?} checked={checked} {}",
            result.press.key.to_string(),
            format_focus_destination(Some(element))
        ),
    }
}

fn format_focus_destination(element: Option<&crate::FocusedElement>) -> String {
    element.map_or_else(
        || "focus=\"body\"".into(),
        |element| {
            format!(
                "focus={:?} focus-role={:?} focus-name={:?}",
                element.element, element.role, element.name
            )
        },
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
    if let Some(rest) = command_rest(line, "set viewport") {
        return parse_set_viewport_command(rest);
    }
    if let Some(rest) =
        command_rest(line, "scrollintoview").or_else(|| command_rest(line, "scrollinto"))
    {
        return parse_target_command(
            rest,
            "browser.jr: scrollintoview requires a reference or selector",
        )
        .map(|target| SessionCommand::Element(ElementCommand::ScrollIntoView(target)));
    }
    if let Some(rest) = command_rest(line, "scroll") {
        return parse_scroll_command(rest);
    }
    if let Some(rest) = command_rest(line, "find") {
        return parse_find_command(rest);
    }
    if let Some(rest) = command_rest(line, "snapshot") {
        return parse_snapshot_session_command(rest);
    }
    if let Some(rest) = command_rest(line, "screenshot") {
        return parse_screenshot_session_command(rest);
    }
    if let Some(rest) = command_rest(line, "fill") {
        return parse_fill_command(rest);
    }
    if let Some(rest) = command_rest(line, "type") {
        return parse_type_command(rest);
    }
    if let Some(rest) = command_rest(line, "keyboard") {
        return parse_keyboard_command(rest);
    }
    if let Some(rest) = command_rest(line, "keydown") {
        return parse_keyboard_state_command(rest, true);
    }
    if let Some(rest) = command_rest(line, "keyup") {
        return parse_keyboard_state_command(rest, false);
    }
    if let Some(rest) = command_rest(line, "press") {
        return parse_press_command(rest);
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
        Some(command @ ("open" | "read" | "back" | "forward" | "reload")) => {
            parse_page_command(command, arguments)
        }
        Some("get") => parse_get_command(arguments),
        Some(command @ ("events" | "help" | "exit")) => parse_lifecycle_command(command, arguments),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

#[derive(Clone, Copy)]
enum SimpleTargetCommand {
    Click,
    Focus,
    Hover,
    GetText,
    GetBoundingBox,
    GetCount,
    GetHtml,
    GetValue,
    Check,
    Uncheck,
    IsChecked,
    IsEditable,
    IsEnabled,
    IsFocused,
    IsHovered,
    IsVisible,
}

impl SimpleTargetCommand {
    fn build(self, target: &str) -> SessionCommand<'_> {
        let command = match self {
            Self::Click => ElementCommand::Click(target),
            Self::Focus => ElementCommand::Focus(target),
            Self::Hover => ElementCommand::Hover(target),
            Self::GetText => ElementCommand::GetText(target),
            Self::GetBoundingBox => ElementCommand::GetBoundingBox(target),
            Self::GetCount => ElementCommand::GetCount(target),
            Self::GetHtml => ElementCommand::GetHtml(target),
            Self::GetValue => ElementCommand::GetValue(target),
            Self::Check => ElementCommand::Check(target),
            Self::Uncheck => ElementCommand::Uncheck(target),
            Self::IsChecked => ElementCommand::IsChecked(target),
            Self::IsEditable => ElementCommand::IsEditable(target),
            Self::IsEnabled => ElementCommand::IsEnabled(target),
            Self::IsFocused => ElementCommand::IsFocused(target),
            Self::IsHovered => ElementCommand::IsHovered(target),
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
            "focus",
            "browser.jr: focus requires a reference or selector",
            SimpleTargetCommand::Focus,
        ),
        (
            "hover",
            "browser.jr: hover requires a reference or selector",
            SimpleTargetCommand::Hover,
        ),
        (
            "get text",
            "browser.jr: get text requires a reference or selector",
            SimpleTargetCommand::GetText,
        ),
        (
            "get box",
            "browser.jr: get box requires a reference or selector",
            SimpleTargetCommand::GetBoundingBox,
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
            "is editable",
            "browser.jr: is editable requires a reference or selector",
            SimpleTargetCommand::IsEditable,
        ),
        (
            "is enabled",
            "browser.jr: is enabled requires a reference or selector",
            SimpleTargetCommand::IsEnabled,
        ),
        (
            "is focused",
            "browser.jr: is focused requires a reference or selector",
            SimpleTargetCommand::IsFocused,
        ),
        (
            "is hovered",
            "browser.jr: is hovered requires a reference or selector",
            SimpleTargetCommand::IsHovered,
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
    let options = parse_find_options(options).ok_or(ERROR)?;
    Ok(SessionCommand::Element(ElementCommand::FindRole {
        role,
        options,
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
        "hovered" => Some((FindRoleAction::Hovered, remaining)),
        "scroll" => Some((FindRoleAction::ScrollIntoView, remaining)),
        "focus" => Some((FindRoleAction::Focus, remaining)),
        "focused" => Some((FindRoleAction::Focused, remaining)),
        "press" => {
            let input = remaining.trim_start_matches(|value: char| value.is_ascii_whitespace());
            let (key, options) = split_first_token(input)?;
            Some((FindRoleAction::Press(key), options))
        }
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
    let boundary = ROLE_OPTION_TOKENS
        .into_iter()
        .filter_map(|token| find_token(input, token))
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

const ROLE_OPTION_TOKENS: [&str; 10] = [
    "--name",
    "--description",
    "--exact",
    "--checked",
    "--disabled",
    "--expanded",
    "--include-hidden",
    "--level",
    "--pressed",
    "--selected",
];

fn parse_find_options(mut options: &str) -> Option<FindRoleOptions<'_>> {
    let mut parsed = FindRoleOptions::default();
    while !options.trim().is_empty() {
        options = options.trim_start_matches(|value: char| value.is_ascii_whitespace());
        let (option, rest) = split_first_token(options)?;
        match option {
            "--name" if parsed.name.is_none() => {
                let (name, remaining) = split_role_text_option(rest, "--name")?;
                parsed.name = Some(name);
                options = remaining;
            }
            "--description" if parsed.description.is_none() => {
                let (description, remaining) = split_role_text_option(rest, "--description")?;
                parsed.description = Some(description);
                options = remaining;
            }
            "--exact" if !parsed.exact => {
                parsed.exact = true;
                options = rest;
            }
            "--checked" if parsed.filters.states.checked.is_none() => {
                (parsed.filters.states.checked, options) = parse_role_bool(rest)?;
            }
            "--disabled" if parsed.filters.states.disabled.is_none() => {
                (parsed.filters.states.disabled, options) = parse_role_bool(rest)?;
            }
            "--expanded" if parsed.filters.states.expanded.is_none() => {
                (parsed.filters.states.expanded, options) = parse_role_bool(rest)?;
            }
            "--include-hidden" if !parsed.filters.include_hidden => {
                parsed.filters.include_hidden = true;
                options = rest;
            }
            "--level" if parsed.filters.level.is_none() => {
                let (level, remaining) = split_first_token(
                    rest.trim_start_matches(|value: char| value.is_ascii_whitespace()),
                )?;
                parsed.filters.level = level.parse::<u32>().ok().filter(|level| *level > 0);
                parsed.filters.level?;
                options = remaining;
            }
            "--pressed" if parsed.filters.states.pressed.is_none() => {
                (parsed.filters.states.pressed, options) = parse_role_bool(rest)?;
            }
            "--selected" if parsed.filters.states.selected.is_none() => {
                (parsed.filters.states.selected, options) = parse_role_bool(rest)?;
            }
            _ => return None,
        }
    }
    (!parsed.exact || parsed.name.is_some() || parsed.description.is_some()).then_some(parsed)
}

fn split_role_text_option<'a>(value: &'a str, current: &str) -> Option<(&'a str, &'a str)> {
    let value = value.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let boundary = ROLE_OPTION_TOKENS
        .into_iter()
        .filter(|token| *token != current)
        .filter_map(|token| find_token(value, token))
        .min();
    let (name, remaining) =
        boundary.map_or((value, ""), |index| (&value[..index], &value[index..]));
    let name = name.trim_end_matches(|character: char| character.is_ascii_whitespace());
    (!name.is_empty()).then_some((name, remaining))
}

fn parse_role_bool(value: &str) -> Option<(Option<bool>, &str)> {
    let value = value.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let (value, remaining) = split_first_token(value)?;
    let value = match value {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    Some((Some(value), remaining))
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

type CommandArguments<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);

fn parse_page_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    match (command, arguments) {
        ("open", (Some(url), None, None)) => Ok(SessionCommand::Page(PageCommand::Open(url))),
        ("read", (url, None, None)) => Ok(SessionCommand::Page(PageCommand::Read(url))),
        ("back", (None, None, None)) => Ok(SessionCommand::Page(PageCommand::Back)),
        ("forward", (None, None, None)) => Ok(SessionCommand::Page(PageCommand::Forward)),
        ("reload", (None, None, None)) => Ok(SessionCommand::Page(PageCommand::Reload)),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_snapshot_session_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: snapshot requires valid snapshot options";
    let mut options = SnapshotSessionOptions::default();
    let mut interactive_was_set = false;
    let mut compact_was_set = false;
    let mut depth_was_set = false;
    let mut remaining = rest;
    loop {
        remaining = remaining.trim_start_matches(|value: char| value.is_ascii_whitespace());
        let Some((option, rest)) = split_first_token(remaining) else {
            break;
        };
        match option {
            "-i" | "--interactive" if !interactive_was_set => {
                options.projection = SnapshotSessionProjection::Interactive;
                interactive_was_set = true;
                remaining = rest;
            }
            "-u" | "--urls" if !options.output.include_urls => {
                options.output.include_urls = true;
                remaining = rest;
            }
            "-c" | "--compact" if !compact_was_set => {
                compact_was_set = true;
                if let SnapshotSessionProjection::Full(snapshot_options) = &mut options.projection {
                    snapshot_options.compact = true;
                }
                remaining = rest;
            }
            "-d" | "--depth" if !depth_was_set => {
                let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
                let (depth, trailing) = split_first_token(rest).ok_or(ERROR)?;
                let depth = depth.parse::<u64>().map_err(|_| ERROR)?;
                if let SnapshotSessionProjection::Full(snapshot_options) = &mut options.projection {
                    snapshot_options.max_depth = Some(depth);
                }
                depth_was_set = true;
                remaining = trailing;
            }
            "-s" | "--selector" if options.selector.is_none() => {
                let (selector, trailing) = split_locator_value(rest).ok_or(ERROR)?;
                options.selector = Some(selector);
                remaining = trailing;
            }
            _ => return Err(ERROR),
        }
    }
    Ok(SessionCommand::Page(PageCommand::Snapshot(options)))
}

fn parse_screenshot_session_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str =
        "browser.jr: screenshot accepts [path.png], --full [path.png], or <selector> <path.png>";
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    if rest.is_empty() {
        return Ok(SessionCommand::Page(PageCommand::Screenshot {
            selector: None,
            path: None,
            full_page: false,
        }));
    }
    if let Some(rest) = strip_option(rest, "--full").or_else(|| strip_option(rest, "-f")) {
        let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
        if rest.is_empty() {
            return Ok(SessionCommand::Page(PageCommand::Screenshot {
                selector: None,
                path: None,
                full_page: true,
            }));
        }
        let (path, trailing) = split_locator_value(rest).ok_or(ERROR)?;
        if !trailing.trim().is_empty() {
            return Err(ERROR);
        }
        return Ok(SessionCommand::Page(PageCommand::Screenshot {
            selector: None,
            path: Some(path),
            full_page: true,
        }));
    }
    let (first, rest) = split_locator_value(rest).ok_or(ERROR)?;
    let rest = rest.trim_start_matches(|value: char| value.is_ascii_whitespace());
    if rest.is_empty() {
        return Ok(SessionCommand::Page(PageCommand::Screenshot {
            selector: None,
            path: Some(first),
            full_page: false,
        }));
    }
    let (path, trailing) = split_locator_value(rest).ok_or(ERROR)?;
    if !trailing.trim().is_empty() {
        return Err(ERROR);
    }
    Ok(SessionCommand::Page(PageCommand::Screenshot {
        selector: Some(first),
        path: Some(path),
        full_page: false,
    }))
}

fn parse_get_command(arguments: CommandArguments<'_>) -> Result<SessionCommand<'_>, &'static str> {
    match arguments {
        (Some("url"), None, None) => Ok(SessionCommand::Page(PageCommand::GetUrl)),
        (Some("title"), None, None) => Ok(SessionCommand::Page(PageCommand::GetTitle)),
        (Some("viewport"), None, None) => Ok(SessionCommand::Page(PageCommand::GetViewport)),
        _ => Err("browser.jr: invalid session command; enter help"),
    }
}

fn parse_lifecycle_command<'a>(
    command: &str,
    arguments: CommandArguments<'a>,
) -> Result<SessionCommand<'a>, &'static str> {
    match (command, arguments) {
        ("events", (None, None, None)) => Ok(SessionCommand::Events),
        ("help", (None, None, None)) => Ok(SessionCommand::Help),
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

fn parse_type_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let (reference, text) = parse_target_and_value(
        rest,
        "browser.jr: type requires a reference or selector and text",
    )?;
    Ok(SessionCommand::Element(ElementCommand::Type(
        reference, text,
    )))
}

fn parse_keyboard_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: keyboard requires inserttext|type and text";
    let (operation, text) = parse_target_and_value(rest, ERROR)?;
    match operation {
        "inserttext" => Ok(SessionCommand::Keyboard(KeyboardCommand::InsertText(text))),
        "type" => Ok(SessionCommand::Keyboard(KeyboardCommand::Type(text))),
        _ => Err(ERROR),
    }
}

fn parse_keyboard_state_command(
    rest: &str,
    down: bool,
) -> Result<SessionCommand<'_>, &'static str> {
    let error = if down {
        "browser.jr: keydown requires one key"
    } else {
        "browser.jr: keyup requires one key"
    };
    let rest = rest.trim_matches(|value: char| value.is_ascii_whitespace());
    let (key, trailing) = split_first_token(rest).ok_or(error)?;
    if !trailing
        .trim_matches(|value: char| value.is_ascii_whitespace())
        .is_empty()
    {
        return Err(error);
    }
    Ok(SessionCommand::Keyboard(if down {
        KeyboardCommand::Down(key)
    } else {
        KeyboardCommand::Up(key)
    }))
}

fn parse_press_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    let rest = rest.trim_matches(|value: char| value.is_ascii_whitespace());
    let (key, trailing) = split_first_token(rest).ok_or("browser.jr: press requires one key")?;
    trailing
        .trim_matches(|value: char| value.is_ascii_whitespace())
        .is_empty()
        .then_some(SessionCommand::Element(ElementCommand::Press(key)))
        .ok_or("browser.jr: press requires one key")
}

fn parse_scroll_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str =
        "browser.jr: scroll requires up|down|left|right and an optional pixel count";
    const DEFAULT_DISTANCE: u64 = 300;
    let mut parts = rest.split_ascii_whitespace();
    let direction = match parts.next() {
        Some("up") => ScrollDirection::Up,
        Some("down") => ScrollDirection::Down,
        Some("left") => ScrollDirection::Left,
        Some("right") => ScrollDirection::Right,
        _ => return Err(ERROR),
    };
    let distance = parts
        .next()
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|_| ERROR)?
        .unwrap_or(DEFAULT_DISTANCE);
    if parts.next().is_some() {
        return Err(ERROR);
    }
    Ok(SessionCommand::Page(PageCommand::Scroll(
        direction, distance,
    )))
}

fn parse_set_viewport_command(rest: &str) -> Result<SessionCommand<'_>, &'static str> {
    const ERROR: &str = "browser.jr: set viewport requires positive width and height";
    let mut parts = rest.split_ascii_whitespace();
    let width = parts
        .next()
        .ok_or(ERROR)?
        .parse::<u64>()
        .map_err(|_| ERROR)?;
    let height = parts
        .next()
        .ok_or(ERROR)?
        .parse::<u64>()
        .map_err(|_| ERROR)?;
    if width == 0 || height == 0 || parts.next().is_some() {
        return Err(ERROR);
    }
    Ok(SessionCommand::Page(PageCommand::SetViewport(
        width, height,
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
        AccessibilitySnapshotOptions, ElementCommand, ExitStatus, FindLocatorKind, FindRoleAction,
        FindRoleFilters, FindRoleOptions, KeyboardCommand, PageCommand, RoleFilterStates,
        SelectCommandValues, SessionCommand, SnapshotOutputOptions, SnapshotSessionOptions,
        SnapshotSessionProjection, build_direct_locator, parse_command, run_session,
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
        assert!(output.contains("type <ref|selector> <text>"));
        assert!(output.contains("keyboard inserttext <text>"));
        assert!(output.contains("keyboard type <text>"));
        assert!(output.contains("keydown <key>"));
        assert!(output.contains("keyup <key>"));
        assert!(output.contains("focus <ref|selector>"));
        assert!(output.contains("hover <ref|selector>"));
        assert!(output.contains("scroll <up|down|left|right> [pixels]"));
        assert!(output.contains("set viewport <width> <height>"));
        assert!(output.contains("scrollintoview <ref|selector>"));
        assert!(output.contains("press <key>"));
        assert!(output.contains("is editable <ref|selector>"));
        assert!(output.contains("is focused <ref|selector>"));
        assert!(output.contains("is hovered <ref|selector>"));
        assert!(output.contains("get box <ref|selector>"));
        assert!(output.contains("get viewport"));
        assert!(output.contains("  events\n"));
        assert!(output.contains("  read [url]\n"));
        assert!(output.contains("  back\n"));
        assert!(output.contains("  forward\n"));
        assert!(output.contains("session closed"));
        assert_eq!(
            String::from_utf8(errors).unwrap(),
            "browser.jr: invalid session command; enter help\n"
        );
    }

    #[test]
    fn events_command_has_no_arguments() {
        assert!(matches!(
            parse_command("events"),
            Ok(SessionCommand::Events)
        ));
        assert!(parse_command("events now").is_err());
    }

    #[test]
    fn keyboard_text_commands_preserve_the_remaining_line() {
        assert!(matches!(
            parse_command("keyboard inserttext hello world  "),
            Ok(SessionCommand::Keyboard(KeyboardCommand::InsertText(
                "hello world  "
            )))
        ));
        assert!(matches!(
            parse_command("keyboard type 😀"),
            Ok(SessionCommand::Keyboard(KeyboardCommand::Type("😀")))
        ));
        assert!(matches!(
            parse_command("keyboard type "),
            Ok(SessionCommand::Keyboard(KeyboardCommand::Type("")))
        ));
        assert!(parse_command("keyboard paste hello").is_err());
        assert!(parse_command("keyboard type").is_err());
    }

    #[test]
    fn keyboard_state_commands_require_one_key() {
        assert!(matches!(
            parse_command("keydown Shift"),
            Ok(SessionCommand::Keyboard(KeyboardCommand::Down("Shift")))
        ));
        assert!(matches!(
            parse_command("keyup a"),
            Ok(SessionCommand::Keyboard(KeyboardCommand::Up("a")))
        ));
        assert!(parse_command("keydown").is_err());
        assert!(parse_command("keyup Shift trailing").is_err());
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
            parse_command("type @e1 hello world"),
            Ok(SessionCommand::Element(ElementCommand::Type(
                "@e1",
                "hello world"
            )))
        ));
        assert!(matches!(
            parse_command("type @e1 "),
            Ok(SessionCommand::Element(ElementCommand::Type("@e1", "")))
        ));
        assert!(parse_command("type @e1").is_err());
        assert!(matches!(
            parse_command("focus @e1"),
            Ok(SessionCommand::Element(ElementCommand::Focus("@e1")))
        ));
        assert!(matches!(
            parse_command("hover @e1"),
            Ok(SessionCommand::Element(ElementCommand::Hover("@e1")))
        ));
        assert!(matches!(
            parse_command("scroll down"),
            Ok(SessionCommand::Page(PageCommand::Scroll(
                crate::ScrollDirection::Down,
                300
            )))
        ));
        assert!(matches!(
            parse_command("scroll left 42"),
            Ok(SessionCommand::Page(PageCommand::Scroll(
                crate::ScrollDirection::Left,
                42
            )))
        ));
        assert!(matches!(
            parse_command("scrollintoview @e1"),
            Ok(SessionCommand::Element(ElementCommand::ScrollIntoView(
                "@e1"
            )))
        ));
        assert!(matches!(
            parse_command("scrollinto '#target'"),
            Ok(SessionCommand::Element(ElementCommand::ScrollIntoView(
                "#target"
            )))
        ));
        assert!(parse_command("scroll").is_err());
        assert!(parse_command("scroll around").is_err());
        assert!(parse_command("scroll down nope").is_err());
        assert!(parse_command("scroll down 1 extra").is_err());
        assert!(matches!(
            parse_command("set viewport 640 480"),
            Ok(SessionCommand::Page(PageCommand::SetViewport(640, 480)))
        ));
        assert!(matches!(
            parse_command("get viewport"),
            Ok(SessionCommand::Page(PageCommand::GetViewport))
        ));
        assert!(parse_command("set viewport 0 480").is_err());
        assert!(parse_command("set viewport 640").is_err());
        assert!(parse_command("set viewport wide 480").is_err());
        assert!(parse_command("set viewport 640 480 extra").is_err());
        assert!(matches!(
            parse_command("press Enter"),
            Ok(SessionCommand::Element(ElementCommand::Press("Enter")))
        ));
        assert!(parse_command("press").is_err());
        assert!(parse_command("press Control A").is_err());
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
            parse_command("get box @e1"),
            Ok(SessionCommand::Element(ElementCommand::GetBoundingBox(
                "@e1"
            )))
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
            parse_command("read"),
            Ok(SessionCommand::Page(PageCommand::Read(None)))
        ));
        assert!(matches!(
            parse_command("read http://127.0.0.1:3000/"),
            Ok(SessionCommand::Page(PageCommand::Read(Some(
                "http://127.0.0.1:3000/"
            ))))
        ));
        assert!(parse_command("read one two").is_err());
        assert!(matches!(
            parse_command("back"),
            Ok(SessionCommand::Page(PageCommand::Back))
        ));
        assert!(matches!(
            parse_command("forward"),
            Ok(SessionCommand::Page(PageCommand::Forward))
        ));
        assert!(parse_command("back now").is_err());
        assert!(parse_command("forward now").is_err());
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
            Ok(SessionCommand::Page(PageCommand::Snapshot(
                SnapshotSessionOptions {
                    selector: None,
                    projection: SnapshotSessionProjection::Interactive,
                    output: SnapshotOutputOptions {
                        include_urls: false,
                    },
                }
            )))
        ));
        assert!(matches!(
            parse_command("snapshot --compact --depth 2"),
            Ok(SessionCommand::Page(PageCommand::Snapshot(
                SnapshotSessionOptions {
                    selector: None,
                    projection: SnapshotSessionProjection::Full(AccessibilitySnapshotOptions {
                        compact: true,
                        max_depth: Some(2),
                    }),
                    output: SnapshotOutputOptions {
                        include_urls: false,
                    },
                }
            )))
        ));
        assert!(matches!(
            parse_command("snapshot --interactive -s \"main > section\""),
            Ok(SessionCommand::Page(PageCommand::Snapshot(
                SnapshotSessionOptions {
                    selector: Some("main > section"),
                    projection: SnapshotSessionProjection::Interactive,
                    output: SnapshotOutputOptions {
                        include_urls: false,
                    },
                }
            )))
        ));
        assert!(matches!(
            parse_command("snapshot -i --urls --compact --depth 0"),
            Ok(SessionCommand::Page(PageCommand::Snapshot(
                SnapshotSessionOptions {
                    selector: None,
                    projection: SnapshotSessionProjection::Interactive,
                    output: SnapshotOutputOptions { include_urls: true },
                }
            )))
        ));
        assert!(matches!(
            parse_command("screenshot"),
            Ok(SessionCommand::Page(PageCommand::Screenshot {
                selector: None,
                path: None,
                full_page: false,
            }))
        ));
        assert!(matches!(
            parse_command("screenshot -f page.png"),
            Ok(SessionCommand::Page(PageCommand::Screenshot {
                selector: None,
                path: Some("page.png"),
                full_page: true,
            }))
        ));
        assert!(matches!(
            parse_command("screenshot \"main > section\" section.png"),
            Ok(SessionCommand::Page(PageCommand::Screenshot {
                selector: Some("main > section"),
                path: Some("section.png"),
                full_page: false,
            }))
        ));
        assert!(parse_command("screenshot --full page.png extra").is_err());
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
            parse_command("is editable @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsEditable("@e1")))
        ));
        assert!(matches!(
            parse_command("is enabled @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsEnabled("@e1")))
        ));
        assert!(matches!(
            parse_command("is focused @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsFocused("@e1")))
        ));
        assert!(matches!(
            parse_command("is visible @e1"),
            Ok(SessionCommand::Element(ElementCommand::IsVisible("@e1")))
        ));
        assert!(parse_command("get value @e1 extra").is_err());
        assert!(parse_command("get html @e1 extra").is_err());
        assert!(parse_command("get box @e1 extra").is_err());
        assert!(parse_command("snapshot -s main").is_ok());
        assert!(parse_command("snapshot -i -s").is_err());
        assert!(parse_command("snapshot -i --depth -1").is_err());
        assert!(parse_command("snapshot -i --urls --urls").is_err());
    }

    #[test]
    fn find_role_parses_name_and_exact_variants() {
        assert!(matches!(
            parse_command("find role button"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: None,
                    exact: false,
                    ..
                },
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --name Save changes"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Save changes"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --name Save changes --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Save changes"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role button --exact --name Save changes"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Save changes"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::Click,
            }))
        ));
        assert!(matches!(
            parse_command("find role heading text --name Skills"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "heading",
                options: FindRoleOptions {
                    name: Some("Skills"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Text,
            }))
        ));
        assert!(matches!(
            parse_command("find role textbox fill hello world --name Email address --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "textbox",
                options: FindRoleOptions {
                    name: Some("Email address"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::Fill("hello world"),
            }))
        ));
        assert!(matches!(
            parse_command("find role textbox press End --name Email address --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "textbox",
                options: FindRoleOptions {
                    name: Some("Email address"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::Press("End"),
            }))
        ));
        assert!(matches!(
            parse_command("find role checkbox check --name Terms"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "checkbox",
                options: FindRoleOptions {
                    name: Some("Terms"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Check,
            }))
        ));
        assert!(matches!(
            parse_command("find role checkbox uncheck --name Terms"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "checkbox",
                options: FindRoleOptions {
                    name: Some("Terms"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Uncheck,
            }))
        ));
        assert!(matches!(
            parse_command("find role button hover --name Menu"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Menu"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Hover,
            }))
        ));
        assert!(matches!(
            parse_command("find role button hovered --name Menu"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Menu"),
                    exact: false,
                    ..
                },
                action: FindRoleAction::Hovered,
            }))
        ));
        assert!(matches!(
            parse_command("find role button scroll --name Target --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "button",
                options: FindRoleOptions {
                    name: Some("Target"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::ScrollIntoView,
            }))
        ));
        assert!(matches!(
            parse_command("find role textbox focused --name Email --exact"),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "textbox",
                options: FindRoleOptions {
                    name: Some("Email"),
                    exact: true,
                    ..
                },
                action: FindRoleAction::Focused,
            }))
        ));
        assert!(matches!(
            parse_command(
                "find role heading text --selected false --level 2 --include-hidden --pressed true --expanded false --disabled true --checked false --description Page section --exact --name Skills"
            ),
            Ok(SessionCommand::Element(ElementCommand::FindRole {
                role: "heading",
                options: FindRoleOptions {
                    name: Some("Skills"),
                    description: Some("Page section"),
                    exact: true,
                    filters: FindRoleFilters {
                        states: RoleFilterStates {
                            checked: Some(false),
                            disabled: Some(true),
                            expanded: Some(false),
                            pressed: Some(true),
                            selected: Some(false),
                        },
                        include_hidden: true,
                        level: Some(2),
                    },
                },
                action: FindRoleAction::Text,
            }))
        ));
        assert!(parse_command("find role").is_err());
        assert!(parse_command("find role button --name").is_err());
        assert!(parse_command("find role button --description").is_err());
        assert!(parse_command("find role button --name --exact").is_err());
        assert!(parse_command("find role button --description Hint --exact").is_ok());
        assert!(parse_command("find role button --exact").is_err());
        assert!(parse_command("find role heading --level 0").is_err());
        assert!(parse_command("find role button --checked mixed").is_err());
        assert!(parse_command("find role button --include-hidden --include-hidden").is_err());
        assert!(parse_command("find role textbox fill --name Email").is_err());
        assert!(parse_command("find role textbox press --name Email").is_err());
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
            parse_command("find css #email press Backspace"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Css,
                value: "#email",
                exact: false,
                action: FindRoleAction::Press("Backspace"),
            }))
        ));
        assert!(matches!(
            parse_command("find css #email focused"),
            Ok(SessionCommand::Element(ElementCommand::FindLocator {
                kind: FindLocatorKind::Css,
                value: "#email",
                exact: false,
                action: FindRoleAction::Focused,
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
            parse_command("type \"form > input\" hello world"),
            Ok(SessionCommand::Element(ElementCommand::Type(
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
            parse_command("is focused \"form > input\""),
            Ok(SessionCommand::Element(ElementCommand::IsFocused(
                "form > input"
            )))
        ));
        assert!(matches!(
            parse_command("is hovered \"form > button\""),
            Ok(SessionCommand::Element(ElementCommand::IsHovered(
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
