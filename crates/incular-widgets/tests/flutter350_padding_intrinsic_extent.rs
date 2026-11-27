use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    IntrinsicHeight, IntrinsicWidth, MaxIntrinsicSize, Padding, SizedBox, Widget,
};

const PROXY: Key = Key::Value(1);
const PADDING: Key = Key::Value(2);
const LEAF: Key = Key::Value(3);

fn keyed(widget: impl Into<Widget>, key: Key) -> Widget {
    widget.into().with_key(key)
}

fn size(tree: &WidgetTree, key: &Key) -> Size {
    let element = tree.element_with_key(key).unwrap();
    tree.element_bounds(element).unwrap().size
}

fn layout_padded(axis: Axis, parent_size: Size, child_size: Size) {
    let child = keyed(SizedBox::builder().width(55.).height(110.).build(), LEAF);
    let measured = MaxIntrinsicSize::new(Size::new(100., 200.), child);
    let padded = keyed(Padding::symmetric(10., 15., measured), PADDING);
    let root = match axis {
        Axis::Horizontal => keyed(IntrinsicWidth::new(padded), PROXY),
        Axis::Vertical => keyed(IntrinsicHeight::new(padded), PROXY),
    };
    let mut tree = WidgetTree::new();
    tree.mount(root).unwrap();
    tree.layout(Constraints::at_most(800., 600.)).unwrap();
    assert_eq!(size(&tree, &PROXY), parent_size);
    assert_eq!(size(&tree, &PADDING), parent_size);
    assert_eq!(size(&tree, &LEAF), child_size);
}

#[test]
fn padding_adds_horizontal_extent_to_child_maximum_width() {
    layout_padded(
        Axis::Horizontal,
        Size::new(120., 140.),
        Size::new(100., 110.),
    );
}

#[test]
fn padding_adds_vertical_extent_to_child_maximum_height() {
    layout_padded(Axis::Vertical, Size::new(75., 230.), Size::new(55., 200.));
}
