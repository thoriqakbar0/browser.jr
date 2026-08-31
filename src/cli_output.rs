use std::io::{self, Write};

use serde_json::{Map, Value, json};

use crate::{
    AccessibilitySnapshot, AccessibilitySnapshotNode, InteractiveElement, InteractiveElementState,
    InteractiveSnapshot,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SnapshotOutputOptions {
    pub(crate) include_urls: bool,
}

pub(crate) fn write_snapshot_json(
    output: &mut impl Write,
    snapshot: &InteractiveSnapshot,
    options: SnapshotOutputOptions,
) -> io::Result<()> {
    let mut refs = Map::new();
    let snapshot_text = snapshot
        .elements
        .iter()
        .map(|element| format_snapshot_element(element, &reference_key(element), options))
        .collect::<Vec<_>>()
        .join("\n");

    for element in &snapshot.elements {
        refs.insert(
            reference_key(element),
            json!({
                "name": element.name,
                "role": element.role,
            }),
        );
    }

    write_json_line(
        output,
        &json!({
            "success": true,
            "data": {
                "origin": snapshot.url,
                "refs": Value::Object(refs),
                "snapshot": snapshot_text,
            },
            "error": Value::Null,
        }),
    )
}

pub(crate) fn write_accessibility_snapshot_json(
    output: &mut impl Write,
    snapshot: &AccessibilitySnapshot,
    options: SnapshotOutputOptions,
) -> io::Result<()> {
    let snapshot_text = snapshot
        .nodes
        .iter()
        .map(|node| format_accessibility_snapshot_node(node, options))
        .collect::<Vec<_>>()
        .join("\n");
    let refs = snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            node.reference.map(|reference| {
                (
                    format!("e{}", reference.ordinal()),
                    json!({
                        "name": node.name(),
                        "role": node.role(),
                    }),
                )
            })
        })
        .collect::<Map<_, _>>();
    write_json_line(
        output,
        &json!({
            "success": true,
            "data": {
                "origin": snapshot.url,
                "refs": Value::Object(refs),
                "snapshot": snapshot_text,
            },
            "error": Value::Null,
        }),
    )
}

pub(crate) fn write_error_json(output: &mut impl Write, message: &str) -> io::Result<()> {
    write_json_line(
        output,
        &json!({
            "success": false,
            "data": Value::Null,
            "error": message,
        }),
    )
}

pub(crate) fn write_session_lifecycle_json(output: &mut impl Write, event: &str) -> io::Result<()> {
    write_json_line(
        output,
        &json!({
            "success": true,
            "data": {
                "event": event,
            },
            "error": Value::Null,
        }),
    )
}

pub(crate) fn write_session_command_json(
    output: &mut impl Write,
    sequence: u64,
    command_output: &str,
    command_error: Option<&str>,
) -> io::Result<()> {
    write_json_line(
        output,
        &json!({
            "success": command_error.is_none(),
            "data": {
                "event": "command",
                "sequence": sequence,
                "output": command_output,
            },
            "error": command_error.map_or(Value::Null, |message| Value::String(message.into())),
        }),
    )
}

fn write_json_line(output: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value).map_err(io::Error::other)?;
    output.write_all(b"\n")
}

pub(crate) fn format_snapshot_element(
    element: &InteractiveElement,
    reference: &str,
    options: SnapshotOutputOptions,
) -> String {
    let state = format_snapshot_state(&element.state);
    let url = options
        .include_urls
        .then_some(element.target_url())
        .flatten()
        .map_or_else(String::new, |url| format!(", url={url}"));
    format!(
        "{}- {} {:?} [ref={reference}{url}]{}",
        "  ".repeat(usize::try_from(element.depth()).expect("snapshot depth fits usize")),
        element.role,
        element.name,
        state
    )
}

