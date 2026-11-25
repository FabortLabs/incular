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

#[test]
fn table_column_policies_follow_the_width_budget() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_widgets::{TableColumnWidth, internal::WidgetTree};

    let measure = |constraints, natural: &[f32], overrides: &[(usize, TableColumnWidth)]| {
        let mut table = Table::new(natural.len(), natural.iter().map(|&w| cell(w, 10.)));
        for &(column, width) in overrides {
            table = table.column_width(column, width);
        }
        let mut tree = WidgetTree::new();
        let root = tree.mount(table.into()).unwrap();
        tree.layout(constraints).unwrap();
        let widths: Vec<_> = (0..natural.len())
            .map(|index| cell_bounds(&tree, root, index).2)
            .collect();
        widths
    };

    assert_eq!(
        measure(Constraints::loose(Size::new(100., 60.)), &[80., 80.], &[]),
        [50., 50.]
    );
    assert_eq!(
        measure(
            Constraints::loose(Size::new(160., 60.)),
            &[5., 125., 10.],
            &[
                (0, TableColumnWidth::Fixed(40.)),
                (2, TableColumnWidth::Flex(2.))
            ]
        ),
        [40., 40., 80.]
    );
    assert_eq!(
        measure(
            Constraints::new(100., 100., 0., 60.),
            &[80., 80.],
            &[
                (0, TableColumnWidth::Fixed(10.)),
                (1, TableColumnWidth::Fixed(20.))
            ]
        ),
        [45., 55.]
    );
    assert_eq!(
        measure(
            Constraints::new(0., f32::INFINITY, 0., 60.),
            &[80., 80.],
            &[(1, TableColumnWidth::Fraction(0.5))]
        ),
        [0., 0.]
    );
}

#[test]
fn flex_cells_do_not_build_for_a_discarded_loose_measurement() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_widgets::{LayoutBuilder, internal::WidgetTree};
    use std::{cell::RefCell, rc::Rc};

    let constraints_seen = Rc::new(RefCell::new(Vec::new()));
    let logged = constraints_seen.clone();
    let child = LayoutBuilder::new(move |_, constraints| {
        logged.borrow_mut().push(constraints);
        cell(125., 10.)
    });
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Table::new(2, [Widget::from(child), cell(80., 10.)]).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 60.)))
        .unwrap();
    assert_eq!(cell_bounds(&tree, root, 0).2, 50.);
    let observed = constraints_seen.borrow();
    assert_eq!(&*observed, &[Constraints::new(50., 50., 0., f32::INFINITY)]);
}

#[test]
fn intrinsic_height_stretches_each_row_from_its_own_tallest_cell() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_widgets::{TableCellVerticalAlignment, internal::WidgetTree};

    let table = Table::new(2, [100., 200., 200., 300.].map(|height| cell(16., height)))
        .default_vertical_alignment(TableCellVerticalAlignment::IntrinsicHeight);
    let mut tree = WidgetTree::new();
    let root = tree.mount(table.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let positions = [0, 1, 2, 3].map(|slot| {
        let (_, y, _, height) = cell_bounds(&tree, root, slot);
        (y, height)
    });
    assert_eq!(
        positions,
        [(0., 200.), (0., 200.), (200., 300.), (200., 300.)]
    );
}

#[test]
fn per_cell_alignment_overrides_default_and_fill_does_not_determine_row_height() {
    use incular_config::{Constraints, TextDirection};
    use incular_core::Size;
    use incular_widgets::{TableCell, TableCellVerticalAlignment as Align, internal::WidgetTree};

    let table = |alignment| {
        Widget::from(
            Table::new(
                3,
                [
                    TableCell::new(cell(20., 100.)).vertical_alignment(alignment),
                    TableCell::new(cell(20., 40.)).vertical_alignment(Align::Top),
                    TableCell::new(cell(20., 10.)).vertical_alignment(Align::Bottom),
                ],
            )
            .text_direction(TextDirection::Rtl)
            .default_vertical_alignment(Align::Middle),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(table(Align::Fill)).unwrap();
    let extent = Constraints::loose(Size::new(300., 200.));
    tree.layout(extent).unwrap();
    let retained = tree.children(root).unwrap()[0];
    assert_eq!(cell_bounds(&tree, root, 0), (200., 0., 100., 40.));
    assert_eq!(cell_bounds(&tree, root, 1), (100., 0., 100., 40.));
    assert_eq!(cell_bounds(&tree, root, 2), (0., 30., 100., 10.));

    tree.update(root, table(Align::IntrinsicHeight)).unwrap();
    tree.layout(extent).unwrap();
    assert_eq!(tree.children(root).unwrap()[0], retained);
    assert_eq!(cell_bounds(&tree, root, 0), (200., 0., 100., 100.));
    assert_eq!(cell_bounds(&tree, root, 2), (0., 90., 100., 10.));

    tree.update(
        root,
        Table::new(1, [cell(20., 100.)])
            .default_vertical_alignment(Align::Fill)
            .into(),
    )
    .unwrap();
    tree.layout(extent).unwrap();
    assert_eq!(cell_bounds(&tree, root, 0).3, 0.);
}

#[test]
fn rtl_intrinsic_height_does_not_stretch_rows_to_the_outer_box() {
    use incular_config::{Constraints, TextDirection};
    use incular_core::Size;
    use incular_widgets::{TableCellVerticalAlignment, internal::WidgetTree};

    let table = Table::new(2, [cell(100., 100.), cell(200., 200.)])
        .text_direction(TextDirection::Rtl)
        .default_vertical_alignment(TableCellVerticalAlignment::IntrinsicHeight);
    let mut tree = WidgetTree::new();
    let root = tree.mount(table.into()).unwrap();
    let outer = Size::new(300., 300.);
    tree.layout(Constraints::tight(outer)).unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, outer);
    assert_eq!(cell_bounds(&tree, root, 0), (150., 0., 150., 200.));
    assert_eq!(cell_bounds(&tree, root, 1), (0., 0., 150., 200.));
}
