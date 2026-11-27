use std::time::Duration;

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, TextDirection};

const CURSOR: Color = Color::rgba(33, 150, 243, 255);

fn is_fully_painted(list: &DisplayList) -> bool {
    list.commands()
        .iter()
        .any(|command| matches!(command, PaintCommand::Rect { color: CURSOR, .. }))
}

#[test]
fn deterministic_cursor_stays_opaque_when_opacity_animation_is_enabled() {
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(
        EditableText::new(TextEditingController::new())
            .text_direction(TextDirection::Ltr)
            .focus_node(focus.clone())
            .max_lines(Some(3))
            .cursor_color(CURSOR)
            .cursor_opacity_animates(true)
            .into(),
    )
    .unwrap();
    runtime.set_debug_deterministic_cursor(true);
    focus.request_focus();
    let constraints = Constraints::tight(Size::new(220., 70.));
    let zero = Instant::now();
    for millis in [0, 200, 400, 650, 900, 4000] {
        let (list, stats) = runtime
            .run_frame_at(constraints, zero + Duration::from_millis(millis))
            .unwrap();
        assert!(is_fully_painted(&list));
        assert_eq!(stats.active_animations, 0);
    }
    assert!(runtime.next_frame_deadline().is_none());
    assert!(focus.has_primary_focus());

    runtime.set_debug_deterministic_cursor(false);
    let now = zero + Duration::from_millis(4100);
    let (list, _) = runtime.run_frame_at(constraints, now).unwrap();
    assert!(is_fully_painted(&list));
    assert_eq!(
        runtime.next_frame_deadline(),
        Some(now + Duration::from_micros(537_500))
    );
    let (list, _) = runtime
        .run_frame_at(constraints, now + Duration::from_millis(650))
        .unwrap();
    assert!(!is_fully_painted(&list));
    assert!(focus.has_primary_focus());
}
