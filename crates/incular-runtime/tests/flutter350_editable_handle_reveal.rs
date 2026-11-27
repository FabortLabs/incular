use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_text::{TextAffinity, TextEditingController};
use incular_widgets::{
    CustomScrollView, Directionality, EditableText, FocusNode, GlobalKey, ScrollController,
    SelectionHandleType, SizedBox, Sliver, SliverMainAxisGroup, SliverToBoxAdapter,
    TextSelectionHandleLayout, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn reveal_with_layout(
    layout: Option<TextSelectionHandleLayout>,
    controls: bool,
    expected_clearance: f32,
) {
    let scroll = ScrollController::new();
    let node = FocusNode::new();
    let key = GlobalKey::new();
    let mut editor = EditableText::new(TextEditingController::new())
        .focus_node(node.clone())
        .text_direction(TextDirection::Ltr);
    if controls {
        editor = editor.selection_handles_builder(|_, _| SizedBox::shrink().into());
    }
    if let Some(layout) = layout {
        editor = editor.collapsed_selection_handle_layout(layout);
    }
    let editor = SizedBox::new()
        .height(44.)
        .child(Widget::from(editor).with_key(key));
    let group = SliverMainAxisGroup::new(vec![
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(600.))) as Box<dyn Sliver>,
        Box::new(SliverToBoxAdapter::new(editor)),
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(500.))),
    ]);
    let scene =
        CustomScrollView::new(vec![Box::new(group) as Box<dyn Sliver>]).controller(scroll.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    assert_eq!(
        runtime.tree().element_bounds(id).unwrap().origin.y + scroll.offset(),
        600.
    );
    assert_eq!(scroll.offset(), 0.);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    let id = runtime.focused_element().unwrap();
    assert_eq!(
        runtime.tree().element_bounds(id).unwrap().origin.y + scroll.offset(),
        600.
    );
    let caret = runtime
        .tree()
        .editable_local_caret_rect(id, 0, TextAffinity::Downstream)
        .unwrap();
    let expected = caret.origin.y + caret.size.height + expected_clearance;
    assert_eq!(scroll.offset(), expected);
    frame(&mut runtime);
    let geometry = runtime
        .tree()
        .text_field_selection_handle_geometry(id)
        .unwrap();
    if let Some(state) = runtime.tree().text_field_selection_handles(id) {
        state.show(geometry);
        let tip = geometry.start.anchor;
        if layout.is_some_and(|layout| layout.size == Size::ZERO) {
            assert_eq!(
                state.hit_test(tip + Offset::new(0., 23.)),
                Some(SelectionHandleType::Collapsed)
            );
            assert_eq!(state.hit_test(tip + Offset::new(0., 25.)), None);
        }
    }
}

#[test]
fn grouped_sliver_reveals_the_minimum_interactive_handle_not_just_the_caret() {
    reveal_with_layout(
        Some(TextSelectionHandleLayout::new(Size::ZERO, Offset::ZERO)),
        true,
        24.,
    );
}

#[test]
fn grouped_sliver_preserves_plain_padding_and_gives_a_larger_handle_its_full_clearance() {
    reveal_with_layout(None, false, 20.);
    reveal_with_layout(None, true, 39.);
    reveal_with_layout(
        Some(TextSelectionHandleLayout::new(
            Size::new(50., 60.),
            Offset::new(25., 6.),
        )),
        true,
        54.,
    );
}
