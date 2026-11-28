use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextEditingValue, TextRange, TextSelection};
use incular_widgets::{
    CallbackTextInputFormatter, EditableText, TextEditingCallback, TextEditingCallbackError,
};

const MESSAGE: &str = "Test EditableText callback error";

fn mount(
    text: &TextEditingController,
    decorate: impl FnOnce(EditableText) -> EditableText,
) -> Runtime {
    let widget = decorate(
        EditableText::new(text.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .max_lines(Some(2)),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    assert!(runtime.focused_element().is_some());
    runtime
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 200.)))
        .unwrap();
}

fn send(runtime: &mut Runtime, text: &str, caret: usize, composing: Option<(usize, usize)>) {
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: text.into(),
        selection_start: caret,
        selection_end: caret,
        composing,
    }));
    frame(runtime);
}

fn observe(runtime: &mut Runtime) -> Rc<RefCell<Vec<TextEditingCallbackError>>> {
    let errors = Rc::new(RefCell::new(Vec::new()));
    runtime.observe_text_editing_error({
        let errors = errors.clone();
        move |error| errors.borrow_mut().push(error.clone())
    });
    errors
}

#[test]
fn selection_callback_panics_reach_the_editing_error_channel() {
    let text = TextEditingController::with_text("flutter is the best!");
    let mut runtime = mount(&text, |editor| {
        editor.on_selection_changed(|_, _| panic!("{MESSAGE}"))
    });
    text.set_selection(TextSelection::collapsed(6));
    frame(&mut runtime);
    let observed = observe(&mut runtime);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(4., 4.),
        });
    }
    frame(&mut runtime);
    assert!(runtime.focused_element().is_some());
    assert_eq!(observed.borrow().len(), 1);
    assert_eq!(
        observed.borrow()[0].callback,
        TextEditingCallback::SelectionChanged
    );
    assert!(observed.borrow()[0].message.contains(MESSAGE));
}

#[test]
fn changed_callback_panics_are_reported_without_losing_the_applied_text() {
    let text = TextEditingController::with_text("flutter is the best!");
    let mut runtime = mount(&text, |editor| editor.on_changed(|_| panic!("{MESSAGE}")));
    let observed = observe(&mut runtime);
    send(&mut runtime, "...", 3, None);
    assert_eq!(text.text(), "...");
    assert!(runtime.focused_element().is_some());
    assert_eq!(observed.borrow().len(), 1);
    assert_eq!(observed.borrow()[0].callback, TextEditingCallback::Changed);
    assert!(observed.borrow()[0].message.contains(MESSAGE));
}

#[test]
fn a_throwing_formatter_reports_the_error_and_keeps_the_unformatted_edit() {
    let text = TextEditingController::with_text("flutter is the best!");
    let mut runtime = mount(&text, |editor| {
        editor.input_formatter(CallbackTextInputFormatter::new(
            |_: &TextEditingValue, _: &TextEditingValue| -> TextEditingValue {
                panic!("{MESSAGE}");
            },
        ))
    });
    let observed = observe(&mut runtime);
    send(&mut runtime, "text", 4, None);
    assert_eq!(text.text(), "text");
    assert_eq!(text.selection(), TextSelection::collapsed(4));
    assert!(runtime.focused_element().is_some());
    assert_eq!(observed.borrow().len(), 1);
    assert_eq!(
        observed.borrow()[0].callback,
        TextEditingCallback::InputFormatter
    );
    assert!(observed.borrow()[0].message.contains(MESSAGE));
}

#[test]
fn on_changed_observes_only_text_changes_accepted_by_the_formatter() {
    let text = TextEditingController::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let reject = Rc::new(Cell::new(false));
    let mut runtime = mount(&text, |editor| {
        editor
            .on_changed({
                let changes = changes.clone();
                move |value| changes.borrow_mut().push(value.to_owned())
            })
            .input_formatter(CallbackTextInputFormatter::new({
                let reject = reject.clone();
                move |old: &TextEditingValue, proposed: &TextEditingValue| {
                    if reject.get() {
                        old.clone()
                    } else {
                        proposed.clone()
                    }
                }
            }))
    });
    send(&mut runtime, "a", 1, Some((0, 1)));
    assert_eq!(*changes.borrow(), ["a"]);
    send(&mut runtime, "a", 1, None);
    assert_eq!(*changes.borrow(), ["a"]);
    send(&mut runtime, "ab", 2, None);
    assert_eq!(*changes.borrow(), ["a", "ab"]);
    reject.set(true);
    send(&mut runtime, "abc", 3, None);
    assert_eq!(*changes.borrow(), ["a", "ab"]);
    assert_eq!(text.text(), "ab");
}

