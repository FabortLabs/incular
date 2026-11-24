//! The mobile text input contract: the input method's editing state, the
//! keyboard requests a tap sends, and the system back button.

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyState, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, TextInputCommand, TextInputState};
use incular_runtime::Runtime;
use incular_text::{TextEditingValue, TextSelection};
use incular_widgets::internal::{TextEditingController, TextRange};
use incular_widgets::{EditableText, KeyboardListener, TextDirection, TextInputFormatter, Widget};
use std::{cell::Cell, rc::Rc};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(220., 50.))
}

fn editor(controller: &TextEditingController) -> Runtime {
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .unwrap();
    runtime.run_frame(constraints()).unwrap();
    runtime
}

fn tap(runtime: &mut Runtime) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(5., 5.),
        });
    }
}

#[test]
fn the_input_methods_state_is_a_user_edit_the_platform_already_knows() {
    let controller = TextEditingController::new();
    let edits = Rc::new(Cell::new(0));
    let observed = edits.clone();
    let _subscription = controller.observe_user_edits(move |_| observed.set(observed.get() + 1));
    let mut runtime = editor(&controller);
    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    assert!(matches!(
        runtime.take_text_input_commands().first(),
        Some(TextInputCommand::SetClient { .. })
    ));

    // "hé" committed and "🙂" still composing, in UTF-8 offsets.
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "hé🙂".into(),
        selection_start: 7,
        selection_end: 7,
        composing: Some((3, 7)),
    }));
    assert_eq!(controller.text(), "hé🙂");
    assert_eq!(controller.composing(), Some(TextRange::new(3, 7)));
    assert_eq!(edits.get(), 1);

    // The platform sent this state, so nothing is echoed back to it.
    runtime.run_frame(constraints()).unwrap();
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(command, TextInputCommand::Update { .. })),
    );
}

#[test]
fn tapping_the_focused_field_asks_for_the_keyboard_again() {
    let controller = TextEditingController::new();
    let mut runtime = editor(&controller);
    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    let first = runtime.take_text_input_commands();
    assert!(
        matches!(
            first.as_slice(),
            [
                TextInputCommand::SetClient { .. },
                TextInputCommand::Show { .. },
            ]
        ),
        "focusing a field attaches it and then shows the keyboard: {first:?}"
    );

    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    assert!(
        runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(command, TextInputCommand::Show { .. }))
    );
}

#[test]
fn a_configuration_change_on_the_focused_field_does_not_show_the_keyboard() {
    let controller = TextEditingController::new();
    let mut runtime = editor(&controller);
    tap(&mut runtime);
    runtime.run_frame(constraints()).unwrap();
    let _ = runtime.take_text_input_commands();

    // A password field's "show password" toggle changes only `obscure_text`.
    let root = runtime.tree().root().expect("the editor is the root");
    runtime
        .tree_mut()
        .update(
            root,
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .obscure_text(true)
                .into(),
        )
        .unwrap();
    runtime.run_frame(constraints()).unwrap();
    let commands = runtime.take_text_input_commands();
    assert!(
        matches!(
            commands.as_slice(),
            [TextInputCommand::SetClient { configuration, .. }] if configuration.obscure_text
        ),
        "reconfiguring keeps the keyboard as the user left it: {commands:?}"
    );
}

#[test]
fn the_back_button_reaches_key_handlers_before_leaving_the_app() {
    let backs = Rc::new(Cell::new(0));
    let observed = backs.clone();
    let handles = Rc::new(Cell::new(true));
    let handled = handles.clone();
    let root: Widget = KeyboardListener::new(incular_widgets::SizedBox::shrink())
        .focus_node(incular_widgets::FocusNode::new())
        .autofocus(true)
        .on_key(move |event| {
            if event.code == Code::BrowserBack && event.state == KeyState::Down {
                observed.set(observed.get() + 1);
                return handled.get();
            }
            false
        })
        .into();
    let mut runtime = Runtime::new(root).unwrap();
    runtime.run_frame(constraints()).unwrap();

    assert!(runtime.handle_back_button());
    assert_eq!(backs.get(), 1);

    // Unhandled, with no popup or navigator to pop: the host leaves the app.
    handles.set(false);
    assert!(!runtime.handle_back_button());
    assert_eq!(backs.get(), 2);
}

#[test]
fn an_unhandled_back_asks_the_pop_route_observers() {
    struct Popper(Rc<Cell<u32>>);
    impl incular_widgets::BackGestureObserver for Popper {
        fn handle_pop_route(&self) -> bool {
            self.0.set(self.0.get() + 1);
            true
        }
    }
    let pops = Rc::new(Cell::new(0));
    let registration = incular_widgets::add_back_gesture_observer(Popper(pops.clone()));
    let mut runtime = Runtime::new(incular_widgets::SizedBox::shrink().into()).unwrap();
    runtime.run_frame(constraints()).unwrap();
    assert!(runtime.handle_back_button());
    assert_eq!(pops.get(), 1);
    drop(registration);
    assert!(!runtime.handle_back_button());
}

#[test]
fn single_line_native_edit_removes_newlines_and_adjusts_selection() {
    let controller = TextEditingController::new();
    let mut runtime = editor(&controller);
    tap(&mut runtime);
    runtime.run_frame(constraints()).expect("focus text field");
    assert!(runtime.focused_element().is_some());

    // Flutter prepends FilteringTextInputFormatter.singleLineFormatter.
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "a\nb".into(),
        selection_start: 3,
        selection_end: 3,
        composing: None,
    }));
    assert_eq!(
        (controller.text().as_str(), controller.selection()),
        ("ab", TextSelection::collapsed(2))
    );
}

#[test]
fn native_selection_only_update_does_not_run_text_formatters() {
    struct CountFormatter(Rc<Cell<u32>>);

    impl TextInputFormatter for CountFormatter {
        fn format_edit_update(
            &self,
            _old: &TextEditingValue,
            new: &TextEditingValue,
        ) -> TextEditingValue {
            self.0.set(self.0.get() + 1);
            new.clone()
        }
    }

    let controller = TextEditingController::with_text("abc");
    let calls = Rc::new(Cell::new(0));
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .input_formatter(CountFormatter(calls.clone()))
            .into(),
    )
    .expect("mount formatted editable text");
    runtime.run_frame(constraints()).expect("initial layout");
    tap(&mut runtime);
    controller.set_selection(TextSelection::collapsed(0));
    runtime.run_frame(constraints()).expect("focus text field");
    calls.set(0);

    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "abc".into(),
        selection_start: 1,
        selection_end: 1,
        composing: None,
    }));
    assert_eq!(controller.selection(), TextSelection::collapsed(1));
    let selection_only_calls = calls.get();

    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "axbc".into(),
        selection_start: 2,
        selection_end: 2,
        composing: None,
    }));
    assert_eq!(controller.text(), "axbc");
    assert_eq!(
        calls.get() - selection_only_calls,
        1,
        "control: text changed"
    );
    assert_eq!(
        selection_only_calls, 0,
        "changing only selection must not invoke text formatters"
    );
}
