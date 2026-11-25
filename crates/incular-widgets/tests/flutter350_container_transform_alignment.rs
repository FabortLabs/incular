use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_widgets::{
    AlignmentGeometry, Container, Directionality, Positioned, Stack, Widget,
    internal::{Key, WidgetTree},
};

fn bounds(tree: &WidgetTree, name: &str) -> Rect {
    let element = tree.element_with_key(&Key::from(name)).expect("element");
    tree.element_bounds(element).expect("bounds")
}

fn box_rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn scene(pivot: Option<AlignmentGeometry>, direction: TextDirection) -> Widget {
    let cyan = Widget::from(Container::new().color(Color::rgba(0, 255, 255, 255)))
        .with_key(Key::from("cyan"));
    let base = Container::new()
        .width(100.0)
        .height(100.0)
        .color(Color::rgba(0, 0, 255, 255));
    let overlay = Container::builder()
        .child(cyan)
        .width(100.0)
        .height(100.0)
        .transform(Transform::scale(0.5))
        .build();
    let overlay = match pivot {
        Some(pivot) => overlay.transform_alignment(pivot),
        None => overlay,
    };
    let stack = Stack::new([
        Positioned::new(base).left(100.0).top(100.0),
        Positioned::new(Widget::from(overlay).with_key(Key::from("overlay")))
            .left(100.0)
            .top(100.0),
    ]);
    Directionality::new(direction, stack).into()
}

fn finish_frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800.0, 600.0)))
        .expect("layout");
    let _ = tree.paint();
}

#[test]
fn container_transform_alignment_preserves_layout_and_moves_the_painted_child() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(
            Some(Alignment::CENTER_RIGHT.into()),
            TextDirection::Ltr,
        ))
        .expect("mount");
    finish_frame(&mut tree);
    assert_eq!(
        bounds(&tree, "overlay"),
        box_rect(100.0, 100.0, 100.0, 100.0)
    );
    assert_eq!(bounds(&tree, "cyan"), box_rect(150.0, 125.0, 50.0, 50.0));

    tree.update(root, scene(None, TextDirection::Ltr))
        .expect("update");
    finish_frame(&mut tree);
    assert_eq!(bounds(&tree, "cyan"), box_rect(100.0, 100.0, 50.0, 50.0));
}

#[test]
fn directional_pivot_updates_when_directionality_changes() {
    let pivot = AlignmentDirectional::CENTER_END.into();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(Some(pivot), TextDirection::Ltr))
        .expect("mount");
    finish_frame(&mut tree);
    assert_eq!(bounds(&tree, "cyan"), box_rect(150.0, 125.0, 50.0, 50.0));

    tree.update(root, scene(Some(pivot), TextDirection::Rtl))
        .expect("update");
    finish_frame(&mut tree);
    assert_eq!(bounds(&tree, "cyan"), box_rect(100.0, 125.0, 50.0, 50.0));
}
