//! Material text field max-length and formatter behavior. Input formatters
//! run on user edits only (Flutter: `_formatAndSetValue` from
//! `userUpdateTextEditingValue`); `controller.text = ...` is stored as given.
//! Each test therefore drives typing through `replace_selection` or
//! `apply_delta`, and asserts that programmatic updates are not formatted.

use incular_config::Constraints;
use incular_core::Size;
use incular_material::TextField;
use incular_runtime::{Runtime, UndoHistoryController};
use incular_text::{TextEditingController, TextEditingDelta, TextEditingValue, TextRange};
use incular_widgets::{
    Directionality, LengthLimitingTextInputFormatter, MaxLengthEnforcement, Stack, Text,
    TextDirection, TextInputFormatter, Widget,
};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn restored_over_long_value_is_not_reformatted_on_mount() {
    let controller = TextEditingController::with_text("abcd");
    let widget: Widget = TextField::new(controller.clone())
        .max_length(Some(2))
        .into();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 80.)))
        .unwrap();
    // Flutter applies formatters only to user edits, so mounting keeps the value.
    assert_eq!(controller.text(), "abcd");
    // A user edit is formatted: the new text is truncated to two characters.
    controller.set_text("");
    controller.replace_selection("abcd");
    assert_eq!(controller.text(), "ab");
}

#[test]
fn rebuild_replaces_callbacks_and_unmount_releases_formatter() {
    let controller = TextEditingController::new();
    let calls = Rc::new(Cell::new(0));
    let build = |limit, calls: Rc<Cell<usize>>| -> Widget {
        TextField::new(controller.clone())
            .max_length(Some(limit))
            .on_changed(move |_| calls.set(calls.get() + 1))
            .into()
    };
    let retained_descriptor = build(1, calls.clone());
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            Stack::new([retained_descriptor.clone()]),
        )
        .into(),
    )
    .unwrap();
    let constraints = Constraints::tight(Size::new(240., 80.));
    runtime.run_frame(constraints).unwrap();
    let scope = runtime.tree().root().unwrap();
    let root = runtime
        .tree()
        .children(scope)
        .and_then(|children| children.first().copied())
        .unwrap();
    for _ in 0..4 {
        runtime
            .tree_mut()
            .update(root, Stack::new([build(1, calls.clone())]).into())
            .unwrap();
        runtime.run_frame(constraints).unwrap();
    }
    // Programmatic: stored as given, no formatter, no onChanged.
    controller.set_text("abc");
    assert_eq!(controller.text(), "abc");
    assert_eq!(calls.get(), 0);
    // User edit: formatted to one character, and onChanged fires once.
    controller.set_text("");
    controller.replace_selection("abc");
    assert_eq!(controller.text(), "a");
    assert_eq!(calls.get(), 1);

    let replacement_calls = Rc::new(Cell::new(0));
    runtime
        .tree_mut()
        .update(
            root,
            Stack::new([build(2, replacement_calls.clone())]).into(),
        )
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    controller.set_text("xyz");
    assert_eq!(controller.text(), "xyz");
    assert_eq!(calls.get(), 1);
    assert_eq!(replacement_calls.get(), 0);
    // The replacement callbacks and limit take effect for user edits.
    controller.set_text("");
    controller.replace_selection("xyz");
    assert_eq!(controller.text(), "xy");
    assert_eq!(calls.get(), 1);
    assert_eq!(replacement_calls.get(), 1);

    runtime
        .tree_mut()
        .update(
            root,
            Stack::new([Widget::from(
                Text::new("gone").text_direction(incular_widgets::TextDirection::Ltr),
            )])
            .into(),
        )
        .unwrap();
    controller.set_text("unmounted");
    assert_eq!(controller.text(), "unmounted");
    assert_eq!(replacement_calls.get(), 1);
    drop(retained_descriptor);
}

struct RecordingFormatter {
    old: Rc<RefCell<Vec<String>>>,
    panic_once: Rc<Cell<bool>>,
}

impl TextInputFormatter for RecordingFormatter {
    fn format_edit_update(
        &self,
        old: &TextEditingValue,
        next: &TextEditingValue,
    ) -> TextEditingValue {
        assert!(!self.panic_once.replace(false), "formatter panic");
        self.old.borrow_mut().push(old.text.clone());
        LengthLimitingTextInputFormatter::new(2).format_edit_update(old, next)
    }
}

