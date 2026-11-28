use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_platform::{MemoryClipboard, TransferReadRequest};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::{Align, Alignment, EditableText, FocusNode};

const TEXT: &str = "Now is the time for\nall good people\nto come to the aid\nof their country.";

fn read_clipboard(runtime: &mut Runtime) -> String {
    runtime
        .read_clipboard(TransferReadRequest::all())
        .unwrap()
        .plain_text_value()
        .unwrap()
        .unwrap()
        .to_string()
}

#[test]
fn read_only_keyboard_shortcuts_still_copy_and_select_but_cannot_edit() {
    for (name, platform, shortcut) in [
        ("Android", TextShortcutPlatform::Android, Modifiers::CONTROL),
        ("Fuchsia", TextShortcutPlatform::Android, Modifiers::CONTROL),
        ("iOS", TextShortcutPlatform::Ios, Modifiers::META),
        ("Linux", TextShortcutPlatform::Linux, Modifiers::CONTROL),
        ("macOS", TextShortcutPlatform::MacOs, Modifiers::META),
        ("Windows", TextShortcutPlatform::Windows, Modifiers::CONTROL),
    ] {
        let editor = TextEditingController::with_text(TEXT);
        let partial = TextSelection::new(0, TEXT.len() / 2);
        editor.set_selection_with_affinity(partial, TextAffinity::Upstream);
        let focus = FocusNode::new();
        let selections = Rc::new(RefCell::new(Vec::new()));
        let field = EditableText::new(editor.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .read_only(true)
            .text_direction(TextDirection::Ltr)
            .size(Size::new(400., 48.))
            .text_align(incular_text::TextAlign::Right)
            .on_selection_changed({
                let selections = selections.clone();
                move |selection, _| selections.borrow_mut().push(selection)
            });
        let mut runtime = Runtime::new(Align::new(Alignment::TOP_LEFT, field).into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.set_clipboard(Box::<MemoryClipboard>::default());
        let frame = |runtime: &mut Runtime| {
            runtime
                .run_frame(Constraints::tight(Size::new(800., 600.)))
                .unwrap();
        };
        frame(&mut runtime);
        assert!(focus.has_primary_focus(), "{name}");
        assert_eq!(editor.selection(), partial, "{name}");
        assert!(selections.borrow().is_empty(), "{name}");
        runtime.set_clipboard_text("read-only");
        let press = |runtime: &mut Runtime, code, logical, modifiers| {
            let mut event = KeyboardEvent::key_down(logical, code);
            event.modifiers = modifiers;
            let _ = runtime.handle_input(InputEvent::Key(event));
            frame(runtime);
        };
        let letter = KeyboardKey::Named(NamedKey::Unidentified);
        press(&mut runtime, Code::KeyV, letter.clone(), shortcut);
        assert_eq!(editor.selection(), partial, "{name}");
        assert!(selections.borrow().is_empty(), "{name}");
        assert_eq!(editor.text(), TEXT, "{name}");

        press(&mut runtime, Code::KeyA, letter.clone(), shortcut);
        let full = TextSelection::new(0, TEXT.len());
        assert_eq!(editor.selection(), full, "{name}");
        assert_eq!(&*selections.borrow(), &[full], "{name}");

        press(&mut runtime, Code::KeyX, letter.clone(), shortcut);
        assert_eq!(read_clipboard(&mut runtime), "read-only", "{name}");
        press(&mut runtime, Code::KeyC, letter, shortcut);
        assert_eq!(read_clipboard(&mut runtime), TEXT, "{name}");
        for (code, logical) in [
            (Code::Delete, NamedKey::Delete),
            (Code::Backspace, NamedKey::Backspace),
        ] {
            press(
                &mut runtime,
                code,
                KeyboardKey::Named(logical),
                Modifiers::empty(),
            );
            assert_eq!(editor.text(), TEXT, "{name}");
            assert_eq!(editor.selection(), full, "{name}");
            assert_eq!(&*selections.borrow(), &[full], "{name}");
        }
    }
}
