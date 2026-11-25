//! Exact Flutter 3.50 services/text_editing_delta_test.dart wire cases.
use incular_text::{
    FlutterTextDeltaError, FlutterTextEdit, FlutterTextEditingDelta, TextAffinity,
    TextEditingValue, TextRange, TextSelection,
};
use serde_json::{Value, json};

fn payload(old: &str, text: &str, span: (i32, i32), caret: i32, composing: (i32, i32)) -> Value {
    json!({
        "oldText": old,
        "deltaText": text,
        "deltaStart": span.0,
        "deltaEnd": span.1,
        "selectionBase": caret,
        "selectionExtent": caret,
        "selectionAffinity": "TextAffinity.downstream",
        "selectionIsDirectional": false,
        "composingBase": composing.0,
        "composingExtent": composing.1,
    })
}

fn decode_and_apply(
    payload: Value,
    expected: FlutterTextEdit,
    composed: Option<TextRange>,
    removed: &str,
    text: &str,
) {
    let delta = FlutterTextEditingDelta::from_json(&payload).unwrap();
    assert_eq!(delta.old_text, payload["oldText"].as_str().unwrap());
    assert_eq!(delta.edit, expected);
    assert_eq!(delta.composing, composed);
    let caret = usize::try_from(payload["selectionBase"].as_i64().unwrap()).unwrap();
    assert_eq!(delta.selection, Some(TextSelection::collapsed(caret)));
    assert_eq!(delta.selection_affinity, TextAffinity::Downstream);
    assert!(!delta.selection_is_directional);
    let removed = matches!(
        expected,
        FlutterTextEdit::Deletion { .. } | FlutterTextEdit::Replacement { .. }
    )
    .then_some(removed);
    assert_eq!(delta.removed_text().unwrap(), removed);
    // The platform delta always applies to oldText, not this possibly
    // newer local value. Offsets in these exact originals are ASCII.
    assert_eq!(
        delta
            .try_apply(&TextEditingValue::new("superseded local text"))
            .unwrap(),
        TextEditingValue::new(text)
            .with_selection(TextSelection::collapsed(caret))
            .with_composing(composed),
    );
}

#[test]
fn insert_into_a_collapsed_selection_or_extend_composing() {
    decode_and_apply(
        payload("", "let there be text", (0, 0), 17, (-1, -1)),
        FlutterTextEdit::Insertion {
            offset: 0,
            text: "let there be text".into(),
        },
        None,
        "",
        "let there be text",
    );
    decode_and_apply(
        payload("hello worl", "world", (6, 10), 11, (6, 11)),
        FlutterTextEdit::Insertion {
            offset: 10,
            text: "d".into(),
        },
        Some(TextRange::new(6, 11)),
        "",
        "hello world",
    );
}

#[test]
fn delete_from_an_explicit_range_or_the_end_of_composing() {
    decode_and_apply(
        payload("let there be text.", "", (1, 2), 1, (-1, -1)),
        FlutterTextEdit::Deletion {
            range: TextRange::new(1, 2),
        },
        None,
        "e",
        "lt there be text.",
    );
    decode_and_apply(
        payload("hello world", "worl", (6, 11), 10, (6, 10)),
        FlutterTextEdit::Deletion {
            range: TextRange::new(10, 11),
        },
        Some(TextRange::new(6, 10)),
        "d",
        "hello worl",
    );
}

#[test]
fn replace_with_longer_shorter_or_equal_utf16_lengths() {
    for (old, previous, replacement, caret, result) in [
        ("hello worfi", "worfi", "working", 13, "hello working"),
        ("hello world", "world", "h", 7, "hello h"),
        ("hello world", "world", "words", 11, "hello words"),
    ] {
        decode_and_apply(
            payload(old, replacement, (6, 11), caret, (6, caret)),
            FlutterTextEdit::Replacement {
                range: TextRange::new(6, 11),
                text: replacement.into(),
            },
            Some(TextRange::new(6, usize::try_from(caret).unwrap())),
            previous,
            result,
        );
    }
}

#[test]
fn non_text_update_sets_selection_and_composition() {
    for (text, span) in [("", (-1, -1)), ("world", (6, 11))] {
        decode_and_apply(
            payload("hello world", text, span, 10, (6, 11)),
            FlutterTextEdit::NonTextUpdate,
            Some(TextRange::new(6, 11)),
            "",
            "hello world",
        );
    }
}

#[test]
fn a_manually_constructed_invalid_delta_fails_to_apply() {
    use FlutterTextDeltaError::InvalidRange;
    let empty = TextEditingValue::empty();
    for (old, edit, caret, error) in [
        (
            "hello worl",
            FlutterTextEdit::Insertion {
                offset: 11,
                text: "d".into(),
            },
            11,
            InvalidRange("replacement"),
        ),
        (
            "hello world",
            FlutterTextEdit::Deletion {
                range: TextRange::new(5, 12),
            },
            5,
            InvalidRange("replacement"),
        ),
        (
            "hello worl",
            FlutterTextEdit::Replacement {
                range: TextRange::new(5, 11),
                text: "world".into(),
            },
            11,
            InvalidRange("replacement"),
        ),
        (
            "hello world",
            FlutterTextEdit::NonTextUpdate,
            12,
            InvalidRange("selection"),
        ),
    ] {
        let delta = FlutterTextEditingDelta {
            old_text: old.into(),
            edit,
            selection: Some(TextSelection::collapsed(caret)),
            selection_affinity: TextAffinity::Downstream,
            selection_is_directional: false,
            composing: None,
        };
        assert_eq!(delta.try_apply(&empty), Err(error));
    }
}

#[test]
fn utf16_reports_byte_offsets_only_after_application() {
    let mut raw = payload("a😀b", "é", (1, 3), 2, (1, 2));
    raw["selectionBase"] = json!(3);
    raw["selectionExtent"] = json!(1);
    raw["selectionAffinity"] = json!("TextAffinity.upstream");
    raw["selectionIsDirectional"] = json!(true);
    let delta = FlutterTextEditingDelta::from_json(&raw).unwrap();
    assert_eq!(delta.selection, Some(TextSelection::new(3, 1)));
    assert_eq!(delta.selection_affinity, TextAffinity::Upstream);
    assert!(delta.selection_is_directional);
    assert_eq!(delta.removed_text().unwrap(), Some("😀"));
    let applied = delta
        .try_apply(&TextEditingValue::new("does not rebase"))
        .unwrap();
    assert_eq!(applied.text, "aéb");
    assert_eq!(applied.selection, TextSelection::new(4, 1));
    assert_eq!(applied.composing, Some(TextRange::new(1, 3)));

    raw["deltaEnd"] = json!(2); // half of the emoji cannot be edited in Rust.
    assert_eq!(
        FlutterTextEditingDelta::from_json(&raw),
        Err(FlutterTextDeltaError::InvalidRange("replacement")),
    );
}

#[test]
fn defaults_are_unspecified_ranges_and_downstream_affinity() {
    let raw = json!({"oldText":"a😀","deltaText":"","deltaStart":-1,"deltaEnd":-1});
    let delta = FlutterTextEditingDelta::from_json(&raw).unwrap();
    assert_eq!(delta.edit, FlutterTextEdit::NonTextUpdate);
    assert_eq!(delta.selection, None);
    assert_eq!(delta.composing, None);
    assert_eq!(delta.selection_affinity, TextAffinity::Downstream);
    assert!(!delta.selection_is_directional);
    assert_eq!(
        delta.try_apply(&TextEditingValue::empty()).unwrap(),
        TextEditingValue::new("a😀")
    );
}
