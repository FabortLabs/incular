use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{Offset, Size, time::Instant};
use incular_runtime::Runtime;
use incular_text::{TextAffinity, TextEditingController, TextSelection, TextStyle};
use incular_widgets::{Align, EditableText, GlobalKey, Transform, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn scene() -> (Runtime, GlobalKey, TextEditingController) {
    let controller =
        TextEditingController::with_text("Line1\nLine2\nLine3\nLine4\nLine5\nLine6\nLine7\nLine8");
    controller.clear_selection();
    let key = GlobalKey::new();
    let editor = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_size(14.))
        .max_lines(Some(2))
        .min_lines(Some(2));
    let content = Widget::from(editor).with_key(key);
    let mut runtime =
        Runtime::new(Align::new(Alignment::CENTER, Transform::scale(0.5, content)).into()).unwrap();
    frame(&mut runtime);
    (runtime, key, controller)
}

#[test]
fn scaled_editor_hit_positions_include_the_internal_scroll_offset() {
    let (mut runtime, key, controller) = scene();
    let tree = runtime.tree();
    let id = tree.element_with_key(&key.into()).unwrap();
    let top_left = tree.element_bounds(id).unwrap().origin;
    assert_eq!(tree.text_field_position_at(id, top_left).unwrap().0, 0);
    assert!(controller.selection_if_set().is_none());

    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        18,
        TextAffinity::Downstream
    ));
    frame(&mut runtime);
    assert_eq!(
        runtime
            .tree()
            .text_field_position_at(id, top_left)
            .unwrap()
            .0,
        12
    );
    assert!(controller.selection_if_set().is_none());
}

#[test]
fn scaled_editor_pointer_selection_uses_global_coordinates_after_scroll() {
    let (mut runtime, key, controller) = scene();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    let top_left = runtime.tree().element_bounds(id).unwrap().origin;
    assert!(controller.selection_if_set().is_none());
    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        18,
        TextAffinity::Downstream
    ));
    frame(&mut runtime);

    let now = Instant::now();
    assert!(
        runtime
            .tree_mut()
            .text_field_set_caret(id, top_left, false, now)
    );
    assert!(runtime.tree_mut().text_field_set_caret(
        id,
        top_left + Offset::new(100., 0.),
        true,
        now
    ));
    frame(&mut runtime);
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection {
            base: 12,
            extent: 17
        })
    );
}

#[test]
fn scaled_editor_selects_whole_words_using_the_scrolled_global_points() {
    let (mut runtime, key, controller) = scene();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    let top_left = runtime.tree().element_bounds(id).unwrap().origin;
    assert!(controller.selection_if_set().is_none());
    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        18,
        TextAffinity::Downstream
    ));
    frame(&mut runtime);
    assert!(runtime.select_editable_words_in_range(id, top_left, top_left + Offset::new(100., 0.)));
    frame(&mut runtime);
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::new(12, 17))
    );
}

#[test]
fn scaled_editor_word_edge_uses_the_scrolled_line_and_preserves_upstream_affinity() {
    let (mut runtime, key, controller) = scene();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        18,
        TextAffinity::Downstream
    ));
    frame(&mut runtime);
    let tree = runtime.tree();
    let top_left = tree.element_bounds(id).unwrap().origin;
    let local = tree
        .editable_local_caret_rect(id, 13, TextAffinity::Downstream)
        .unwrap();
    let point = top_left
        + Offset::new(
            local.origin.x * 0.5,
            (local.origin.y + local.size.height * 0.5) * 0.5,
        );
    assert_eq!(tree.text_field_position_at(id, point).unwrap().0, 13);
    assert!(controller.selection_if_set().is_none());
    assert!(runtime.select_editable_word_edge_at(id, point));
    frame(&mut runtime);
    assert_eq!(
        controller.selection_if_set(),
        Some(TextSelection::collapsed(17))
    );
    assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
}
