//! A refused focus request must not displace the currently composing editor.
use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, TextInputCommand, TextInputState};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{Column, EditableText, ExcludeFocus, TextDirection, Widget};

fn field(controller: &TextEditingController, enabled: bool) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .enabled(enabled)
        .size(Size::new(180., 40.))
}

fn pointer(runtime: &mut Runtime, y: f32, phase: PointerPhase) {
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase,
        position: Offset::new(100., y),
    });
}

fn native_edit(runtime: &mut Runtime, text: &str, composing: Option<(usize, usize)>) {
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: text.into(),
        selection_start: text.len(),
        selection_end: text.len(),
        composing,
    }));
}

#[test]
fn disabled_or_focus_excluded_editor_does_not_interrupt_active_ime() {
    for excluded in [false, true] {
        let a = TextEditingController::with_text("a");
        let b = TextEditingController::with_text("b");
        let second: Widget = if excluded {
            ExcludeFocus::new(field(&b, true)).into()
        } else {
            field(&b, false).into()
        };
        let mut runtime =
            Runtime::new(Column::new([field(&a, true).into(), second]).into()).unwrap();
        let constraints = Constraints::tight(Size::new(180., 80.));
        runtime.run_frame(constraints).unwrap();
        pointer(&mut runtime, 20., PointerPhase::Down);
        pointer(&mut runtime, 20., PointerPhase::Up);
        let incumbent = runtime.focused_element().expect("first editor has focus");
        native_edit(&mut runtime, "aあ", Some((1, 4)));
        runtime.run_frame(constraints).unwrap();
        let composing = a.composing().expect("active first-editor composition");
        let _ = runtime.take_text_input_commands();

        pointer(&mut runtime, 60., PointerPhase::Down);
        assert_eq!(
            runtime.focused_element(),
            Some(incumbent),
            "excluded={excluded}"
        );
        assert_eq!(a.composing(), Some(composing));
        pointer(&mut runtime, 60., PointerPhase::Up);
        runtime.run_frame(constraints).unwrap();
        assert_eq!(runtime.focused_element(), Some(incumbent));
        assert_eq!(a.composing(), Some(composing));
        assert!(
            runtime
                .take_text_input_commands()
                .iter()
                .all(|command| !matches!(
                    command,
                    TextInputCommand::SetClient { .. }
                        | TextInputCommand::Clear { .. }
                        | TextInputCommand::Hide { .. }
                ))
        );

        native_edit(&mut runtime, "a界", None);
        assert_eq!((a.text(), b.text()), ("a界".into(), "b".into()));
        assert_eq!(a.composing(), None);
    }
}
