use std::time::Duration;

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode, FocusScope, FocusScopeNode, TextDirection};

const CARET: Color = Color::rgba(231, 12, 42, 255);

fn editor(controller: &TextEditingController, focus: &FocusNode) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .cursor_color(CARET)
}

fn caret_paints(list: &DisplayList) -> usize {
    list.commands()
        .iter()
        .filter(|paint| matches!(paint, PaintCommand::Rect { color: CARET, .. }))
        .count()
}

fn viewport() -> Constraints {
    Constraints::tight(Size::new(220., 70.))
}

#[test]
fn mounted_cursor_preserves_configured_width_height_and_radius() {
    let focus = FocusNode::new();
    let controller = TextEditingController::new();
    let field = editor(&controller, &focus)
        .autofocus(true)
        .cursor_width(10.)
        .cursor_height(Some(10.))
        .cursor_radius(2.);
    assert_eq!(field.configured_cursor_width(), 10.);
    assert_eq!(field.configured_cursor_height(), Some(10.));
    assert_eq!(field.configured_cursor_radius(), 2.);
    let mut runtime = Runtime::new(field.into()).unwrap();
    let (list, _) = runtime.run_frame(viewport()).unwrap();
    let cursors: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|paint| match paint {
            PaintCommand::RRect { rrect, brush } if *brush == CARET.into() => Some(rrect),
            _ => None,
        })
        .collect();
    assert_eq!(cursors.len(), 1);
    assert_eq!(cursors[0].rect.size, Size::new(10., 10.));
    assert_eq!(cursors[0].radii.top_left, 2.);
}

#[test]
fn explicitly_hidden_cursor_never_paints_even_when_focused() {
    let focus = FocusNode::new();
    let controller = TextEditingController::new();
    let mut runtime = Runtime::new(editor(&controller, &focus).show_cursor(false).into()).unwrap();
    focus.request_focus();
    let now = Instant::now();
    for millis in [0, 200, 400, 600] {
        let (list, _) = runtime
            .run_frame_at(viewport(), now + Duration::from_millis(millis))
            .unwrap();
        assert!(focus.has_primary_focus());
        assert_eq!(caret_paints(&list), 0);
    }
}

#[test]
fn blur_removes_the_caret_and_refocusing_restores_it() {
    let focus = FocusNode::new();
    let controller = TextEditingController::new();
    let mut runtime = Runtime::new(editor(&controller, &focus).autofocus(true).into()).unwrap();
    let now = Instant::now();
    let (focused, _) = runtime.run_frame_at(viewport(), now).unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(caret_paints(&focused), 1);

    focus.unfocus();
    for millis in [0, 29] {
        let (blurred, stats) = runtime
            .run_frame_at(viewport(), now + Duration::from_millis(millis))
            .unwrap();
        assert_eq!(caret_paints(&blurred), 0);
        assert_eq!(stats.active_animations, 0);
    }
    focus.request_focus();
    let (focused, _) = runtime
        .run_frame_at(viewport(), now + Duration::from_millis(129))
        .unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(caret_paints(&focused), 1);
}

#[test]
fn toggling_show_cursor_repaints_without_replacing_or_unfocusing_the_editor() {
    let focus = FocusNode::new();
    let controller = TextEditingController::new();
    let build = |visible| {
        editor(&controller, &focus)
            .show_cursor(visible)
            .cursor_opacity_animates(true)
            .into()
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    focus.request_focus();
    let root = runtime.tree().root().unwrap();
    let id = runtime.tree().render_id(root);
    for expected in [false, true, true, false, false] {
        runtime.schedule_update(root, build(expected)).unwrap();
        let (frame, _) = runtime.run_frame(viewport()).unwrap();
        assert_eq!(caret_paints(&frame), usize::from(expected));
        assert_eq!(runtime.tree().render_id(root), id);
        assert!(focus.has_primary_focus());
    }
}

#[test]
fn autofocus_in_a_focused_scope_collapses_an_unset_selection_at_the_end() {
    let focus = FocusNode::new();
    let scope = FocusScopeNode::new();
    let text = "hello world";
    let controller = TextEditingController::new();
    controller.set_text(text);
    assert!(controller.selection_if_set().is_none());
    let child = editor(&controller, &focus).autofocus(true);
    let scope = FocusScope::new(child).node(scope).autofocus(true);
    let mut runtime = Runtime::new(scope.into()).unwrap();
    let (frame, _) = runtime.run_frame(viewport()).unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(controller.selection(), TextSelection::collapsed(text.len()));
    assert_eq!(caret_paints(&frame), 1);
}
