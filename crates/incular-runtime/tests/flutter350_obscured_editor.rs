//! Pinned Flutter obscured editor code-point behavior.

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText,
    internal::{TextEditingController, TextSelection},
};

fn key(runtime: &mut Runtime, name: NamedKey, code: Code) {
    assert!(
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(KeyboardKey::Named(name), code))
    );
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(KeyboardKey::Named(name), code));
}

fn editor(obscured: bool) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::new();
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .obscure_text(obscured);
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime
        .run_frame(Constraints::tight(Size::new(240., 50.)))
        .unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(50., 10.),
        });
    }
    assert!(runtime.focused_element().is_some());
    (runtime, controller)
}

#[test]
fn obscured_family_emoji_walks_and_deletes_one_code_point_at_a_time() {
    let (mut runtime, controller) = editor(true);
    let _ = runtime.handle_input(InputEvent::Text("👨‍👩‍👦".into()));
    let caret = || {
        let text = controller.text();
        assert!(controller.selection().is_collapsed());
        text[..controller.selection().extent].encode_utf16().count()
    };
    assert_eq!(controller.text(), "👨‍👩‍👦");
    assert_eq!(caret(), 8);
    for column in [6, 5, 3, 2, 0] {
        key(&mut runtime, NamedKey::ArrowLeft, Code::ArrowLeft);
        assert_eq!(caret(), column);
    }
    for column in [2, 3, 5, 6, 8] {
        key(&mut runtime, NamedKey::ArrowRight, Code::ArrowRight);
        assert_eq!(caret(), column);
    }
    for expected in ["👨‍👩‍", "👨‍👩", "👨‍", "👨", ""] {
        key(&mut runtime, NamedKey::Backspace, Code::Backspace);
        assert_eq!(controller.text(), expected);
        assert_eq!(caret(), expected.encode_utf16().count());
    }
}

#[test]
fn focused_desktop_replacement_and_explicit_caret_delete_different_ranges() {
    let (mut runtime, controller) = editor(false);
    controller.set_text("é👩‍💻");
    assert_eq!(
        controller.selection(),
        TextSelection::new(0, controller.text().len())
    );
    key(&mut runtime, NamedKey::Backspace, Code::Backspace);
    assert!(controller.text().is_empty());

    controller.set_text("é👩‍💻");
    controller.set_selection(TextSelection::collapsed(controller.text().len()));
    key(&mut runtime, NamedKey::Backspace, Code::Backspace);
    assert_eq!(controller.text(), "é");
    controller.set_selection(TextSelection::collapsed(0));
    key(&mut runtime, NamedKey::Delete, Code::Delete);
    assert!(controller.text().is_empty());
}

#[test]
fn restoring_obscuring_while_focused_repaints_every_glyph_as_a_mask() {
    use incular_platform::{PlatformEvent, TextInputState};
    use incular_rendering::PaintCommand;
    use incular_widgets::{FocusNode, Widget};

    for platform in [TextShortcutPlatform::Ios, TextShortcutPlatform::Android] {
        let controller = TextEditingController::new();
        let focus = FocusNode::new();
        let build = |obscure| {
            Widget::from(
                EditableText::new(controller.clone())
                    .focus_node(focus.clone())
                    .text_direction(TextDirection::Ltr)
                    .obscure_text(obscure),
            )
        };
        let mut runtime = Runtime::new(build(true)).unwrap();
        runtime.set_text_shortcut_platform(platform);
        let size = Constraints::tight(Size::new(240., 50.));
        let paint = |runtime: &mut Runtime| {
            runtime
                .run_frame(size)
                .unwrap()
                .0
                .commands()
                .iter()
                .filter_map(|command| match command {
                    PaintCommand::GlyphRun { run, .. } => {
                        Some(run.glyphs.iter().map(|glyph| glyph.id))
                    }
                    _ => None,
                })
                .flatten()
                .collect::<Vec<_>>()
        };
        focus.request_focus();
        let _ = paint(&mut runtime);
        let field = runtime.focused_element().unwrap();
        let mut masked = Vec::new();
        for plain in ["H", "HH", "HHH"] {
            let _ =
                runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
                    text: plain.to_owned(),
                    selection_start: plain.len(),
                    selection_end: plain.len(),
                    composing: None,
                }));
            assert_eq!(controller.text(), plain);
            masked = paint(&mut runtime);
            assert_eq!(masked.len(), plain.len());
            assert!(masked.iter().all(|glyph| *glyph == masked[0]));
        }
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, build(false)).unwrap();
        let visible = paint(&mut runtime);
        assert_eq!(visible.len(), 3);
        assert!(visible.iter().all(|glyph| *glyph == visible[0]));
        assert_ne!(visible, masked);

        runtime.schedule_update(root, build(true)).unwrap();
        assert_eq!(paint(&mut runtime), masked);
        assert_eq!(runtime.focused_element(), Some(field));
        assert!(focus.has_primary_focus());
        assert_eq!(controller.text(), "HHH");
        assert_eq!(controller.selection(), TextSelection::collapsed(3));
    }
}
