use crate::keyboard::{
    ControlActivationKey, FocusTraversalDirection, FocusTraversalEffect, FocusedElement,
    KeyboardEventKey, KeyboardKey, KeyboardModifier, KeyboardPressEventKind, KeyboardTextEffect,
    ModifiedKeyError, NavigationPressEffect, PressEffect, RadioGroupDirection, TextPressEffect,
};
use crate::layout::{
    BoundingBox, LayoutError, LayoutInput, LayoutKernel, LayoutMutation, LayoutProgram,
    LayoutSnapshot,
};
use crate::loading::{LoadError, load_html, resolve_url_reference};
use crate::locator::{Locator, LocatorMatch, LocatorPosition, RoleLocator, RoleMatch};
use crate::non_empty::NonEmpty;
use crate::page::{
    AccessibilityNodeSource, ControlState, HitTestCandidate, HitTestLayer, InteractiveAction,
    InteractiveElementSource, LocatorElementSource, SelectValueError, SelectorIndex,
    SelectorQueryError, SequentialFocusSource, TextValueError,
    page_semantics_from_html_with_viewport, paint_commands_from_html,
};
use crate::rules::{
    RuleResult, WidthFinding, evaluate_horizontal_overflow, evaluate_max_element_width,
};
use crate::selection::SelectOptionTarget;
use crate::snapshot::{
    AccessibilitySnapshot, AccessibilitySnapshotOptions, InteractiveElementRef,
    InteractiveSnapshot, Snapshot, SnapshotCaptureIdentity, SnapshotId,
};
use crate::{
    CaptureRect, CaptureTarget, DEFAULT_VIEWPORT_HEIGHT, DEFAULT_VIEWPORT_WIDTH, PaintScene,
    PreparedScreenshot,
};
use http::Uri;

mod private {
    pub trait Sealed {}
}

pub trait SessionRequest: private::Sealed {
    type Reply;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError>;
}

#[derive(Debug)]
pub struct Session {
    layout: LayoutKernel,
    identities: IdentityCounters,
    last_snapshot: Option<Snapshot>,
    latest_interactive_snapshot: Option<LatestInteractiveSnapshot>,
    current_page: Option<CurrentPage>,
    history: NavigationHistory,
    viewport: ViewportSize,
    keyboard: KeyboardState,
    dom_events: Vec<DomEvent>,
}

/// One supported native DOM event type recorded by a browser.jr action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomEventType {
    BeforeInput,
    Blur,
    Change,
    Click,
    Focus,
    FocusIn,
    FocusOut,
    Input,
    KeyDown,
    KeyPress,
    KeyUp,
    MouseDown,
    MouseEnter,
    MouseLeave,
    MouseMove,
    MouseOut,
    MouseOver,
    MouseUp,
    PointerDown,
    PointerEnter,
    PointerLeave,
    PointerMove,
    PointerOut,
    PointerOver,
    PointerUp,
}

impl DomEventType {
    pub const fn bubbles(self) -> bool {
        !matches!(
            self,
            Self::Blur
                | Self::Focus
                | Self::MouseEnter
                | Self::MouseLeave
                | Self::PointerEnter
                | Self::PointerLeave
        )
    }

    pub const fn composed(self) -> bool {
        !matches!(
            self,
            Self::Change
                | Self::MouseEnter
                | Self::MouseLeave
                | Self::PointerEnter
                | Self::PointerLeave
        )
    }
}

impl std::fmt::Display for DomEventType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BeforeInput => "beforeinput",
            Self::Blur => "blur",
            Self::Change => "change",
            Self::Click => "click",
            Self::Focus => "focus",
            Self::FocusIn => "focusin",
            Self::FocusOut => "focusout",
            Self::Input => "input",
            Self::KeyDown => "keydown",
            Self::KeyPress => "keypress",
            Self::KeyUp => "keyup",
            Self::MouseDown => "mousedown",
            Self::MouseEnter => "mouseenter",
            Self::MouseLeave => "mouseleave",
            Self::MouseMove => "mousemove",
            Self::MouseOut => "mouseout",
            Self::MouseOver => "mouseover",
            Self::MouseUp => "mouseup",
            Self::PointerDown => "pointerdown",
            Self::PointerEnter => "pointerenter",
            Self::PointerLeave => "pointerleave",
            Self::PointerMove => "pointermove",
            Self::PointerOut => "pointerout",
            Self::PointerOver => "pointerover",
            Self::PointerUp => "pointerup",
        })
    }
}

