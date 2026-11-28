use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextEditingValue, TextSelection};
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
