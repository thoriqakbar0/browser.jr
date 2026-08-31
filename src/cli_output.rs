use std::io::{self, Write};

use serde_json::{Map, Value, json};

use crate::{InteractiveElement, InteractiveElementState, InteractiveSnapshot};

pub(crate) fn write_snapshot_json(
    output: &mut impl Write,
    snapshot: &InteractiveSnapshot,
) -> io::Result<()> {
    let mut refs = Map::new();
    let snapshot_text = snapshot
        .elements
        .iter()
        .map(format_snapshot_element)
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

fn write_json_line(output: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value).map_err(io::Error::other)?;
    output.write_all(b"\n")
}

fn format_snapshot_element(element: &InteractiveElement) -> String {
    let state = match &element.state {
        InteractiveElementState::Unavailable => String::new(),
        InteractiveElementState::Value(value) => format!(": {value:?}"),
        InteractiveElementState::Checked(checked) => format!(" [checked={checked}]"),
    };
    format!(
        "- {} {:?} [ref={}]{}",
        element.role,
        element.name,
        reference_key(element),
        state
    )
}

fn reference_key(element: &InteractiveElement) -> String {
    format!("e{}", element.reference.ordinal())
}

#[cfg(test)]
mod tests {
    use super::{write_error_json, write_snapshot_json};
    use crate::{
        InteractiveElement, InteractiveElementRef, InteractiveElementState, InteractiveSnapshot,
        SnapshotId,
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
                element: "input".into(),
                role: "textbox".into(),
                name: "Name \"quoted\"".into(),
                state: InteractiveElementState::Value("line one\nline two".into()),
            }],
        };

        write_snapshot_json(&mut output, &snapshot).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(value["success"], true);
        assert_eq!(value["data"]["origin"], "http://localhost:3000");
        assert_eq!(value["data"]["refs"]["e1"]["name"], "Name \"quoted\"");
        assert_eq!(
            value["data"]["snapshot"],
            "- textbox \"Name \\\"quoted\\\"\" [ref=e1]: \"line one\\nline two\""
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
}