/// One data-minimized record of a supported native DOM event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomEvent {
    pub event_type: DomEventType,
    pub document_epoch: u64,
    pub target: String,
    pub target_ordinal: usize,
    pub related_target: Option<DomEventTargetIdentity>,
    pub path: Vec<String>,
    pub bubbles: bool,
    pub composed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomEventTargetIdentity {
    pub target: String,
    pub target_ordinal: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DomEventTarget {
    document_epoch: u64,
    target: String,
    target_ordinal: usize,
    path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PointerActionContext {
    previous_pointer: Option<DomEventTarget>,
    previous_focus: Option<DomEventTarget>,
    source_index: usize,
    target: DomEventTarget,
}

#[derive(Clone, Copy)]
struct HitTestPoint {
    x: i64,
    y: i64,
    scroll_x: u64,
    scroll_y: u64,
    target_layer: HitTestLayer,
}

/// Drains native DOM event records created since the prior drain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TakeDomEvents;

#[derive(Debug, Default)]
struct KeyboardState {
    pressed: Vec<PressedKeyboardKey>,
}

#[derive(Debug)]
struct PressedKeyboardKey {
    key: KeyboardEventKey,
    records_key_up: bool,
    pending_space_activation: Option<PendingSpaceActivation>,
}

#[derive(Debug)]
struct PendingSpaceActivation {
    key: KeyboardKey,
    target: DomEventTarget,
}

impl KeyboardState {
    fn is_pressed(&self, key: &KeyboardEventKey) -> bool {
        self.pressed.iter().any(|pressed| pressed.key == *key)
    }

    fn modifiers(&self) -> Vec<KeyboardModifier> {
        self.pressed
            .iter()
            .filter_map(|pressed| pressed.key.modifier())
            .collect()
    }

    fn record_down(
        &mut self,
        key: KeyboardEventKey,
        records_key_up: bool,
        pending_space_activation: Option<PendingSpaceActivation>,
    ) {
        if let Some(pressed) = self.pressed.iter_mut().find(|pressed| pressed.key == key) {
            pressed.records_key_up |= records_key_up;
        } else {
            self.pressed.push(PressedKeyboardKey {
                key,
                records_key_up,
                pending_space_activation,
            });
        }
    }

    fn release(&mut self, key: &KeyboardEventKey) -> Option<PressedKeyboardKey> {
        let index = self
            .pressed
            .iter()
            .position(|pressed| pressed.key == *key)?;
        Some(self.pressed.remove(index))
    }
}

#[derive(Debug, Default)]
struct NavigationHistory {
    entries: Vec<String>,
    current: Option<usize>,
}

impl NavigationHistory {
    fn record(&mut self, url: String) {
        let next = self.current.map_or(0, |current| current + 1);
        self.entries.truncate(next);
        self.entries.push(url);
        self.current = Some(self.entries.len() - 1);
    }

    fn previous(&self) -> Option<(usize, String)> {
        let index = self.current?.checked_sub(1)?;
        Some((index, self.entries[index].clone()))
    }

    fn next(&self) -> Option<(usize, String)> {
        let index = self.current?.checked_add(1)?;
        self.entries.get(index).cloned().map(|url| (index, url))
    }

    fn move_to(&mut self, index: usize) {
        assert!(index < self.entries.len(), "history target must exist");
        self.current = Some(index);
    }
}

#[derive(Debug)]
struct LatestInteractiveSnapshot {
    id: SnapshotId,
    element_indices: Vec<Option<usize>>,
}

#[derive(Debug)]
struct IdentityCounters {
    next_snapshot_id: u64,
    next_document_epoch: u64,
}

#[derive(Debug)]
struct CurrentPage {
    epoch: u64,
    url: String,
    html: String,
    title: String,
    text: String,
    locator_elements: Vec<LocatorElementSource>,
    interactive_elements: Vec<InteractiveElementSource>,
    accessibility_tree: Vec<AccessibilityNodeSource>,
    selector_index: SelectorIndex,
    focused_interactive_index: Option<usize>,
    hovered_source_index: Option<usize>,
    sequential_focus: SequentialFocusSource,
    document_width: u64,
    document_height: u64,
    scroll_x: u64,
    scroll_y: u64,
}

#[derive(Debug)]
struct ResolvedLocator {
    matched: LocatorMatch,
    source_index: usize,
    interactive_index: Option<usize>,
}

#[derive(Debug)]
enum PagePressError {
    NoFocusedElement,
    Unsupported { element: String, reason: String },
}

#[derive(Debug)]
enum FocusedPressError {
    Press(PagePressError),
    Navigation { element: String, error: LoadError },
}

#[derive(Debug)]
enum FocusedPressDisposition {
    Local,
    Ignored {
        element: FocusedElement,
    },
    Navigate {
        element: FocusedElement,
        target: String,
    },
}

#[derive(Debug)]
enum FormSubmissionError {
    Unsupported(String),
    Navigation(LoadError),
}

#[derive(Debug)]
enum CheckedMutationError {
    Blocked { reason: String },
    Unsupported { reason: String },
}

impl CheckedMutationError {
    fn reason(self) -> String {
        match self {
            Self::Blocked { reason } | Self::Unsupported { reason } => reason,
        }
    }
}

#[derive(Debug)]
enum LocatorOperationError {
    NoPage,
    NotFound,
    Ambiguous {
        match_count: usize,
    },
    Query {
        reason: String,
    },
    InspectionBlocked {
        inspection: LocatorInspection,
        reason: String,
    },
    SensitiveAttribute {
        name: String,
    },
    SelectOptionNotFound {
        target: SelectOptionTarget,
    },
    SelectOptionDisabled {
        target: SelectOptionTarget,
    },
    Navigation(LoadError),
    ActionBlocked {
        action: LocatorAction,
        check: ActionabilityCheck,
        reason: String,
    },
    UnsupportedAction {
        action: LocatorAction,
        reason: String,
    },
}

impl Session {
    pub fn new() -> Self {
        Self {
            layout: LayoutKernel::new(LayoutProgram::initial()),
            identities: IdentityCounters {
                next_snapshot_id: 1,
                next_document_epoch: 1,
            },
            last_snapshot: None,
            latest_interactive_snapshot: None,
            current_page: None,
            history: NavigationHistory::default(),
            viewport: ViewportSize::default(),
            keyboard: KeyboardState::default(),
            dom_events: Vec::new(),
        }
    }

    pub fn execute<R>(&mut self, request: R) -> Result<R::Reply, SessionError>
    where
        R: SessionRequest,
    {
        request.execute(self)
    }

    fn load_page(&mut self, url: String) -> Result<OpenedPage, LoadError> {
        let loaded = load_html(&url)?;
        let url = loaded.final_url;
        let html = loaded.html;
        let semantics = page_semantics_from_html_with_viewport(
            &html,
            self.viewport.width,
            self.viewport.height,
        );
        let epoch = self.identities.next_document_epoch;
        self.identities.next_document_epoch = self
            .identities
            .next_document_epoch
            .checked_add(1)
            .expect("document epoch exhausted");
        let reply = OpenedPage {
            url: url.clone(),
            interactive_element_count: semantics.elements.interactive_elements.len(),
        };
        self.layout = LayoutKernel::new(LayoutProgram::initial());
        self.last_snapshot = None;
        self.latest_interactive_snapshot = None;
        self.current_page = Some(CurrentPage {
            epoch,
            url,
            html,
            title: semantics.document.title,
            text: semantics.document.text,
            locator_elements: semantics.elements.locator_elements,
            interactive_elements: semantics.elements.interactive_elements,
            accessibility_tree: semantics.document.accessibility_tree,
            selector_index: semantics.selector_index,
            focused_interactive_index: None,
            hovered_source_index: None,
            sequential_focus: semantics.sequential_focus,
            document_width: semantics.extent.document_width,
            document_height: semantics.extent.document_height,
            scroll_x: 0,
            scroll_y: 0,
        });
        Ok(reply)
    }

    fn navigate_to(&mut self, url: String) -> Result<OpenedPage, LoadError> {
        let page = self.load_page(url)?;
        self.history.record(page.url.clone());
        Ok(page)
    }

    fn navigate_history(
        &mut self,
        target: Option<(usize, String)>,
    ) -> Result<HistoryNavigationResult, SessionError> {
        let current_url = self
            .current_page
            .as_ref()
            .ok_or(SessionError::NoPage)?
            .url
            .clone();
        let Some((index, url)) = target else {
            return Ok(HistoryNavigationResult::NoEntry { current_url });
        };
        let page = self.load_page(url).map_err(SessionError::Load)?;
        self.history.move_to(index);
        Ok(HistoryNavigationResult::Navigated(page))
    }

    fn element_index_for(&self, reference: InteractiveElementRef) -> Result<usize, SessionError> {
        let page = self.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let Some(snapshot) = &self.latest_interactive_snapshot else {
            return Err(SessionError::StaleElementReference { reference });
        };
        if reference.document_epoch() != page.epoch || snapshot.id != reference.snapshot() {
            return Err(SessionError::StaleElementReference { reference });
        }
        let ordinal = reference
            .ordinal()
            .checked_sub(1)
            .and_then(|ordinal| usize::try_from(ordinal).ok())
            .expect("interactive snapshot references use nonzero usize ordinals");
        snapshot
            .element_indices
            .get(ordinal)
            .copied()
            .flatten()
            .filter(|index| page.interactive_elements.get(*index).is_some())
            .ok_or(SessionError::StaleElementReference { reference })
    }

    fn locator_matches_for(
        &self,
        locator: &Locator,
    ) -> Result<Vec<ResolvedLocator>, LocatorOperationError> {
        let page = self
            .current_page
            .as_ref()
            .ok_or(LocatorOperationError::NoPage)?;
        let mut matches = if let Some(css) = locator.css() {
            page.selector_index.css_matches(css.selector())?
        } else if let Some(xpath) = locator.xpath() {
            page.selector_index.xpath_matches(xpath.expression())?
        } else {
            let mut matches = Vec::new();
            for (index, element) in page.locator_elements.iter().enumerate() {
                let interactive = element
                    .interactive_index
                    .and_then(|index| page.interactive_elements.get(index));
                match element.matches(locator, interactive) {
                    Ok(true) => matches.push(index),
                    Ok(false) => {}
                    Err(reason) => {
                        return Err(LocatorOperationError::Query {
                            reason: format!(
                                "accessible visibility is unavailable for {}: {reason}",
                                element.element
                            ),
                        });
                    }
                }
            }
            matches
        };
        if locator.uses_descendant_text() {
            let candidates = matches.clone();
            matches.retain(|candidate| {
                !candidates.iter().any(|other| {
                    other != candidate
                        && locator_element_is_descendant(&page.locator_elements, *other, *candidate)
                })
            });
        }
        if let Some(position) = locator.position() {
            let selected = match position {
                LocatorPosition::First => matches.first().copied(),
                LocatorPosition::Last => matches.last().copied(),
                LocatorPosition::Nth(index) => matches.get(index).copied(),
            };
            matches.clear();
            matches.extend(selected);
        }
        Ok(matches
            .into_iter()
            .map(|index| {
                let element = &page.locator_elements[index];
                ResolvedLocator {
                    matched: LocatorMatch::new(
                        &element.element,
                        element.role(),
                        element.name(),
                        element.text(),
                    ),
                    source_index: index,
                    interactive_index: element.interactive_index,
                }
            })
            .collect())
    }

    fn locator_match_for(
        &self,
        locator: &Locator,
    ) -> Result<ResolvedLocator, LocatorOperationError> {
        let mut matches = self.locator_matches_for(locator)?;
        if matches.is_empty() {
            return Err(LocatorOperationError::NotFound);
        }
        if matches.len() > 1 {
            return Err(LocatorOperationError::Ambiguous {
                match_count: matches.len(),
            });
        }
        Ok(matches.pop().expect("one locator match remains"))
    }

    fn locator_interactive_index(
        &self,
        resolved: &ResolvedLocator,
        action: LocatorAction,
    ) -> Result<usize, LocatorOperationError> {
        resolved
            .interactive_index
            .ok_or_else(|| LocatorOperationError::UnsupportedAction {
                action,
                reason: resolved.matched.role.as_ref().map_or_else(
                    || "matched element has no implemented interactive behavior".into(),
                    |role| format!("role {role} has no implemented interactive behavior"),
                ),
            })
    }

    fn editable_state(&self, source_index: usize) -> Result<Option<bool>, String> {
        let page = self
            .current_page
            .as_ref()
            .expect("editable inspection requires a current page");
        let element = &page.locator_elements[source_index];
        if let Some(editable) = element.native_editable() {
            if editable && self.has_disabled_fieldset_ancestor(source_index) {
                return Err("disabled fieldset editable state is not implemented".into());
            }
            return Ok(Some(editable));
        }
        let mut candidate = Some(source_index);
        while let Some(index) = candidate {
            let element = &page.locator_elements[index];
            if let Some(editable) = element.content_editable_value() {
                return Ok(editable.then_some(true));
            }
            candidate = element.parent;
        }
        Ok(None)
    }

    fn has_disabled_fieldset_ancestor(&self, source_index: usize) -> bool {
        let page = self
            .current_page
            .as_ref()
            .expect("editable inspection requires a current page");
        let mut candidate = page.locator_elements[source_index].parent;
        while let Some(index) = candidate {
            let element = &page.locator_elements[index];
            if element.is_disabled_fieldset() {
                return true;
            }
            candidate = element.parent;
        }
        false
    }

    fn dom_event_target(&self, interactive_index: usize) -> DomEventTarget {
        self.current_page
            .as_ref()
            .expect("DOM event targets require a current page")
            .dom_event_target(interactive_index)
    }

    fn pointer_action_context(&self, source_index: usize) -> PointerActionContext {
        let page = self
            .current_page
            .as_ref()
            .expect("pointer actions require a current page");
        let previous_pointer = page
            .hovered_source_index
            .map(|index| page.dom_event_target_for_source(index));
        let previous_focus = page
            .focused_interactive_index
            .map(|index| page.dom_event_target(index));
        let target = page.dom_event_target_for_source(source_index);
        PointerActionContext {
            previous_pointer,
            previous_focus,
            source_index,
            target,
        }
    }

    fn finish_pointer_move(&mut self, context: &PointerActionContext) {
        self.commit_pointer_target(context.source_index);
        self.record_pointer_transition(context.previous_pointer.as_ref(), &context.target);
    }

    fn finish_pointer_click(
        &mut self,
        context: &PointerActionContext,
        activation_events: &[DomEventType],
    ) {
        self.commit_pointer_target(context.source_index);
        let current_focus = self.current_page.as_ref().and_then(|page| {
            page.focused_interactive_index
                .map(|index| page.dom_event_target(index))
        });
        self.record_pointer_transition(context.previous_pointer.as_ref(), &context.target);
        self.record_dom_events(
            &context.target,
            &[DomEventType::PointerDown, DomEventType::MouseDown],
        );
        self.record_focus_transition(context.previous_focus.as_ref(), current_focus.as_ref());
        self.record_dom_events(
            &context.target,
            &[
                DomEventType::PointerUp,
                DomEventType::MouseUp,
                DomEventType::Click,
            ],
        );
        self.record_dom_events(&context.target, activation_events);
    }

    fn commit_pointer_target(&mut self, source_index: usize) {
        self.current_page
            .as_mut()
            .expect("pointer actions require a current page")
            .hovered_source_index = Some(source_index);
    }

    fn record_pointer_transition(
        &mut self,
        previous: Option<&DomEventTarget>,
        current: &DomEventTarget,
    ) {
        if previous.is_some_and(|target| same_dom_event_target(target, current)) {
            self.record_dom_events(
                current,
                &[DomEventType::PointerMove, DomEventType::MouseMove],
            );
            return;
        }
        if let Some(previous) = previous {
            self.record_related_dom_events(
                previous,
                Some(current),
                &[DomEventType::PointerOut, DomEventType::PointerLeave],
            );
        }
        self.record_related_dom_events(
            current,
            previous,
            &[DomEventType::PointerOver, DomEventType::PointerEnter],
        );
        if let Some(previous) = previous {
            self.record_related_dom_events(
                previous,
                Some(current),
                &[DomEventType::MouseOut, DomEventType::MouseLeave],
            );
        }
        self.record_related_dom_events(
            current,
            previous,
            &[DomEventType::MouseOver, DomEventType::MouseEnter],
        );
        self.record_dom_events(
            current,
            &[DomEventType::PointerMove, DomEventType::MouseMove],
        );
    }

    fn record_focus_transition(
        &mut self,
        previous: Option<&DomEventTarget>,
        current: Option<&DomEventTarget>,
    ) {
        if matches!((previous, current), (Some(left), Some(right)) if same_dom_event_target(left, right))
        {
            return;
        }
        if let Some(previous) = previous {
            self.record_related_dom_events(
                previous,
                current,
                &[DomEventType::Blur, DomEventType::FocusOut],
            );
        }
        if let Some(current) = current {
            self.record_related_dom_events(
                current,
                previous,
                &[DomEventType::Focus, DomEventType::FocusIn],
            );
        }
    }

    fn record_dom_events(&mut self, target: &DomEventTarget, types: &[DomEventType]) {
        self.record_related_dom_events(target, None, types);
    }

    fn record_related_dom_events(
        &mut self,
        target: &DomEventTarget,
        related_target: Option<&DomEventTarget>,
        types: &[DomEventType],
    ) {
        self.dom_events
            .extend(types.iter().map(|event_type| DomEvent {
                event_type: *event_type,
                document_epoch: target.document_epoch,
                target: target.target.clone(),
                target_ordinal: target.target_ordinal,
                related_target: related_target.map(|target| DomEventTargetIdentity {
                    target: target.target.clone(),
                    target_ordinal: target.target_ordinal,
                }),
                path: target.path.clone(),
                bubbles: event_type.bubbles(),
                composed: event_type.composed(),
            }));
    }
}

fn same_dom_event_target(left: &DomEventTarget, right: &DomEventTarget) -> bool {
    left.document_epoch == right.document_epoch && left.target_ordinal == right.target_ordinal
}

impl private::Sealed for TakeDomEvents {}

impl SessionRequest for TakeDomEvents {
    type Reply = Vec<DomEvent>;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        Ok(std::mem::take(&mut session.dom_events))
    }
}

impl CurrentPage {
    fn source_index_for_interactive(&self, interactive_index: usize) -> usize {
        self.interactive_elements[interactive_index].source_index
    }

    fn dom_event_target(&self, interactive_index: usize) -> DomEventTarget {
        let source_index = self.source_index_for_interactive(interactive_index);
        self.dom_event_target_for_source(source_index)
    }

    fn dom_event_target_for_source(&self, source_index: usize) -> DomEventTarget {
        let mut path = Vec::new();
        let mut current = Some(source_index);
        while let Some(index) = current {
            if self.locator_elements[index].content_ordinal.is_some() {
                path.push(self.locator_elements[index].element.clone());
            }
            current = self.locator_elements[index].parent;
        }
        DomEventTarget {
            document_epoch: self.epoch,
            target: path[0].clone(),
            target_ordinal: self.locator_elements[source_index]
                .content_ordinal
                .expect("event targets have a document content ordinal"),
            path,
        }
    }

    fn prepare_hover(
        &mut self,
        source_index: usize,
        viewport: ViewportSize,
    ) -> Result<(), (ActionabilityCheck, String)> {
        match self.locator_elements[source_index].visible() {
            Ok(true) => {
                self.locator_elements[source_index]
                    .stable()
                    .map_err(|reason| (ActionabilityCheck::Stable, reason.into()))?;
                self.receives_events(source_index, viewport)
                    .map_err(|reason| (ActionabilityCheck::ReceivesEvents, reason))?;
            }
            Ok(false) => {
                return Err((
                    ActionabilityCheck::Visible,
                    "element is hidden or has an empty box".into(),
                ));
            }
            Err(reason) => {
                return Err((ActionabilityCheck::Visible, reason.into()));
            }
        }
        self.auto_scroll_into_view(source_index, viewport);
        Ok(())
    }

    fn is_hovered(&self, source_index: usize) -> bool {
        self.hovered_source_index == Some(source_index)
    }

    fn scroll(
        &mut self,
        direction: ScrollDirection,
        distance: u64,
        viewport: ViewportSize,
    ) -> PageScroll {
        let previous_x = self.scroll_x;
        let previous_y = self.scroll_y;
        match direction {
            ScrollDirection::Up => self.scroll_y = self.scroll_y.saturating_sub(distance),
            ScrollDirection::Down => {
                self.scroll_y = self
                    .scroll_y
                    .saturating_add(distance)
                    .min(self.max_scroll_y(viewport));
            }
            ScrollDirection::Left => self.scroll_x = self.scroll_x.saturating_sub(distance),
            ScrollDirection::Right => {
                self.scroll_x = self
                    .scroll_x
                    .saturating_add(distance)
                    .min(self.max_scroll_x(viewport));
            }
        }
        self.scroll_result(previous_x, previous_y)
    }

    fn scroll_into_view(
        &mut self,
        source_index: usize,
        viewport: ViewportSize,
    ) -> Result<PageScroll, String> {
        let Some((bounding_box, scrolls_with_document)) =
            self.locator_elements[source_index].document_bounding_box()?
        else {
            return Err("element is hidden or has an empty box".into());
        };
        Ok(self.scroll_box_into_view(bounding_box, scrolls_with_document, viewport))
    }

    fn auto_scroll_into_view(&mut self, source_index: usize, viewport: ViewportSize) {
        let Ok(Some((bounding_box, scrolls_with_document))) =
            self.locator_elements[source_index].document_bounding_box()
        else {
            return;
        };
        self.scroll_box_into_view(bounding_box, scrolls_with_document, viewport);
    }

    fn receives_events(&self, source_index: usize, viewport: ViewportSize) -> Result<(), String> {
        let Some(point) = self.hit_test_point(source_index, viewport)? else {
            return Ok(());
        };
        let Some(hit_index) = self.hit_test_source_at(source_index, point)? else {
            return Err(format!(
                "no supported element receives pointer events at ({}, {})",
                point.x, point.y
            ));
        };
        if hit_index == source_index
            || locator_element_is_descendant(&self.locator_elements, hit_index, source_index)
        {
            return Ok(());
        }
        Err(format!(
            "{} intercepts pointer events at ({}, {})",
            self.locator_elements[hit_index].element, point.x, point.y
        ))
    }

    fn hit_test_point(
        &self,
        source_index: usize,
        viewport: ViewportSize,
    ) -> Result<Option<HitTestPoint>, String> {
        let Ok(Some((target_box, scrolls_with_document))) =
            self.locator_elements[source_index].document_bounding_box()
        else {
            return Ok(None);
        };
        let (scroll_x, scroll_y) =
            self.scroll_offsets_for_box(target_box, scrolls_with_document, viewport);
        let target_box = self.locator_elements[source_index]
            .bounding_box(scroll_x, scroll_y)
            .map_err(str::to_owned)?
            .ok_or_else(|| "element is hidden or has an empty box".to_owned())?;
        let (point_x, point_y) = action_point(target_box, viewport)
            .ok_or_else(|| "element has no action point inside the viewport".to_owned())?;
        Ok(Some(HitTestPoint {
            x: point_x,
            y: point_y,
            scroll_x,
            scroll_y,
            target_layer: hit_test_layer_for_scroll(scrolls_with_document),
        }))
    }

    fn hit_test_source_at(
        &self,
        source_index: usize,
        point: HitTestPoint,
    ) -> Result<Option<usize>, String> {
        let mut hit = None::<(HitTestLayer, usize)>;
        for index in 0..self.locator_elements.len() {
            let Some(layer) = self.hit_test_candidate_layer(source_index, index, point)? else {
                continue;
            };
            if hit.is_none_or(|current| (layer, index) >= current) {
                hit = Some((layer, index));
            }
        }
        Ok(hit.map(|(_, index)| index))
    }

    fn hit_test_candidate_layer(
        &self,
        source_index: usize,
        candidate_index: usize,
        point: HitTestPoint,
    ) -> Result<Option<HitTestLayer>, String> {
        let candidate = &self.locator_elements[candidate_index];
        match candidate.hit_test_candidate(point.scroll_x, point.scroll_y) {
            HitTestCandidate::ReceivesEvents {
                layer,
                bounding_box,
            } => Ok(bounding_box_contains(bounding_box, point.x, point.y).then_some(layer)),
            HitTestCandidate::IgnoresEvents => Ok(None),
            HitTestCandidate::Unsupported {
                layer,
                bounding_box,
                reason,
            } => self.unsupported_hit_test_candidate(
                source_index,
                candidate_index,
                point,
                layer,
                bounding_box,
                reason,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn unsupported_hit_test_candidate(
        &self,
        source_index: usize,
        candidate_index: usize,
        point: HitTestPoint,
        layer: HitTestLayer,
        bounding_box: Option<BoundingBox>,
        reason: &str,
    ) -> Result<Option<HitTestLayer>, String> {
        let same_subtree = candidate_index != source_index
            && (locator_element_is_descendant(
                &self.locator_elements,
                candidate_index,
                source_index,
            ) || locator_element_is_descendant(
                &self.locator_elements,
                source_index,
                candidate_index,
            ));
        let cannot_cover = layer < point.target_layer
            || bounding_box
                .is_some_and(|bounding_box| !bounding_box_contains(bounding_box, point.x, point.y));
        if candidate_index != source_index && (same_subtree || cannot_cover) {
            return Ok(None);
        }
        Err(format!(
            "hit-test evidence for {} is not implemented: {reason}",
            self.locator_elements[candidate_index].element
        ))
    }

    fn scroll_box_into_view(
        &mut self,
        bounding_box: BoundingBox,
        scrolls_with_document: bool,
        viewport: ViewportSize,
    ) -> PageScroll {
        let previous_x = self.scroll_x;
        let previous_y = self.scroll_y;
        (self.scroll_x, self.scroll_y) =
            self.scroll_offsets_for_box(bounding_box, scrolls_with_document, viewport);
        self.scroll_result(previous_x, previous_y)
    }

    fn scroll_offsets_for_box(
        &self,
        bounding_box: BoundingBox,
        scrolls_with_document: bool,
        viewport: ViewportSize,
    ) -> (u64, u64) {
        if !scrolls_with_document {
            return (self.scroll_x, self.scroll_y);
        }
        (
            scroll_axis_into_view(
                self.scroll_x,
                self.max_scroll_x(viewport),
                bounding_box.x,
                bounding_box.width,
                viewport.width,
            ),
            scroll_axis_into_view(
                self.scroll_y,
                self.max_scroll_y(viewport),
                bounding_box.y,
                bounding_box.height,
                viewport.height,
            ),
        )
    }

    fn resize(&mut self, viewport: ViewportSize) -> PageScroll {
        let mut semantics =
            page_semantics_from_html_with_viewport(&self.html, viewport.width, viewport.height);
        assert_eq!(
            semantics.elements.interactive_elements.len(),
            self.interactive_elements.len(),
            "viewport reflow must preserve interactive source identity"
        );
        for (next, current) in semantics
            .elements
            .interactive_elements
            .iter_mut()
            .zip(&self.interactive_elements)
        {
            assert_eq!(
                next.source_index, current.source_index,
                "viewport reflow must preserve interactive source order"
            );
            next.control_state = current.control_state.clone();
        }
        let previous_x = self.scroll_x;
        let previous_y = self.scroll_y;
        self.title = semantics.document.title;
        self.text = semantics.document.text;
        self.locator_elements = semantics.elements.locator_elements;
        self.interactive_elements = semantics.elements.interactive_elements;
        self.accessibility_tree = semantics.document.accessibility_tree;
        self.selector_index = semantics.selector_index;
        self.sequential_focus = semantics.sequential_focus;
        self.document_width = semantics.extent.document_width;
        self.document_height = semantics.extent.document_height;
        self.scroll_x = self.scroll_x.min(self.max_scroll_x(viewport));
        self.scroll_y = self.scroll_y.min(self.max_scroll_y(viewport));
        self.scroll_result(previous_x, previous_y)
    }

    fn max_scroll_x(&self, viewport: ViewportSize) -> u64 {
        self.document_width.saturating_sub(viewport.width)
    }

    fn max_scroll_y(&self, viewport: ViewportSize) -> u64 {
        self.document_height.saturating_sub(viewport.height)
    }

    fn scroll_result(&self, previous_x: u64, previous_y: u64) -> PageScroll {
        PageScroll {
            x: self.scroll_x,
            y: self.scroll_y,
            moved: self.scroll_x != previous_x || self.scroll_y != previous_y,
        }
    }

    fn form_submission_url(
        &self,
        submitter_index: Option<usize>,
        form_owner: usize,
    ) -> Result<String, FormSubmissionError> {
        let submitter_source =
            submitter_index.map(|index| self.source_index_for_interactive(index));
        let submitter = submitter_source.map(|index| &self.locator_elements[index]);
        let form = &self.locator_elements[form_owner];
        let method = submitter
            .and_then(|source| source.attribute("formmethod"))
            .or_else(|| form.attribute("method"))
            .unwrap_or("get");
        if method.eq_ignore_ascii_case("post") || method.eq_ignore_ascii_case("dialog") {
            return Err(FormSubmissionError::Unsupported(format!(
                "form method {method:?} is not implemented"
            )));
        }
        let target = submitter
            .and_then(|source| source.attribute("formtarget"))
            .or_else(|| form.attribute("target"))
            .unwrap_or_default();
        if !target.is_empty() && !target.eq_ignore_ascii_case("_self") {
            return Err(FormSubmissionError::Unsupported(
                "form target browsing contexts are not implemented".into(),
            ));
        }
        let action = submitter
            .and_then(|source| source.attribute("formaction"))
            .or_else(|| form.attribute("action"))
            .unwrap_or_default();
        let target =
            resolve_navigation_url(&self.url, action).map_err(FormSubmissionError::Navigation)?;
        let entries = self.form_entries(form_owner, submitter_source)?;
        form_get_url(&target, &entries).map_err(FormSubmissionError::Navigation)
    }

    fn implicit_submission_url(
        &self,
        field_index: usize,
    ) -> Result<Option<String>, FormSubmissionError> {
        let field_source = self.source_index_for_interactive(field_index);
        let Some(form_owner) = self.locator_elements[field_source].form_owner else {
            return Ok(None);
        };
        if let Some((submitter_source, submitter)) =
            self.locator_elements
                .iter()
                .enumerate()
                .find(|(_, source)| {
                    source.form_owner == Some(form_owner) && source.is_native_submit_button()
                })
        {
            if submitter.is_disabled() || self.has_disabled_fieldset_ancestor(submitter_source) {
                return Ok(None);
            }
            let submitter_index = submitter
                .interactive_index
                .expect("native submit buttons have an interactive index");
            return match &self.interactive_elements[submitter_index].action {
                InteractiveAction::SubmitForm { .. } => self
                    .form_submission_url(Some(submitter_index), form_owner)
                    .map(Some),
                InteractiveAction::Unsupported { reason } => {
                    Err(FormSubmissionError::Unsupported(reason.clone()))
                }
                InteractiveAction::Navigate { .. }
                | InteractiveAction::Activate
                | InteractiveAction::ToggleCheckbox
                | InteractiveAction::SelectRadio => {
                    unreachable!("native submit button action must submit or report unsupported")
                }
            };
        }
        let blocking_fields = self
            .locator_elements
            .iter()
            .filter(|source| {
                source.form_owner == Some(form_owner) && source.blocks_implicit_submission()
            })
            .count();
        if blocking_fields > 1 {
            return Ok(None);
        }
        self.form_submission_url(None, form_owner).map(Some)
    }

    fn form_entries(
        &self,
        form_owner: usize,
        submitter_source: Option<usize>,
    ) -> Result<Vec<(String, String)>, FormSubmissionError> {
        let mut entries = Vec::new();
        for (source_index, source) in self.locator_elements.iter().enumerate() {
            if source.form_owner != Some(form_owner)
                || source.is_disabled()
                || self.has_disabled_fieldset_ancestor(source_index)
            {
                continue;
            }
            let Some(name) = source.attribute("name").filter(|name| !name.is_empty()) else {
                continue;
            };
            let values = self.form_control_values(source_index, submitter_source)?;
            entries.extend(values.into_iter().map(|value| (name.into(), value)));
        }
        Ok(entries)
    }

    fn form_control_values(
        &self,
        source_index: usize,
        submitter_source: Option<usize>,
    ) -> Result<Vec<String>, FormSubmissionError> {
        let source = &self.locator_elements[source_index];
        let submitter = Some(source_index) == submitter_source;
        if let Some(interactive_index) = source.interactive_index {
            return self.interactive_elements[interactive_index]
                .form_values(submitter)
                .map_err(FormSubmissionError::Unsupported);
        }
        if source.tag() == "input"
            && source
                .attribute("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
        {
            return Ok(vec![source.attribute("value").unwrap_or_default().into()]);
        }
        Err(FormSubmissionError::Unsupported(format!(
            "form submission for element {} is not implemented",
            source.element
        )))
    }

    fn has_disabled_fieldset_ancestor(&self, source_index: usize) -> bool {
        let mut candidate = self.locator_elements[source_index].parent;
        while let Some(index) = candidate {
            let element = &self.locator_elements[index];
            if element.is_disabled_fieldset() {
                return true;
            }
            candidate = element.parent;
        }
        false
    }

    fn apply_press(&mut self, key: &KeyboardKey) -> Result<PressEffect, PagePressError> {
        if let Some(direction) = key.focus_traversal_direction() {
            return self.traverse_focus(direction);
        }
        let index = self
            .focused_interactive_index
            .ok_or(PagePressError::NoFocusedElement)?;
        let focused = self.interactive_elements[index].focused_element();
        let action = self.interactive_elements[index].action.clone();
        if matches!(action, InteractiveAction::SelectRadio)
            && let Some(direction) = key.radio_group_direction()
        {
            return self.move_radio_selection(index, direction);
        }
        let activates = matches!(
            (&action, key.control_activation_key()),
            (InteractiveAction::Activate, Some(_))
                | (
                    InteractiveAction::ToggleCheckbox,
                    Some(ControlActivationKey::Space)
                )
                | (
                    InteractiveAction::SelectRadio,
                    Some(ControlActivationKey::Space)
                )
        );
        if activates {
            let effect = apply_native_click(self, index, action.clone()).map_err(|reason| {
                PagePressError::Unsupported {
                    element: focused.element.clone(),
                    reason,
                }
            })?;
            return Ok(match effect {
                NativeClickEffect::Activated => PressEffect::Activated { element: focused },
                NativeClickEffect::Checked { checked, .. } => PressEffect::Checked {
                    element: focused,
                    checked,
                },
            });
        }
        if key.control_activation_key().is_some()
            && self.interactive_elements[index].role() == "button"
            && let InteractiveAction::Unsupported { reason } = &action
        {
            return Err(PagePressError::Unsupported {
                element: focused.element,
                reason: reason.clone(),
            });
        }
        let element = &mut self.interactive_elements[index];
        match element.press_key(key) {
            Ok((value, outcome)) => Ok(PressEffect::Text(TextPressEffect {
                element: focused,
                value,
                selection: outcome.selection,
                changed: outcome.changed,
            })),
            Err(TextValueError::Blocked { reason } | TextValueError::Unsupported { reason }) => {
                Err(PagePressError::Unsupported {
                    element: focused.element,
                    reason,
                })
            }
        }
    }

    fn apply_keyboard_text(&mut self, text: &str) -> KeyboardTextEffect {
        let Some(index) = self.focused_interactive_index else {
            return KeyboardTextEffect::Ignored { element: None };
        };
        let focused = self.interactive_elements[index].focused_element();
        self.interactive_elements[index].insert_text(text).map_or(
            KeyboardTextEffect::Ignored {
                element: Some(focused.clone()),
            },
            |(value, outcome)| {
                KeyboardTextEffect::Text(TextPressEffect {
                    element: focused,
                    value,
                    selection: outcome.selection,
                    changed: outcome.changed,
                })
            },
        )
    }

    fn apply_keyboard_type(&mut self, text: &str) -> KeyboardTextEffect {
        let Some(index) = self.focused_interactive_index else {
            return KeyboardTextEffect::Ignored { element: None };
        };
        let focused = self.interactive_elements[index].focused_element();
        self.interactive_elements[index].type_text(text).map_or(
            KeyboardTextEffect::Ignored {
                element: Some(focused.clone()),
            },
            |(value, outcome)| {
                KeyboardTextEffect::Text(TextPressEffect {
                    element: focused,
                    value,
                    selection: outcome.selection,
                    changed: outcome.changed,
                })
            },
        )
    }

    fn move_radio_selection(
        &mut self,
        index: usize,
        direction: RadioGroupDirection,
    ) -> Result<PressEffect, PagePressError> {
        let ControlState::Radio(current) = &self.interactive_elements[index].control_state else {
            unreachable!("radio arrow movement starts from a radio")
        };
        let group = current.group.clone();
        let mut candidates = Vec::new();
        for (candidate, element) in self.interactive_elements.iter().enumerate() {
            let ControlState::Radio(state) = &element.control_state else {
                continue;
            };
            if state.group != group || element.enabled() == Some(false) {
                continue;
            }
            match element.visible() {
                Ok(true) => candidates.push(candidate),
                Ok(false) => {}
                Err(reason) => {
                    return Err(PagePressError::Unsupported {
                        element: self.interactive_elements[index].element().into(),
                        reason: format!("radio arrow visibility is unavailable: {reason}"),
                    });
                }
            }
        }
        let position = candidates
            .iter()
            .position(|candidate| *candidate == index)
            .ok_or_else(|| PagePressError::Unsupported {
                element: self.interactive_elements[index].element().into(),
                reason: "focused radio is not an eligible group member".into(),
            })?;
        let next_position = match direction {
            RadioGroupDirection::Previous => {
                position.checked_sub(1).unwrap_or(candidates.len() - 1)
            }
            RadioGroupDirection::Next => (position + 1) % candidates.len(),
        };
        let target = candidates[next_position];
        self.set_checked(target, true)
            .map_err(|error| PagePressError::Unsupported {
                element: self.interactive_elements[index].element().into(),
                reason: error.reason(),
            })?;
        self.focused_interactive_index = Some(target);
        Ok(PressEffect::Checked {
            element: self.interactive_elements[target].focused_element(),
            checked: true,
        })
    }

    fn set_checked(
        &mut self,
        index: usize,
        replacement: bool,
    ) -> Result<bool, CheckedMutationError> {
        self.validate_set_checked(index, replacement)?;
        if let ControlState::Checkbox(state) = &mut self.interactive_elements[index].control_state {
            state.set_checked(replacement);
            return Ok(replacement);
        }
        let group = match &self.interactive_elements[index].control_state {
            ControlState::Radio(state) => state.group.clone(),
            ControlState::Text(_)
            | ControlState::Checkbox(_)
            | ControlState::Select(_)
            | ControlState::Unavailable => unreachable!("checked mutation was validated"),
        };
        if !replacement {
            return Ok(false);
        }
        for element in &mut self.interactive_elements {
            if let ControlState::Radio(state) = &mut element.control_state
                && state.group == group
            {
                state.set_checked(false);
            }
        }
        let ControlState::Radio(state) = &mut self.interactive_elements[index].control_state else {
            unreachable!("radio target remains a radio during checked-state mutation")
        };
        state.set_checked(true);
        self.update_radio_focus_order(&group, index);
        Ok(true)
    }

    fn validate_set_checked(
        &self,
        index: usize,
        replacement: bool,
    ) -> Result<bool, CheckedMutationError> {
        match &self.interactive_elements[index].control_state {
            ControlState::Checkbox(state) => {
                if let Some(reason) = state.block_reason() {
                    return Err(CheckedMutationError::Blocked {
                        reason: reason.into(),
                    });
                }
                Ok(state.checked())
            }
            ControlState::Radio(state) => {
                if let Some(reason) = state.block_reason() {
                    return Err(CheckedMutationError::Blocked {
                        reason: reason.into(),
                    });
                }
                if !replacement && state.checked() {
                    return Err(CheckedMutationError::Unsupported {
                        reason: "checked radios cannot be unchecked by activation".into(),
                    });
                }
                Ok(state.checked())
            }
            ControlState::Text(_) | ControlState::Select(_) | ControlState::Unavailable => {
                Err(CheckedMutationError::Unsupported {
                    reason: format!(
                        "checked-state mutation for role {} is not implemented",
                        self.interactive_elements[index].role()
                    ),
                })
            }
        }
    }

    fn update_radio_focus_order(&mut self, group: &crate::page::RadioGroup, index: usize) {
        if self.interactive_elements[index].visible() != Ok(true)
            || self.interactive_elements[index].enabled() == Some(false)
        {
            return;
        }
        let SequentialFocusSource::Supported { order } = &self.sequential_focus else {
            return;
        };
        let represented = order.iter().any(|candidate| {
            matches!(
                &self.interactive_elements[*candidate].control_state,
                ControlState::Radio(state) if &state.group == group
            )
        });
        if !represented {
            return;
        }
        let mut updated = order.clone();
        updated.retain(|candidate| {
            !matches!(
                &self.interactive_elements[*candidate].control_state,
                ControlState::Radio(state) if &state.group == group
            )
        });
        let insertion = updated
            .iter()
            .position(|candidate| {
                !self.interactive_elements[*candidate].has_positive_tabindex() && *candidate > index
            })
            .unwrap_or(updated.len());
        updated.insert(insertion, index);
        self.sequential_focus = SequentialFocusSource::Supported { order: updated };
    }

    fn traverse_focus(
        &mut self,
        direction: FocusTraversalDirection,
    ) -> Result<PressEffect, PagePressError> {
        let order = match &self.sequential_focus {
            SequentialFocusSource::Supported { order } => order,
            SequentialFocusSource::Unsupported { reason } => {
                return Err(PagePressError::Unsupported {
                    element: self
                        .focused_interactive_index
                        .map(|index| self.interactive_elements[index].element().to_owned())
                        .unwrap_or_else(|| "document body".into()),
                    reason: reason.clone(),
                });
            }
        };
        let previous_index = self.focused_interactive_index;
        let current_index = match previous_index {
            None => match direction {
                FocusTraversalDirection::Forward => order.first().copied(),
                FocusTraversalDirection::Reverse => order.last().copied(),
            },
            Some(previous) => focus_after(order, previous, direction),
        };
        let previous =
            previous_index.map(|index| self.interactive_elements[index].focused_element());
        let current = current_index.map(|index| self.interactive_elements[index].focused_element());
        self.focused_interactive_index = current_index;
        Ok(PressEffect::FocusTraversal(FocusTraversalEffect {
            previous,
            current,
        }))
    }
}

fn scroll_axis_into_view(
    current: u64,
    maximum: u64,
    origin: i64,
    size: u64,
    viewport_size: u64,
) -> u64 {
    let current = i128::from(current);
    let start = i128::from(origin);
    let end = start + i128::from(size);
    let viewport_end = current + i128::from(viewport_size);
    let target = if start >= current && end <= viewport_end {
        current
    } else if size > viewport_size || start < current {
        start
    } else {
        end - i128::from(viewport_size)
    };
    u64::try_from(target.max(0))
        .unwrap_or(u64::MAX)
        .min(maximum)
}

fn action_point(bounding_box: BoundingBox, viewport: ViewportSize) -> Option<(i64, i64)> {
    let left = i128::from(bounding_box.x).max(0);
    let top = i128::from(bounding_box.y).max(0);
    let right = (i128::from(bounding_box.x) + i128::from(bounding_box.width))
        .min(i128::from(viewport.width));
    let bottom = (i128::from(bounding_box.y) + i128::from(bounding_box.height))
        .min(i128::from(viewport.height));
    if left >= right || top >= bottom {
        return None;
    }
    Some((
        i64::try_from((left + right) / 2).ok()?,
        i64::try_from((top + bottom) / 2).ok()?,
    ))
}

fn hit_test_layer_for_scroll(scrolls_with_document: bool) -> HitTestLayer {
    if scrolls_with_document {
        HitTestLayer::Normal
    } else {
        HitTestLayer::Fixed
    }
}

fn bounding_box_contains(bounding_box: BoundingBox, x: i64, y: i64) -> bool {
    let x = i128::from(x);
    let y = i128::from(y);
    let left = i128::from(bounding_box.x);
    let top = i128::from(bounding_box.y);
    x >= left
        && x < left + i128::from(bounding_box.width)
        && y >= top
        && y < top + i128::from(bounding_box.height)
}

fn focus_after(
    order: &[usize],
    current: usize,
    direction: FocusTraversalDirection,
) -> Option<usize> {
    if let Some(position) = order.iter().position(|candidate| *candidate == current) {
        return match direction {
            FocusTraversalDirection::Forward => order.get(position + 1).copied(),
            FocusTraversalDirection::Reverse => position
                .checked_sub(1)
                .and_then(|previous| order.get(previous))
                .copied(),
        };
    }
    match direction {
        FocusTraversalDirection::Forward => order
            .iter()
            .copied()
            .filter(|candidate| *candidate > current)
            .min(),
        FocusTraversalDirection::Reverse => order
            .iter()
            .copied()
            .filter(|candidate| *candidate < current)
            .max(),
    }
}

fn locator_element_is_descendant(
    elements: &[LocatorElementSource],
    mut candidate: usize,
    ancestor: usize,
) -> bool {
    while let Some(parent) = elements[candidate].parent {
        if parent == ancestor {
            return true;
        }
        candidate = parent;
    }
    false
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenPage {
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenedPage {
    pub url: String,
    pub interactive_element_count: usize,
}

impl private::Sealed for OpenPage {}

impl SessionRequest for OpenPage {
    type Reply = OpenedPage;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.navigate_to(self.url).map_err(SessionError::Load)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReloadPage;

impl private::Sealed for ReloadPage {}

impl SessionRequest for ReloadPage {
    type Reply = OpenedPage;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let url = session
            .current_page
            .as_ref()
            .ok_or(SessionError::NoPage)?
            .url
            .clone();
        session.load_page(url).map_err(SessionError::Load)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoBack;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoForward;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryNavigationResult {
    Navigated(OpenedPage),
    NoEntry { current_url: String },
}

impl private::Sealed for GoBack {}

impl SessionRequest for GoBack {
    type Reply = HistoryNavigationResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let target = session.history.previous();
        session.navigate_history(target)
    }
}

impl private::Sealed for GoForward {}

impl SessionRequest for GoForward {
    type Reply = HistoryNavigationResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let target = session.history.next();
        session.navigate_history(target)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetPageUrl;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageUrl {
    pub url: String,
}

impl private::Sealed for GetPageUrl {}

impl SessionRequest for GetPageUrl {
    type Reply = PageUrl;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(PageUrl {
            url: page.url.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetPageText;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageText {
    pub text: String,
}

impl private::Sealed for GetPageText {}

impl SessionRequest for GetPageText {
    type Reply = PageText;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(PageText {
            text: page.text.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetPageTitle;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageTitle {
    pub title: String,
}

impl private::Sealed for GetPageTitle {}

impl SessionRequest for GetPageTitle {
    type Reply = PageTitle;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(PageTitle {
            title: page.title.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureInteractiveSnapshot;

impl private::Sealed for CaptureInteractiveSnapshot {}

impl SessionRequest for CaptureInteractiveSnapshot {
    type Reply = InteractiveSnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let element_indices = (0..session
            .current_page
            .as_ref()
            .ok_or(SessionError::NoPage)?
            .interactive_elements
            .len())
            .collect();
        Ok(capture_interactive_snapshot(session, element_indices))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureInteractiveSnapshotWithin {
    pub locator: Locator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureAccessibilitySnapshot {
    pub options: AccessibilitySnapshotOptions,
}

impl private::Sealed for CaptureAccessibilitySnapshot {}

impl SessionRequest for CaptureAccessibilitySnapshot {
    type Reply = AccessibilitySnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        Ok(capture_accessibility_snapshot(session, None, self.options))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureAccessibilitySnapshotWithin {
    pub locator: Locator,
    pub options: AccessibilitySnapshotOptions,
}

impl private::Sealed for CaptureAccessibilitySnapshotWithin {}

impl SessionRequest for CaptureAccessibilitySnapshotWithin {
    type Reply = AccessibilitySnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = self.locator;
        let resolved = session
            .locator_match_for(&locator)
            .map_err(|error| locator_session_error(locator, error))?;
        Ok(capture_accessibility_snapshot(
            session,
            Some(resolved.source_index),
            self.options,
        ))
    }
}

fn capture_accessibility_snapshot(
    session: &mut Session,
    scope_source_index: Option<usize>,
    options: AccessibilitySnapshotOptions,
) -> AccessibilitySnapshot {
    let page = session
        .current_page
        .as_ref()
        .expect("snapshot capture requires a current page");
    let source_nodes = page
        .accessibility_tree
        .iter()
        .filter(|node| {
            let Some(scope) = scope_source_index else {
                return true;
            };
            let Some(owner_source_index) = node.origin.owner_source_index() else {
                return false;
            };
            owner_source_index == scope
                || locator_element_is_descendant(&page.locator_elements, owner_source_index, scope)
        })
        .cloned()
        .collect::<Vec<_>>();
    let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
    let snapshot = AccessibilitySnapshot::from_nodes(
        SnapshotCaptureIdentity {
            id: snapshot_id,
            document_epoch: page.epoch,
            url: page.url.clone(),
        },
        &source_nodes,
        &page.interactive_elements,
        options,
    );
    let mut element_indices = vec![None; page.interactive_elements.len()];
    for reference in snapshot.nodes.iter().filter_map(|node| node.reference) {
        let index = usize::try_from(reference.ordinal() - 1)
            .expect("reference ordinals fit the interactive element index");
        element_indices[index] = Some(index);
    }
    session.latest_interactive_snapshot = Some(LatestInteractiveSnapshot {
        id: snapshot_id,
        element_indices,
    });
    snapshot
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrepareScreenshot {
    pub target: CaptureTarget,
}

impl private::Sealed for PrepareScreenshot {}

impl SessionRequest for PrepareScreenshot {
    type Reply = PreparedScreenshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let target = self.target;
        let viewport = session.viewport;
        let (capture_bounds, fixed_offset_x, fixed_offset_y) = match &target {
            CaptureTarget::Viewport => {
                let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
                (
                    screenshot_rect(
                        page.scroll_x,
                        page.scroll_y,
                        viewport.width,
                        viewport.height,
                    )
                    .map_err(|reason| SessionError::UnsupportedScreenshot {
                        target: target.clone(),
                        reason,
                    })?,
                    page.scroll_x,
                    page.scroll_y,
                )
            }
            CaptureTarget::FullPage => {
                let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
                (
                    screenshot_rect(0, 0, page.document_width, page.document_height).map_err(
                        |reason| SessionError::UnsupportedScreenshot {
                            target: target.clone(),
                            reason,
                        },
                    )?,
                    0,
                    0,
                )
            }
            CaptureTarget::Element(locator) => {
                let resolved = session
                    .locator_match_for(locator)
                    .map_err(|error| locator_session_error(locator.clone(), error))?;
                {
                    let page = session.current_page.as_mut().ok_or(SessionError::NoPage)?;
                    page.scroll_into_view(resolved.source_index, viewport)
                        .map_err(|reason| SessionError::UnsupportedScreenshot {
                            target: target.clone(),
                            reason,
                        })?;
                }
                let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
                let Some((bounds, scrolls_with_document)) = page.locator_elements
                    [resolved.source_index]
                    .document_bounding_box()
                    .map_err(|reason| SessionError::UnsupportedScreenshot {
                        target: target.clone(),
                        reason: reason.into(),
                    })?
                else {
                    return Err(SessionError::UnsupportedScreenshot {
                        target: target.clone(),
                        reason: "element is hidden or has an empty box".into(),
                    });
                };
                let (x, y) = if scrolls_with_document {
                    (Some(bounds.x), Some(bounds.y))
                } else {
                    (
                        add_screenshot_offset(bounds.x, page.scroll_x),
                        add_screenshot_offset(bounds.y, page.scroll_y),
                    )
                };
                (
                    CaptureRect::new(
                        x.ok_or_else(|| SessionError::UnsupportedScreenshot {
                            target: target.clone(),
                            reason: "element screenshot x coordinate overflows".into(),
                        })?,
                        y.ok_or_else(|| SessionError::UnsupportedScreenshot {
                            target: target.clone(),
                            reason: "element screenshot y coordinate overflows".into(),
                        })?,
                        bounds.width,
                        bounds.height,
                    )
                    .map_err(|error| SessionError::UnsupportedScreenshot {
                        target: target.clone(),
                        reason: error.to_string(),
                    })?,
                    page.scroll_x,
                    page.scroll_y,
                )
            }
            CaptureTarget::Rect(bounds) => {
                let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
                (*bounds, page.scroll_x, page.scroll_y)
            }
        };
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let commands = paint_commands_from_html(
            &page.html,
            viewport.width,
            capture_bounds,
            fixed_offset_x,
            fixed_offset_y,
        )
        .map_err(|reason| SessionError::UnsupportedScreenshot {
            target: target.clone(),
            reason,
        })?;
        Ok(PreparedScreenshot {
            target,
            scene: PaintScene {
                capture_bounds,
                commands,
            },
        })
    }
}

fn screenshot_rect(x: u64, y: u64, width: u64, height: u64) -> Result<CaptureRect, String> {
    let x = i64::try_from(x).map_err(|_| "screenshot x coordinate exceeds limits")?;
    let y = i64::try_from(y).map_err(|_| "screenshot y coordinate exceeds limits")?;
    CaptureRect::new(x, y, width, height).map_err(|error| error.to_string())
}

fn add_screenshot_offset(value: i64, offset: u64) -> Option<i64> {
    value.checked_add(i64::try_from(offset).ok()?)
}

impl private::Sealed for CaptureInteractiveSnapshotWithin {}

impl SessionRequest for CaptureInteractiveSnapshotWithin {
    type Reply = InteractiveSnapshot;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = self.locator;
        let resolved = match session.locator_match_for(&locator) {
            Ok(resolved) => resolved,
            Err(error) => return Err(locator_session_error(locator, error)),
        };
        let page = session
            .current_page
            .as_ref()
            .expect("resolved locator requires a current page");
        let element_indices = page
            .locator_elements
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                *index == resolved.source_index
                    || locator_element_is_descendant(
                        &page.locator_elements,
                        *index,
                        resolved.source_index,
                    )
            })
            .filter_map(|(_, element)| element.interactive_index)
            .collect();
        Ok(capture_interactive_snapshot(session, element_indices))
    }
}

fn capture_interactive_snapshot(
    session: &mut Session,
    element_indices: Vec<usize>,
) -> InteractiveSnapshot {
    let page = session
        .current_page
        .as_ref()
        .expect("snapshot capture requires a current page");
    let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
    let element_depths = interactive_snapshot_depths(page, &element_indices);
    let snapshot = InteractiveSnapshot::from_document_indices(
        SnapshotCaptureIdentity {
            id: snapshot_id,
            document_epoch: page.epoch,
            url: page.url.clone(),
        },
        &page.interactive_elements,
        &element_indices,
        &element_depths,
    );
    session.latest_interactive_snapshot = Some(LatestInteractiveSnapshot {
        id: snapshot_id,
        element_indices: reference_element_indices(
            page.interactive_elements.len(),
            &element_indices,
        ),
    });
    snapshot
}

fn interactive_snapshot_depths(page: &CurrentPage, element_indices: &[usize]) -> Vec<u64> {
    element_indices
        .iter()
        .map(|interactive_index| {
            let mut depth = 0_u64;
            let mut parent = page.locator_elements
                [page.interactive_elements[*interactive_index].source_index]
                .parent;
            while let Some(source_index) = parent {
                if page.locator_elements[source_index]
                    .interactive_index
                    .is_some_and(|index| element_indices.contains(&index))
                {
                    depth = depth.saturating_add(1);
                }
                parent = page.locator_elements[source_index].parent;
            }
            depth
        })
        .collect()
}

fn reference_element_indices(
    element_count: usize,
    included_indices: &[usize],
) -> Vec<Option<usize>> {
    let mut references = vec![None; element_count];
    for &index in included_indices {
        references[index] = Some(index);
    }
    references
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindByRole {
    pub locator: RoleLocator,
}

impl private::Sealed for FindByRole {}

impl SessionRequest for FindByRole {
    type Reply = RoleMatch;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match session.locator_match_for(&locator) {
            Ok(resolved) => Ok(resolved.matched.into_role_match()),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorAction {
    Click,
    Fill,
    Type,
    Focus,
    Press,
    Select,
    Check,
    Uncheck,
    Hover,
    ScrollIntoView,
}

pub type RoleAction = LocatorAction;

impl std::fmt::Display for LocatorAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Click => "click",
            Self::Fill => "fill",
            Self::Type => "type",
            Self::Focus => "focus",
            Self::Press => "press",
            Self::Select => "select",
            Self::Check => "check",
            Self::Uncheck => "uncheck",
            Self::Hover => "hover",
            Self::ScrollIntoView => "scroll into view",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportSize {
    pub width: u64,
    pub height: u64,
}

impl Default for ViewportSize {
    fn default() -> Self {
        Self {
            width: DEFAULT_VIEWPORT_WIDTH,
            height: DEFAULT_VIEWPORT_HEIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetViewportSize {
    pub width: u64,
    pub height: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetViewportSize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportResize {
    pub viewport: ViewportSize,
    pub resized: bool,
    pub scroll: PageScroll,
}

impl private::Sealed for SetViewportSize {}

impl SessionRequest for SetViewportSize {
    type Reply = ViewportResize;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        if self.width == 0 || self.height == 0 {
            return Err(SessionError::InvalidViewportSize {
                width: self.width,
                height: self.height,
            });
        }
        let viewport = ViewportSize {
            width: self.width,
            height: self.height,
        };
        let resized = session.viewport != viewport;
        session.viewport = viewport;
        let scroll = session.current_page.as_mut().map_or(
            PageScroll {
                x: 0,
                y: 0,
                moved: false,
            },
            |page| page.resize(viewport),
        );
        Ok(ViewportResize {
            viewport,
            resized,
            scroll,
        })
    }
}

impl private::Sealed for GetViewportSize {}

impl SessionRequest for GetViewportSize {
    type Reply = ViewportSize;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        Ok(session.viewport)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollPage {
    pub direction: ScrollDirection,
    pub distance: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageScroll {
    pub x: u64,
    pub y: u64,
    pub moved: bool,
}

impl private::Sealed for ScrollPage {}

impl SessionRequest for ScrollPage {
    type Reply = PageScroll;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let viewport = session.viewport;
        let page = session.current_page.as_mut().ok_or(SessionError::NoPage)?;
        Ok(page.scroll(self.direction, self.distance, viewport))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionabilityCheck {
    Visible,
    Stable,
    ReceivesEvents,
    Enabled,
    Editable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorInspection {
    BoundingBox,
    Html,
    Value,
    Checked,
    Enabled,
    Editable,
    Visible,
}

impl std::fmt::Display for LocatorInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BoundingBox => "bounding box",
            Self::Html => "HTML",
            Self::Value => "value",
            Self::Checked => "checked state",
            Self::Enabled => "enabled state",
            Self::Editable => "editable state",
            Self::Visible => "visibility",
        })
    }
}

impl std::fmt::Display for ActionabilityCheck {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Visible => "visible",
            Self::Stable => "stable",
            Self::ReceivesEvents => "receives events",
            Self::Enabled => "enabled",
            Self::Editable => "editable",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindAllByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorMatches {
    pub matches: Vec<LocatorMatch>,
}

impl private::Sealed for FindAllByLocator {}

impl SessionRequest for FindAllByLocator {
    type Reply = LocatorMatches;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_matches_for(&self.locator) {
            Ok(matches) => Ok(LocatorMatches {
                matches: matches
                    .into_iter()
                    .map(|resolved| resolved.matched)
                    .collect(),
            }),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountByLocator {
    pub locator: Locator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorCount {
    pub count: usize,
}

impl private::Sealed for CountByLocator {}

impl SessionRequest for CountByLocator {
    type Reply = LocatorCount;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_matches_for(&self.locator) {
            Ok(matches) => Ok(LocatorCount {
                count: matches.len(),
            }),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetHtmlByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetBoundingBoxByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorBoundingBox {
    pub matched: LocatorMatch,
    pub value: Option<BoundingBox>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScrollIntoViewByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorScroll {
    pub matched: LocatorMatch,
    pub scroll: PageScroll,
}

impl private::Sealed for ScrollIntoViewByLocator {}

impl SessionRequest for ScrollIntoViewByLocator {
    type Reply = LocatorScroll;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_scroll_into_view_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for GetBoundingBoxByLocator {}

impl SessionRequest for GetBoundingBoxByLocator {
    type Reply = LocatorBoundingBox;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_bounding_box_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorHtml {
    pub matched: LocatorMatch,
    pub html: String,
}

impl private::Sealed for GetHtmlByLocator {}

impl SessionRequest for GetHtmlByLocator {
    type Reply = LocatorHtml;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_html_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetValueByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorValue {
    pub matched: LocatorMatch,
    pub value: String,
}

impl private::Sealed for GetValueByLocator {}

impl SessionRequest for GetValueByLocator {
    type Reply = LocatorValue;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_value_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAttributeByLocator {
    pub locator: Locator,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorAttribute {
    pub matched: LocatorMatch,
    pub name: String,
    pub value: Option<String>,
}

impl private::Sealed for GetAttributeByLocator {}

impl SessionRequest for GetAttributeByLocator {
    type Reply = LocatorAttribute;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let name = normalize_attribute_name(self.name)?;
        match execute_get_attribute_by_locator(session, &self.locator, name) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetCheckedByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorChecked {
    pub matched: LocatorMatch,
    pub checked: bool,
}

impl private::Sealed for GetCheckedByLocator {}

impl SessionRequest for GetCheckedByLocator {
    type Reply = LocatorChecked;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_checked_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetEnabledByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorEnabled {
    pub matched: LocatorMatch,
    pub enabled: bool,
}

impl private::Sealed for GetEnabledByLocator {}

impl SessionRequest for GetEnabledByLocator {
    type Reply = LocatorEnabled;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_enabled_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetEditableByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorEditable {
    pub matched: LocatorMatch,
    pub editable: bool,
}

impl private::Sealed for GetEditableByLocator {}

impl SessionRequest for GetEditableByLocator {
    type Reply = LocatorEditable;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_editable_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetFocusedByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorFocused {
    pub matched: LocatorMatch,
    pub focused: bool,
}

impl private::Sealed for GetFocusedByLocator {}

impl SessionRequest for GetFocusedByLocator {
    type Reply = LocatorFocused;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_focused_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetHoveredByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorHovered {
    pub matched: LocatorMatch,
    pub hovered: bool,
}

impl private::Sealed for GetHoveredByLocator {}

impl SessionRequest for GetHoveredByLocator {
    type Reply = LocatorHovered;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_hovered_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetVisibleByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatorVisible {
    pub matched: LocatorMatch,
    pub visible: bool,
}

impl private::Sealed for GetVisibleByLocator {}

impl SessionRequest for GetVisibleByLocator {
    type Reply = LocatorVisible;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_get_visible_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for FindByLocator {}

impl SessionRequest for FindByLocator {
    type Reply = LocatorMatch;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match session.locator_match_for(&self.locator) {
            Ok(resolved) => Ok(resolved.matched),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickByLocatorResult {
    Navigated {
        matched: LocatorMatch,
        page: OpenedPage,
    },
    Activated {
        matched: LocatorMatch,
    },
    Checked {
        matched: LocatorMatch,
        checked: bool,
    },
}

impl private::Sealed for ClickByLocator {}

impl SessionRequest for ClickByLocator {
    type Reply = ClickByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_click_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByLocator {
    pub locator: Locator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByLocatorResult {
    pub matched: LocatorMatch,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeByLocator {
    pub locator: Locator,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeByLocatorResult {
    pub matched: LocatorMatch,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusByLocatorResult {
    pub matched: LocatorMatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressByLocator {
    pub locator: Locator,
    pub key: KeyboardKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressByLocatorResult {
    pub matched: LocatorMatch,
    pub press: PressResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectByLocator {
    pub locator: Locator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectByLocatorResult {
    pub matched: LocatorMatch,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsByLocator {
    pub locator: Locator,
    pub options: NonEmpty<SelectOptionTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsByLocatorResult {
    pub matched: LocatorMatch,
    pub selected: NonEmpty<String>,
}

impl private::Sealed for SelectByLocator {}

impl SessionRequest for SelectByLocator {
    type Reply = SelectByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_select_by_locator(session, &self.locator, self.value) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for SelectOptionsByLocator {}

impl SessionRequest for SelectOptionsByLocator {
    type Reply = SelectOptionsByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_select_options_by_locator(session, &self.locator, self.options) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for FillByLocator {}

impl SessionRequest for FillByLocator {
    type Reply = FillByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_fill_by_locator(session, &self.locator, self.value) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for TypeByLocator {}

impl SessionRequest for TypeByLocator {
    type Reply = TypeByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_type_by_locator(session, &self.locator, &self.text) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for FocusByLocator {}

impl SessionRequest for FocusByLocator {
    type Reply = FocusByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_focus_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

impl private::Sealed for PressByLocator {}

impl SessionRequest for PressByLocator {
    type Reply = PressByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_press_by_locator(session, &self.locator, self.key) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByLocator {
    pub locator: Locator,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByLocatorResult {
    pub matched: LocatorMatch,
    pub checked: bool,
}

impl private::Sealed for SetCheckedByLocator {}

impl SessionRequest for SetCheckedByLocator {
    type Reply = SetCheckedByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_set_checked_by_locator(session, &self.locator, self.checked) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByLocator {
    pub locator: Locator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByLocatorResult {
    pub matched: LocatorMatch,
}

impl private::Sealed for HoverByLocator {}

impl SessionRequest for HoverByLocator {
    type Reply = HoverByLocatorResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        match execute_hover_by_locator(session, &self.locator) {
            Ok(result) => Ok(result),
            Err(error) => Err(locator_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickByRole {
    pub locator: RoleLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickByRoleResult {
    Navigated {
        matched: RoleMatch,
        page: OpenedPage,
    },
    Activated {
        matched: RoleMatch,
    },
    Checked {
        matched: RoleMatch,
        checked: bool,
    },
}

impl private::Sealed for ClickByRole {}

impl SessionRequest for ClickByRole {
    type Reply = ClickByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_click_by_locator(session, &locator) {
            Ok(ClickByLocatorResult::Navigated { matched, page }) => {
                Ok(ClickByRoleResult::Navigated {
                    matched: matched.into_role_match(),
                    page,
                })
            }
            Ok(ClickByLocatorResult::Activated { matched }) => Ok(ClickByRoleResult::Activated {
                matched: matched.into_role_match(),
            }),
            Ok(ClickByLocatorResult::Checked { matched, checked }) => {
                Ok(ClickByRoleResult::Checked {
                    matched: matched.into_role_match(),
                    checked,
                })
            }
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByRole {
    pub locator: RoleLocator,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillByRoleResult {
    pub matched: RoleMatch,
    pub value: String,
}

impl private::Sealed for FillByRole {}

impl SessionRequest for FillByRole {
    type Reply = FillByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_fill_by_locator(session, &locator, self.value) {
            Ok(result) => Ok(FillByRoleResult {
                matched: result.matched.into_role_match(),
                value: result.value,
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByRole {
    pub locator: RoleLocator,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedByRoleResult {
    pub matched: RoleMatch,
    pub checked: bool,
}

impl private::Sealed for SetCheckedByRole {}

impl SessionRequest for SetCheckedByRole {
    type Reply = SetCheckedByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_set_checked_by_locator(session, &locator, self.checked) {
            Ok(result) => Ok(SetCheckedByRoleResult {
                matched: result.matched.into_role_match(),
                checked: result.checked,
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByRole {
    pub locator: RoleLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverByRoleResult {
    pub matched: RoleMatch,
}

impl private::Sealed for HoverByRole {}

impl SessionRequest for HoverByRole {
    type Reply = HoverByRoleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let locator = Locator::from(self.locator.clone());
        match execute_hover_by_locator(session, &locator) {
            Ok(result) => Ok(HoverByRoleResult {
                matched: result.matched.into_role_match(),
            }),
            Err(error) => Err(role_session_error(self.locator, error)),
        }
    }
}

fn execute_get_value_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorValue, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = resolved.interactive_index.ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Value,
            "matched element has no implemented value state",
        )
    })?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    let value = element.value().ok_or_else(|| {
        let reason = format!(
            "value inspection for role {} is not implemented",
            element.role()
        );
        LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Value,
            reason,
        }
    })?;
    Ok(LocatorValue {
        matched: resolved.matched,
        value: value.into(),
    })
}

fn execute_get_html_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorHtml, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let page = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page");
    if page
        .selector_index
        .inner_html_contains_sensitive_value(resolved.source_index)?
    {
        return Err(LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Html,
            reason: "inner HTML contains a password value attribute".into(),
        });
    }
    let html = page.selector_index.inner_html(resolved.source_index)?;
    Ok(LocatorHtml {
        matched: resolved.matched,
        html,
    })
}

fn execute_get_attribute_by_locator(
    session: &Session,
    locator: &Locator,
    name: String,
) -> Result<LocatorAttribute, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    if element.attribute_is_sensitive(&name) {
        return Err(LocatorOperationError::SensitiveAttribute { name });
    }
    Ok(LocatorAttribute {
        matched: resolved.matched,
        value: element.attribute(&name).map(str::to_owned),
        name,
    })
}

fn execute_get_checked_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorChecked, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = resolved.interactive_index.ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Checked,
            "matched element has no implemented checked state",
        )
    })?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    let checked = element.checked().ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Checked,
            &format!(
                "checked-state inspection for role {} is not implemented",
                element.role()
            ),
        )
    })?;
    Ok(LocatorChecked {
        matched: resolved.matched,
        checked,
    })
}

fn execute_get_enabled_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorEnabled, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    let enabled = element.enabled().ok_or_else(|| {
        unsupported_locator_inspection(
            &resolved,
            LocatorInspection::Enabled,
            "matched element has no implemented native enabled state",
        )
    })?;
    Ok(LocatorEnabled {
        matched: resolved.matched,
        enabled,
    })
}

fn execute_get_editable_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorEditable, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let editable = session
        .editable_state(resolved.source_index)
        .map_err(|reason| LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Editable,
            reason,
        })?
        .ok_or_else(|| {
            unsupported_locator_inspection(
                &resolved,
                LocatorInspection::Editable,
                "matched element has no implemented editable state",
            )
        })?;
    Ok(LocatorEditable {
        matched: resolved.matched,
        editable,
    })
}

fn execute_get_focused_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorFocused, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let page = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page");
    let element = &page.locator_elements[resolved.source_index];
    let focused = (element.is_body() && page.focused_interactive_index.is_none())
        || resolved
            .interactive_index
            .is_some_and(|index| page.focused_interactive_index == Some(index));
    Ok(LocatorFocused {
        matched: resolved.matched,
        focused,
    })
}

fn execute_get_hovered_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorHovered, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let hovered = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .is_hovered(resolved.source_index);
    Ok(LocatorHovered {
        matched: resolved.matched,
        hovered,
    })
}

fn execute_get_visible_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorVisible, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .locator_elements[resolved.source_index];
    let visible = element
        .visible()
        .map_err(|reason| LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::Visible,
            reason: reason.into(),
        })?;
    Ok(LocatorVisible {
        matched: resolved.matched,
        visible,
    })
}

fn execute_get_bounding_box_by_locator(
    session: &Session,
    locator: &Locator,
) -> Result<LocatorBoundingBox, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let page = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page");
    let element = &page.locator_elements[resolved.source_index];
    let value = element
        .bounding_box(page.scroll_x, page.scroll_y)
        .map_err(|reason| LocatorOperationError::InspectionBlocked {
            inspection: LocatorInspection::BoundingBox,
            reason: reason.into(),
        })?;
    Ok(LocatorBoundingBox {
        matched: resolved.matched,
        value,
    })
}

fn execute_scroll_into_view_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<LocatorScroll, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let viewport = session.viewport;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let scroll = page
        .scroll_into_view(resolved.source_index, viewport)
        .map_err(|reason| LocatorOperationError::UnsupportedAction {
            action: LocatorAction::ScrollIntoView,
            reason,
        })?;
    Ok(LocatorScroll {
        matched: resolved.matched,
        scroll,
    })
}

fn unsupported_locator_inspection(
    resolved: &ResolvedLocator,
    inspection: LocatorInspection,
    fallback: &str,
) -> LocatorOperationError {
    let reason = resolved.matched.role.as_ref().map_or_else(
        || fallback.into(),
        |role| format!("{inspection} inspection for role {role} is not implemented"),
    );
    LocatorOperationError::InspectionBlocked { inspection, reason }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeClickEffect {
    Activated,
    Checked { checked: bool, changed: bool },
}

fn apply_native_click(
    page: &mut CurrentPage,
    index: usize,
    action: InteractiveAction,
) -> Result<NativeClickEffect, String> {
    validate_native_click(page, index, &action)?;
    let effect = match action {
        InteractiveAction::Activate => NativeClickEffect::Activated,
        InteractiveAction::ToggleCheckbox => {
            let current = page.interactive_elements[index]
                .checked()
                .ok_or_else(|| "checkbox click state is not implemented".to_owned())?;
            let replacement = !current;
            let checked = page
                .set_checked(index, replacement)
                .map_err(CheckedMutationError::reason)?;
            NativeClickEffect::Checked {
                checked,
                changed: checked != current,
            }
        }
        InteractiveAction::SelectRadio => {
            let current = page.interactive_elements[index]
                .checked()
                .ok_or_else(|| "radio click state is not implemented".to_owned())?;
            let checked = page
                .set_checked(index, true)
                .map_err(CheckedMutationError::reason)?;
            NativeClickEffect::Checked {
                checked,
                changed: checked != current,
            }
        }
        InteractiveAction::Navigate { .. }
        | InteractiveAction::SubmitForm { .. }
        | InteractiveAction::Unsupported { .. } => {
            return Err("native click effect is not available".into());
        }
    };
    page.focused_interactive_index = Some(index);
    Ok(effect)
}

fn validate_native_click(
    page: &CurrentPage,
    index: usize,
    action: &InteractiveAction,
) -> Result<(), String> {
    if let Some(reason) = page.interactive_elements[index].focus_block_reason() {
        return Err(reason);
    }
    match action {
        InteractiveAction::Activate => Ok(()),
        InteractiveAction::ToggleCheckbox => {
            let replacement = !page.interactive_elements[index]
                .checked()
                .ok_or_else(|| "checkbox click state is not implemented".to_owned())?;
            page.validate_set_checked(index, replacement)
                .map(|_| ())
                .map_err(CheckedMutationError::reason)
        }
        InteractiveAction::SelectRadio => page
            .validate_set_checked(index, true)
            .map(|_| ())
            .map_err(CheckedMutationError::reason),
        InteractiveAction::Navigate { .. }
        | InteractiveAction::SubmitForm { .. }
        | InteractiveAction::Unsupported { .. } => {
            Err("native click effect is not available".into())
        }
    }
}

fn execute_click_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<ClickByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Click)?;
    let element = &session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page")
        .interactive_elements[index];
    require_locator_visible(element, LocatorAction::Click)?;
    require_locator_enabled(element, LocatorAction::Click)?;
    let page = session
        .current_page
        .as_ref()
        .expect("resolved locator requires a current page");
    require_locator_stable(
        &page.locator_elements[resolved.source_index],
        LocatorAction::Click,
    )?;
    require_locator_receives_events(
        page,
        resolved.source_index,
        LocatorAction::Click,
        session.viewport,
    )?;
    let action = element.action.clone();
    let viewport = session.viewport;
    match action {
        InteractiveAction::Navigate { href } => {
            let current_url = session
                .current_page
                .as_ref()
                .expect("resolved locator requires a current page")
                .url
                .clone();
            let target = resolve_navigation_url(&current_url, &href)
                .map_err(LocatorOperationError::Navigation)?;
            session
                .current_page
                .as_mut()
                .expect("resolved locator requires a current page")
                .auto_scroll_into_view(resolved.source_index, viewport);
            let context = session.pointer_action_context(resolved.source_index);
            session
                .current_page
                .as_mut()
                .expect("resolved locator requires a current page")
                .focused_interactive_index = Some(index);
            session.finish_pointer_click(&context, &[]);
            let page = session
                .navigate_to(target)
                .map_err(LocatorOperationError::Navigation)?;
            Ok(ClickByLocatorResult::Navigated {
                matched: resolved.matched,
                page,
            })
        }
        InteractiveAction::SubmitForm { form_owner } => {
            session
                .current_page
                .as_mut()
                .expect("resolved locator requires a current page")
                .auto_scroll_into_view(resolved.source_index, viewport);
            let context = session.pointer_action_context(resolved.source_index);
            session
                .current_page
                .as_mut()
                .expect("resolved locator requires a current page")
                .focused_interactive_index = Some(index);
            session.finish_pointer_click(&context, &[]);
            let target = session
                .current_page
                .as_ref()
                .expect("resolved locator requires a current page")
                .form_submission_url(Some(index), form_owner)
                .map_err(|error| match error {
                    FormSubmissionError::Unsupported(reason) => {
                        LocatorOperationError::UnsupportedAction {
                            action: LocatorAction::Click,
                            reason,
                        }
                    }
                    FormSubmissionError::Navigation(error) => {
                        LocatorOperationError::Navigation(error)
                    }
                })?;
            let page = session
                .navigate_to(target)
                .map_err(LocatorOperationError::Navigation)?;
            Ok(ClickByLocatorResult::Navigated {
                matched: resolved.matched,
                page,
            })
        }
        action @ (InteractiveAction::Activate
        | InteractiveAction::ToggleCheckbox
        | InteractiveAction::SelectRadio) => {
            validate_native_click(
                session
                    .current_page
                    .as_ref()
                    .expect("resolved locator requires a current page"),
                index,
                &action,
            )
            .map_err(|reason| LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Click,
                reason,
            })?;
            session
                .current_page
                .as_mut()
                .expect("resolved locator requires a current page")
                .auto_scroll_into_view(resolved.source_index, viewport);
            let context = session.pointer_action_context(resolved.source_index);
            let effect = {
                let page = session
                    .current_page
                    .as_mut()
                    .expect("resolved locator requires a current page");
                apply_native_click(page, index, action).map_err(|reason| {
                    LocatorOperationError::UnsupportedAction {
                        action: LocatorAction::Click,
                        reason,
                    }
                })?
            };
            match effect {
                NativeClickEffect::Activated => {
                    session.finish_pointer_click(&context, &[]);
                    Ok(ClickByLocatorResult::Activated {
                        matched: resolved.matched,
                    })
                }
                NativeClickEffect::Checked { checked, changed } => {
                    let events = if changed {
                        &[DomEventType::Input, DomEventType::Change][..]
                    } else {
                        &[][..]
                    };
                    session.finish_pointer_click(&context, events);
                    Ok(ClickByLocatorResult::Checked {
                        matched: resolved.matched,
                        checked,
                    })
                }
            }
        }
        InteractiveAction::Unsupported { reason } => {
            Err(LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Click,
                reason,
            })
        }
    }
}

fn execute_fill_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: String,
) -> Result<FillByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Fill)?;
    let event_target = session.dom_event_target(index);
    let result = {
        let page = session
            .current_page
            .as_mut()
            .expect("resolved locator requires a current page");
        let element = &mut page.interactive_elements[index];
        require_locator_visible(element, LocatorAction::Fill)?;
        match element.replace_text(replacement) {
            Ok(value) => {
                let value = value.into();
                page.focused_interactive_index = Some(index);
                Ok(FillByLocatorResult {
                    matched: resolved.matched,
                    value,
                })
            }
            Err(TextValueError::Blocked { reason }) => Err(LocatorOperationError::ActionBlocked {
                action: LocatorAction::Fill,
                check: ActionabilityCheck::Editable,
                reason,
            }),
            Err(TextValueError::Unsupported { reason }) => {
                Err(LocatorOperationError::UnsupportedAction {
                    action: LocatorAction::Fill,
                    reason,
                })
            }
        }
    }?;
    session.record_dom_events(
        &event_target,
        &[DomEventType::BeforeInput, DomEventType::Input],
    );
    Ok(result)
}

fn execute_type_by_locator(
    session: &mut Session,
    locator: &Locator,
    text: &str,
) -> Result<TypeByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Type)?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &mut page.interactive_elements[index];
    require_locator_visible(element, LocatorAction::Type)?;
    match element.append_text(text) {
        Ok(value) => Ok(TypeByLocatorResult {
            matched: resolved.matched,
            value: value.into(),
        }),
        Err(TextValueError::Blocked { reason }) => Err(LocatorOperationError::ActionBlocked {
            action: LocatorAction::Type,
            check: ActionabilityCheck::Editable,
            reason,
        }),
        Err(TextValueError::Unsupported { reason }) => {
            Err(LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Type,
                reason,
            })
        }
    }
}

fn execute_focus_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<FocusByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Focus)?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &page.interactive_elements[index];
    if let Some(reason) = element.focus_block_reason() {
        return Err(LocatorOperationError::UnsupportedAction {
            action: LocatorAction::Focus,
            reason,
        });
    }
    page.focused_interactive_index = Some(index);
    Ok(FocusByLocatorResult {
        matched: resolved.matched,
    })
}

fn execute_focused_press(
    session: &mut Session,
    key: &KeyboardKey,
) -> Result<PressEffect, FocusedPressError> {
    let disposition = {
        let page = session
            .current_page
            .as_ref()
            .expect("focused press requires a current page");
        focused_press_disposition(page, key)?
    };
    match disposition {
        FocusedPressDisposition::Navigate { element, target } => {
            let opened =
                session
                    .navigate_to(target)
                    .map_err(|error| FocusedPressError::Navigation {
                        element: element.element.clone(),
                        error,
                    })?;
            return Ok(PressEffect::Navigated(NavigationPressEffect {
                element,
                url: opened.url,
                interactive_element_count: opened.interactive_element_count,
            }));
        }
        FocusedPressDisposition::Ignored { element } => {
            return Ok(PressEffect::Ignored { element });
        }
        FocusedPressDisposition::Local => {}
    }
    session
        .current_page
        .as_mut()
        .expect("focused press requires a current page")
        .apply_press(key)
        .map_err(FocusedPressError::Press)
}

fn focused_press_disposition(
    page: &CurrentPage,
    key: &KeyboardKey,
) -> Result<FocusedPressDisposition, FocusedPressError> {
    let Some(index) = page.focused_interactive_index else {
        return Ok(FocusedPressDisposition::Local);
    };
    let element = page.interactive_elements[index].focused_element();
    let target = match &page.interactive_elements[index].action {
        InteractiveAction::Navigate { href }
            if key.control_activation_key() == Some(ControlActivationKey::Enter) =>
        {
            resolve_navigation_url(&page.url, href).map_err(|error| {
                FocusedPressError::Navigation {
                    element: element.element.clone(),
                    error,
                }
            })?
        }
        InteractiveAction::SubmitForm { form_owner } if key.control_activation_key().is_some() => {
            page.form_submission_url(Some(index), *form_owner)
                .map_err(|error| match error {
                    FormSubmissionError::Unsupported(reason) => {
                        FocusedPressError::Press(PagePressError::Unsupported {
                            element: element.element.clone(),
                            reason,
                        })
                    }
                    FormSubmissionError::Navigation(error) => FocusedPressError::Navigation {
                        element: element.element.clone(),
                        error,
                    },
                })?
        }
        _ if key.control_activation_key() == Some(ControlActivationKey::Enter)
            && page.interactive_elements[index].is_single_line_text_control() =>
        {
            let target = page
                .implicit_submission_url(index)
                .map_err(|error| match error {
                    FormSubmissionError::Unsupported(reason) => {
                        FocusedPressError::Press(PagePressError::Unsupported {
                            element: element.element.clone(),
                            reason,
                        })
                    }
                    FormSubmissionError::Navigation(error) => FocusedPressError::Navigation {
                        element: element.element.clone(),
                        error,
                    },
                })?;
            return Ok(match target {
                Some(target) => FocusedPressDisposition::Navigate { element, target },
                None => FocusedPressDisposition::Ignored { element },
            });
        }
        _ => return Ok(FocusedPressDisposition::Local),
    };
    Ok(FocusedPressDisposition::Navigate { element, target })
}

fn execute_press_by_locator(
    session: &mut Session,
    locator: &Locator,
    key: KeyboardKey,
) -> Result<PressByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Press)?;
    let key = key
        .with_modifiers(&session.keyboard.modifiers())
        .map_err(|error| LocatorOperationError::UnsupportedAction {
            action: LocatorAction::Press,
            reason: error.reason,
        })?;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let element = &page.interactive_elements[index];
    if let Some(reason) = element.focus_block_reason() {
        return Err(LocatorOperationError::UnsupportedAction {
            action: LocatorAction::Press,
            reason,
        });
    }
    page.focused_interactive_index = Some(index);
    let event_context = PressEventContext {
        target: session.dom_event_target(index),
        checked: session
            .current_page
            .as_ref()
            .expect("locator press requires a current page")
            .interactive_elements[index]
            .checked(),
    };
    match execute_focused_press(session, &key) {
        Ok(effect) => {
            record_complete_press_events(session, &key, &effect, &event_context);
            Ok(PressByLocatorResult {
                matched: resolved.matched,
                press: PressResult { key, effect },
            })
        }
        Err(FocusedPressError::Press(PagePressError::Unsupported { reason, .. })) => {
            Err(LocatorOperationError::UnsupportedAction {
                action: LocatorAction::Press,
                reason,
            })
        }
        Err(FocusedPressError::Press(PagePressError::NoFocusedElement)) => {
            unreachable!("locator press installs focus before applying its key")
        }
        Err(FocusedPressError::Navigation { error, .. }) => {
            Err(LocatorOperationError::Navigation(error))
        }
    }
}

fn execute_select_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: String,
) -> Result<SelectByLocatorResult, LocatorOperationError> {
    let result = execute_select_options_by_locator(
        session,
        locator,
        NonEmpty::one(SelectOptionTarget::Value(replacement)),
    )?;
    Ok(SelectByLocatorResult {
        matched: result.matched,
        value: result.selected[0].clone(),
    })
}

fn execute_select_options_by_locator(
    session: &mut Session,
    locator: &Locator,
    options: NonEmpty<SelectOptionTarget>,
) -> Result<SelectOptionsByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, LocatorAction::Select)?;
    let event_target = session.dom_event_target(index);
    let result = {
        let page = session
            .current_page
            .as_mut()
            .expect("resolved locator requires a current page");
        let element = &mut page.interactive_elements[index];
        require_locator_visible(element, LocatorAction::Select)?;
        match element.select_options(&options) {
            Ok(selected) => Ok(SelectOptionsByLocatorResult {
                matched: resolved.matched,
                selected,
            }),
            Err(SelectValueError::Blocked { reason }) => {
                Err(LocatorOperationError::ActionBlocked {
                    action: LocatorAction::Select,
                    check: ActionabilityCheck::Enabled,
                    reason,
                })
            }
            Err(SelectValueError::Unsupported { reason }) => {
                Err(LocatorOperationError::UnsupportedAction {
                    action: LocatorAction::Select,
                    reason,
                })
            }
            Err(SelectValueError::OptionNotFound { target }) => {
                Err(LocatorOperationError::SelectOptionNotFound { target })
            }
            Err(SelectValueError::OptionDisabled { target }) => {
                Err(LocatorOperationError::SelectOptionDisabled { target })
            }
        }
    }?;
    session.record_dom_events(&event_target, &[DomEventType::Input, DomEventType::Change]);
    Ok(result)
}

fn execute_set_checked_by_locator(
    session: &mut Session,
    locator: &Locator,
    replacement: bool,
) -> Result<SetCheckedByLocatorResult, LocatorOperationError> {
    let action = if replacement {
        LocatorAction::Check
    } else {
        LocatorAction::Uncheck
    };
    let resolved = session.locator_match_for(locator)?;
    let index = session.locator_interactive_index(&resolved, action)?;
    let viewport = session.viewport;
    let page = session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page");
    let current = page.interactive_elements[index].checked().ok_or_else(|| {
        LocatorOperationError::UnsupportedAction {
            action,
            reason: format!(
                "checked-state mutation for role {} is not implemented",
                page.interactive_elements[index].role()
            ),
        }
    })?;
    if current == replacement {
        return Ok(SetCheckedByLocatorResult {
            matched: resolved.matched,
            checked: current,
        });
    }
    page.validate_set_checked(index, replacement)
        .map_err(|error| match error {
            CheckedMutationError::Blocked { reason } => LocatorOperationError::ActionBlocked {
                action,
                check: ActionabilityCheck::Enabled,
                reason,
            },
            CheckedMutationError::Unsupported { reason } => {
                LocatorOperationError::UnsupportedAction { action, reason }
            }
        })?;
    require_locator_visible(&page.interactive_elements[index], action)?;
    let source_index = page.source_index_for_interactive(index);
    require_locator_stable(&page.locator_elements[source_index], action)?;
    require_locator_receives_events(page, source_index, action, viewport)?;
    page.auto_scroll_into_view(source_index, viewport);
    let result = match page.set_checked(index, replacement) {
        Ok(checked) => Ok(SetCheckedByLocatorResult {
            matched: resolved.matched,
            checked,
        }),
        Err(CheckedMutationError::Blocked { reason }) => {
            Err(LocatorOperationError::ActionBlocked {
                action,
                check: ActionabilityCheck::Enabled,
                reason,
            })
        }
        Err(CheckedMutationError::Unsupported { reason }) => {
            Err(LocatorOperationError::UnsupportedAction { action, reason })
        }
    }?;
    let context = session.pointer_action_context(source_index);
    session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page")
        .focused_interactive_index = Some(index);
    session.finish_pointer_click(&context, &[DomEventType::Input, DomEventType::Change]);
    Ok(result)
}

fn execute_hover_by_locator(
    session: &mut Session,
    locator: &Locator,
) -> Result<HoverByLocatorResult, LocatorOperationError> {
    let resolved = session.locator_match_for(locator)?;
    let viewport = session.viewport;
    session
        .current_page
        .as_mut()
        .expect("resolved locator requires a current page")
        .prepare_hover(resolved.source_index, viewport)
        .map_err(|(check, reason)| LocatorOperationError::ActionBlocked {
            action: LocatorAction::Hover,
            check,
            reason,
        })?;
    let context = session.pointer_action_context(resolved.source_index);
    session.finish_pointer_move(&context);
    Ok(HoverByLocatorResult {
        matched: resolved.matched,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClickElement {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClickResult {
    Navigated {
        reference: InteractiveElementRef,
        page: OpenedPage,
    },
    Activated {
        reference: InteractiveElementRef,
    },
    Checked {
        reference: InteractiveElementRef,
        checked: bool,
    },
}

impl private::Sealed for ClickElement {}

impl SessionRequest for ClickElement {
    type Reply = ClickResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        let visible = element
            .visible()
            .map_err(|reason| SessionError::UnsupportedClick {
                reference: self.reference,
                reason: reason.into(),
            })?;
        if !visible {
            return Err(SessionError::UnsupportedClick {
                reference: self.reference,
                reason: "hidden elements cannot be clicked".into(),
            });
        }
        if element.enabled() == Some(false) {
            return Err(SessionError::UnsupportedClick {
                reference: self.reference,
                reason: "disabled controls cannot be clicked".into(),
            });
        }
        let source_index = element.source_index;
        session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .locator_elements[source_index]
            .stable()
            .map_err(|reason| SessionError::UnsupportedClick {
                reference: self.reference,
                reason: format!("stable check failed: {reason}"),
            })?;
        session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .receives_events(source_index, session.viewport)
            .map_err(|reason| SessionError::UnsupportedClick {
                reference: self.reference,
                reason: format!("receives events check failed: {reason}"),
            })?;
        let action = element.action.clone();
        let viewport = session.viewport;
        match action {
            InteractiveAction::Navigate { href } => {
                let current_url = session
                    .current_page
                    .as_ref()
                    .expect("validated reference requires a current page")
                    .url
                    .clone();
                let target = resolve_navigation_url(&current_url, &href).map_err(|error| {
                    SessionError::Navigation {
                        reference: self.reference,
                        error,
                    }
                })?;
                session
                    .current_page
                    .as_mut()
                    .expect("validated reference requires a current page")
                    .auto_scroll_into_view(source_index, viewport);
                let context = session.pointer_action_context(source_index);
                session
                    .current_page
                    .as_mut()
                    .expect("validated reference requires a current page")
                    .focused_interactive_index = Some(index);
                session.finish_pointer_click(&context, &[]);
                let page =
                    session
                        .navigate_to(target)
                        .map_err(|error| SessionError::Navigation {
                            reference: self.reference,
                            error,
                        })?;
                Ok(ClickResult::Navigated {
                    reference: self.reference,
                    page,
                })
            }
            InteractiveAction::SubmitForm { form_owner } => {
                session
                    .current_page
                    .as_mut()
                    .expect("validated reference requires a current page")
                    .auto_scroll_into_view(source_index, viewport);
                let context = session.pointer_action_context(source_index);
                session
                    .current_page
                    .as_mut()
                    .expect("validated reference requires a current page")
                    .focused_interactive_index = Some(index);
                session.finish_pointer_click(&context, &[]);
                let target = session
                    .current_page
                    .as_ref()
                    .expect("validated reference requires a current page")
                    .form_submission_url(Some(index), form_owner)
                    .map_err(|error| match error {
                        FormSubmissionError::Unsupported(reason) => {
                            SessionError::UnsupportedClick {
                                reference: self.reference,
                                reason,
                            }
                        }
                        FormSubmissionError::Navigation(error) => SessionError::Navigation {
                            reference: self.reference,
                            error,
                        },
                    })?;
                let page =
                    session
                        .navigate_to(target)
                        .map_err(|error| SessionError::Navigation {
                            reference: self.reference,
                            error,
                        })?;
                Ok(ClickResult::Navigated {
                    reference: self.reference,
                    page,
                })
            }
            action @ (InteractiveAction::Activate
            | InteractiveAction::ToggleCheckbox
            | InteractiveAction::SelectRadio) => {
                validate_native_click(
                    session
                        .current_page
                        .as_ref()
                        .expect("validated reference requires a current page"),
                    index,
                    &action,
                )
                .map_err(|reason| SessionError::UnsupportedClick {
                    reference: self.reference,
                    reason,
                })?;
                session
                    .current_page
                    .as_mut()
                    .expect("validated reference requires a current page")
                    .auto_scroll_into_view(source_index, viewport);
                let context = session.pointer_action_context(source_index);
                let effect = {
                    let page = session
                        .current_page
                        .as_mut()
                        .expect("validated reference requires a current page");
                    apply_native_click(page, index, action).map_err(|reason| {
                        SessionError::UnsupportedClick {
                            reference: self.reference,
                            reason,
                        }
                    })?
                };
                match effect {
                    NativeClickEffect::Activated => {
                        session.finish_pointer_click(&context, &[]);
                        Ok(ClickResult::Activated {
                            reference: self.reference,
                        })
                    }
                    NativeClickEffect::Checked { checked, changed } => {
                        let events = if changed {
                            &[DomEventType::Input, DomEventType::Change][..]
                        } else {
                            &[][..]
                        };
                        session.finish_pointer_click(&context, events);
                        Ok(ClickResult::Checked {
                            reference: self.reference,
                            checked,
                        })
                    }
                }
            }
            InteractiveAction::Unsupported { reason } => Err(SessionError::UnsupportedClick {
                reference: self.reference,
                reason,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillElement {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillResult {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeElement {
    pub reference: InteractiveElementRef,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeResult {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusElement {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusResult {
    pub reference: InteractiveElementRef,
    pub element: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoverElement {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoverResult {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressKey {
    pub key: KeyboardKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyDown {
    pub key: KeyboardEventKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyDownResult {
    pub key: KeyboardEventKey,
    pub repeat: bool,
    pub deferred: bool,
    pub press: Option<PressResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyUp {
    pub key: KeyboardEventKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyUpResult {
    pub key: KeyboardEventKey,
    pub was_pressed: bool,
    pub press: Option<PressResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardInsertText {
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardType {
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardTextResult {
    pub effect: KeyboardTextEffect,
}

impl KeyboardTextResult {
    pub fn text(&self) -> Option<&TextPressEffect> {
        match &self.effect {
            KeyboardTextEffect::Text(effect) => Some(effect),
            KeyboardTextEffect::Ignored { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressResult {
    pub key: KeyboardKey,
    pub effect: PressEffect,
}

impl PressResult {
    pub fn text(&self) -> Option<&TextPressEffect> {
        match &self.effect {
            PressEffect::Text(effect) => Some(effect),
            PressEffect::FocusTraversal(_)
            | PressEffect::Navigated(_)
            | PressEffect::Ignored { .. }
            | PressEffect::Activated { .. }
            | PressEffect::Checked { .. } => None,
        }
    }

    pub fn focus_traversal(&self) -> Option<&FocusTraversalEffect> {
        match &self.effect {
            PressEffect::FocusTraversal(effect) => Some(effect),
            PressEffect::Text(_)
            | PressEffect::Navigated(_)
            | PressEffect::Ignored { .. }
            | PressEffect::Activated { .. }
            | PressEffect::Checked { .. } => None,
        }
    }

    pub fn navigated(&self) -> Option<&NavigationPressEffect> {
        match &self.effect {
            PressEffect::Navigated(effect) => Some(effect),
            PressEffect::Text(_)
            | PressEffect::FocusTraversal(_)
            | PressEffect::Ignored { .. }
            | PressEffect::Activated { .. }
            | PressEffect::Checked { .. } => None,
        }
    }

    pub fn ignored(&self) -> Option<&FocusedElement> {
        match &self.effect {
            PressEffect::Ignored { element } => Some(element),
            PressEffect::Text(_)
            | PressEffect::FocusTraversal(_)
            | PressEffect::Navigated(_)
            | PressEffect::Activated { .. }
            | PressEffect::Checked { .. } => None,
        }
    }

    pub fn activated(&self) -> Option<&FocusedElement> {
        match &self.effect {
            PressEffect::Activated { element } => Some(element),
            PressEffect::Text(_)
            | PressEffect::FocusTraversal(_)
            | PressEffect::Navigated(_)
            | PressEffect::Ignored { .. }
            | PressEffect::Checked { .. } => None,
        }
    }

    pub fn checked(&self) -> Option<(&FocusedElement, bool)> {
        match &self.effect {
            PressEffect::Checked { element, checked } => Some((element, *checked)),
            PressEffect::Text(_)
            | PressEffect::FocusTraversal(_)
            | PressEffect::Navigated(_)
            | PressEffect::Ignored { .. }
            | PressEffect::Activated { .. } => None,
        }
    }
}

impl private::Sealed for FillElement {}

impl SessionRequest for FillElement {
    type Reply = FillResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let event_target = session.dom_event_target(index);
        let result = {
            let page = session
                .current_page
                .as_mut()
                .expect("validated reference requires a current page");
            let element = &mut page.interactive_elements[index];
            match element.replace_text(self.value) {
                Ok(value) => {
                    let value = value.into();
                    page.focused_interactive_index = Some(index);
                    Ok(FillResult {
                        reference: self.reference,
                        value,
                    })
                }
                Err(
                    TextValueError::Blocked { reason } | TextValueError::Unsupported { reason },
                ) => Err(SessionError::UnsupportedFill {
                    reference: self.reference,
                    reason,
                }),
            }
        }?;
        session.record_dom_events(
            &event_target,
            &[DomEventType::BeforeInput, DomEventType::Input],
        );
        Ok(result)
    }
}

impl private::Sealed for TypeElement {}

impl SessionRequest for TypeElement {
    type Reply = TypeResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page");
        let element = &mut page.interactive_elements[index];
        match element.append_text(&self.text) {
            Ok(value) => Ok(TypeResult {
                reference: self.reference,
                value: value.into(),
            }),
            Err(TextValueError::Blocked { reason } | TextValueError::Unsupported { reason }) => {
                Err(SessionError::UnsupportedType {
                    reference: self.reference,
                    reason,
                })
            }
        }
    }
}

impl private::Sealed for FocusElement {}

impl SessionRequest for FocusElement {
    type Reply = FocusResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page");
        let element = &page.interactive_elements[index];
        if let Some(reason) = element.focus_block_reason() {
            return Err(SessionError::UnsupportedFocus {
                reference: self.reference,
                reason,
            });
        }
        let element = element.element().into();
        page.focused_interactive_index = Some(index);
        Ok(FocusResult {
            reference: self.reference,
            element,
        })
    }
}

impl private::Sealed for HoverElement {}

impl SessionRequest for HoverElement {
    type Reply = HoverResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let viewport = session.viewport;
        let source_index = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .source_index_for_interactive(interactive_index);
        session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page")
            .prepare_hover(source_index, viewport)
            .map_err(|(check, reason)| SessionError::UnsupportedHover {
                reference: self.reference,
                reason: format!("{check} check failed: {reason}"),
            })?;
        let context = session.pointer_action_context(source_index);
        session.finish_pointer_move(&context);
        Ok(HoverResult {
            reference: self.reference,
        })
    }
}

impl private::Sealed for PressKey {}

impl private::Sealed for KeyDown {}

impl SessionRequest for KeyDown {
    type Reply = KeyDownResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let repeat = session.keyboard.is_pressed(&self.key);
        let event_context = focused_press_event_context(session);
        let effective = self
            .key
            .press_key(&session.keyboard.modifiers())
            .map_err(|error| modified_key_session_error(session, error))?;
        let pending_space_activation = effective
            .as_ref()
            .and_then(|key| pending_space_activation(session, key));
        let deferred = pending_space_activation.is_some();
        let press = if deferred {
            None
        } else {
            effective
                .map(|key| execute_press_request(session, key, PressInvocation::KeyDown))
                .transpose()?
        };
        let records_key_up = event_context.as_ref().is_some_and(|context| {
            record_key_down_events(session, press.as_ref(), deferred, context)
        });
        session
            .keyboard
            .record_down(self.key.clone(), records_key_up, pending_space_activation);
        Ok(KeyDownResult {
            key: self.key,
            repeat,
            deferred,
            press,
        })
    }
}

impl private::Sealed for KeyUp {}

impl SessionRequest for KeyUp {
    type Reply = KeyUpResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let event_context = focused_press_event_context(session);
        let released = session.keyboard.release(&self.key);
        if released
            .as_ref()
            .is_some_and(|pressed| pressed.records_key_up)
            && let Some(context) = &event_context
        {
            session.record_dom_events(&context.target, &[DomEventType::KeyUp]);
        }
        let press = released
            .as_ref()
            .and_then(|pressed| pressed.pending_space_activation.as_ref())
            .filter(|activation| {
                event_context
                    .as_ref()
                    .is_some_and(|context| context.target == activation.target)
            })
            .map(|activation| {
                execute_press_request(session, activation.key.clone(), PressInvocation::KeyUp)
            })
            .transpose()?;
        Ok(KeyUpResult {
            key: self.key,
            was_pressed: released.is_some(),
            press,
        })
    }
}

impl private::Sealed for KeyboardInsertText {}

impl SessionRequest for KeyboardInsertText {
    type Reply = KeyboardTextResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        execute_keyboard_text(session, &self.text, KeyboardTextOperation::InsertText)
    }
}

impl private::Sealed for KeyboardType {}

impl SessionRequest for KeyboardType {
    type Reply = KeyboardTextResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        execute_keyboard_text(session, &self.text, KeyboardTextOperation::Type)
    }
}

#[derive(Clone, Copy)]
enum KeyboardTextOperation {
    InsertText,
    Type,
}

fn execute_keyboard_text(
    session: &mut Session,
    text: &str,
    operation: KeyboardTextOperation,
) -> Result<KeyboardTextResult, SessionError> {
    let event_target = {
        let page = session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        if text.is_empty() {
            None
        } else {
            page.focused_interactive_index.and_then(|index| {
                page.interactive_elements[index]
                    .keyboard_text_editable()
                    .map(|editable| {
                        (
                            index,
                            editable,
                            page.interactive_elements[index].is_multiline_text_control(),
                        )
                    })
            })
        }
    };
    let event_target = event_target
        .map(|(index, editable, multiline)| (session.dom_event_target(index), editable, multiline));
    let page = session
        .current_page
        .as_mut()
        .expect("keyboard text page was validated");
    let effect = match operation {
        KeyboardTextOperation::InsertText => page.apply_keyboard_text(text),
        KeyboardTextOperation::Type => page.apply_keyboard_type(text),
    };
    if let Some((target, editable, multiline)) = &event_target {
        match operation {
            KeyboardTextOperation::InsertText if *editable => {
                session.record_dom_events(target, &[DomEventType::BeforeInput, DomEventType::Input])
            }
            KeyboardTextOperation::InsertText => {
                session.record_dom_events(target, &[DomEventType::BeforeInput]);
            }
            KeyboardTextOperation::Type => {
                for character in text.chars() {
                    session.record_dom_events(
                        target,
                        keyboard_type_event_sequence(character, *editable, *multiline),
                    );
                }
            }
        }
    }
    Ok(KeyboardTextResult { effect })
}

fn keyboard_type_event_sequence(
    character: char,
    editable: bool,
    multiline: bool,
) -> &'static [DomEventType] {
    const KEY_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::KeyUp,
    ];
    const KEY_AND_INPUT_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::BeforeInput,
        DomEventType::Input,
        DomEventType::KeyUp,
    ];
    const INPUT_EVENTS: &[DomEventType] = &[DomEventType::BeforeInput, DomEventType::Input];

    let is_printable_ascii = (' '..='~').contains(&character);
    let is_line_break = matches!(character, '\r' | '\n');
    if is_printable_ascii || is_line_break {
        if editable && (!is_line_break || multiline) {
            KEY_AND_INPUT_EVENTS
        } else {
            KEY_EVENTS
        }
    } else if editable {
        INPUT_EVENTS
    } else {
        &[]
    }
}

impl SessionRequest for PressKey {
    type Reply = PressResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        session.current_page.as_ref().ok_or(SessionError::NoPage)?;
        let key = self
            .key
            .with_modifiers(&session.keyboard.modifiers())
            .map_err(|error| modified_key_session_error(session, error))?;
        execute_press_request(session, key, PressInvocation::Complete)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PressInvocation {
    Complete,
    KeyDown,
    KeyUp,
}

fn pending_space_activation(
    session: &Session,
    key: &KeyboardKey,
) -> Option<PendingSpaceActivation> {
    if key.control_activation_key() != Some(ControlActivationKey::Space) {
        return None;
    }
    let page = session
        .current_page
        .as_ref()
        .expect("pending Space activation requires a current page");
    let index = page.focused_interactive_index?;
    matches!(
        page.interactive_elements[index].action,
        InteractiveAction::SubmitForm { .. }
            | InteractiveAction::Activate
            | InteractiveAction::ToggleCheckbox
            | InteractiveAction::SelectRadio
    )
    .then(|| PendingSpaceActivation {
        key: key.clone(),
        target: session.dom_event_target(index),
    })
}

fn modified_key_session_error(session: &Session, error: ModifiedKeyError) -> SessionError {
    let page = session
        .current_page
        .as_ref()
        .expect("modified key validation requires a current page");
    let element = page.focused_interactive_index.map_or_else(
        || "body".into(),
        |index| page.interactive_elements[index].element().into(),
    );
    SessionError::UnsupportedPress {
        key: error.key,
        element,
        reason: error.reason,
    }
}

fn execute_press_request(
    session: &mut Session,
    key: KeyboardKey,
    invocation: PressInvocation,
) -> Result<PressResult, SessionError> {
    session.current_page.as_ref().ok_or(SessionError::NoPage)?;
    let event_context = matches!(
        invocation,
        PressInvocation::Complete | PressInvocation::KeyUp
    )
    .then(|| focused_press_event_context(session))
    .flatten();
    match execute_focused_press(session, &key) {
        Ok(effect) => {
            if let Some(context) = event_context {
                match invocation {
                    PressInvocation::Complete => {
                        record_complete_press_events(session, &key, &effect, &context)
                    }
                    PressInvocation::KeyUp => {
                        record_key_up_activation_events(session, &key, &effect, &context)
                    }
                    PressInvocation::KeyDown => {}
                }
            }
            Ok(PressResult { key, effect })
        }
        Err(FocusedPressError::Press(PagePressError::NoFocusedElement)) => {
            Err(SessionError::NoFocusedElement)
        }
        Err(FocusedPressError::Press(PagePressError::Unsupported { element, reason })) => {
            Err(SessionError::UnsupportedPress {
                key,
                element,
                reason,
            })
        }
        Err(FocusedPressError::Navigation { element, error }) => {
            Err(SessionError::PressNavigation {
                key,
                element,
                error,
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PressEventContext {
    target: DomEventTarget,
    checked: Option<bool>,
}

fn focused_press_event_context(session: &Session) -> Option<PressEventContext> {
    let page = session
        .current_page
        .as_ref()
        .expect("press event context requires a current page");
    let index = page.focused_interactive_index?;
    Some(PressEventContext {
        target: session.dom_event_target(index),
        checked: page.interactive_elements[index].checked(),
    })
}

fn record_complete_press_events(
    session: &mut Session,
    key: &KeyboardKey,
    effect: &PressEffect,
    context: &PressEventContext,
) {
    const KEY_EVENTS: &[DomEventType] = &[DomEventType::KeyDown, DomEventType::KeyUp];
    const PRINTABLE_KEY_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::KeyUp,
    ];
    const PRINTABLE_INPUT_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::BeforeInput,
        DomEventType::Input,
        DomEventType::KeyUp,
    ];
    const EDITING_INPUT_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::BeforeInput,
        DomEventType::Input,
        DomEventType::KeyUp,
    ];
    const ENTER_ACTIVATION_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::Click,
        DomEventType::KeyUp,
    ];
    const SPACE_ACTIVATION_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::KeyUp,
        DomEventType::Click,
    ];
    const SPACE_CHECKED_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::KeyUp,
        DomEventType::Click,
        DomEventType::Input,
        DomEventType::Change,
    ];
    const SPACE_NO_CHANGE_EVENTS: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::KeyUp,
    ];

    let events = match effect {
        PressEffect::Text(_) if key.has_embedded_modifiers() => &[][..],
        PressEffect::Text(text) => match (key.press_event_kind(), text.changed) {
            (KeyboardPressEventKind::PrintableAscii | KeyboardPressEventKind::Enter, true) => {
                PRINTABLE_INPUT_EVENTS
            }
            (KeyboardPressEventKind::PrintableAscii | KeyboardPressEventKind::Enter, false) => {
                PRINTABLE_KEY_EVENTS
            }
            (KeyboardPressEventKind::Editing, true) => EDITING_INPUT_EVENTS,
            (KeyboardPressEventKind::Editing | KeyboardPressEventKind::Other, false) => KEY_EVENTS,
            (KeyboardPressEventKind::Other, true) => KEY_EVENTS,
            (KeyboardPressEventKind::OtherCharacter, _) => &[],
        },
        PressEffect::Ignored { .. } if key.press_event_kind() == KeyboardPressEventKind::Enter => {
            PRINTABLE_KEY_EVENTS
        }
        PressEffect::Activated { .. } => match key.control_activation_key() {
            Some(ControlActivationKey::Enter) => ENTER_ACTIVATION_EVENTS,
            Some(ControlActivationKey::Space) => SPACE_ACTIVATION_EVENTS,
            None => &[],
        },
        PressEffect::Checked { checked, .. }
            if key.control_activation_key() == Some(ControlActivationKey::Space) =>
        {
            if context.checked.is_some_and(|before| before != *checked) {
                SPACE_CHECKED_EVENTS
            } else {
                SPACE_NO_CHANGE_EVENTS
            }
        }
        PressEffect::Navigated(_)
            if key.control_activation_key() == Some(ControlActivationKey::Space) =>
        {
            SPACE_ACTIVATION_EVENTS
        }
        PressEffect::FocusTraversal(_)
        | PressEffect::Navigated(_)
        | PressEffect::Ignored { .. }
        | PressEffect::Checked { .. } => &[],
    };
    session.record_dom_events(&context.target, events);
}

fn record_key_up_activation_events(
    session: &mut Session,
    key: &KeyboardKey,
    effect: &PressEffect,
    context: &PressEventContext,
) {
    const CLICK: &[DomEventType] = &[DomEventType::Click];
    const CHANGED_CHECKED: &[DomEventType] = &[
        DomEventType::Click,
        DomEventType::Input,
        DomEventType::Change,
    ];

    let events = match effect {
        PressEffect::Activated { .. } | PressEffect::Navigated(_)
            if key.control_activation_key() == Some(ControlActivationKey::Space) =>
        {
            CLICK
        }
        PressEffect::Checked { checked, .. }
            if key.control_activation_key() == Some(ControlActivationKey::Space)
                && context.checked.is_some_and(|before| before != *checked) =>
        {
            CHANGED_CHECKED
        }
        PressEffect::Text(_)
        | PressEffect::FocusTraversal(_)
        | PressEffect::Navigated(_)
        | PressEffect::Ignored { .. }
        | PressEffect::Activated { .. }
        | PressEffect::Checked { .. } => &[],
    };
    session.record_dom_events(&context.target, events);
}

fn record_key_down_events(
    session: &mut Session,
    press: Option<&PressResult>,
    deferred: bool,
    context: &PressEventContext,
) -> bool {
    const KEY_DOWN: &[DomEventType] = &[DomEventType::KeyDown];
    const PRINTABLE_KEY_DOWN: &[DomEventType] = &[DomEventType::KeyDown, DomEventType::KeyPress];
    const PRINTABLE_INPUT_DOWN: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::BeforeInput,
        DomEventType::Input,
    ];
    const EDITING_INPUT_DOWN: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::BeforeInput,
        DomEventType::Input,
    ];
    const ENTER_ACTIVATION_DOWN: &[DomEventType] = &[
        DomEventType::KeyDown,
        DomEventType::KeyPress,
        DomEventType::Click,
    ];

    let events = if deferred {
        PRINTABLE_KEY_DOWN
    } else {
        match press {
            None => KEY_DOWN,
            Some(press) => match &press.effect {
                PressEffect::Text(_) if press.key.has_embedded_modifiers() => &[][..],
                PressEffect::Text(text) => match (press.key.press_event_kind(), text.changed) {
                    (
                        KeyboardPressEventKind::PrintableAscii | KeyboardPressEventKind::Enter,
                        true,
                    ) => PRINTABLE_INPUT_DOWN,
                    (
                        KeyboardPressEventKind::PrintableAscii | KeyboardPressEventKind::Enter,
                        false,
                    ) => PRINTABLE_KEY_DOWN,
                    (KeyboardPressEventKind::Editing, true) => EDITING_INPUT_DOWN,
                    (KeyboardPressEventKind::Editing | KeyboardPressEventKind::Other, false)
                    | (KeyboardPressEventKind::Other, true) => KEY_DOWN,
                    (KeyboardPressEventKind::OtherCharacter, _) => &[],
                },
                PressEffect::Ignored { .. }
                    if press.key.press_event_kind() == KeyboardPressEventKind::Enter =>
                {
                    PRINTABLE_KEY_DOWN
                }
                PressEffect::Activated { .. }
                    if press.key.control_activation_key() == Some(ControlActivationKey::Enter) =>
                {
                    ENTER_ACTIVATION_DOWN
                }
                PressEffect::FocusTraversal(_) if !press.key.has_embedded_modifiers() => KEY_DOWN,
                PressEffect::FocusTraversal(_)
                | PressEffect::Navigated(_)
                | PressEffect::Ignored { .. }
                | PressEffect::Activated { .. }
                | PressEffect::Checked { .. } => &[],
            },
        }
    };
    session.record_dom_events(&context.target, events);
    !events.is_empty()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectElement {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectResult {
    pub reference: InteractiveElementRef,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptions {
    pub reference: InteractiveElementRef,
    pub options: NonEmpty<SelectOptionTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionsResult {
    pub reference: InteractiveElementRef,
    pub selected: NonEmpty<String>,
}

impl private::Sealed for SelectElement {}

impl SessionRequest for SelectElement {
    type Reply = SelectResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let event_target = session.dom_event_target(index);
        let result = {
            let element = &mut session
                .current_page
                .as_mut()
                .expect("validated reference requires a current page")
                .interactive_elements[index];
            match element.select_value(&self.value) {
                Ok(value) => Ok(SelectResult {
                    reference: self.reference,
                    value: value.into(),
                }),
                Err(
                    SelectValueError::Blocked { reason } | SelectValueError::Unsupported { reason },
                ) => Err(SessionError::UnsupportedSelect {
                    reference: self.reference,
                    reason,
                }),
                Err(SelectValueError::OptionNotFound { target }) => {
                    Err(reference_option_not_found(self.reference, target))
                }
                Err(SelectValueError::OptionDisabled { target }) => {
                    Err(reference_option_disabled(self.reference, target))
                }
            }
        }?;
        session.record_dom_events(&event_target, &[DomEventType::Input, DomEventType::Change]);
        Ok(result)
    }
}

impl private::Sealed for SelectOptions {}

impl SessionRequest for SelectOptions {
    type Reply = SelectOptionsResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let event_target = session.dom_event_target(index);
        let result = {
            let element = &mut session
                .current_page
                .as_mut()
                .expect("validated reference requires a current page")
                .interactive_elements[index];
            match element.select_options(&self.options) {
                Ok(selected) => Ok(SelectOptionsResult {
                    reference: self.reference,
                    selected,
                }),
                Err(
                    SelectValueError::Blocked { reason } | SelectValueError::Unsupported { reason },
                ) => Err(SessionError::UnsupportedSelect {
                    reference: self.reference,
                    reason,
                }),
                Err(SelectValueError::OptionNotFound { target }) => {
                    Err(reference_option_not_found(self.reference, target))
                }
                Err(SelectValueError::OptionDisabled { target }) => {
                    Err(reference_option_disabled(self.reference, target))
                }
            }
        }?;
        session.record_dom_events(&event_target, &[DomEventType::Input, DomEventType::Change]);
        Ok(result)
    }
}

fn reference_option_not_found(
    reference: InteractiveElementRef,
    target: SelectOptionTarget,
) -> SessionError {
    match target {
        SelectOptionTarget::Value(value) => SessionError::SelectOptionNotFound { reference, value },
        target => SessionError::SelectOptionTargetNotFound { reference, target },
    }
}

fn reference_option_disabled(
    reference: InteractiveElementRef,
    target: SelectOptionTarget,
) -> SessionError {
    match target {
        SelectOptionTarget::Value(value) => SessionError::SelectOptionDisabled { reference, value },
        target => SessionError::SelectOptionTargetDisabled { reference, target },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementValue {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementBoundingBox {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollElementIntoView {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElementScroll {
    pub reference: InteractiveElementRef,
    pub scroll: PageScroll,
}

impl private::Sealed for ScrollElementIntoView {}

impl SessionRequest for ScrollElementIntoView {
    type Reply = ElementScroll;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let viewport = session.viewport;
        let page = session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page");
        let source_index = page.source_index_for_interactive(interactive_index);
        let scroll = page
            .scroll_into_view(source_index, viewport)
            .map_err(|reason| SessionError::UnsupportedScrollIntoView {
                reference: self.reference,
                reason,
            })?;
        Ok(ElementScroll {
            reference: self.reference,
            scroll,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementBoundingBox {
    pub reference: InteractiveElementRef,
    pub value: Option<BoundingBox>,
}

impl private::Sealed for GetElementBoundingBox {}

impl SessionRequest for GetElementBoundingBox {
    type Reply = ElementBoundingBox;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page");
        let source_index = page.source_index_for_interactive(interactive_index);
        let value = page.locator_elements[source_index]
            .bounding_box(page.scroll_x, page.scroll_y)
            .map_err(|reason| SessionError::UnsupportedBoundingBox {
                reference: self.reference,
                reason: reason.into(),
            })?;
        Ok(ElementBoundingBox {
            reference: self.reference,
            value,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementValue {
    pub reference: InteractiveElementRef,
    pub value: String,
}

impl private::Sealed for GetElementValue {}

impl SessionRequest for GetElementValue {
    type Reply = ElementValue;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        if let Some(value) = element.value() {
            return Ok(ElementValue {
                reference: self.reference,
                value: value.into(),
            });
        }
        let reason = format!(
            "value inspection for role {} is not implemented",
            element.role()
        );
        Err(SessionError::UnsupportedValue {
            reference: self.reference,
            reason,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementText {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementText {
    pub reference: InteractiveElementRef,
    pub text: String,
}

impl private::Sealed for GetElementText {}

impl SessionRequest for GetElementText {
    type Reply = ElementText;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        Ok(ElementText {
            reference: self.reference,
            text: element.text().into(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementHtml {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementHtml {
    pub reference: InteractiveElementRef,
    pub html: String,
}

impl private::Sealed for GetElementHtml {}

impl SessionRequest for GetElementHtml {
    type Reply = ElementHtml;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page");
        let source_index = page
            .locator_elements
            .iter()
            .position(|element| element.interactive_index == Some(interactive_index))
            .expect("every interactive element has one locator source");
        let contains_sensitive_value = page
            .selector_index
            .inner_html_contains_sensitive_value(source_index)
            .map_err(|error| SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: error.to_string(),
            })?;
        if contains_sensitive_value {
            return Err(SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: "inner HTML contains a password value attribute".into(),
            });
        }
        let html = page
            .selector_index
            .inner_html(source_index)
            .map_err(|error| SessionError::UnsupportedHtml {
                reference: self.reference,
                reason: error.to_string(),
            })?;
        Ok(ElementHtml {
            reference: self.reference,
            html,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetElementAttribute {
    pub reference: InteractiveElementRef,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementAttribute {
    pub reference: InteractiveElementRef,
    pub name: String,
    pub value: Option<String>,
}

impl private::Sealed for GetElementAttribute {}

impl SessionRequest for GetElementAttribute {
    type Reply = ElementAttribute;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let name = normalize_attribute_name(self.name)?;
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        if element.attribute_is_sensitive(&name) {
            return Err(SessionError::SensitiveAttribute {
                reference: self.reference,
                name,
            });
        }
        Ok(ElementAttribute {
            reference: self.reference,
            value: element.attribute(&name).map(str::to_owned),
            name,
        })
    }
}

fn normalize_attribute_name(name: String) -> Result<String, SessionError> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(SessionError::InvalidAttributeName { name });
    }
    Ok(name.to_ascii_lowercase())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementEnabled {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementEnabled {
    pub reference: InteractiveElementRef,
    pub enabled: bool,
}

impl private::Sealed for GetElementEnabled {}

impl SessionRequest for GetElementEnabled {
    type Reply = ElementEnabled;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        let enabled = element
            .enabled()
            .ok_or(SessionError::UnsupportedEnabledState {
                reference: self.reference,
                reason: format!(
                    "enabled-state inspection for role {} is not implemented",
                    element.role()
                ),
            })?;
        Ok(ElementEnabled {
            reference: self.reference,
            enabled,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementEditable {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementEditable {
    pub reference: InteractiveElementRef,
    pub editable: bool,
}

impl private::Sealed for GetElementEditable {}

impl SessionRequest for GetElementEditable {
    type Reply = ElementEditable;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let source_index = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .source_index_for_interactive(interactive_index);
        let editable = session
            .editable_state(source_index)
            .map_err(|reason| SessionError::UnsupportedEditableState {
                reference: self.reference,
                reason,
            })?
            .ok_or_else(|| {
                let element = &session
                    .current_page
                    .as_ref()
                    .expect("validated reference requires a current page")
                    .interactive_elements[interactive_index];
                SessionError::UnsupportedEditableState {
                    reference: self.reference,
                    reason: format!(
                        "editable-state inspection for role {} is not implemented",
                        element.role()
                    ),
                }
            })?;
        Ok(ElementEditable {
            reference: self.reference,
            editable,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementFocused {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementFocused {
    pub reference: InteractiveElementRef,
    pub focused: bool,
}

impl private::Sealed for GetElementFocused {}

impl SessionRequest for GetElementFocused {
    type Reply = ElementFocused;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page");
        Ok(ElementFocused {
            reference: self.reference,
            focused: page.focused_interactive_index == Some(index),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementHovered {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementHovered {
    pub reference: InteractiveElementRef,
    pub hovered: bool,
}

impl private::Sealed for GetElementHovered {}

impl SessionRequest for GetElementHovered {
    type Reply = ElementHovered;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let interactive_index = session.element_index_for(self.reference)?;
        let page = session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page");
        let source_index = page.source_index_for_interactive(interactive_index);
        Ok(ElementHovered {
            reference: self.reference,
            hovered: page.is_hovered(source_index),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementVisible {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementVisible {
    pub reference: InteractiveElementRef,
    pub visible: bool,
}

impl private::Sealed for GetElementVisible {}

impl SessionRequest for GetElementVisible {
    type Reply = ElementVisible;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        let visible = element
            .visible()
            .map_err(|reason| SessionError::UnsupportedVisibility {
                reference: self.reference,
                reason: reason.into(),
            })?;
        Ok(ElementVisible {
            reference: self.reference,
            visible,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetElementChecked {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCheckedResult {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

impl private::Sealed for SetElementChecked {}

impl SessionRequest for SetElementChecked {
    type Reply = SetCheckedResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let viewport = session.viewport;
        let page = session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page");
        let current = page.interactive_elements[index].checked().ok_or_else(|| {
            SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: format!(
                    "checked-state mutation for role {} is not implemented",
                    page.interactive_elements[index].role()
                ),
            }
        })?;
        if current == self.checked {
            return Ok(SetCheckedResult {
                reference: self.reference,
                checked: current,
            });
        }
        page.validate_set_checked(index, self.checked)
            .map_err(|error| SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: error.reason(),
            })?;
        let visible = page.interactive_elements[index]
            .visible()
            .map_err(|reason| SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: reason.into(),
            })?;
        if !visible {
            return Err(SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: "element is hidden or has an empty box".into(),
            });
        }
        let source_index = page.source_index_for_interactive(index);
        page.locator_elements[source_index]
            .stable()
            .map_err(|reason| SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: format!("stable check failed: {reason}"),
            })?;
        page.receives_events(source_index, viewport)
            .map_err(|reason| SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: format!("receives events check failed: {reason}"),
            })?;
        page.auto_scroll_into_view(source_index, viewport);
        let result = match page.set_checked(index, self.checked) {
            Ok(checked) => Ok(SetCheckedResult {
                reference: self.reference,
                checked,
            }),
            Err(error) => Err(SessionError::UnsupportedCheck {
                reference: self.reference,
                reason: error.reason(),
            }),
        }?;
        let context = session.pointer_action_context(source_index);
        session
            .current_page
            .as_mut()
            .expect("validated reference requires a current page")
            .focused_interactive_index = Some(index);
        session.finish_pointer_click(&context, &[DomEventType::Input, DomEventType::Change]);
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetElementChecked {
    pub reference: InteractiveElementRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementChecked {
    pub reference: InteractiveElementRef,
    pub checked: bool,
}

impl private::Sealed for GetElementChecked {}

impl SessionRequest for GetElementChecked {
    type Reply = ElementChecked;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let index = session.element_index_for(self.reference)?;
        let element = &session
            .current_page
            .as_ref()
            .expect("validated reference requires a current page")
            .interactive_elements[index];
        match element.checked() {
            Some(checked) => Ok(ElementChecked {
                reference: self.reference,
                checked,
            }),
            None => Err(SessionError::UnsupportedCheckedState {
                reference: self.reference,
                reason: format!(
                    "checked-state inspection for role {} is not implemented",
                    element.role()
                ),
            }),
        }
    }
}

fn form_get_url(target: &str, entries: &[(String, String)]) -> Result<String, LoadError> {
    let uri = target
        .parse::<Uri>()
        .map_err(|error| LoadError::InvalidUrl(error.to_string()))?;
    let scheme = uri
        .scheme_str()
        .ok_or_else(|| LoadError::InvalidUrl("the form action has no scheme".into()))?;
    let authority = uri
        .authority()
        .ok_or_else(|| LoadError::InvalidUrl("the form action has no authority".into()))?;
    let path = uri.path_and_query().map_or("/", |value| value.path());
    let data = entries
        .iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                encode_form_component(name),
                encode_form_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let existing = uri
        .path_and_query()
        .and_then(|value| value.query())
        .unwrap_or_default();
    let query = match (existing.is_empty(), data.is_empty()) {
        (true, _) => data,
        (false, true) => existing.into(),
        (false, false) => format!("{existing}&{data}"),
    };
    if query.is_empty() {
        Ok(format!("{scheme}://{authority}{path}"))
    } else {
        Ok(format!("{scheme}://{authority}{path}?{query}"))
    }
}

fn encode_form_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let normalized = normalize_form_line_endings(value);
    let mut encoded = String::with_capacity(normalized.len());
    for byte in normalized.bytes() {
        match byte {
            b' ' => encoded.push('+'),
            b'*' | b'-' | b'.' | b'_' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
                encoded.push(char::from(byte));
            }
            _ => {
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    encoded
}

fn normalize_form_line_endings(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                normalized.push_str("\r\n");
            }
            '\n' => normalized.push_str("\r\n"),
            _ => normalized.push(character),
        }
    }
    normalized
}

fn resolve_navigation_url(base: &str, href: &str) -> Result<String, LoadError> {
    if href.contains('#') {
        return Err(LoadError::UnsupportedTarget(
            "link fragments are not implemented".into(),
        ));
    }
    resolve_url_reference(base, href)
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LintLayout {
    pub input: LayoutInput,
}

impl private::Sealed for LintLayout {}

impl SessionRequest for LintLayout {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.clean_layout(self.input)?;
        Ok(install_layout_result(session, layout))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckElementWidth {
    pub element: String,
    pub maximum_width: u64,
}

impl private::Sealed for CheckElementWidth {}

impl SessionRequest for CheckElementWidth {
    type Reply = RuleResult<WidthFinding>;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let snapshot = session
            .last_snapshot
            .as_ref()
            .ok_or(SessionError::NoSnapshot)?;
        Ok(evaluate_max_element_width(
            snapshot,
            &self.element,
            self.maximum_width,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyMutation {
    pub mutation: LayoutMutation,
}

impl private::Sealed for ApplyMutation {}

impl SessionRequest for ApplyMutation {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.apply_mutation(self.mutation)?;
        Ok(install_layout_result(session, layout))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyMutations {
    pub mutations: Vec<LayoutMutation>,
}

impl private::Sealed for ApplyMutations {}

impl SessionRequest for ApplyMutations {
    type Reply = RuleResult;

    fn execute(self, session: &mut Session) -> Result<Self::Reply, SessionError> {
        let layout = session.layout.apply_mutations(self.mutations)?;
        Ok(install_layout_result(session, layout))
    }
}

fn install_layout_result(session: &mut Session, layout: LayoutSnapshot) -> RuleResult {
    let snapshot_id = SnapshotId::next(&mut session.identities.next_snapshot_id);
    let snapshot = Snapshot::from_layout(snapshot_id, layout);
    let result = evaluate_horizontal_overflow(&snapshot);
    session.last_snapshot = Some(snapshot);
    result
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    Load(LoadError),
    Navigation {
        reference: InteractiveElementRef,
        error: LoadError,
    },
    PressNavigation {
        key: KeyboardKey,
        element: String,
        error: LoadError,
    },
    Layout(LayoutError),
    NoPage,
    NoSnapshot,
    InvalidViewportSize {
        width: u64,
        height: u64,
    },
    UnsupportedScreenshot {
        target: CaptureTarget,
        reason: String,
    },
    StaleElementReference {
        reference: InteractiveElementRef,
    },
    RoleLocatorNotFound {
        locator: RoleLocator,
    },
    RoleLocatorAmbiguous {
        locator: RoleLocator,
        match_count: usize,
    },
    LocatorNotFound {
        locator: Locator,
    },
    LocatorAmbiguous {
        locator: Locator,
        match_count: usize,
    },
    LocatorQuery {
        locator: Locator,
        reason: String,
    },
    UnsupportedLocatorInspection {
        locator: Locator,
        inspection: LocatorInspection,
        reason: String,
    },
    SensitiveLocatorAttribute {
        locator: Locator,
        name: String,
    },
    LocatorSelectOptionNotFound {
        locator: Locator,
        value: String,
    },
    LocatorSelectOptionDisabled {
        locator: Locator,
        value: String,
    },
    LocatorSelectOptionTargetNotFound {
        locator: Locator,
        target: SelectOptionTarget,
    },
    LocatorSelectOptionTargetDisabled {
        locator: Locator,
        target: SelectOptionTarget,
    },
    RoleNavigation {
        locator: RoleLocator,
        error: LoadError,
    },
    LocatorNavigation {
        locator: Locator,
        error: LoadError,
    },
    RoleActionBlocked {
        locator: RoleLocator,
        action: RoleAction,
        check: ActionabilityCheck,
        reason: String,
    },
    LocatorActionBlocked {
        locator: Locator,
        action: LocatorAction,
        check: ActionabilityCheck,
        reason: String,
    },
    UnsupportedRoleAction {
        locator: RoleLocator,
        action: RoleAction,
        reason: String,
    },
    UnsupportedLocatorAction {
        locator: Locator,
        action: LocatorAction,
        reason: String,
    },
    UnsupportedClick {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedFill {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedType {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedFocus {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedHover {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedScrollIntoView {
        reference: InteractiveElementRef,
        reason: String,
    },
    NoFocusedElement,
    UnsupportedPress {
        key: KeyboardKey,
        element: String,
        reason: String,
    },
    UnsupportedSelect {
        reference: InteractiveElementRef,
        reason: String,
    },
    SelectOptionNotFound {
        reference: InteractiveElementRef,
        value: String,
    },
    SelectOptionDisabled {
        reference: InteractiveElementRef,
        value: String,
    },
    SelectOptionTargetNotFound {
        reference: InteractiveElementRef,
        target: SelectOptionTarget,
    },
    SelectOptionTargetDisabled {
        reference: InteractiveElementRef,
        target: SelectOptionTarget,
    },
    UnsupportedValue {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedBoundingBox {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedHtml {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedCheck {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedCheckedState {
        reference: InteractiveElementRef,
        reason: String,
    },
    InvalidAttributeName {
        name: String,
    },
    SensitiveAttribute {
        reference: InteractiveElementRef,
        name: String,
    },
    UnsupportedEnabledState {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedEditableState {
        reference: InteractiveElementRef,
        reason: String,
    },
    UnsupportedVisibility {
        reference: InteractiveElementRef,
        reason: String,
    },
}

fn locator_session_error(locator: Locator, error: LocatorOperationError) -> SessionError {
    match error {
        LocatorOperationError::NoPage => SessionError::NoPage,
        LocatorOperationError::NotFound => SessionError::LocatorNotFound { locator },
        LocatorOperationError::Ambiguous { match_count } => SessionError::LocatorAmbiguous {
            locator,
            match_count,
        },
        LocatorOperationError::Query { reason } => SessionError::LocatorQuery { locator, reason },
        LocatorOperationError::InspectionBlocked { inspection, reason } => {
            SessionError::UnsupportedLocatorInspection {
                locator,
                inspection,
                reason,
            }
        }
        LocatorOperationError::SensitiveAttribute { name } => {
            SessionError::SensitiveLocatorAttribute { locator, name }
        }
        LocatorOperationError::SelectOptionNotFound { target } => match target {
            SelectOptionTarget::Value(value) => {
                SessionError::LocatorSelectOptionNotFound { locator, value }
            }
            target => SessionError::LocatorSelectOptionTargetNotFound { locator, target },
        },
        LocatorOperationError::SelectOptionDisabled { target } => match target {
            SelectOptionTarget::Value(value) => {
                SessionError::LocatorSelectOptionDisabled { locator, value }
            }
            target => SessionError::LocatorSelectOptionTargetDisabled { locator, target },
        },
        LocatorOperationError::Navigation(error) => {
            SessionError::LocatorNavigation { locator, error }
        }
        LocatorOperationError::ActionBlocked {
            action,
            check,
            reason,
        } => SessionError::LocatorActionBlocked {
            locator,
            action,
            check,
            reason,
        },
        LocatorOperationError::UnsupportedAction { action, reason } => {
            SessionError::UnsupportedLocatorAction {
                locator,
                action,
                reason,
            }
        }
    }
}

fn role_session_error(locator: RoleLocator, error: LocatorOperationError) -> SessionError {
    match error {
        LocatorOperationError::NoPage => SessionError::NoPage,
        LocatorOperationError::NotFound => SessionError::RoleLocatorNotFound { locator },
        LocatorOperationError::Ambiguous { match_count } => SessionError::RoleLocatorAmbiguous {
            locator,
            match_count,
        },
        LocatorOperationError::Query { reason } => SessionError::LocatorQuery {
            locator: Locator::from(locator),
            reason,
        },
        LocatorOperationError::InspectionBlocked { .. }
        | LocatorOperationError::SensitiveAttribute { .. }
        | LocatorOperationError::SelectOptionNotFound { .. }
        | LocatorOperationError::SelectOptionDisabled { .. } => {
            unreachable!("role requests do not execute generic locator reads or selection")
        }
        LocatorOperationError::Navigation(error) => SessionError::RoleNavigation { locator, error },
        LocatorOperationError::ActionBlocked {
            action,
            check,
            reason,
        } => SessionError::RoleActionBlocked {
            locator,
            action,
            check,
            reason,
        },
        LocatorOperationError::UnsupportedAction { action, reason } => {
            SessionError::UnsupportedRoleAction {
                locator,
                action,
                reason,
            }
        }
    }
}

impl From<SelectorQueryError> for LocatorOperationError {
    fn from(error: SelectorQueryError) -> Self {
        Self::Query {
            reason: error.to_string(),
        }
    }
}

fn require_locator_visible(
    element: &InteractiveElementSource,
    action: LocatorAction,
) -> Result<(), LocatorOperationError> {
    match element.visible() {
        Ok(true) => Ok(()),
        Ok(false) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Visible,
            reason: "element is hidden or has an empty box".into(),
        }),
        Err(reason) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Visible,
            reason: reason.into(),
        }),
    }
}

fn require_locator_stable(
    element: &LocatorElementSource,
    action: LocatorAction,
) -> Result<(), LocatorOperationError> {
    element
        .stable()
        .map_err(|reason| LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Stable,
            reason: reason.into(),
        })
}

fn require_locator_receives_events(
    page: &CurrentPage,
    source_index: usize,
    action: LocatorAction,
    viewport: ViewportSize,
) -> Result<(), LocatorOperationError> {
    page.receives_events(source_index, viewport)
        .map_err(|reason| LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::ReceivesEvents,
            reason,
        })
}

fn require_locator_enabled(
    element: &InteractiveElementSource,
    action: LocatorAction,
) -> Result<(), LocatorOperationError> {
    match element.enabled() {
        Some(true) => Ok(()),
        Some(false) => Err(LocatorOperationError::ActionBlocked {
            action,
            check: ActionabilityCheck::Enabled,
            reason: "element is disabled".into(),
        }),
        None => Err(LocatorOperationError::UnsupportedAction {
            action,
            reason: format!(
                "enabled-state evidence for role {} is not implemented",
                element.role()
            ),
        }),
    }
}

impl From<LoadError> for SessionError {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<LayoutError> for SessionError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_navigation_url;
    use crate::LoadError;

    #[test]
    fn resolves_relative_paths_and_queries() {
        let base = "http://localhost:3000/guide/current?old=1";

        assert_eq!(
            resolve_navigation_url(base, "../next?q=1").unwrap(),
            "http://localhost:3000/next?q=1"
        );
        assert_eq!(
            resolve_navigation_url(base, "child").unwrap(),
            "http://localhost:3000/guide/child"
        );
        assert_eq!(
            resolve_navigation_url(base, "?new=1").unwrap(),
            "http://localhost:3000/guide/current?new=1"
        );
        assert_eq!(resolve_navigation_url(base, "").unwrap(), base);
    }

    #[test]
    fn preserves_absolute_and_network_targets_for_loader_policy() {
        let base = "http://localhost:3000/current";

        assert_eq!(
            resolve_navigation_url(base, "http://example.com/away").unwrap(),
            "http://example.com/away"
        );
        assert_eq!(
            resolve_navigation_url(base, "//example.com/away").unwrap(),
            "http://example.com/away"
        );
    }

    #[test]
    fn rejects_fragments_until_same_document_navigation_exists() {
        let result = resolve_navigation_url("http://localhost:3000/current", "#details");

        assert!(matches!(result, Err(LoadError::UnsupportedTarget(_))));
    }
}