#[test]
fn identical_and_selection_only_packets_do_not_run_formatters() {
    let text = TextEditingController::new();
    let calls = Rc::new(Cell::new(0));
    let mut runtime = mount(&text, |editor| {
        editor.input_formatter(CallbackTextInputFormatter::new({
            let calls = calls.clone();
            move |_: &TextEditingValue, proposed: &TextEditingValue| {
                calls.set(calls.get() + 1);
                proposed.clone()
            }
        }))
    });
    send(&mut runtime, "a", 0, None);
    assert_eq!(calls.get(), 1);
    send(&mut runtime, "a", 0, None);
    assert_eq!(calls.get(), 1);
    send(&mut runtime, "a", 1, None);
    assert_eq!(calls.get(), 1);
    assert_eq!(text.selection(), TextSelection::collapsed(1));
    send(&mut runtime, "b", 1, None);
    assert_eq!(calls.get(), 2);
}

#[derive(Clone, Debug, PartialEq)]
struct FormatterCall {
    before: TextEditingValue,
    proposal: TextEditingValue,
}

fn record_formats(text: &TextEditingController) -> (Runtime, Rc<RefCell<Vec<FormatterCall>>>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let runtime = mount(text, |editor| {
        editor.input_formatter(CallbackTextInputFormatter::new({
            let calls = calls.clone();
            move |before: &TextEditingValue, proposal: &TextEditingValue| {
                calls.borrow_mut().push(FormatterCall {
                    before: before.clone(),
                    proposal: proposal.clone(),
                });
                proposal.clone()
            }
        }))
    });
    (runtime, calls)
}

#[test]
fn text_proposals_filter_identical_packets_but_not_repeated_values_after_a_change() {
    let text = TextEditingController::new();
    let (mut runtime, calls) = record_formats(&text);
    for value in [
        "",
        "a",
        "aa",
        "aaa",
        "aa",
        "aaa",
        "aaaa",
        "aa",
        "aaaaaaa",
        "aa",
        "aaaaaaaaa",
        "aaaaaaaaa",
    ] {
        send(&mut runtime, value, 0, None);
        assert_eq!(text.text(), value);
    }
    // Each distinct incoming value is compared against the immediately
    // preceding accepted value, not cached across an intervening edit.
    let calls = calls.borrow();
    let edges: Vec<_> = calls
        .iter()
        .map(|call| (call.before.text.as_str(), call.proposal.text.as_str()))
        .collect();
    assert_eq!(
        edges,
        [
            ("", "a"),
            ("a", "aa"),
            ("aa", "aaa"),
            ("aaa", "aa"),
            ("aa", "aaa"),
            ("aaa", "aaaa"),
            ("aaaa", "aa"),
            ("aa", "aaaaaaa"),
            ("aaaaaaa", "aa"),
            ("aa", "aaaaaaaaa"),
        ]
    );
}

#[test]
fn repeats_selection_and_composing_open_do_not_mask_the_following_commit() {
    let text = TextEditingController::new();
    let (mut runtime, calls) = record_formats(&text);
    for (value, expected) in [("01", 1), ("012", 2), ("0123", 3), ("0123", 3), ("0123", 3)] {
        send(&mut runtime, value, 0, None);
        assert_eq!(calls.borrow().len(), expected);
    }
    for _ in 0..3 {
        send(&mut runtime, "0123", 2, None);
        assert_eq!(calls.borrow().len(), 3);
        assert_eq!(text.selection(), TextSelection::collapsed(2));
    }
    send(&mut runtime, "0123", 2, Some((1, 2)));
    assert_eq!(calls.borrow().len(), 3);
    assert_eq!(calls.borrow()[2].before.composing, None);
    assert_eq!(calls.borrow()[2].proposal.composing, None);
    send(&mut runtime, "01234", 2, None);
    let calls = calls.borrow();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[3].before.composing, Some(TextRange::new(1, 2)));
    assert_eq!(calls[3].proposal.composing, None);
    let edges: Vec<_> = calls
        .iter()
        .map(|call| (call.before.text.as_str(), call.proposal.text.as_str()))
        .collect();
    assert_eq!(
        edges,
        [
            ("", "01"),
            ("01", "012"),
            ("012", "0123"),
            ("0123", "01234")
        ]
    );
}

#[test]
fn first_matching_packet_keeps_the_later_composition_commit_observable() {
    let text = TextEditingController::with_text("test");
    let (mut runtime, calls) = record_formats(&text);
    text.set_selection(TextSelection::collapsed(4));
    frame(&mut runtime);
    assert!(calls.borrow().is_empty());
    send(&mut runtime, "test", 4, None);
    assert!(calls.borrow().is_empty());
    send(&mut runtime, "test", 4, Some((1, 2)));
    assert!(calls.borrow().is_empty());
    send(&mut runtime, "test", 4, None);
    let calls = calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].before.text, "test");
    assert_eq!(calls[0].before.composing, Some(TextRange::new(1, 2)));
    assert_eq!(calls[0].proposal.composing, None);
    assert_eq!(text.text(), "test");
}
