//! Flutter 3.50.0 Table direction and row shape observations.
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Center, Directionality, SizedBox, Table, TableColumnWidth, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::panic::AssertUnwindSafe;

const HOST: Size = Size {
    width: 800.,
    height: 600.,
};
fn compare_columns(direction: TextDirection) {
    let columns =
        (0u64..9).map(|i| Widget::from(Text::new(format!("cell {i}"))).with_key(Key::Value(i)));
    let table = Table::new(3, columns)
        .column_width(0, TableColumnWidth::Fixed(100.))
        .column_width(1, TableColumnWidth::Fixed(110.))
        .column_width(2, TableColumnWidth::Fixed(125.))
        .default_column_width(TableColumnWidth::Fixed(333.));
    let mut tree = WidgetTree::new();
    let wrapper = Center::new(Widget::from(table).with_key(Key::Value(90)));
    tree.mount(Directionality::new(direction, wrapper).into())
        .unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    let rect = |id| {
        tree.element_bounds(tree.element_with_key(&Key::Value(id)).unwrap())
            .unwrap()
    };
    let full = rect(90);
    assert_eq!(full.size.width, 335.);
    let widths = [100., 110., 125.];
    let expected = if direction == TextDirection::Ltr {
        [0., 100., 210.]
    } else {
        [235., 125., 0.]
    };
    let mut observed = Vec::new();
    for row in 0u64..3 {
        for (col, &width) in widths.iter().enumerate() {
            let cell = rect(3 * row + col as u64);
            assert_eq!(cell.size.width, width);
            observed.push(cell.origin.x - full.origin.x);
        }
    }
    assert_eq!(observed, expected.repeat(3));
}
#[test]
fn fixed_column_widths_and_offsets_ltr_match() {
    compare_columns(TextDirection::Ltr);
}
#[test]
fn rtl_starts_fixed_columns_at_the_right_edge() {
    compare_columns(TextDirection::Rtl);
}

#[test]
fn a_partly_filled_final_row_is_not_silently_accepted() {
    let scene = |count| {
        Directionality::new(
            TextDirection::Ltr,
            Table::new(2, (0..count).map(|_| SizedBox::square(20.))),
        )
        .into()
    };
    let mut control = WidgetTree::new();
    control.mount(scene(4)).unwrap();
    control.layout(Constraints::tight(HOST)).unwrap();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let mut tree = WidgetTree::new();
        tree.mount(scene(3))
            .and_then(|_| tree.layout(Constraints::tight(HOST)))
    }));
    assert!(
        !matches!(result, Ok(Ok(_))),
        "table accepted row lengths 2 and 1"
    );
}

#[test]
fn retained_columns_follow_ambient_direction_until_explicitly_overridden() {
    let make_table = || {
        Table::new(
            2,
            (0..2).map(|i| Widget::from(SizedBox::square(20.)).with_key(Key::Value(i))),
        )
        .column_width(0, TableColumnWidth::Fixed(56.))
        .column_width(1, TableColumnWidth::Fixed(33.))
    };
    let original = Widget::from(make_table()).with_key(Key::Value(90));
    let host = |direction, table| Directionality::new(direction, Center::new(table)).into();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(host(TextDirection::Ltr, original.clone()))
        .unwrap();
    let lefts = |tree: &WidgetTree| {
        let rect = |key| {
            tree.element_bounds(tree.element_with_key(&Key::Value(key)).unwrap())
                .unwrap()
        };
        [
            rect(0).origin.x - rect(90).origin.x,
            rect(1).origin.x - rect(90).origin.x,
        ]
    };
    tree.layout(Constraints::tight(HOST)).unwrap();
    let first = tree.element_with_key(&Key::Value(0));
    assert_eq!(lefts(&tree), [0., 56.]);

    tree.update(root, host(TextDirection::Rtl, original))
        .unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    assert_eq!(lefts(&tree), [33., 0.]);
    assert_eq!(tree.element_with_key(&Key::Value(0)), first);

    let explicit =
        Widget::from(make_table().text_direction(TextDirection::Ltr)).with_key(Key::Value(90));
    tree.update(root, host(TextDirection::Rtl, explicit))
        .unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    assert_eq!(lefts(&tree), [0., 56.]);
    assert_eq!(tree.element_with_key(&Key::Value(0)), first);
}
