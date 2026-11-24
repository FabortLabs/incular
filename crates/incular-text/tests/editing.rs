use incular_text::{
    TextEditingController, TextEditingDelta, TextEditingValue, TextRange, TextSelection,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

#[test]
fn selection_and_replacement_preserve_utf8_boundaries() {
    let value = TextEditingValue::new("a🙂b").with_selection(TextSelection::new(1, 5));
    assert_eq!(value.selection, TextSelection::new(1, 5));
    let range = value.selection.range();
    let replaced = value.replace(range, "x");
    assert_eq!(replaced.text, "axb");
    assert_eq!(replaced.selection, TextSelection::new(1, 2));
    let controller = TextEditingController::with_text("a🙂b");
    controller.set_selection(TextSelection::new(1, 5));
    controller.replace_selection("x");
    assert_eq!(controller.text(), "axb");
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
}

#[test]
fn controller_notifies_listeners_and_applies_deltas() {
    let controller = TextEditingController::with_text("hello");
    let calls = Rc::new(Cell::new(0));
    let calls_for_listener = calls.clone();
    let token = controller.add_listener(move |_| {
        calls_for_listener.set(calls_for_listener.get() + 1);
    });
    controller.set_selection(TextSelection::new(0, 5));
    controller.apply_delta(&TextEditingDelta::replace(TextRange::new(0, 5), "world"));
    assert_eq!(controller.text(), "world");
    assert_eq!(calls.get(), 2);
    assert!(controller.remove_listener(token));
}

#[test]
fn reentrant_mutation_during_notify_terminates_without_panic() {
    // Listeners run after the controller releases its state borrow
    // (cloned out beforehand), so a listener that edits terminates
    // instead of tripping the borrow guard.
    let controller = TextEditingController::with_text("hi");
    let fired = Rc::new(Cell::new(0));
    let observed = fired.clone();
    let inner = controller.clone();
    let _token = controller.add_listener(move |value| {
        observed.set(observed.get() + 1);
        if !value.text.ends_with('!') {
            inner.set_text(format!("{}!", value.text));
        }
    });
    controller.set_text("go");
    assert_eq!(controller.text(), "go!");
    assert_eq!(fired.get(), 2);
}

#[test]
fn committed_preedit_preserves_composition_started_by_listener() {
    for commit in ["", "x"] {
        let controller = TextEditingController::new();
        controller.set_preedit("old", None);
        let editor = controller.clone();
        let fired = Rc::new(Cell::new(false));
        let observed = fired.clone();
        let token = controller.add_listener(move |_| {
            if !observed.replace(true) {
                assert_eq!(editor.preedit(), None);
                editor.set_preedit("new", None);
            }
        });
        controller.commit_preedit(commit);
        assert!(fired.get());
        assert_eq!(controller.text(), commit);
        assert_eq!(controller.preedit().as_deref(), Some("new"));
        assert!(controller.remove_listener(token));
    }
}

#[test]
fn restoration_during_open_preedit_clears_overlay_and_keeps_editing() {
    use incular_core::{RestorationKey, RestorationScope};
    use serde_json::json;

    #[derive(Default)]
    struct MemoryBackend(RefCell<BTreeMap<Vec<RestorationKey>, serde_json::Value>>);
    impl incular_core::RestorationBackend for MemoryBackend {
        fn read_value(&self, path: &[RestorationKey]) -> Option<serde_json::Value> {
            self.0.borrow().get(path).cloned()
        }
        fn write_value(&self, path: &[RestorationKey], value: serde_json::Value) {
            self.0.borrow_mut().insert(path.to_vec(), value);
        }
        fn remove_value(&self, path: &[RestorationKey]) {
            self.0.borrow_mut().remove(path);
        }
    }

    let scope = RestorationScope::root(Rc::new(MemoryBackend::default()));
    let key = RestorationKey::new("document").unwrap();
    let controller = TextEditingController::with_text("ab");
    controller.set_preedit("xy", None);
    assert_eq!(controller.preedit().as_deref(), Some("xy"));
    scope.set_json(
        &key,
        json!({"text": "restored", "selection": {"base": 8, "extent": 8}}),
    );
    controller.bind_restoration(scope, key);
    assert_eq!(controller.preedit(), None);
    assert_eq!(controller.text(), "restored");
    controller.insert("!");
    assert_eq!(controller.text(), "restored!");
}

#[test]
fn deletion_moves_by_codepoint_not_by_byte() {
    let controller = TextEditingController::with_text("a🙂");
    controller.delete_backward();
    assert_eq!(controller.text(), "a");
    controller.delete_backward();
    assert_eq!(controller.text(), "");
}

/// Flutter `_formatAndSetValue` runs formatters only for user edits;
/// `controller.text = ...` stores the text as given and `onChanged` stays
/// silent. Ported from the text__editable_programmatic_controller_update twin.
#[test]
fn programmatic_updates_bypass_transforms_and_user_observers() {
    let controller = TextEditingController::new();
    let transform_calls = Rc::new(Cell::new(0));
    let calls = transform_calls.clone();
    let _transform = controller.register_edit_transform(move |_old, next| {
        calls.set(calls.get() + 1);
        let digits: String = next.text.chars().filter(char::is_ascii_digit).collect();
        TextEditingValue::new(digits)
    });
    let user_changes = Rc::new(RefCell::new(Vec::new()));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let user_sink = user_changes.clone();
    let _user = controller.observe_user_edits(move |value| {
        user_sink.borrow_mut().push(value.text.clone());
    });
    let all_sink = changes.clone();
    let _all = controller.observe(move |value| all_sink.borrow_mut().push(value.text.clone()));

    controller.set_text("a1b2");
    assert_eq!(controller.text(), "a1b2");
    assert_eq!(transform_calls.get(), 0);
    assert!(user_changes.borrow().is_empty());
    assert_eq!(*changes.borrow(), ["a1b2"]);

    controller.set_selection(TextSelection::collapsed(1));
    assert_eq!(transform_calls.get(), 0);

    // The caret is at byte 1 ("a|1b2"), so the edit gives "ac31b2" and the
    // formatter keeps its digits.
    controller.replace_selection("c3");
    assert_eq!(controller.text(), "312");
    assert_eq!(transform_calls.get(), 1);
    assert_eq!(*user_changes.borrow(), ["312"]);
}

/// A user edit that the formatters leave unchanged is not a text change, so
/// user-edit observers stay silent (Flutter compares `oldValue.text`).
#[test]
fn user_edit_that_formatting_leaves_unchanged_does_not_notify() {
    let controller = TextEditingController::with_text("ab");
    let _transform = controller
        .register_edit_transform(|_old, next| TextEditingValue::new(next.text.replace('x', "")));
    let notified = Rc::new(Cell::new(0));
    let count = notified.clone();
    let _user = controller.observe_user_edits(move |_| count.set(count.get() + 1));
    controller.set_selection(TextSelection::collapsed(2));
    controller.insert("x");
    assert_eq!(controller.text(), "ab");
    assert_eq!(notified.get(), 0);
}

/// text-005: Flutter's `TextEditingController(text:)` starts with the invalid
/// selection `TextSelection.collapsed(offset: -1)` (editable_text.dart
/// `TextEditingController`, services/text_input.dart `TextEditingValue`), and
/// the `text` setter resets it. EditableText collapses it at the end on focus.
#[test]
fn initial_text_leaves_the_selection_unset_until_one_is_applied() {
    let controller = TextEditingController::with_text("abc");
    assert_eq!(controller.selection_if_set(), None);
    // The compatible accessor keeps reporting the collapsed end.
    assert_eq!(controller.selection(), TextSelection::collapsed(3));
    let notified = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = notified.clone();
    let _subscription = controller.observe(move |_| count.set(count.get() + 1));
    // Focus places the caret at the end; the change from unset notifies even
    // though the stored offsets are the same.
    controller.collapse_unset_selection();
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::collapsed(3))
    );
    assert_eq!(notified.get(), 1);
    // A second focus leaves a set selection alone.
    controller.set_selection(TextSelection::new(0, 1));
    controller.collapse_unset_selection();
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::new(0, 1))
    );

    controller.set_text("xy");
    assert_eq!(controller.selection_if_set(), None);
    controller.insert_text("z");
    assert!(controller.selection_if_set().is_some());

    assert_eq!(TextEditingController::new().selection_if_set(), None);
    let explicit = TextEditingController::from_value(
        TextEditingValue::new("ab").with_selection(TextSelection::collapsed(1)),
    );
    assert_eq!(
        explicit.selection_if_set(),
        Some(TextSelection::collapsed(1))
    );
}

