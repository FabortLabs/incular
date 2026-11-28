use incular_config::{Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_platform::{MemoryClipboard, TransferReadRequest};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{SelectionChangedCause, TextEditingController, TextSelection};
use incular_widgets::EditableText;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn press(runtime: &mut Runtime, code: Code, key: NamedKey, modifiers: Modifiers) {
    for mut event in [
        KeyboardEvent::key_down(KeyboardKey::Named(key), code),
        KeyboardEvent::key_up(KeyboardKey::Named(key), code),
    ] {
        event.modifiers = modifiers;
        let _ = runtime.handle_input(InputEvent::Key(event));
    }
    frame(runtime);
}

#[test]
fn keyboard_and_toolbar_paste_collapse_forward_and_reversed_selections_at_inserted_end() {
    for toolbar in [false, true] {
        let controller = TextEditingController::new();
        let mut runtime = Runtime::new(
            EditableText::new(controller.clone())
                .autofocus(true)
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
        runtime.set_clipboard(Box::<MemoryClipboard>::default());
        frame(&mut runtime);
        let editor = runtime.focused_element().unwrap();
        assert_eq!(controller.text(), "");
        let _ = runtime.handle_input(InputEvent::Text("12345".into()));
        frame(&mut runtime);
        let state = |text: &str, range: TextSelection| {
            assert_eq!(controller.text(), text, "toolbar {toolbar}");
            assert_eq!(controller.selection(), range, "toolbar {toolbar}");
        };
        state("12345", TextSelection::collapsed(5));
        for _ in 0..5 {
            press(
                &mut runtime,
                Code::ArrowLeft,
                NamedKey::ArrowLeft,
                Modifiers::SHIFT,
            );
        }
        state("12345", TextSelection::new(5, 0));
        press(
            &mut runtime,
            Code::KeyC,
            NamedKey::Unidentified,
            Modifiers::CONTROL,
        );
        state("12345", TextSelection::new(5, 0));
        assert_eq!(
            runtime
                .read_clipboard(TransferReadRequest::all())
                .unwrap()
                .plain_text_value()
                .unwrap()
                .unwrap()
                .as_ref(),
            "12345"
        );

        let paste = |runtime: &mut Runtime| {
            if toolbar {
                assert!(runtime.paste_text_selection(editor, SelectionChangedCause::Toolbar));
                frame(runtime);
            } else {
                press(
                    runtime,
                    Code::KeyV,
                    NamedKey::Unidentified,
                    Modifiers::CONTROL,
                );
            }
        };
        paste(&mut runtime);
        state("12345", TextSelection::collapsed(5));
        press(
            &mut runtime,
            Code::ArrowLeft,
            NamedKey::ArrowLeft,
            Modifiers::empty(),
        );
        press(
            &mut runtime,
            Code::ArrowRight,
            NamedKey::ArrowRight,
            Modifiers::SHIFT,
        );
        state("12345", TextSelection::new(4, 5));
        paste(&mut runtime);
        state("123412345", TextSelection::collapsed(9));
        press(
            &mut runtime,
            Code::KeyA,
            NamedKey::Unidentified,
            Modifiers::CONTROL,
        );
        state("123412345", TextSelection::new(0, 9));
        paste(&mut runtime);
        state("12345", TextSelection::collapsed(5));
    }
}
