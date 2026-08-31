#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardKey {
    kind: KeyboardKeyKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardEventKey {
    kind: KeyboardEventKeyKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardModifier {
    Alt,
    Control,
    ControlOrMeta,
    Meta,
    Shift,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KeyboardEventKeyKind {
    Modifier(KeyboardModifier),
    Press(KeyboardKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeyboardKeyKind {
    Character(char),
    Enter,
    Backspace,
    Delete,
    ArrowLeft { extend: bool },
    ArrowRight { extend: bool },
    ArrowUp,
    ArrowDown,
    Home { extend: bool },
    End { extend: bool },
    SelectAll,
    Tab { reverse: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyboardPressEventKind {
    PrintableAscii,
    OtherCharacter,
    Enter,
    Editing,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ControlActivationKey {
    Enter,
    Space,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardKeyError {
    key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextSelection {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FocusTraversalDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RadioGroupDirection {
    Previous,
    Next,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusedElement {
    pub element: String,
    pub role: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextPressEffect {
    pub element: FocusedElement,
    pub value: String,
    pub selection: TextSelection,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyboardTextEffect {
    Text(TextPressEffect),
    Ignored { element: Option<FocusedElement> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationPressEffect {
    pub element: FocusedElement,
    pub url: String,
    pub interactive_element_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusTraversalEffect {
    pub previous: Option<FocusedElement>,
    pub current: Option<FocusedElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PressEffect {
    Text(TextPressEffect),
    FocusTraversal(FocusTraversalEffect),
    Navigated(NavigationPressEffect),
    Ignored {
        element: FocusedElement,
    },
    Activated {
        element: FocusedElement,
    },
    Checked {
        element: FocusedElement,
        checked: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TextSelectionState {
    anchor: usize,
    focus: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TextKeyOutcome {
    pub(crate) changed: bool,
    pub(crate) selection: TextSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextKeyError {
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModifiedKeyError {
    pub(crate) key: KeyboardKey,
    pub(crate) reason: String,
}

impl KeyboardKey {
    pub fn new(key: impl Into<String>) -> Result<Self, KeyboardKeyError> {
        let key = key.into();
        let kind = match key.as_str() {
            "Enter" => KeyboardKeyKind::Enter,
            "Space" => KeyboardKeyKind::Character(' '),
            "Backspace" => KeyboardKeyKind::Backspace,
            "Delete" => KeyboardKeyKind::Delete,
            "ArrowLeft" => KeyboardKeyKind::ArrowLeft { extend: false },
            "ArrowRight" => KeyboardKeyKind::ArrowRight { extend: false },
            "ArrowUp" => KeyboardKeyKind::ArrowUp,
            "ArrowDown" => KeyboardKeyKind::ArrowDown,
            "Home" => KeyboardKeyKind::Home { extend: false },
            "End" => KeyboardKeyKind::End { extend: false },
            "Shift+ArrowLeft" => KeyboardKeyKind::ArrowLeft { extend: true },
            "Shift+ArrowRight" => KeyboardKeyKind::ArrowRight { extend: true },
            "Shift+Home" => KeyboardKeyKind::Home { extend: true },
            "Shift+End" => KeyboardKeyKind::End { extend: true },
            "ControlOrMeta+A" | "ControlOrMeta+a" | "Control+A" | "Control+a" | "Meta+A"
            | "Meta+a" => KeyboardKeyKind::SelectAll,
            "Tab" => KeyboardKeyKind::Tab { reverse: false },
            "Shift+Tab" => KeyboardKeyKind::Tab { reverse: true },
            _ => {
                let mut characters = key.chars();
                let Some(character) = characters.next() else {
                    return Err(KeyboardKeyError { key });
                };
                if characters.next().is_some() || character.is_control() {
                    return Err(KeyboardKeyError { key });
                }
                KeyboardKeyKind::Character(character)
            }
        };
        Ok(Self { kind })
    }

    pub(crate) fn focus_traversal_direction(&self) -> Option<FocusTraversalDirection> {
        match self.kind {
            KeyboardKeyKind::Tab { reverse: false } => Some(FocusTraversalDirection::Forward),
            KeyboardKeyKind::Tab { reverse: true } => Some(FocusTraversalDirection::Reverse),
            KeyboardKeyKind::Character(_)
            | KeyboardKeyKind::Enter
            | KeyboardKeyKind::Backspace
            | KeyboardKeyKind::Delete
            | KeyboardKeyKind::ArrowLeft { .. }
            | KeyboardKeyKind::ArrowRight { .. }
            | KeyboardKeyKind::ArrowUp
            | KeyboardKeyKind::ArrowDown
            | KeyboardKeyKind::Home { .. }
            | KeyboardKeyKind::End { .. }
            | KeyboardKeyKind::SelectAll => None,
        }
    }

    pub(crate) fn control_activation_key(&self) -> Option<ControlActivationKey> {
        match self.kind {
            KeyboardKeyKind::Enter => Some(ControlActivationKey::Enter),
            KeyboardKeyKind::Character(' ') => Some(ControlActivationKey::Space),
            KeyboardKeyKind::Character(_)
            | KeyboardKeyKind::Backspace
            | KeyboardKeyKind::Delete
            | KeyboardKeyKind::ArrowLeft { .. }
            | KeyboardKeyKind::ArrowRight { .. }
            | KeyboardKeyKind::ArrowUp
            | KeyboardKeyKind::ArrowDown
            | KeyboardKeyKind::Home { .. }
            | KeyboardKeyKind::End { .. }
            | KeyboardKeyKind::SelectAll
            | KeyboardKeyKind::Tab { .. } => None,
        }
    }

    pub(crate) fn radio_group_direction(&self) -> Option<RadioGroupDirection> {
        match self.kind {
            KeyboardKeyKind::ArrowLeft { extend: false } | KeyboardKeyKind::ArrowUp => {
                Some(RadioGroupDirection::Previous)
            }
            KeyboardKeyKind::ArrowRight { extend: false } | KeyboardKeyKind::ArrowDown => {
                Some(RadioGroupDirection::Next)
            }
            KeyboardKeyKind::Character(_)
            | KeyboardKeyKind::Enter
            | KeyboardKeyKind::Backspace
            | KeyboardKeyKind::Delete
            | KeyboardKeyKind::ArrowLeft { extend: true }
            | KeyboardKeyKind::ArrowRight { extend: true }
            | KeyboardKeyKind::Home { .. }
            | KeyboardKeyKind::End { .. }
            | KeyboardKeyKind::SelectAll
            | KeyboardKeyKind::Tab { .. } => None,
        }
    }

    pub(crate) fn apply_to_text(
        &self,
        value: &mut String,
        selection: &mut TextSelectionState,
        multiline: bool,
        editable: bool,
    ) -> Result<TextKeyOutcome, TextKeyError> {
        selection.clamp_to(value);
        let changed = match self.kind {
            KeyboardKeyKind::Character(character) => {
                replace_selection(value, selection, &character.to_string(), editable)
            }
            KeyboardKeyKind::Enter if multiline => {
                replace_selection(value, selection, "\n", editable)
            }
            KeyboardKeyKind::Enter => {
                return Err(TextKeyError {
                    reason: "Enter behavior outside a textarea is not implemented".into(),
                });
            }
            KeyboardKeyKind::Backspace => delete_backward(value, selection, editable),
            KeyboardKeyKind::Delete => delete_forward(value, selection, editable),
            KeyboardKeyKind::ArrowLeft { extend } => {
                selection.move_left(value, extend);
                false
            }
            KeyboardKeyKind::ArrowRight { extend } => {
                selection.move_right(value, extend);
                false
            }
            KeyboardKeyKind::ArrowUp | KeyboardKeyKind::ArrowDown => {
                return Err(TextKeyError {
                    reason: "vertical text movement is not implemented".into(),
                });
            }
            KeyboardKeyKind::Home { extend } => {
                let target = if multiline {
                    line_start(value, selection.focus)
                } else {
                    0
                };
                selection.move_to(target, extend);
                false
            }
            KeyboardKeyKind::End { extend } => {
                let target = if multiline {
                    line_end(value, selection.focus)
                } else {
                    utf16_len(value)
                };
                selection.move_to(target, extend);
                false
            }
            KeyboardKeyKind::SelectAll => {
                selection.anchor = 0;
                selection.focus = utf16_len(value);
                false
            }
            KeyboardKeyKind::Tab { .. } => {
                return Err(TextKeyError {
                    reason: "Tab changes page focus instead of a text value".into(),
                });
            }
        };
        Ok(TextKeyOutcome {
            changed,
            selection: selection.range(),
        })
    }

    pub(crate) fn press_event_kind(&self) -> KeyboardPressEventKind {
        match self.kind {
            KeyboardKeyKind::Character(character) if (' '..='~').contains(&character) => {
                KeyboardPressEventKind::PrintableAscii
            }
            KeyboardKeyKind::Character(_) => KeyboardPressEventKind::OtherCharacter,
            KeyboardKeyKind::Enter => KeyboardPressEventKind::Enter,
            KeyboardKeyKind::Backspace | KeyboardKeyKind::Delete => KeyboardPressEventKind::Editing,
            KeyboardKeyKind::ArrowLeft { .. }
            | KeyboardKeyKind::ArrowRight { .. }
            | KeyboardKeyKind::ArrowUp
            | KeyboardKeyKind::ArrowDown
            | KeyboardKeyKind::Home { .. }
            | KeyboardKeyKind::End { .. }
            | KeyboardKeyKind::SelectAll
            | KeyboardKeyKind::Tab { .. } => KeyboardPressEventKind::Other,
        }
    }

    pub(crate) fn has_embedded_modifiers(&self) -> bool {
        matches!(
            self.kind,
            KeyboardKeyKind::ArrowLeft { extend: true }
                | KeyboardKeyKind::ArrowRight { extend: true }
                | KeyboardKeyKind::Home { extend: true }
                | KeyboardKeyKind::End { extend: true }
                | KeyboardKeyKind::SelectAll
                | KeyboardKeyKind::Tab { reverse: true }
        )
    }

    pub(crate) fn with_modifiers(
        &self,
        modifiers: &[KeyboardModifier],
    ) -> Result<Self, ModifiedKeyError> {
        let shift = modifiers.contains(&KeyboardModifier::Shift);
        let control_or_meta = modifiers.iter().any(|modifier| {
            matches!(
                modifier,
                KeyboardModifier::Control
                    | KeyboardModifier::ControlOrMeta
                    | KeyboardModifier::Meta
            )
        });
        let alt = modifiers.contains(&KeyboardModifier::Alt);
        if alt || (control_or_meta && !matches!(self.kind, KeyboardKeyKind::Character('a' | 'A'))) {
            return Err(ModifiedKeyError {
                key: self.clone(),
                reason: "the held modifier combination has no modeled default effect".into(),
            });
        }
        let kind = match self.kind {
            KeyboardKeyKind::Character(character)
                if control_or_meta && character.eq_ignore_ascii_case(&'a') =>
            {
                KeyboardKeyKind::SelectAll
            }
            KeyboardKeyKind::Character(character) if shift => {
                KeyboardKeyKind::Character(shifted_character(character))
            }
            KeyboardKeyKind::ArrowLeft { extend } => KeyboardKeyKind::ArrowLeft {
                extend: extend || shift,
            },
            KeyboardKeyKind::ArrowRight { extend } => KeyboardKeyKind::ArrowRight {
                extend: extend || shift,
            },
            KeyboardKeyKind::Home { extend } => KeyboardKeyKind::Home {
                extend: extend || shift,
            },
            KeyboardKeyKind::End { extend } => KeyboardKeyKind::End {
                extend: extend || shift,
            },
            KeyboardKeyKind::Tab { reverse } => KeyboardKeyKind::Tab {
                reverse: reverse || shift,
            },
            _ => self.kind.clone(),
        };
        Ok(Self { kind })
    }
}

impl KeyboardEventKey {
    pub fn new(key: impl Into<String>) -> Result<Self, KeyboardKeyError> {
        let key = key.into();
        let kind = match key.as_str() {
            "Alt" | "AltLeft" | "AltRight" => KeyboardEventKeyKind::Modifier(KeyboardModifier::Alt),
            "Control" | "ControlLeft" | "ControlRight" => {
                KeyboardEventKeyKind::Modifier(KeyboardModifier::Control)
            }
            "ControlOrMeta" => KeyboardEventKeyKind::Modifier(KeyboardModifier::ControlOrMeta),
            "Meta" | "MetaLeft" | "MetaRight" => {
                KeyboardEventKeyKind::Modifier(KeyboardModifier::Meta)
            }
            "Shift" | "ShiftLeft" | "ShiftRight" => {
                KeyboardEventKeyKind::Modifier(KeyboardModifier::Shift)
            }
            _ => {
                let press = KeyboardKey::new(key.clone())?;
                if press.has_embedded_modifiers() {
                    return Err(KeyboardKeyError { key });
                }
                KeyboardEventKeyKind::Press(press)
            }
        };
        Ok(Self { kind })
    }

    pub fn modifier(&self) -> Option<KeyboardModifier> {
        match self.kind {
            KeyboardEventKeyKind::Modifier(modifier) => Some(modifier),
            KeyboardEventKeyKind::Press(_) => None,
        }
    }

    pub(crate) fn press_key(
        &self,
        modifiers: &[KeyboardModifier],
    ) -> Result<Option<KeyboardKey>, ModifiedKeyError> {
        match &self.kind {
            KeyboardEventKeyKind::Modifier(_) => Ok(None),
            KeyboardEventKeyKind::Press(key) => key.with_modifiers(modifiers).map(Some),
        }
    }
}

impl std::fmt::Display for KeyboardKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            KeyboardKeyKind::Character(' ') => formatter.write_str("Space"),
            KeyboardKeyKind::Character(character) => character.fmt(formatter),
            KeyboardKeyKind::Enter => formatter.write_str("Enter"),
            KeyboardKeyKind::Backspace => formatter.write_str("Backspace"),
            KeyboardKeyKind::Delete => formatter.write_str("Delete"),
            KeyboardKeyKind::ArrowLeft { extend: false } => formatter.write_str("ArrowLeft"),
            KeyboardKeyKind::ArrowRight { extend: false } => formatter.write_str("ArrowRight"),
            KeyboardKeyKind::ArrowUp => formatter.write_str("ArrowUp"),
            KeyboardKeyKind::ArrowDown => formatter.write_str("ArrowDown"),
            KeyboardKeyKind::Home { extend: false } => formatter.write_str("Home"),
            KeyboardKeyKind::End { extend: false } => formatter.write_str("End"),
            KeyboardKeyKind::ArrowLeft { extend: true } => formatter.write_str("Shift+ArrowLeft"),
            KeyboardKeyKind::ArrowRight { extend: true } => formatter.write_str("Shift+ArrowRight"),
            KeyboardKeyKind::Home { extend: true } => formatter.write_str("Shift+Home"),
            KeyboardKeyKind::End { extend: true } => formatter.write_str("Shift+End"),
            KeyboardKeyKind::SelectAll => formatter.write_str("ControlOrMeta+A"),
            KeyboardKeyKind::Tab { reverse: false } => formatter.write_str("Tab"),
            KeyboardKeyKind::Tab { reverse: true } => formatter.write_str("Shift+Tab"),
        }
    }
}

impl std::fmt::Display for KeyboardEventKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            KeyboardEventKeyKind::Modifier(modifier) => modifier.fmt(formatter),
            KeyboardEventKeyKind::Press(key) => key.fmt(formatter),
        }
    }
}

impl std::fmt::Display for KeyboardModifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Alt => "Alt",
            Self::Control => "Control",
            Self::ControlOrMeta => "ControlOrMeta",
            Self::Meta => "Meta",
            Self::Shift => "Shift",
        })
    }
}

impl std::fmt::Display for KeyboardKeyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unsupported key {:?}; expected one non-control character or a documented key",
            self.key
        )
    }
}

impl std::error::Error for KeyboardKeyError {}

impl FocusedElement {
    pub(crate) fn new(element: &str, role: &str, name: &str) -> Self {
        Self {
            element: element.into(),
            role: role.into(),
            name: name.into(),
        }
    }
}

impl TextSelection {
    pub fn start(self) -> usize {
        self.start
    }

    pub fn end(self) -> usize {
        self.end
    }
}

impl TextSelectionState {
    pub(crate) fn collapsed_at(offset: usize) -> Self {
        Self {
            anchor: offset,
            focus: offset,
        }
    }

    pub(crate) fn collapse_at_end(&mut self, value: &str) {
        *self = Self::collapsed_at(utf16_len(value));
    }

    pub(crate) fn insert_text(
        &mut self,
        value: &mut String,
        text: &str,
        editable: bool,
    ) -> TextKeyOutcome {
        self.clamp_to(value);
        let before = value.clone();
        if !text.is_empty() {
            replace_selection(value, self, text, editable);
        }
        TextKeyOutcome {
            changed: *value != before,
            selection: self.range(),
        }
    }

    pub(crate) fn type_text(
        &mut self,
        value: &mut String,
        text: &str,
        multiline: bool,
        editable: bool,
    ) -> TextKeyOutcome {
        self.clamp_to(value);
        let before = value.clone();
        for character in text.chars() {
            let replacement = match character {
                '\r' | '\n' if multiline => "\n".into(),
                '\r' | '\n' => continue,
                _ => character.to_string(),
            };
            replace_selection(value, self, &replacement, editable);
        }
        TextKeyOutcome {
            changed: *value != before,
            selection: self.range(),
        }
    }

    fn range(self) -> TextSelection {
        TextSelection {
            start: self.anchor.min(self.focus),
            end: self.anchor.max(self.focus),
        }
    }

    fn clamp_to(&mut self, value: &str) {
        let end = utf16_len(value);
        if self.anchor > end || !is_utf16_boundary(value, self.anchor) {
            self.anchor = end;
        }
        if self.focus > end || !is_utf16_boundary(value, self.focus) {
            self.focus = end;
        }
    }

    fn move_left(&mut self, value: &str, extend: bool) {
        if extend {
            self.focus = previous_utf16_boundary(value, self.focus);
        } else {
            let range = self.range();
            let target = if range.start != range.end {
                range.start
            } else {
                previous_utf16_boundary(value, self.focus)
            };
            *self = Self::collapsed_at(target);
        }
    }

    fn move_right(&mut self, value: &str, extend: bool) {
        if extend {
            self.focus = next_utf16_boundary(value, self.focus);
        } else {
            let range = self.range();
            let target = if range.start != range.end {
                range.end
            } else {
                next_utf16_boundary(value, self.focus)
            };
            *self = Self::collapsed_at(target);
        }
    }

    fn move_to(&mut self, target: usize, extend: bool) {
        if extend {
            self.focus = target;
        } else {
            *self = Self::collapsed_at(target);
        }
    }
}

fn replace_selection(
    value: &mut String,
    selection: &mut TextSelectionState,
    replacement: &str,
    editable: bool,
) -> bool {
    if !editable {
        return false;
    }
    let range = selection.range();
    replace_utf16_range(value, range, replacement);
    *selection = TextSelectionState::collapsed_at(range.start + utf16_len(replacement));
    true
}

fn shifted_character(character: char) -> char {
    match character {
        'a'..='z' => character.to_ascii_uppercase(),
        '`' => '~',
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        _ => character,
    }
}

fn delete_backward(value: &mut String, selection: &mut TextSelectionState, editable: bool) -> bool {
    if !editable {
        return false;
    }
    let range = selection.range();
    if range.start != range.end {
        replace_utf16_range(value, range, "");
        *selection = TextSelectionState::collapsed_at(range.start);
        return true;
    }
    let start = previous_utf16_boundary(value, range.start);
    if start == range.start {
        return false;
    }
    replace_utf16_range(
        value,
        TextSelection {
            start,
            end: range.start,
        },
        "",
    );
    *selection = TextSelectionState::collapsed_at(start);
    true
}

fn delete_forward(value: &mut String, selection: &mut TextSelectionState, editable: bool) -> bool {
    if !editable {
        return false;
    }
    let range = selection.range();
    if range.start != range.end {
        replace_utf16_range(value, range, "");
        *selection = TextSelectionState::collapsed_at(range.start);
        return true;
    }
    let end = next_utf16_boundary(value, range.end);
    if end == range.end {
        return false;
    }
    replace_utf16_range(
        value,
        TextSelection {
            start: range.end,
            end,
        },
        "",
    );
    *selection = TextSelectionState::collapsed_at(range.end);
    true
}

fn replace_utf16_range(value: &mut String, range: TextSelection, replacement: &str) {
    let start = byte_index_at_utf16_boundary(value, range.start);
    let end = byte_index_at_utf16_boundary(value, range.end);
    value.replace_range(start..end, replacement);
}

fn line_start(value: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16_boundary(value, offset);
    value[..byte]
        .rfind('\n')
        .map_or(0, |newline| utf16_len(&value[..=newline]))
}

fn line_end(value: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16_boundary(value, offset);
    value[byte..].find('\n').map_or_else(
        || utf16_len(value),
        |newline| offset + utf16_len(&value[byte..byte + newline]),
    )
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn is_utf16_boundary(value: &str, offset: usize) -> bool {
    offset == utf16_len(value)
        || value
            .char_indices()
            .scan(0, |units, (_, character)| {
                let current = *units;
                *units += character.len_utf16();
                Some(current)
            })
            .any(|boundary| boundary == offset)
}

fn byte_index_at_utf16_boundary(value: &str, offset: usize) -> usize {
    if offset == utf16_len(value) {
        return value.len();
    }
    let mut units = 0;
    for (byte, character) in value.char_indices() {
        if units == offset {
            return byte;
        }
        units += character.len_utf16();
    }
    panic!("text selection offset must be a UTF-16 scalar boundary");
}

fn previous_utf16_boundary(value: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16_boundary(value, offset);
    value[..byte]
        .chars()
        .next_back()
        .map_or(0, |character| offset - character.len_utf16())
}

fn next_utf16_boundary(value: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16_boundary(value, offset);
    value[byte..]
        .chars()
        .next()
        .map_or(offset, |character| offset + character.len_utf16())
}

#[cfg(test)]
mod tests {
    use super::{KeyboardEventKey, KeyboardKey, KeyboardModifier, TextSelectionState};

    fn apply(
        value: &mut String,
        selection: &mut TextSelectionState,
        key: &str,
        multiline: bool,
    ) -> (bool, usize, usize) {
        let result = KeyboardKey::new(key)
            .unwrap()
            .apply_to_text(value, selection, multiline, true)
            .unwrap();
        (
            result.changed,
            result.selection.start(),
            result.selection.end(),
        )
    }

    #[test]
    fn editing_keys_use_utf16_offsets_and_unicode_scalar_boundaries() {
        let mut value = "a😀b".to_owned();
        let mut selection = TextSelectionState::collapsed_at(0);

        apply(&mut value, &mut selection, "ArrowRight", false);
        assert_eq!(
            apply(&mut value, &mut selection, "ArrowRight", false),
            (false, 3, 3)
        );
        assert_eq!(
            apply(&mut value, &mut selection, "Backspace", false),
            (true, 1, 1)
        );
        assert_eq!(value, "ab");
    }

    #[test]
    fn selection_extension_replaces_the_selected_range() {
        let mut value = "abc".to_owned();
        let mut selection = TextSelectionState::collapsed_at(0);

        apply(&mut value, &mut selection, "ArrowRight", false);
        assert_eq!(
            apply(&mut value, &mut selection, "Shift+ArrowRight", false),
            (false, 1, 2)
        );
        assert_eq!(apply(&mut value, &mut selection, "X", false), (true, 2, 2));
        assert_eq!(value, "aXc");
    }

    #[test]
    fn event_keys_normalize_modifiers_and_apply_held_state() {
        assert_eq!(
            KeyboardEventKey::new("ShiftLeft").unwrap(),
            KeyboardEventKey::new("Shift").unwrap()
        );
        assert!(KeyboardEventKey::new("Shift+Tab").is_err());

        let modifiers = [KeyboardModifier::Shift];
        assert_eq!(
            KeyboardEventKey::new("a")
                .unwrap()
                .press_key(&modifiers)
                .unwrap()
                .unwrap()
                .to_string(),
            "A"
        );
        assert_eq!(
            KeyboardEventKey::new("1")
                .unwrap()
                .press_key(&modifiers)
                .unwrap()
                .unwrap()
                .to_string(),
            "!"
        );
        assert_eq!(
            KeyboardEventKey::new("Tab")
                .unwrap()
                .press_key(&modifiers)
                .unwrap()
                .unwrap()
                .to_string(),
            "Shift+Tab"
        );

        let modifiers = [KeyboardModifier::ControlOrMeta];
        assert_eq!(
            KeyboardEventKey::new("a")
                .unwrap()
                .press_key(&modifiers)
                .unwrap()
                .unwrap()
                .to_string(),
            "ControlOrMeta+A"
        );
        assert!(
            KeyboardEventKey::new("b")
                .unwrap()
                .press_key(&modifiers)
                .is_err()
        );
    }

    #[test]
    fn text_insertion_replaces_selection_and_preserves_readonly_state() {
        let mut value = "a😀b".to_owned();
        let mut selection = TextSelectionState::collapsed_at(1);
        apply(&mut value, &mut selection, "Shift+ArrowRight", false);

        let inserted = selection.insert_text(&mut value, "xy", true);
        assert_eq!(value, "axyb");
        assert_eq!(
            (inserted.selection.start(), inserted.selection.end()),
            (3, 3)
        );
        assert!(inserted.changed);

        let readonly = selection.insert_text(&mut value, "z", false);
        assert_eq!(value, "axyb");
        assert_eq!(
            (readonly.selection.start(), readonly.selection.end()),
            (3, 3)
        );
        assert!(!readonly.changed);
    }

    #[test]
    fn typed_text_applies_scalars_and_normalizes_line_breaks() {
        let mut textarea_value = String::new();
        let mut textarea_selection = TextSelectionState::collapsed_at(0);
        let textarea = textarea_selection.type_text(&mut textarea_value, "a\r\n😀", true, true);
        assert_eq!(textarea_value, "a\n\n😀");
        assert_eq!(
            (textarea.selection.start(), textarea.selection.end()),
            (5, 5)
        );
        assert!(textarea.changed);

        let mut input_value = String::new();
        let mut input_selection = TextSelectionState::collapsed_at(0);
        let input = input_selection.type_text(&mut input_value, "a\r\n😀", false, true);
        assert_eq!(input_value, "a😀");
        assert_eq!((input.selection.start(), input.selection.end()), (3, 3));
        assert!(input.changed);

        let readonly = input_selection.type_text(&mut input_value, "x", false, false);
        assert_eq!(input_value, "a😀");
        assert!(!readonly.changed);
    }

    #[test]
    fn textarea_home_and_end_use_logical_line_boundaries() {
        let mut value = "ab\ncd".to_owned();
        let mut selection = TextSelectionState::collapsed_at(0);

        for _ in 0..4 {
            apply(&mut value, &mut selection, "ArrowRight", true);
        }
        assert_eq!(
            apply(&mut value, &mut selection, "Home", true),
            (false, 3, 3)
        );
        assert_eq!(
            apply(&mut value, &mut selection, "End", true),
            (false, 5, 5)
        );
    }

    #[test]
    fn readonly_text_navigation_works_without_mutation() {
        let mut value = "abc".to_owned();
        let mut selection = TextSelectionState::collapsed_at(0);

        let movement = KeyboardKey::new("ArrowRight")
            .unwrap()
            .apply_to_text(&mut value, &mut selection, false, false)
            .unwrap();
        let result = KeyboardKey::new("X")
            .unwrap()
            .apply_to_text(&mut value, &mut selection, false, false)
            .unwrap();

        assert_eq!(
            (movement.selection.start(), movement.selection.end()),
            (1, 1)
        );
        assert!(!result.changed);
        assert_eq!(value, "abc");
        assert_eq!(result.selection.start(), 1);
    }

    #[test]
    fn shifted_line_boundaries_preserve_the_selection_anchor() {
        let mut value = "ab\ncd".to_owned();
        let mut selection = TextSelectionState::collapsed_at(0);
        for _ in 0..4 {
            apply(&mut value, &mut selection, "ArrowRight", true);
        }

        assert_eq!(
            apply(&mut value, &mut selection, "Shift+Home", true),
            (false, 3, 4)
        );
        assert_eq!(
            apply(&mut value, &mut selection, "Shift+End", true),
            (false, 4, 5)
        );
    }

    #[test]
    fn backspace_removes_a_combining_mark_as_one_scalar() {
        let mut value = "aéb".to_owned();
        let mut selection = TextSelectionState::collapsed_at(3);

        assert_eq!(
            apply(&mut value, &mut selection, "Backspace", false),
            (true, 2, 2)
        );
        assert_eq!(value, "aeb");
    }
}
