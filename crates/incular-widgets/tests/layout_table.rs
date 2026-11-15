//! Table descriptor behavior tests.

use incular_widgets::{Table, Text, Widget};

#[test]
fn table_builder_defaults_match_explicit_default() {
    assert_eq!(Table::builder().build(), Table::default());
}

#[test]
fn table_builder_preserves_normalization_and_lowering() {
    let table = Table::builder()
        .columns(0)
        .column_spacing(-3.0)
        .row_spacing(-5.0)
        .children(vec![Text::new("a").into(), Text::new("b").into()])
        .build();
    let expected = Table::new(1, [Text::new("a"), Text::new("b")])
        .column_spacing(0.0)
        .row_spacing(0.0);
    assert_eq!(table, expected);
    assert_eq!(Widget::from(table).debug_type_name(), "Table");
}

#[test]
fn constructor_accepts_arbitrary_widget_descriptors() {
    let table = Table::new(2, [Text::new("cell")]);
    assert_eq!(Widget::from(table).debug_type_name(), "Table");
}

fn cell_bounds(
    tree: &incular_widgets::internal::WidgetTree,
    table: incular_widgets::internal::ElementId,
    index: usize,
) -> (f32, f32, f32, f32) {
    let child = tree.children(table).expect("children")[index];
    let bounds = tree.element_bounds(child).expect("bounds");
    (
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    )
}

fn cell(width: f32, height: f32) -> Widget {
    Widget::box_(
        incular_core::Size::new(width, height),
        incular_core::Color::WHITE,
    )
}

#[test]
fn default_flex_columns_fill_the_bounded_width() {
    // Flutter's default `FlexColumnWidth()` shares the remaining width
    // equally: two columns in an 800px table are 400px each.
    let mut tree = incular_widgets::internal::WidgetTree::new();
    let root = tree
        .mount(Table::new(2, [cell(10., 10.), cell(20., 10.)]).into())
        .expect("mount");
    tree.layout(incular_config::Constraints::loose(incular_core::Size::new(
        800., 200.,
    )))
    .expect("layout");
    assert_eq!(tree.element_bounds(root).expect("bounds").size.width, 800.);
    assert_eq!(cell_bounds(&tree, root, 1).0, 400.);
}

#[test]
fn fixed_column_leaves_the_rest_to_flex_columns() {
    let mut tree = incular_widgets::internal::WidgetTree::new();
    let root = tree
        .mount(
            Table::new(2, [cell(10., 10.), cell(20., 10.)])
                .column_width(0, incular_widgets::TableColumnWidth::Fixed(100.))
                .into(),
        )
        .expect("mount");
    tree.layout(incular_config::Constraints::loose(incular_core::Size::new(
        800., 200.,
    )))
    .expect("layout");
    assert_eq!(cell_bounds(&tree, root, 1).0, 100.);
}

#[test]
fn intrinsic_columns_size_to_content() {
    let mut tree = incular_widgets::internal::WidgetTree::new();
    let root = tree
        .mount(
            Table::new(2, [cell(10., 10.), cell(20., 10.)])
                .default_column_width(incular_widgets::TableColumnWidth::Intrinsic)
                .into(),
        )
        .expect("mount");
    tree.layout(incular_config::Constraints::loose(incular_core::Size::new(
        800., 200.,
    )))
    .expect("layout");
    assert_eq!(tree.element_bounds(root).expect("bounds").size.width, 30.);
    assert_eq!(cell_bounds(&tree, root, 1).0, 10.);
}

#[test]
fn middle_vertical_alignment_centers_short_cells_in_their_row() {
    let mut tree = incular_widgets::internal::WidgetTree::new();
    let root = tree
        .mount(
            Table::new(2, [cell(10., 10.), cell(10., 30.)])
                .default_vertical_alignment(incular_widgets::TableCellVerticalAlignment::Middle)
                .into(),
        )
        .expect("mount");
    tree.layout(incular_config::Constraints::loose(incular_core::Size::new(
        800., 200.,
    )))
    .expect("layout");
    assert_eq!(cell_bounds(&tree, root, 0).1, 10.);
}
