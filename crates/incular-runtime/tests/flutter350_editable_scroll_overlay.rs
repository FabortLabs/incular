use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{
    EditableText, SizedBox, Widget,
    internal::{ElementId, Key},
};

fn pointer(runtime: &mut Runtime, phase: PointerPhase, position: Offset, at: Instant) {
    let terminal = matches!(phase, PointerPhase::Up | PointerPhase::Cancel);
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 2,
            kind: PointerDeviceKind::Touch,
            buttons: if terminal { 0 } else { PRIMARY_POINTER_BUTTON },
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            phase,
            position,
            sample: PointerSampleMetadata::default(),
        },
        at,
    );
}

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(400., 200.)), at)
        .unwrap();
}

fn toolbar_visible(runtime: &Runtime, field: ElementId) -> bool {
    runtime
        .tree()
        .text_field_toolbar_state(field)
        .unwrap()
        .is_showing()
}

#[test]
fn a_short_touch_scroll_hides_then_restores_the_toolbar_without_semantics() {
    const TOOLBAR: &str = "editor scroll test toolbar";
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let controller =
            TextEditingController::with_text("Lorem ipsum dolor sit amet ".repeat(200));
        let editor = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None)
            .context_menu_builder(|_, _| Widget::from(SizedBox::shrink()).with_key(TOOLBAR))
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.set_platform_semantics_enabled(false);
        assert!(!runtime.semantics_enabled());

        let clock = Instant::now();
        let time = |milliseconds| clock + Duration::from_millis(milliseconds);
        let middle = Offset::new(200., 100.);
        frame(&mut runtime, time(0));
        pointer(&mut runtime, PointerPhase::Down, middle, time(10));
        frame(&mut runtime, time(510));
        pointer(&mut runtime, PointerPhase::Up, middle, time(530));
        frame(&mut runtime, time(530));
        let field = runtime.focused_element().unwrap();
        let selection = controller.selection();
        assert!(!selection.is_collapsed());
        assert!(toolbar_visible(&runtime, field));
        let previous_offset = runtime.tree().text_field_scroll_offset(field).unwrap();

        pointer(&mut runtime, PointerPhase::Down, middle, time(1000));
        let moved = middle - Offset::new(0., 20.);
        pointer(&mut runtime, PointerPhase::Move, moved, time(1020));
        frame(&mut runtime, time(1020));
        assert!(
            runtime.tree().text_field_scroll_offset(field).unwrap().y > previous_offset.y,
            "the editor's viewport must consume the short drag ({platform:?})"
        );
        assert_eq!(controller.selection(), selection);
        assert!(!toolbar_visible(&runtime, field));

        pointer(&mut runtime, PointerPhase::Up, moved, time(1040));
        frame(&mut runtime, time(1040));
        assert!(
            runtime.frame_requested(),
            "restoration must schedule its own paint"
        );
        frame(&mut runtime, time(1060));
        assert!(
            runtime
                .tree()
                .element_with_key(&Key::from(TOOLBAR))
                .is_some()
        );
        assert!(toolbar_visible(&runtime, field));
        assert_eq!(controller.selection(), selection);
    }
}