pub(crate) fn format_accessibility_snapshot_node(
    node: &AccessibilitySnapshotNode,
    options: SnapshotOutputOptions,
) -> String {
    let name = if node.name().is_empty() {
        String::new()
    } else {
        format!(" {:?}", node.name())
    };
    let reference = node.reference.map_or_else(String::new, |reference| {
        let url = options
            .include_urls
            .then_some(node.target_url())
            .flatten()
            .map_or_else(String::new, |url| format!(", url={url}"));
        format!(" [ref=e{}{url}]", reference.ordinal())
    });
    let state = format_snapshot_state(&node.state);
    format!(
        "{}- {}{name}{reference}{state}",
        "  ".repeat(usize::try_from(node.depth).expect("snapshot depth fits usize")),
        node.role(),
    )
}

fn format_snapshot_state(state: &InteractiveElementState) -> String {
    match state {
        InteractiveElementState::Unavailable => String::new(),
        InteractiveElementState::Value(value) => format!(": {value:?}"),
        InteractiveElementState::Checked(checked) => format!(" [checked={checked}]"),
    }
}

fn reference_key(element: &InteractiveElement) -> String {
    format!("e{}", element.reference.ordinal())
}

#[cfg(test)]
mod tests {
    use super::{
        SnapshotOutputOptions, write_error_json, write_session_command_json,
        write_session_lifecycle_json, write_snapshot_json,
    };
    use crate::{
        InteractiveElement, InteractiveElementRef, InteractiveElementSourceInfo,
        InteractiveElementState, InteractiveSnapshot, SnapshotId,
    };

    #[test]
    fn snapshot_json_escapes_names_and_values() {
        let mut next_snapshot_id = 1;
        let id = SnapshotId::next(&mut next_snapshot_id);
        let mut output = Vec::new();
        let snapshot = InteractiveSnapshot {
            id,
            url: "http://localhost:3000".into(),
            elements: vec![InteractiveElement {
                reference: InteractiveElementRef::new(1, id, 1),
                source: InteractiveElementSourceInfo {
                    element: "input".into(),
                    target_url: Some("http://localhost:3000/ignored".into()),
                    depth: 0,
                },
                role: "textbox".into(),
                name: "Name \"quoted\"".into(),
                state: InteractiveElementState::Value("line one\nline two".into()),
            }],
        };

        write_snapshot_json(&mut output, &snapshot, Default::default()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(value["success"], true);
        assert_eq!(value["data"]["origin"], "http://localhost:3000");
        assert_eq!(value["data"]["refs"]["e1"]["name"], "Name \"quoted\"");
        assert_eq!(
            value["data"]["snapshot"],
            "- textbox \"Name \\\"quoted\\\"\" [ref=e1]: \"line one\\nline two\""
        );

        let mut output = Vec::new();
        write_snapshot_json(
            &mut output,
            &snapshot,
            SnapshotOutputOptions { include_urls: true },
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            value["data"]["snapshot"],
            "- textbox \"Name \\\"quoted\\\"\" [ref=e1, url=http://localhost:3000/ignored]: \"line one\\nline two\""
        );
    }

    #[test]
    fn error_json_uses_one_stable_envelope() {
        let mut output = Vec::new();

        write_error_json(&mut output, "browser.jr: bad input").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(value["success"], false);
        assert!(value["data"].is_null());
        assert_eq!(value["error"], "browser.jr: bad input");
    }

    #[test]
    fn session_json_identifies_lifecycle_and_command_lines() {
        let mut output = Vec::new();

        write_session_lifecycle_json(&mut output, "ready").unwrap();
        write_session_command_json(&mut output, 7, "title=\"Docs\"", None).unwrap();
        write_session_command_json(
            &mut output,
            8,
            "",
            Some("browser.jr: invalid session command; enter help"),
        )
        .unwrap();

        let values = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values[0]["data"]["event"], "ready");
        assert_eq!(values[1]["success"], true);
        assert_eq!(values[1]["data"]["sequence"], 7);
        assert_eq!(values[1]["data"]["output"], "title=\"Docs\"");
        assert_eq!(values[2]["success"], false);
        assert_eq!(
            values[2]["error"],
            "browser.jr: invalid session command; enter help"
        );
    }
}
