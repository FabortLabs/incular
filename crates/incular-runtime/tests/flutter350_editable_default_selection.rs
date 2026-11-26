use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{DefaultSelectionStyle, EditableText, FocusNode, TextDirection, Widget};

const INHERITED: Color = Color::rgba(255, 165, 0, 255);
const EXPLICIT: Color = Color::rgba(4, 168, 42, 255);
const PREVIOUS_DEFAULT: Color = Color::rgba(72, 120, 220, 150);

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(180., 80.)))
        .unwrap()
        .0
}

fn color_rectangles(list: &DisplayList, tint: Color) -> usize {
    list.commands()
        .iter()
        .filter(|cmd| matches!(cmd, PaintCommand::Rect { color, .. } if *color == tint))
        .count()
}

#[test]
fn raw_editable_neither_invents_a_highlight_nor_inherits_the_selection_style() {
    let controller = TextEditingController::with_text("test test");
    let selected = TextSelection::new(5, 7);
    controller.set_selection(selected);
    let focus = FocusNode::new();
    let build = |color| {
        let field = EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .select_all_on_focus(false);
        let field = match color {
            Some(color) => field.selection_color(color),
            None => field.clear_selection_color(),
        };
        Widget::from(DefaultSelectionStyle::new(field).selection_color(INHERITED))
    };
    // The first scene uses the literal constructor default, not the new setter.
    let default = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .select_all_on_focus(false);
    let mut runtime = Runtime::new(
        DefaultSelectionStyle::new(default)
            .selection_color(INHERITED)
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let initial = frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), selected);
    assert_eq!(color_rectangles(&initial, INHERITED), 0);
    assert_eq!(color_rectangles(&initial, PREVIOUS_DEFAULT), 0);
    assert!(
        !initial
            .commands()
            .iter()
            .any(|command| { matches!(command, PaintCommand::Border { .. }) })
    );

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(Some(EXPLICIT)))
        .unwrap();
    assert!(color_rectangles(&frame(&mut runtime), EXPLICIT) > 0);
    runtime.schedule_update(root, build(None)).unwrap();
    let clear = frame(&mut runtime);
    assert_eq!(color_rectangles(&clear, EXPLICIT), 0);
    assert_eq!(color_rectangles(&clear, INHERITED), 0);
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), selected);
}

#[test]
fn logical_selection_and_affinity_survive_focusing_then_blurring_a_raw_editor() {
    use incular_text::TextAffinity;

    let controller = TextEditingController::with_text("test test");
    let selected = TextSelection::new(5, 7);
    let affinity = TextAffinity::Upstream;
    controller.set_selection_with_affinity(selected, affinity);
    let node = FocusNode::new();
    let field = EditableText::new(controller.clone())
        .focus_node(node.clone())
        .text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);

    frame(&mut runtime);
    assert!(!node.has_focus());
    assert_eq!(controller.text(), "test test");
    assert_eq!(controller.selection(), selected);
    assert_eq!(controller.caret_affinity(), affinity);

    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert_eq!(controller.selection(), selected);
    assert_eq!(controller.caret_affinity(), affinity);

    node.unfocus();
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert_eq!(controller.text(), "test test");
    assert_eq!(controller.selection(), selected);
    assert_eq!(controller.caret_affinity(), affinity);
}
