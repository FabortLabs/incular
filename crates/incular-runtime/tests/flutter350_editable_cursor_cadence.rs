use std::time::Duration;

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, TextDirection};

const CARET: Color = Color::rgba(231, 12, 42, 255);

fn viewport() -> Constraints {
    Constraints::tight(Size::new(220., 70.))
}

fn active_editor(animates: bool) -> Runtime {
    Runtime::new(
        EditableText::new(TextEditingController::new())
            .text_direction(TextDirection::Ltr)
            .autofocus(true)
            .cursor_color(CARET)
            .cursor_opacity_animates(animates)
            .into(),
    )
    .unwrap()
}

fn alpha(list: &DisplayList) -> u8 {
    list.commands()
        .iter()
        .find_map(|paint| match paint {
            PaintCommand::Rect { color, .. }
                if color.red == CARET.red
                    && color.green == CARET.green
                    && color.blue == CARET.blue =>
            {
                Some(color.alpha)
            }
            _ => None,
        })
        .unwrap_or(0)
}

#[test]
fn animated_cursor_repaints_on_the_discrete_flutter_keyframes() {
    let mut runtime = active_editor(true);
    let start = Instant::now();
    let (initial, _) = runtime.run_frame_at(viewport(), start).unwrap();
    assert_eq!(alpha(&initial), 255);
    for (micros, expected) in [
        (499_999, 255),
        (537_499, 255),
        (574_999, 191),
        (612_499, 128),
        (649_999, 64),
        (849_999, 0),
        (887_499, 0),
        (924_999, 64),
        (962_499, 128),
        (999_999, 191),
        (1_000_000, 255),
    ] {
        let now = start + Duration::from_micros(micros);
        let (paint, _) = runtime.run_frame_at(viewport(), now).unwrap();
        assert_eq!(alpha(&paint), expected, "at {micros}us");
        assert!(
            runtime
                .next_frame_deadline()
                .is_some_and(|deadline| deadline > now)
        );
    }
}

#[test]
fn non_animated_caret_uses_timer_edges_without_active_frame_animations() {
    let mut runtime = active_editor(false);
    let start = Instant::now();
    runtime.run_frame_at(viewport(), start).unwrap();
    assert!(
        runtime
            .next_frame_deadline()
            .is_some_and(|deadline| deadline <= start + Duration::from_millis(500)),
        "the initial history boundary may precede the first caret phase"
    );
    for i in 1..=40 {
        let now = start + Duration::from_millis(i * 100);
        let (paint, stats) = runtime.run_frame_at(viewport(), now).unwrap();
        let expected = if (i / 5) % 2 == 0 { 255 } else { 0 };
        assert_eq!(alpha(&paint), expected, "at {}ms", i * 100);
        assert_eq!(stats.active_animations, 0);
        assert!(!stats.requested_another_frame);
        assert_eq!(stats.laid_out_render_objects, 0);
        if i == 5 {
            assert_eq!(
                runtime.next_frame_deadline(),
                Some(start + Duration::from_millis(1000))
            );
        }
    }
}

#[test]
fn hiding_or_blurring_the_caret_disarms_its_future_frame_and_show_restarts_it() {
    let controller = TextEditingController::new();
    let node = FocusNode::new();
    let editor = |show| {
        EditableText::new(controller.clone())
            .focus_node(node.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .cursor_color(CARET)
            .show_cursor(show)
            .into()
    };
    let mut runtime = Runtime::new(editor(true)).unwrap();
    let root = runtime.tree().root().unwrap();
    let zero = Instant::now();
    runtime.run_frame_at(viewport(), zero).unwrap();
    assert!(runtime.next_frame_deadline().is_some());

    runtime.schedule_update(root, editor(false)).unwrap();
    let (paint, _) = runtime
        .run_frame_at(viewport(), zero + Duration::from_millis(510))
        .unwrap();
    assert_eq!(alpha(&paint), 0);
    assert_eq!(runtime.next_frame_deadline(), None);
    assert!(node.has_primary_focus());

    runtime.schedule_update(root, editor(true)).unwrap();
    let now = zero + Duration::from_millis(600);
    let (paint, _) = runtime.run_frame_at(viewport(), now).unwrap();
    assert_eq!(alpha(&paint), 255);
    assert_eq!(
        runtime.next_frame_deadline(),
        Some(now + Duration::from_millis(500))
    );

    node.unfocus();
    let (paint, _) = runtime
        .run_frame_at(viewport(), now + Duration::from_millis(10))
        .unwrap();
    assert_eq!(alpha(&paint), 0);
    assert_eq!(runtime.next_frame_deadline(), None);
}

#[test]
fn desktop_arrow_keys_restart_caret_timing_from_the_input_event() {
    use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey};
    use incular_runtime::TextShortcutPlatform;
    use incular_text::TextSelection;

    let controller =
        TextEditingController::with_text("Some text long enough to move the cursor around");
    controller.set_selection(TextSelection::collapsed(22));
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .cursor_color(CARET)
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::MacOs);
    let viewport = Constraints::tight(Size::new(800., 70.));
    let zero = Instant::now();
    let frame_at = |runtime: &mut Runtime, millis| {
        let (list, _) = runtime
            .run_frame_at(viewport, zero + Duration::from_millis(millis))
            .unwrap();
        alpha(&list)
    };
    let press = |runtime: &mut Runtime, millis, key, code| {
        let now = zero + Duration::from_millis(millis);
        let key = KeyboardKey::Named(key);
        let _ = runtime.handle_input_at(
            InputEvent::Key(KeyboardEvent::key_down(key.clone(), code)),
            now,
        );
        assert_eq!(frame_at(runtime, millis), 255);
        let _ = runtime.handle_input_at(InputEvent::Key(KeyboardEvent::key_up(key, code)), now);
        assert_eq!(frame_at(runtime, millis), 255);
    };
    for millis in [0, 499] {
        assert_eq!(frame_at(&mut runtime, millis), 255);
    }
    press(&mut runtime, 499, NamedKey::ArrowLeft, Code::ArrowLeft);
    assert_eq!(controller.selection(), TextSelection::collapsed(21));
    for millis in [699, 998] {
        assert_eq!(frame_at(&mut runtime, millis), 255);
    }
    for _ in 0..2 {
        press(&mut runtime, 998, NamedKey::ArrowRight, Code::ArrowRight);
    }
    assert_eq!(controller.selection(), TextSelection::collapsed(23));
    for millis in [1198, 1497] {
        assert_eq!(frame_at(&mut runtime, millis), 255);
    }
    assert_eq!(frame_at(&mut runtime, 1498), 0);
}
