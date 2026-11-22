//! Flutter on Android and iOS: a finger tapping outside the focused text
//! field leaves it focused (`EditableText._defaultOnTapOutside`), but a tap
//! on another text field moves focus there.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    Column, EditableText, SizedBox, TextDirection, Widget, internal::TextEditingController,
};
use std::time::{Duration, Instant};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 300.))
}

/// A touch tap, reusing pointer 0 as Android does for every gesture.
fn tap(runtime: &mut Runtime, position: Offset, now: &mut Instant) {
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 0,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position,
            },
            *now,
        );
        *now += Duration::from_millis(50);
    }
    *now += Duration::from_millis(500);
    runtime.run_frame_at(bounds(), *now).unwrap();
}

fn type_text(runtime: &mut Runtime, now: Instant) {
    let _ = runtime.handle_input_at(InputEvent::Text("x".into()), now);
    runtime.run_frame_at(bounds(), now).unwrap();
}

fn fields(
    platform: TextShortcutPlatform,
) -> (
    Runtime,
    TextEditingController,
    TextEditingController,
    Instant,
) {
    let a = TextEditingController::with_text("a");
    let b = TextEditingController::with_text("b");
    let field = |controller: &TextEditingController| -> Widget {
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(400., 40.))
            .into()
    };
    let column = Column::new(vec![
        field(&a),
        field(&b),
        SizedBox::new().width(400.).height(100.).into(),
    ]);
    let mut runtime = Runtime::new(Widget::from(column)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    let now = Instant::now();
    runtime.run_frame_at(bounds(), now).unwrap();
    (runtime, a, b, now)
}

#[test]
fn a_touch_tap_on_another_field_moves_focus_on_mobile() {
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let (mut runtime, a, b, mut now) = fields(platform);
        tap(&mut runtime, Offset::new(100., 20.), &mut now);
        type_text(&mut runtime, now);
        tap(&mut runtime, Offset::new(100., 60.), &mut now);
        type_text(&mut runtime, now);
        tap(&mut runtime, Offset::new(100., 20.), &mut now);
        type_text(&mut runtime, now);
        assert_eq!(
            (a.text(), b.text()),
            ("axx".into(), "bx".into()),
            "{platform:?}"
        );
    }
}

#[test]
fn a_touch_tap_outside_any_field_keeps_focus_on_mobile() {
    let (mut runtime, a, _b, mut now) = fields(TextShortcutPlatform::Android);
    tap(&mut runtime, Offset::new(100., 20.), &mut now);
    tap(&mut runtime, Offset::new(100., 150.), &mut now);
    type_text(&mut runtime, now);
    assert_eq!(a.text(), "ax");
}
