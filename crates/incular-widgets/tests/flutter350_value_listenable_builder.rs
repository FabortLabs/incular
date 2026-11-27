use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Size, ValueNotifier};
use incular_widgets::internal::{ElementId, Key, WidgetTree};
use incular_widgets::{
    Center, Directionality, Placeholder, SizedBox, Text, ValueListenableBuilder, Widget,
};

type Values = ValueNotifier<Option<&'static str>>;
const PLACEHOLDER: Key = Key::Value(100);
const BUILDER: Key = Key::Value(101);

fn scene(values: &Values) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        Widget::from(ValueListenableBuilder::new(
            values.clone(),
            |_, value, _| match value {
                None => Widget::from(Placeholder::new()).with_key(PLACEHOLDER),
                Some(value) => Text::new(value).into(),
            },
        ))
        .with_key(BUILDER),
    )
    .into()
}

fn frame(tree: &mut WidgetTree, area: Size) {
    tree.layout(Constraints::tight(area)).unwrap();
    let _ = tree.paint();
    tree.update_semantics();
}

fn host(child: impl Into<Widget>) -> Widget {
    SizedBox::expand().child(child).into()
}

fn mount(values: &Values) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(host(scene(values))).unwrap();
    frame(&mut tree, Size::new(800., 600.));
    (tree, root)
}

fn label(tree: &WidgetTree) -> Option<&str> {
    let mut labels = tree
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref());
    let result = labels.next();
    assert!(labels.next().is_none());
    result
}

#[test]
fn optional_value_builds_the_placeholder() {
    let values = Values::new(None);
    let (tree, _) = mount(&values);
    assert!(tree.element_with_key(&PLACEHOLDER).is_some());
    assert_eq!(label(&tree), None);
}

#[test]
fn builder_reads_the_initial_value() {
    let (tree, _) = mount(&Values::new(Some("Bachman")));
    assert_eq!(label(&tree), Some("Bachman"));
    assert!(tree.element_with_key(&PLACEHOLDER).is_none());
}

#[test]
fn builder_replaces_its_text_when_the_value_changes() {
    let values = Values::new(None);
    let (mut tree, _) = mount(&values);
    assert!(values.set_value(Some("Gilfoyle")));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), Some("Gilfoyle"));
    assert!(values.set_value(Some("Dinesh")));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), Some("Dinesh"));
}

fn replace_listenable() -> (WidgetTree, Values, Values) {
    let original = Values::new(None);
    let (mut tree, root) = mount(&original);
    let builder = tree.element_with_key(&BUILDER).unwrap();
    original.set_value(Some("Gilfoyle"));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), Some("Gilfoyle"));
    let replacement = Values::new(Some("Hendricks"));
    tree.update(root, host(scene(&replacement))).unwrap();
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(tree.element_with_key(&BUILDER), Some(builder));
    assert_eq!(label(&tree), Some("Hendricks"));
    (tree, original, replacement)
}

#[test]
fn same_mounted_builder_can_switch_to_a_new_listenable() {
    let (mut tree, _, replacement) = replace_listenable();
    replacement.set_value(Some("Bachman"));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), Some("Bachman"));
}

#[test]
fn old_value_source_is_unsubscribed_and_cannot_change_the_widget() {
    let (mut tree, old, replacement) = replace_listenable();
    assert!(!old.has_listeners());
    assert!(replacement.has_listeners());
    let builds = tree.diagnostics().rebuilds;
    old.set_value(Some("Big Head"));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(tree.diagnostics().rebuilds, builds);
    assert_eq!(label(&tree), Some("Hendricks"));
}

#[test]
fn removing_the_builder_drops_its_value_subscription() {
    let values = Values::new(None);
    let (mut tree, root) = mount(&values);
    values.set_value(Some("Gilfoyle"));
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), Some("Gilfoyle"));
    assert!(values.has_listeners());
    tree.update(root, host(Placeholder::new())).unwrap();
    frame(&mut tree, Size::new(800., 600.));
    assert_eq!(label(&tree), None);
    assert!(!values.has_listeners());
}

#[test]
fn mounted_builder_measures_zero_in_a_zero_area_view() {
    let values = Values::new(None);
    let widget = Center::new(
        Widget::from(ValueListenableBuilder::new(values, |_, _, _| {
            Placeholder::new().into()
        }))
        .with_key(BUILDER),
    );
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    frame(&mut tree, Size::ZERO);
    let builder = tree.element_with_key(&BUILDER).unwrap();
    let render = tree.render_id(builder).unwrap();
    assert_eq!(tree.render_size(render), Some(Size::ZERO));
}

#[test]
fn static_child_is_reused_and_equal_values_do_not_rebuild() {
    let values = ValueNotifier::new(0u8);
    let times = Rc::new(Cell::new(0));
    let count = times.clone();
    let widget = Widget::from(
        ValueListenableBuilder::new(values.clone(), move |_, _, child| {
            count.set(count.get() + 1);
            child.unwrap()
        })
        .child(Widget::from(Placeholder::new()).with_key(PLACEHOLDER)),
    );
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    let area = Size::new(800., 600.);
    frame(&mut tree, area);
    let child = tree.element_with_key(&PLACEHOLDER).unwrap();
    assert!(!values.set_value(0));
    frame(&mut tree, area);
    assert_eq!(times.get(), 1);
    assert!(values.set_value(1));
    frame(&mut tree, area);
    assert_eq!(times.get(), 2);
    assert_eq!(tree.element_with_key(&PLACEHOLDER), Some(child));
}
