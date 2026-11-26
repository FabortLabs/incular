use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::{AnnotatedRegion, SizedBox, Transform, Widget, internal::WidgetTree};

fn render(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    tree
}

fn region() -> AnnotatedRegion<i32> {
    AnnotatedRegion::new(1, SizedBox::new().width(100.).height(100.))
}

#[test]
fn annotated_widget_publishes_its_typed_value_in_the_layer_tree() {
    let tree = render(region());
    let values: Vec<_> = tree
        .compositor_layers()
        .flattened_annotations()
        .iter()
        .filter_map(|entry| entry.annotation.cloned::<i32>())
        .collect();
    assert_eq!(values, [1]);
}

#[test]
fn the_layer_lookup_only_finds_the_widget_value_in_its_translated_region() {
    let tree = render(Transform::translation(Offset::new(25., 25.), region()));
    let layers = tree.compositor_layers();
    assert_eq!(layers.find_annotation::<i32>(Offset::new(10., 10.)), None);
    assert_eq!(
        layers.find_annotation::<i32>(Offset::new(50., 50.)),
        Some(1)
    );
    let entries = layers.find_annotation_entries::<i32>(Offset::new(50., 50.));
    assert_eq!(entries[0].local_position, Offset::new(25., 25.));
}
