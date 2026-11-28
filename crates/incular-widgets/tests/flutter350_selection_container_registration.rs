//! Flutter 3.50.0 mounted selection registry boundaries.
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{
    Column, SelectionAreaController, SelectionContainer, SizedBox, Text, Widget,
    internal::WidgetTree,
};

fn paragraphs(labels: &[&str]) -> Widget {
    Column::new(
        labels
            .iter()
            .map(|label| Text::new(*label).text_direction(TextDirection::Ltr)),
    )
    .into()
}

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(400., 300.)))
        .unwrap();
}

#[test]
fn one_nested_container_is_registered_for_its_three_selectable_paragraphs() {
    let outer = SelectionAreaController::new();
    let inner = SelectionAreaController::new();
    let mut tree = WidgetTree::new();
    tree.mount(
        SelectionContainer::with_controller(
            outer.clone(),
            SelectionContainer::with_controller(inner.clone(), paragraphs(&["a", "b", "c"])),
        )
        .into(),
    )
    .unwrap();
    frame(&mut tree);
    assert_eq!(outer.registered_child_count(), 1);
    assert_eq!(inner.registered_child_count(), 3);
    assert!(outer.selection_geometry().has_content);
    assert!(inner.selection_geometry().has_content);
    assert!(!outer.selection_geometry().has_selection());
}

#[test]
fn a_disabled_child_never_registers_itself_or_its_text() {
    let outer = SelectionAreaController::new();
    let disabled = SelectionContainer::disabled(paragraphs(&["a", "b", "c"]));
    let child = disabled.controller();
    let mut tree = WidgetTree::new();
    tree.mount(SelectionContainer::with_controller(outer.clone(), disabled).into())
        .unwrap();
    frame(&mut tree);
    assert_eq!(outer.registered_child_count(), 0);
    assert_eq!(child.registered_child_count(), 0);
    assert!(!outer.selection_geometry().has_content);

    // A disabled subtree still lays out and paints its text.
    assert!(!tree.paint().commands().is_empty());
}

#[test]
fn replacing_a_delegate_preserves_parent_registration_but_detaches_the_old_delegate() {
    let outer = SelectionAreaController::new();
    let old = SelectionAreaController::new();
    let new = SelectionAreaController::new();
    let child = |inner: SelectionAreaController| -> Widget {
        Widget::from(SelectionContainer::with_controller(
            inner,
            paragraphs(&["dummy"]),
        ))
        .with_key(47_u64)
    };
    let build = |inner| SelectionContainer::with_controller(outer.clone(), child(inner)).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(old.clone())).unwrap();
    frame(&mut tree);
    let retained = tree.element_with_key(&47_u64.into()).unwrap();
    assert_eq!(outer.registered_child_count(), 1);
    assert_eq!(old.registered_child_count(), 1);

    tree.update(root, build(new.clone())).unwrap();
    frame(&mut tree);
    assert_eq!(tree.element_with_key(&47_u64.into()), Some(retained));
    assert_eq!(outer.registered_child_count(), 1);
    assert!(outer.selection_geometry().has_content);
    assert_eq!(old.registered_child_count(), 0);
    assert_eq!(new.registered_child_count(), 1);
}

#[test]
fn nested_registration_reaches_the_outer_registrar_in_the_same_layout_pass() {
    let outer = SelectionAreaController::new();
    let middle = SelectionAreaController::new();
    let inner = SelectionAreaController::new();
    let build = |populated| {
        SelectionContainer::with_controller(
            outer.clone(),
            SelectionContainer::with_controller(
                middle.clone(),
                SelectionContainer::with_controller(
                    inner.clone(),
                    paragraphs(if populated { &["dummy"] } else { &[] }),
                ),
            ),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(false)).unwrap();
    frame(&mut tree);
    for (populated, count) in [(false, 0), (true, 1), (false, 0)] {
        tree.update(root, build(populated)).unwrap();
        frame(&mut tree);
        for controller in [&outer, &middle, &inner] {
            assert_eq!(controller.registered_child_count(), count);
            assert_eq!(controller.selection_geometry().has_content, populated);
        }
    }
}

#[test]
fn a_zero_area_container_can_mount_and_lay_out() {
    let container = SelectionContainer::new(SizedBox::shrink());
    let controller = container.controller();
    let mut tree = WidgetTree::new();
    tree.mount(container.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::ZERO));
    assert_eq!(controller.registered_child_count(), 0);
}
