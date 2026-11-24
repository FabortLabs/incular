//! Flutter 3.50.0 retained visibility and nested SafeArea regressions.
//! See audits/flutter-3.50.0-parity/visibility-safe-area.md for the paired tests.

use std::time::{Duration, Instant};

use incular_config::{Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_rendering::PaintCommand;
use incular_runtime::{Runtime, Signal, TextShortcutPlatform};
use incular_widgets::{
    Column, EditableText, SafeArea, SizedBox, TextDirection, Visibility, Widget,
    internal::TextEditingController,
};

#[test]
fn hidden_state_retaining_visibility_stops_editing() {
    let controller = TextEditingController::with_text("a");
    let visible = Signal::new(true);
    let build = {
        let controller = controller.clone();
        let visible = visible.clone();
        move || -> Widget {
            Column::new([
                Widget::from(
                    Visibility::new(
                        EditableText::new(controller.clone())
                            .text_direction(TextDirection::Ltr)
                            .size(Size::new(400., 40.)),
                    )
                    .maintain_state(true)
                    .visible(visible.get()),
                ),
                SizedBox::new().width(400.).height(100.).into(),
            ])
            .into()
        }
    };

    let bounds = Constraints::tight(Size::new(400., 300.));
    let frame = Duration::from_millis(16);
    let mut now = Instant::now();
    let mut runtime = Runtime::new(build()).expect("runtime");
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.run_frame_at(bounds, now).expect("initial frame");
    let root = runtime.tree().root().expect("root");
    runtime
        .register_builder(root, build)
        .expect("register scene");
    now += frame;
    runtime.run_frame_at(bounds, now).expect("visible frame");

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
                position: Offset::new(100., 20.),
            },
            now,
        );
        now += Duration::from_millis(50);
    }
    now += Duration::from_millis(500);
    runtime.run_frame_at(bounds, now).expect("focus frame");
    assert!(
        runtime.focused_element().is_some(),
        "the visible editor must first receive focus"
    );

    visible.set(false);
    now += frame;
    runtime.run_frame_at(bounds, now).expect("hidden frame");
    let hidden_focus = runtime.focused_element();

    now += frame;
    let _ = runtime.handle_input_at(InputEvent::Text("x".into()), now);
    runtime.run_frame_at(bounds, now).expect("text input frame");
    assert_eq!(
        (hidden_focus, controller.text()),
        (None, "a".into()),
        "a hidden editor should release focus and stop accepting input"
    );
}

fn nested_safe_area_child_size(maintain_bottom_view_padding: bool) -> Size {
    let viewport = Size::new(200., 200.);
    let bottom = EdgeInsets::only(0., 0., 0., 20.);
    let child = Widget::box_(Size::new(50., 50.), Color::WHITE);
    let outer = SafeArea::new(SafeArea::new(child))
        .maintain_bottom_view_padding(maintain_bottom_view_padding);
    let mut runtime = Runtime::new(outer.into()).expect("runtime");
    let _ = runtime.set_environment(RuntimeEnvironment {
        viewport,
        safe_insets: bottom,
        view_padding: bottom,
        ..RuntimeEnvironment::default()
    });
    let (display_list, _) = runtime
        .run_frame(Constraints::tight(viewport))
        .expect("SafeArea frame");
    let mut rectangles = display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, .. } => Some(rect.size),
            _ => None,
        });
    let size = rectangles.next().expect("painted SafeArea child");
    assert!(
        rectangles.next().is_none(),
        "only the child paints a rectangle"
    );
    size
}

#[test]
fn nested_ordinary_safe_areas_consume_bottom_padding_only_once() {
    assert_eq!(nested_safe_area_child_size(false), Size::new(200., 180.));
}

#[test]
fn an_outer_safe_area_with_maintained_view_padding_still_consumes_bottom_padding() {
    assert_eq!(nested_safe_area_child_size(true), Size::new(200., 180.));
}
