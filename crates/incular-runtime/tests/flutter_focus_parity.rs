//! A neutral Flutter EditableText requests focus on its recognized tap,
//! not on raw touch down. A canceled tap must not steal focus.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{Column, EditableText, TextDirection, Widget};
use std::time::{Duration, Instant};

#[test]
fn canceled_touch_on_another_editable_keeps_current_focus() {
    let field = |text| {
        Widget::from(
            EditableText::new(TextEditingController::with_text(text))
                .text_direction(TextDirection::Ltr)
                .size(Size::new(180., 40.)),
        )
    };
    let mut runtime = Runtime::new(Column::new([field("a"), field("b")]).into())
        .expect("mount two editable fields");
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let bounds = Constraints::tight(Size::new(180., 80.));
    let mut now = Instant::now();
    runtime.run_frame_at(bounds, now).expect("initial layout");

    let touch = |runtime: &mut Runtime, now: &mut Instant, y, phase| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 0,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: (phase != PointerPhase::Cancel).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(20., y),
            },
            *now,
        );
        *now += Duration::from_millis(16);
        runtime.run_frame_at(bounds, *now).expect("process touch");
    };

    touch(&mut runtime, &mut now, 20., PointerPhase::Down);
    touch(&mut runtime, &mut now, 20., PointerPhase::Up);
    let original = runtime
        .focused_element()
        .expect("completed first tap focuses the first editable");

    touch(&mut runtime, &mut now, 60., PointerPhase::Down);
    touch(&mut runtime, &mut now, 60., PointerPhase::Cancel);
    let after_cancel = runtime.focused_element();

    now += Duration::from_millis(500);
    touch(&mut runtime, &mut now, 60., PointerPhase::Down);
    touch(&mut runtime, &mut now, 60., PointerPhase::Up);
    let second = runtime
        .focused_element()
        .expect("completed second tap focuses the second editable");
    assert_ne!(second, original, "control: the second field is tappable");
    assert_eq!(
        after_cancel,
        Some(original),
        "a canceled contact must not transfer focus to the second editable"
    );
}
