//! Editing-delta comparison with pinned Flutter 3.50.0
//! `services/text_editing_delta_test.dart`. Rust derives the delta from whole
//! editing values, so these are payload/application checks, not wire ports.

use std::{cell::Cell, rc::Rc};

use incular_text::{
    InvalidTextDeltaRange, TextEditingController, TextEditingDelta, TextEditingValue, TextRange,
    TextSelection,
};

// Upstream :138, deleting at the end of the composing region.
#[test]
fn deletion_delta_preserves_the_remaining_composing_range() {
    let old = TextEditingValue::new("hello world");
    let caret = TextSelection::collapsed(10);
    let composing = TextRange::new(6, 10);
    let new = TextEditingValue::new("hello worl").with_composing(Some(composing));
    let delta = TextEditingDelta::between(&old.text, &new);

    assert_eq!(delta.range, TextRange::new(10, 11));
    assert_eq!(&old.text[delta.range.as_range()], "d");
    assert!(delta.replacement.is_empty());
    assert_eq!(delta.selection, Some(caret));
    assert_eq!(delta.composing, Some(composing));
    assert_eq!(delta.apply(&old), new);
}

// Upstream :235, shortening the composing region with a replacement.
#[test]
fn shorter_replacement_delta_keeps_the_selection_and_composition_at_new_end() {
    let old = TextEditingValue::new("hello world");
    let caret = TextSelection::collapsed(7);
    let composing = TextRange::new(6, 7);
    let new = TextEditingValue::new("hello h").with_composing(Some(composing));
    let delta = TextEditingDelta::between(&old.text, &new);

    assert_eq!(delta.range, TextRange::new(6, 11));
    assert_eq!(&old.text[delta.range.as_range()], "world");
    assert_eq!(delta.replacement, "h");
    assert_eq!(delta.selection, Some(caret));
    assert_eq!(delta.composing, Some(composing));
    assert_eq!(delta.apply(&old), new);
}

// Upstream :336. A collapsed range with empty replacement is Incular's
// no-text-change representation, not Flutter's negative-offset sentinel.
#[test]
fn nontext_delta_updates_selection_and_composing_without_touching_text() {
    let old = TextEditingValue::new("hello world");
    let caret = TextSelection::collapsed(10);
    let composing = TextRange::new(6, 11);
    let new = old
        .clone()
        .with_selection(caret)
        .with_composing(Some(composing));
    let delta = TextEditingDelta::between(&old.text, &new);

    assert!(delta.range.is_empty());
    assert!(delta.replacement.is_empty());
    assert_eq!(delta.selection, Some(caret));
    assert_eq!(delta.composing, Some(composing));
    assert_eq!(delta.apply(&old), new);
}

// Upstream :68 rejects this malformed insertion with a debug assertion.
// Rust cannot supply a separate oldText field; compare how public apply
// handles the same out-of-range insertion against the empty value.
#[test]
fn applying_an_out_of_range_insertion_does_not_perform_a_different_edit() {
    let delta = TextEditingDelta {
        range: TextRange::collapsed(11),
        replacement: "d".into(),
        selection: Some(TextSelection::collapsed(11)),
        composing: None,
    };
    let old = TextEditingValue::empty();
    assert_eq!(
        delta.try_apply(&old),
        Err(InvalidTextDeltaRange {
            range: TextRange::collapsed(11),
            text_byte_len: 0,
        })
    );
    assert_eq!(
        delta.apply(&old),
        old,
        "invalid insertion must not move to byte zero"
    );
}

#[test]
fn malformed_delta_does_not_dispatch_to_controller_formatters() {
    let controller = TextEditingController::with_text("é🙂");
    let original = controller.value();
    let calls = Rc::new(Cell::new(0));
    let _formatter = controller.register_edit_transform({
        let calls = calls.clone();
        move |_, new| {
            calls.set(calls.get() + 1);
            new.clone()
        }
    });
    for range in [
        TextRange::collapsed(1),
        TextRange::new(6, 2),
        TextRange::new(0, 7),
    ] {
        let delta = TextEditingDelta::replace(range, "x");
        assert_eq!(
            controller.try_apply_delta(&delta),
            Err(InvalidTextDeltaRange {
                range,
                text_byte_len: 6,
            })
        );
        controller.apply_delta(&delta);
    }
    assert_eq!(controller.value(), original);
    assert_eq!(calls.get(), 0);
    controller
        .try_apply_delta(&TextEditingDelta::replace(TextRange::new(0, 2), "x"))
        .expect("valid edit");
    assert_eq!(controller.value().text, "x🙂");
    assert_eq!(calls.get(), 1);
}
