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

#[test]
fn an_explicit_registrar_sees_one_container_not_three_paragraphs() {
    use incular_widgets::SelectionRegistrar;
    let registrar = SelectionRegistrar::new();
    let delegate = SelectionAreaController::new();
    let container =
        SelectionContainer::with_controller(delegate.clone(), paragraphs(&["a", "b", "c"]))
            .registrar(registrar.clone());
    let mut tree = WidgetTree::new();
    tree.mount(container.into()).unwrap();
    frame(&mut tree);
    assert_eq!(registrar.registered_child_count(), 1);
    assert_eq!(delegate.registered_child_count(), 3);
    assert!(delegate.selection_geometry().has_content);
    drop(tree);
    assert_eq!(registrar.registered_child_count(), 0);
}

#[test]
fn explicit_and_context_registrars_track_empty_nonempty_and_disabled_descendants() {
    use incular_widgets::{SelectionRegistrar, SelectionRegistrarScope};
    for explicit in [false, true] {
        let registrar = SelectionRegistrar::new();
        let delegate = SelectionAreaController::new();
        let build = |state| -> Widget {
            let child = match state {
                0 => paragraphs(&[]),
                1 => paragraphs(&["first"]),
                _ => SelectionContainer::disabled(paragraphs(&["first"])).into(),
            };
            let container = SelectionContainer::with_controller(delegate.clone(), child);
            if explicit {
                container.registrar(registrar.clone()).into()
            } else {
                SelectionRegistrarScope::new(registrar.clone(), container).into()
            }
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(build(0)).unwrap();
        frame(&mut tree);
        assert_eq!(registrar.registered_child_count(), 0);
        for (state, count) in [(1, 1), (2, 0), (1, 1), (0, 0)] {
            tree.update(root, build(state)).unwrap();
            frame(&mut tree);
            assert_eq!(registrar.registered_child_count(), count);
            assert_eq!(delegate.registered_child_count(), count);
        }
    }
}

#[test]
fn swapping_nested_delegates_keeps_the_external_registration() {
    use incular_widgets::SelectionRegistrar;
    let registrar = SelectionRegistrar::new();
    let delegate = SelectionAreaController::new();
    let first = SelectionAreaController::new();
    let second = SelectionAreaController::new();
    let make = |inner: SelectionAreaController| {
        SelectionContainer::with_controller(
            delegate.clone(),
            Widget::from(SelectionContainer::with_controller(
                inner,
                paragraphs(&["dummy"]),
            ))
            .with_key(3_u64),
        )
        .registrar(registrar.clone())
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(first.clone())).unwrap();
    frame(&mut tree);
    assert_eq!(registrar.registered_child_count(), 1);
    assert!(delegate.selection_geometry().has_content);
    assert_eq!(first.registered_child_count(), 1);
    tree.update(root, make(second.clone())).unwrap();
    frame(&mut tree);
    assert_eq!(registrar.registered_child_count(), 1);
    assert!(delegate.selection_geometry().has_content);
    assert_eq!(first.registered_child_count(), 0);
    assert_eq!(second.registered_child_count(), 1);
}

#[test]
fn an_explicit_registrar_wins_over_the_inherited_registrar_and_can_be_replaced() {
    use incular_widgets::{SelectionRegistrar, SelectionRegistrarScope};
    let inherited = SelectionRegistrar::new();
    let explicit = SelectionRegistrar::new();
    let delegate = SelectionAreaController::new();
    let build = |use_explicit| {
        let child = SelectionContainer::with_controller(delegate.clone(), paragraphs(&["label"]));
        let child = if use_explicit {
            child.registrar(explicit.clone())
        } else {
            child
        };
        SelectionRegistrarScope::new(inherited.clone(), child).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(true)).unwrap();
    frame(&mut tree);
    assert_eq!(explicit.registered_child_count(), 1);
    assert_eq!(inherited.registered_child_count(), 0);
    tree.update(root, build(false)).unwrap();
    frame(&mut tree);
    assert_eq!(explicit.registered_child_count(), 0);
    assert_eq!(inherited.registered_child_count(), 1);
}

#[test]
fn select_all_bridges_nested_containers_and_projects_their_local_selection() {
    let region = SelectionAreaController::new();
    let nested = SelectionAreaController::new();
    let label = |value| Widget::from(Text::new(value).text_direction(TextDirection::Ltr));
    let content = Column::new([
        label("first").with_key(11_u64),
        SelectionContainer::with_controller(nested.clone(), label("second").with_key(22_u64))
            .into(),
        SelectionContainer::disabled(label("ignored")).into(),
        label("third").with_key(33_u64),
    ]);
    let mut tree = WidgetTree::new();
    tree.mount(Widget::selection_area(region.clone(), content.into()))
        .unwrap();
    frame(&mut tree);
    assert_eq!(region.registered_child_count(), 3);
    let first = tree.element_with_key(&11_u64.into()).unwrap();
    assert!(tree.selectable_text_select_all(first));
    assert_eq!(region.selected_text(), "firstsecondthird");
    assert_eq!(nested.selected_text(), "second");

    let second = tree.element_with_key(&22_u64.into()).unwrap();
    let third = tree.element_with_key(&33_u64.into()).unwrap();
    assert!(tree.selectable_text_select(third, incular_text::TextSelection::collapsed(5)));
    assert!(tree.selectable_text_set_extent(second, 0));
    assert_eq!(region.selected_text(), "secondthird");
    assert_eq!(nested.selected_text(), "second");
    assert_eq!(
        nested.selection().range,
        Some(incular_widgets::SelectedContentRange::new(6, 0)),
    );
    assert!(tree.selectable_text_clear_selection(second));
    assert!(nested.selected_text().is_empty());
    assert!(region.selected_text().is_empty());
    assert!(nested.selection_geometry().has_content);
}

#[test]
fn a_child_delegate_formats_selected_paragraphs_before_the_outer_region_callback() {
    use incular_core::Offset;
    use incular_widgets::SelectionContainerDelegate;
    use std::{cell::RefCell, rc::Rc};

    let region = SelectionAreaController::new();
    let delegate =
        SelectionContainerDelegate::new().combine_selected_content_with(|parts| parts.join("\n"));
    let nested = delegate.controller();
    let observed = Rc::new(RefCell::new(None));
    region.set_selection_changed_listener({
        let observed = observed.clone();
        move |content| *observed.borrow_mut() = content
    });
    let label =
        |text, key| Widget::from(Text::new(text).text_direction(TextDirection::Ltr)).with_key(key);
    let child = SelectionContainer::with_delegate(
        delegate,
        Column::new([label("Hello World!", 1_u64), label("How are you!", 2_u64)]),
    );
    let mut tree = WidgetTree::new();
    tree.mount(Widget::selection_area(region.clone(), child.into()))
        .unwrap();
    frame(&mut tree);
    assert_eq!(region.registered_child_count(), 1);
    assert_eq!(nested.registered_child_count(), 2);

    let first = tree.element_with_key(&1_u64.into()).unwrap();
    let second = tree.element_with_key(&2_u64.into()).unwrap();
    let point = |id, byte| {
        let (transform, size) = tree.element_geometry(id).expect("mounted label");
        let layout = tree.text_layout(tree.render_id(id).unwrap()).unwrap();
        let x = layout
            .line_caret_positions(0)
            .iter()
            .find(|stop| stop.offset == byte)
            .expect("visible shaped caret")
            .x;
        transform.transform_point(Offset::new(x, size.height / 2.))
    };
    let start = point(first, 4);
    let finish = point(second, 10);
    assert!(tree.selectable_text_set_selection(first, start, false));
    assert!(observed.borrow().is_none());
    assert!(tree.selectable_text_set_selection(second, finish, true));
    let expected = "o World!\nHow are yo";
    assert_eq!(nested.selected_text(), expected);
    assert_eq!(region.selected_text(), expected);
    assert_eq!(
        observed
            .borrow()
            .as_ref()
            .map(|value| value.plain_text.as_str()),
        Some(expected)
    );
}

#[test]
fn formatters_compose_hierarchically_and_only_for_selected_children() {
    use incular_widgets::SelectionContainerDelegate;
    let inner =
        SelectionContainerDelegate::new().combine_selected_content_with(|parts| parts.join("\n"));
    let inner_controller = inner.controller();
    let outer =
        SelectionContainerDelegate::new().combine_selected_content_with(|parts| parts.join("|"));
    let outer_controller = outer.controller();
    let label = |text| Widget::from(Text::new(text).text_direction(TextDirection::Ltr));
    let widget = SelectionContainer::with_delegate(
        outer,
        Column::new([
            label("a").with_key(1_u64),
            SelectionContainer::with_delegate(inner, Column::new([label("b"), label("c")])).into(),
            label("d"),
        ]),
    );
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    frame(&mut tree);
    let first = tree.element_with_key(&1_u64.into()).unwrap();
    assert!(tree.selectable_text_select_all(first));
    assert_eq!(outer_controller.selected_text(), "a|b\nc|d");
    assert_eq!(inner_controller.selected_text(), "b\nc");
    assert!(tree.selectable_text_select(first, incular_text::TextSelection::new(0, 1)));
    assert_eq!(outer_controller.selected_text(), "a");
    assert!(inner_controller.selected_text().is_empty());
}
