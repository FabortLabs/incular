use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_platform::{FlutterFloatingCursorPoint, TextInputClientId, TextInputCommand};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::{Column, EditableText, SizedBox};
use serde_json::{Value, json};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(180., 60.))
}
const CURSOR: Color = Color::rgba(17, 31, 251, 255);

fn mount() -> (Runtime, TextEditingController, TextInputClientId) {
    let controller = TextEditingController::with_text("ABCDEFGHIJ");
    let widget = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180., 40.))
        .cursor_color(CURSOR);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    runtime.run_frame(constraints()).unwrap();
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap();
    (runtime, controller, client)
}

fn wire(runtime: &mut Runtime, sender: Value, phase: &str, position: Value) -> Option<bool> {
    let call = json!({
        "method": "TextInputClient.updateFloatingCursor",
        "args": [sender, format!("FloatingCursorDragState.{phase}"), position],
    });
    runtime
        .handle_flutter_floating_cursor_call(call.to_string().as_bytes())
        .unwrap()
}

fn floating_caret_rect(list: &DisplayList) -> Option<incular_core::Rect> {
    list.commands().iter().find_map(|command| match command {
        PaintCommand::RRect {
            rrect,
            brush: Brush::Solid(color),
        } if *color
            == Color {
                alpha: 191,
                ..CURSOR
            } =>
        {
            Some(rrect.rect)
        }
        _ => None,
    })
}

fn visible_floating_caret(list: &DisplayList) -> bool {
    floating_caret_rect(list).is_some()
}

#[test]
fn cursor_updates_accept_integer_coordinates_without_replacing_the_editing_value() {
    let (mut runtime, controller, client) = mount();
    let sender = if cfg!(debug_assertions) {
        json!(-1)
    } else {
        json!(client.get())
    };
    let old = controller.value();
    assert_eq!(
        wire(&mut runtime, sender, "update", json!({"X":2, "Y":3})),
        Some(true)
    );
    assert_eq!(controller.value(), old);

    let point = FlutterFloatingCursorPoint::from_flutter_json(
        "FloatingCursorDragState.update",
        &json!({"X":2, "Y":3}),
    )
    .unwrap();
    assert_eq!(point.offset, Offset::new(2., 3.));
    let point = FlutterFloatingCursorPoint::from_flutter_json(
        "FloatingCursorDragState.update",
        &json!({"X":-2.5, "Y":3}),
    )
    .unwrap();
    assert_eq!(point.offset, Offset::new(-2.5, 3.));
}

fn horizontal_delta(runtime: &Runtime, from: usize, to: usize) -> f32 {
    let field = runtime.focused_element().unwrap();
    let caret = |index| {
        runtime
            .tree()
            .floating_caret_geometry(field, index, TextAffinity::Downstream)
            .unwrap()
            .caret
    };
    let first = caret(from);
    caret(to).origin.x - first.origin.x - first.size.width * 0.5
}

#[test]
fn floating_caret_moves_visually_then_changes_a_collapsed_selection_after_settling() {
    let (mut runtime, controller, client) = mount();
    let sender = json!(client.get());
    controller.set_selection(TextSelection::collapsed(0));
    runtime.run_frame(constraints()).unwrap();
    let dx = horizontal_delta(&runtime, 0, 4);

    assert_eq!(
        wire(
            &mut runtime,
            sender.clone(),
            "start",
            json!({"X":100, "Y":100})
        ),
        Some(true)
    );
    let field = runtime.focused_element().unwrap();
    let geometry = runtime
        .tree()
        .floating_caret_geometry(field, 0, TextAffinity::Downstream)
        .unwrap();
    let painted = floating_caret_rect(&runtime.run_frame(constraints()).unwrap().0).unwrap();
    let y = geometry.caret.origin.y
        + (geometry.caret.size.height - geometry.preferred_height) * 0.5
        - 1.;
    assert!((painted.origin.y - y).abs() < 0.01);
    assert_eq!(
        wire(
            &mut runtime,
            sender.clone(),
            "update",
            json!({"X":dx, "Y":0})
        ),
        Some(true)
    );
    assert!(visible_floating_caret(
        &runtime.run_frame(constraints()).unwrap().0
    ));
    assert_eq!(controller.selection(), TextSelection::collapsed(0));

    assert_eq!(
        wire(&mut runtime, sender, "end", json!({"X":0, "Y":0})),
        Some(true)
    );
    let after_end = Instant::now();
    let (settling, stats) = runtime
        .run_frame_at(constraints(), after_end + Duration::from_millis(60))
        .unwrap();
    assert!(visible_floating_caret(&settling));
    assert!(stats.requested_another_frame);
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
    let (settled, _) = runtime
        .run_frame_at(constraints(), after_end + Duration::from_millis(130))
        .unwrap();
    assert!(!visible_floating_caret(&settled));
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
}

