//! Original Flutter 3.50.0 Table sizing and width-deficit tests.
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    ConstrainedBox, Directionality, SizedBox, Table, TableColumnWidth, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
const HOST: Size = Size {
    width: 800.,
    height: 600.,
};

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(HOST)).unwrap();
    let _ = tree.paint();
}
fn ltr(widget: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, widget).into()
}
fn size(tree: &WidgetTree, key: u64) -> Size {
    let id = tree
        .element_with_key(&Key::Value(key))
        .expect("mounted cell");
    tree.element_bounds(id).unwrap().size
}

#[test]
fn six_equal_flex_columns_layout_even_with_a_tiny_rounding_deficit() {
    let mut tree = WidgetTree::new();
    tree.mount(ltr(Table::new(6, (0..12).map(|_| SizedBox::square(16.)))))
        .unwrap();
    frame(&mut tree);
}
#[test]
fn unequal_flex_columns_layout_under_deficit_constraints() {
    let mut table = Table::new(7, (0..14).map(|_| SizedBox::square(1.)))
        .column_width(0, TableColumnWidth::Flex(1.));
    for col in 1..7 {
        table = table.column_width(col, TableColumnWidth::Flex(0.123));
    }
    let mut tree = WidgetTree::new();
    tree.mount(ConstrainedBox::new(Constraints::tight(Size::new(600., 800.)), ltr(table)).into())
        .unwrap();
    frame(&mut tree);
}

const NORMAL: [&str; 9] = ["AAA", "B", "C", "D", "E", "FFFFFF", "G", "H", "III"];
const WIDE: [&str; 9] = ["AAAAAA", "B", "C", "D", "EEE", "F", "G", "H", "III"];
const SMALL: [&str; 9] = ["A", "B", "C", "D", "EEE", "F", "G", "H", "III"];
fn texts(names: [&str; 9], width: TableColumnWidth) -> Widget {
    ltr(Table::new(
        3,
        names
            .into_iter()
            .enumerate()
            .map(|(i, value)| Widget::from(Text::new(value)).with_key(Key::Value(i as u64))),
    )
    .default_column_width(width))
}
fn check(tree: &WidgetTree, order: std::cmp::Ordering) {
    let a = size(tree, 0);
    let b = size(tree, 1);
    assert_eq!(a, size(tree, 3));
    assert_eq!(a, size(tree, 6));
    assert_eq!(a.width.partial_cmp(&b.width), Some(order));
    assert_eq!(a.height, b.height);
}
#[test]
fn widest_cell_sets_its_entire_intrinsic_column_width() {
    let mut tree = WidgetTree::new();
    tree.mount(texts(NORMAL, TableColumnWidth::Intrinsic))
        .unwrap();
    frame(&mut tree);
    check(&tree, std::cmp::Ordering::Greater);
}
#[test]
fn intrinsic_column_width_recomputes_when_the_widest_cell_shrinks() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(texts(WIDE, TableColumnWidth::Intrinsic))
        .unwrap();
    frame(&mut tree);
    tree.update(root, texts(SMALL, TableColumnWidth::Intrinsic))
        .unwrap();
    frame(&mut tree);
    check(&tree, std::cmp::Ordering::Less);
}
#[test]
fn switching_from_flex_to_intrinsic_width_recomputes_columns() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(texts(NORMAL, TableColumnWidth::Flex(1.)))
        .unwrap();
    frame(&mut tree);
    check(&tree, std::cmp::Ordering::Equal);
    tree.update(root, texts(NORMAL, TableColumnWidth::Intrinsic))
        .unwrap();
    frame(&mut tree);
    check(&tree, std::cmp::Ordering::Greater);
}