#[test]
fn a_delta_between_two_values_replaces_only_the_changed_middle() {
    let new =
        TextEditingValue::new("hello brave world").with_selection(TextSelection::collapsed(11));
    let delta = TextEditingDelta::between("hello world", &new);
    assert_eq!(delta.range, TextRange::new(6, 6));
    assert_eq!(delta.replacement, "brave ");
    assert_eq!(delta.selection, Some(TextSelection::collapsed(11)));
    assert_eq!(
        delta.apply(&TextEditingValue::new("hello world")).text,
        new.text
    );

    // Deleting one of two equal characters still yields a valid range.
    let delta = TextEditingDelta::between("aa", &TextEditingValue::new("a"));
    assert_eq!(
        (delta.range, delta.replacement.as_str()),
        (TextRange::new(1, 2), "")
    );

    // Multi-byte characters that share a leading byte split on boundaries.
    let delta = TextEditingDelta::between("é", &TextEditingValue::new("è"));
    assert_eq!(
        (delta.range, delta.replacement.as_str()),
        (TextRange::new(0, 2), "è")
    );
    let delta = TextEditingDelta::between("a🙂", &TextEditingValue::new("a🙃"));
    assert_eq!(
        (delta.range, delta.replacement.as_str()),
        (TextRange::new(1, 5), "🙃")
    );
}