#[test]
fn floating_caret_does_not_erase_an_ime_selection_and_stale_senders_are_ignored() {
    let (mut runtime, controller, client) = mount();
    let sender = json!(client.get());
    controller.set_selection(TextSelection::collapsed(2));
    runtime.run_frame(constraints()).unwrap();
    let dx = horizontal_delta(&runtime, 2, 5);
    assert_eq!(
        wire(
            &mut runtime,
            json!(client.get() + 1),
            "start",
            json!({"X":0, "Y":0})
        ),
        Some(false)
    );
    assert!(!visible_floating_caret(
        &runtime.run_frame(constraints()).unwrap().0
    ));
    wire(&mut runtime, sender.clone(), "start", json!({"X":0, "Y":0}));
    wire(
        &mut runtime,
        sender.clone(),
        "update",
        json!({"X":dx, "Y":0}),
    );
    let selection = TextSelection::new(2, 5);
    controller.set_selection(selection);
    wire(&mut runtime, sender, "end", json!({"X":0, "Y":0}));
    runtime
        .run_frame_at(constraints(), Instant::now() + Duration::from_millis(130))
        .unwrap();
    assert_eq!(controller.selection(), selection);
    assert_eq!(controller.text(), "ABCDEFGHIJ");
}

#[test]
fn reversing_after_overshoot_reenters_from_the_edge_without_a_long_dead_zone() {
    let (mut runtime, controller, client) = mount();
    controller.set_selection(TextSelection::collapsed(0));
    runtime.run_frame(constraints()).unwrap();
    let sender = json!(client.get());
    wire(&mut runtime, sender.clone(), "start", json!({"X":0, "Y":0}));
    wire(
        &mut runtime,
        sender.clone(),
        "update",
        json!({"X":1000, "Y":0}),
    );
    let edge = floating_caret_rect(&runtime.run_frame(constraints()).unwrap().0).unwrap();
    wire(
        &mut runtime,
        sender.clone(),
        "update",
        json!({"X":995, "Y":0}),
    );
    let reversal = floating_caret_rect(&runtime.run_frame(constraints()).unwrap().0).unwrap();
    assert!((reversal.origin.x - edge.origin.x).abs() < 0.01);
    wire(
        &mut runtime,
        sender.clone(),
        "update",
        json!({"X":970, "Y":0}),
    );
    let reentered = floating_caret_rect(&runtime.run_frame(constraints()).unwrap().0).unwrap();
    assert!((reentered.origin.x - (edge.origin.x - 25.)).abs() < 0.01);
    assert_eq!(controller.selection(), TextSelection::collapsed(0));

    wire(&mut runtime, sender, "end", json!({"X":0, "Y":0}));
    runtime
        .run_frame_at(constraints(), Instant::now() + Duration::from_millis(130))
        .unwrap();
    assert!(controller.selection().extent < controller.text().len());
}