#[test]
fn formatter_history_uses_accepted_values_and_recovers_from_panic() {
    let controller = TextEditingController::new();
    let history = UndoHistoryController::new(controller.clone());
    let old = Rc::new(RefCell::new(Vec::new()));
    let panic_once = Rc::new(Cell::new(false));
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            TextField::new(controller.clone()).input_formatter(RecordingFormatter {
                old: old.clone(),
                panic_once: panic_once.clone(),
            }),
        )
        .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 80.)))
        .unwrap();
    // A user edit whose formatter panics leaves the value unchanged.
    panic_once.set(true);
    assert!(catch_unwind(AssertUnwindSafe(|| controller.replace_selection("bad"))).is_err());
    assert_eq!(controller.text(), "");
    controller.replace_selection("abcd");
    assert_eq!(controller.text(), "ab");
    // Programmatic updates bypass the formatter, so it records nothing for them.
    controller.set_text("xyz");
    assert_eq!(&*old.borrow(), &[String::new()]);
    assert_eq!(controller.text(), "xyz");
    assert!(history.undo());
    assert_eq!(controller.text(), "ab");
    assert!(history.undo());
    assert_eq!(controller.text(), "");
    assert!(!history.undo());
    runtime.shutdown();
    controller.set_text("disposed");
    assert_eq!(controller.text(), "disposed");
}

#[test]
fn changed_callback_nested_edit_is_formatted_once_per_effective_value() {
    let controller = TextEditingController::new();
    let nested = controller.clone();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            TextField::new(controller.clone())
                .max_length(Some(2))
                .on_changed(move |value| {
                    observed.borrow_mut().push(value.to_owned());
                    if value == "ab" {
                        // A nested user edit replaces the whole text.
                        nested.apply_delta(&TextEditingDelta {
                            range: TextRange::new(0, 2),
                            replacement: "xyz".into(),
                            selection: None,
                            composing: None,
                        });
                    }
                }),
        )
        .into(),
    )
    .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 80.)))
        .unwrap();
    // Programmatic: no formatter and no onChanged.
    controller.set_text("abcd");
    assert_eq!(controller.text(), "abcd");
    assert!(calls.borrow().is_empty());
    controller.set_text("");
    controller.replace_selection("abcd");
    assert_eq!(controller.text(), "xy");
    assert_eq!(&*calls.borrow(), &["ab", "xy"]);
    drop(runtime);
    controller.replace_selection("!");
    assert_eq!(&*calls.borrow(), &["ab", "xy"]);
}

#[test]
fn material_max_length_uses_grapheme_safe_formatter() {
    let controller = TextEditingController::new();
    let widget: Widget = TextField::new(controller.clone())
        .max_length(Some(1))
        .into();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 80.)))
        .unwrap();
    let family = "👩‍👩‍👧‍👦";
    controller.set_text(format!("{family}x"));
    assert_eq!(controller.text(), format!("{family}x"));
    controller.set_text("");
    controller.replace_selection(&format!("{family}x"));
    assert_eq!(controller.text(), family);
    assert_eq!(controller.value().selection.extent, controller.text().len());
}

#[test]
fn material_formatter_defers_until_controller_composition_commits() {
    let controller = TextEditingController::new();
    let widget: Widget = TextField::new(controller.clone())
        .input_formatter(LengthLimitingTextInputFormatter::with_enforcement(
            1,
            MaxLengthEnforcement::TruncateAfterCompositionEnds,
        ))
        .into();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 80.)))
        .unwrap();
    // Programmatic composing values are stored as given, never formatted.
    controller.set_value(TextEditingValue::new("éx").with_composing(Some(TextRange::new(0, 3))));
    assert_eq!(controller.text(), "éx");
    controller.set_value(controller.value().with_composing(None));
    assert_eq!(controller.text(), "éx");

    // User typing into an active composition defers truncation...
    controller.set_text("");
    controller.apply_delta(&TextEditingDelta {
        range: TextRange::new(0, 0),
        replacement: "éx".into(),
        selection: Some(incular_text::TextSelection::collapsed(3)),
        composing: Some(TextRange::new(0, 3)),
    });
    assert_eq!(controller.text(), "éx");
    // ...and the user's commit (composing cleared) truncates it.
    controller.apply_delta(&TextEditingDelta {
        range: TextRange::new(0, 0),
        replacement: String::new(),
        selection: Some(incular_text::TextSelection::collapsed(3)),
        composing: None,
    });
    assert_eq!(controller.text(), "é");
    assert!(controller.value().composing.is_none());
    assert_eq!(controller.value().selection.extent, 2);
}