#[test]
fn removing_the_editor_while_the_floating_caret_resets_cancels_frame_work() {
    let controller = TextEditingController::with_text("A");
    let editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .cursor_color(CURSOR)
        .cursor_opacity_animates(true);
    let mut runtime = Runtime::new(Column::new([editor]).into()).unwrap();
    let root = runtime.tree().root().unwrap();
    let start = Instant::now();
    runtime.run_frame_at(constraints(), start).unwrap();
    assert!(runtime.focused_element().is_some());
    assert!(
        runtime.next_frame_deadline().is_some(),
        "ordinary blink is armed"
    );
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .unwrap();
    let sender = json!(client.get());
    assert_eq!(
        wire(&mut runtime, sender.clone(), "start", json!({"X":0, "Y":0})),
        Some(true)
    );
    let (dragging, _) = runtime
        .run_frame_at(constraints(), start + Duration::from_millis(150))
        .unwrap();
    assert!(visible_floating_caret(&dragging));
    runtime
        .run_frame_at(constraints(), start + Duration::from_millis(650))
        .unwrap();
    assert_eq!(
        wire(&mut runtime, sender, "end", json!({"X":0, "Y":0})),
        Some(true)
    );
    let ending = Instant::now();
    let (settling, stats) = runtime
        .run_frame_at(constraints(), ending + Duration::from_millis(10))
        .unwrap();
    assert!(visible_floating_caret(&settling));
    assert!(stats.requested_another_frame, "caret reset is active");

    let value = controller.value();
    runtime
        .schedule_update(root, Column::new([SizedBox::shrink()]).into())
        .unwrap();
    let (removed, stats) = runtime
        .run_frame_at(constraints(), ending + Duration::from_millis(20))
        .unwrap();
    assert!(!visible_floating_caret(&removed));
    assert!(runtime.focused_element().is_none());
    assert!(!stats.requested_another_frame);
    assert!(runtime.next_frame_deadline().is_none());
    let (_, stats) = runtime
        .run_frame_at(constraints(), ending + Duration::from_secs(2))
        .unwrap();
    assert!(!stats.requested_another_frame);
    assert_eq!(controller.value(), value);
}

#[test]
fn a_distant_floating_caret_paints_its_background_anchor_only_during_drag() {
    use incular_text::TextStyle;

    const BACKGROUND: Color = Color::rgba(135, 86, 52, 255);
    let constraints = Constraints::tight(Size::new(400., 120.));
    let controller = TextEditingController::with_text("AAAA");
    controller.set_selection(TextSelection::collapsed(0));
    let editor = EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_size(80.))
        .cursor_color(CURSOR)
        .background_cursor_color(BACKGROUND);
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    runtime.run_frame(constraints).unwrap();
    let sender = json!(
        runtime
            .take_text_input_commands()
            .into_iter()
            .find_map(|command| match command {
                TextInputCommand::SetClient { configuration, .. } => {
                    Some(configuration.client.get())
                }
                _ => None,
            })
            .unwrap()
    );
    let field = runtime.focused_element().unwrap();
    let anchor = runtime
        .tree()
        .floating_caret_geometry(field, 0, TextAffinity::Downstream)
        .unwrap()
        .caret;
    let anchor_paint = |list: &DisplayList| {
        list.commands()
            .iter()
            .enumerate()
            .find_map(|(index, command)| match command {
                PaintCommand::Rect { rect, color } if *color == BACKGROUND => Some((index, *rect)),
                _ => None,
            })
    };
    let move_caret = |runtime: &mut Runtime, phase: &str, x: f32| {
        assert_eq!(
            wire(runtime, sender.clone(), phase, json!({"X":x, "Y":0})),
            Some(true)
        );
        runtime.run_frame(constraints).unwrap().0
    };

    let start = move_caret(&mut runtime, "start", 0.);
    assert!(visible_floating_caret(&start));
    assert!(anchor_paint(&start).is_none());
    let overlapping = move_caret(&mut runtime, "update", 10.);
    assert!(visible_floating_caret(&overlapping));
    assert!(anchor_paint(&overlapping).is_none());
    let separated = move_caret(&mut runtime, "update", 20.);
    let (anchor_index, regular) = anchor_paint(&separated).unwrap();
    assert_eq!(regular, anchor);
    assert!(
        separated.commands()[anchor_index + 1..]
            .iter()
            .any(|command| {
                matches!(command, PaintCommand::RRect { brush: Brush::Solid(color), .. }
            if *color == Color { alpha: 191, ..CURSOR })
            })
    );
    assert_eq!(controller.selection(), TextSelection::collapsed(0));

    assert_eq!(
        wire(&mut runtime, sender, "end", json!({"X":0, "Y":0})),
        Some(true)
    );
    let ending = Instant::now();
    let (settling, _) = runtime
        .run_frame_at(constraints, ending + Duration::from_millis(20))
        .unwrap();
    assert!(visible_floating_caret(&settling));
    assert!(anchor_paint(&settling).is_none());
    let (settled, _) = runtime
        .run_frame_at(constraints, ending + Duration::from_millis(140))
        .unwrap();
    assert!(!visible_floating_caret(&settled));
    assert!(anchor_paint(&settled).is_none());
    assert!(settled.commands().iter().any(|command| {
        matches!(command, PaintCommand::Rect { color, .. } if *color == CURSOR)
    }));
}
