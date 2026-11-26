//! Focused equivalents of the pinned RenderTable update and sizing cases.

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    GlobalKey, SizedBox, Table, TableColumnWidth, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const HOST: Size = Size { width: 800., height: 600. };

#[test]
fn an_empty_table_shrinks_under_loose_constraints() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Table::new(1, std::iter::empty::<Widget>()).text_direction(TextDirection::Ltr).into())
        .unwrap();
    tree.layout(Constraints::loose(HOST)).unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::ZERO);
    assert!(tree.children(root).unwrap().is_empty());
    assert!(tree.paint().commands().is_empty());
}

#[test]
fn six_flex_cells_share_a_tight_one_hundred_pixel_width() {
    let table = Table::new(6, (0..6).map(|_| SizedBox::shrink()))
        .text_direction(TextDirection::Ltr);
    let mut tree = WidgetTree::new();
    let root = tree.mount(table.into()).unwrap();
    tree.layout(Constraints::new(100., 100., 0., 600.)).unwrap();
    let children = tree.children(root).unwrap();
    assert_eq!(children.len(), 6);
    for &cell in children {
        let width = tree.element_bounds(cell).unwrap().size.width;
        assert!((width - 100. / 6.).abs() <= 0.000_1, "{width}");
    }
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::new(100., 0.));
}

#[test]
fn dropping_the_last_row_detaches_its_cell_immediately() {
    let key = Key::Value(24);
    let scene = |rows| {
        Table::new(5, (0..rows * 5).map(|slot| {
            if slot == 24 {
                Widget::from(SizedBox::square(10.)).with_key(key.clone())
            } else {
                SizedBox::shrink().into()
            }
        }))
        .text_direction(TextDirection::Ltr)
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(5)).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    let removed = tree.element_with_key(&key).unwrap();
    let render = tree.render_id(root);
    assert!(tree.element_exists(removed));

    tree.update(root, scene(4)).unwrap();
    assert!(!tree.element_exists(removed));
    assert!(tree.element_with_key(&key).is_none());
    assert_eq!(tree.children(root).unwrap().len(), 20);
    assert_eq!(tree.render_id(root), render);
}

#[test]
fn replacing_flat_cells_retains_moved_keys_and_keeps_a_two_by_three_grid() {
    let keys: [GlobalKey; 3] = std::array::from_fn(|_| GlobalKey::new());
    let scene = |replacement| {
        Table::new(3, (0..6).map(|slot| {
            let cell = Widget::from(SizedBox::square(10.));
            if slot % 2 == usize::from(replacement) {
                cell.with_key(keys[slot / 2])
            } else {
                cell
            }
        }))
        .text_direction(TextDirection::Ltr)
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    let initial = keys.map(|key| tree.element_with_key(&key.into()).unwrap());
    let render = tree.render_id(root);
    for _ in 0..2 {
        tree.update(root, scene(true)).unwrap();
        tree.layout(Constraints::tight(HOST)).unwrap();
        assert_eq!(tree.render_id(root), render);
        let children = tree.children(root).unwrap();
        assert_eq!(children.len(), 6);
        for (index, cell) in initial.into_iter().enumerate() {
            assert_eq!(children[index * 2 + 1], cell);
        }
        let bounds = |slot: usize| tree.element_bounds(children[slot]).unwrap();
        assert!(bounds(0).origin.x < bounds(1).origin.x && bounds(1).origin.x < bounds(2).origin.x);
        assert_eq!(bounds(0).origin.y, bounds(2).origin.y);
        assert!(bounds(3).origin.y > bounds(0).origin.y);
        assert_eq!(bounds(3).origin.y, bounds(5).origin.y);
    }
}

#[test]
fn uneven_rtl_flex_columns_stay_finite_and_fill_the_tight_width() {
    let mut table = Table::new(7, (0..7).map(|_| SizedBox::square(100.)))
        .text_direction(TextDirection::Rtl)
        .column_width(0, TableColumnWidth::Flex(1.));
    for column in 1..7 {
        table = table.column_width(column, TableColumnWidth::Flex(0.123));
    }
    let mut tree = WidgetTree::new();
    let root = tree.mount(table.into()).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, HOST);
    let children = tree.children(root).unwrap();
    assert_eq!(children.len(), 7);
    let base = 800. / (1. + 6. * 0.123);
    let mut right = 800.;
    for (column, &cell) in children.iter().enumerate() {
        let bounds = tree.element_bounds(cell).unwrap();
        let expected_width = if column == 0 { base } else { 0.123 * base };
        assert!((bounds.size.width - expected_width).abs() < 0.001, "{bounds:?}");
        assert_eq!(bounds.size.height, 100.);
        assert!((bounds.origin.x + bounds.size.width - right).abs() < 0.001);
        right = bounds.origin.x;
    }
    assert!(right.abs() < 0.001);
}

#[test]
fn an_explicit_table_border_paints_even_with_no_row_height() {
    use incular_core::{Color, Offset};
    use incular_rendering::PaintCommand;
    use incular_widgets::{BorderSide, TableBorder};

    let scene = |columns, cells, border: Option<TableBorder>| {
        let table = Table::new(columns, (0..cells).map(|_| SizedBox::shrink()))
            .text_direction(TextDirection::Rtl);
        Widget::from(if let Some(border) = border { table.border(border) } else { table })
    };
    let mut tree = WidgetTree::new();
    let explicit = TableBorder::all(BorderSide::default());
    let root = tree.mount(scene(1, 0, Some(explicit))).unwrap();
    let render = tree.render_id(root);
    for (columns, cells, paths) in [(1, 0, 4), (1, 1, 4), (1, 2, 5), (2, 2, 5), (2, 4, 6), (3, 6, 6)] {
        tree.update(root, scene(columns, cells, Some(explicit))).unwrap();
        tree.layout(Constraints::tight(HOST)).unwrap();
        let list = tree.paint();
        let produced = list
            .commands()
            .iter()
            .filter(|op| matches!(op, PaintCommand::StrokePath { .. } | PaintCommand::FillPath { .. }))
            .count();
        assert_eq!(produced, paths, "{columns} columns and {cells} cells");
        assert_eq!(tree.render_id(root), render);
        if cells != 0 {
            let black_at = |point| list.commands().iter().any(|op| match op {
                PaintCommand::FillPath { path, fill_rule, brush: incular_rendering::Brush::Solid(Color::BLACK) } =>
                    path.contains(point, *fill_rule),
                _ => false,
            });
            assert!(black_at(Offset::new(400., 0.5)), "the authored stroke reaches the first row");
            assert!(!black_at(Offset::new(400., 1.5)), "the table has no intrinsic row height");
        }
    }

    tree.update(root, scene(3, 6, None)).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    assert!(tree.paint().commands().iter().all(|op| {
        !matches!(op, PaintCommand::StrokePath { .. } | PaintCommand::FillPath { .. })
    }));
}

#[test]
fn a_rounded_table_border_keeps_both_dividers_without_an_extra_outer_line_at_zero_height() {
    use incular_core::{Color, Offset};
    use incular_rendering::{Brush, FillRule, PaintCommand};
    use incular_widgets::{BorderRadius, BorderSide, Radius, TableBorder};
    let border = TableBorder::all(BorderSide::default())
        .border_radius(BorderRadius::all(Radius::circular(8.)));
    let scene = |cells| {
        Table::new(2, (0..cells).map(|_| SizedBox::shrink()))
            .text_direction(TextDirection::Ltr)
            .border(border)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(0)).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    tree.update(root, scene(4)).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    let list = tree.paint();
    let paths: Vec<_> = list.commands().iter().filter(|op| {
        matches!(op, PaintCommand::StrokePath { .. } | PaintCommand::FillPath { .. })
    }).collect();
    let [
        PaintCommand::StrokePath { path: vertical, stroke: v, .. },
        PaintCommand::StrokePath { path: horizontal, stroke: h, .. },
        PaintCommand::FillPath { path: outer_ring, fill_rule: FillRule::EvenOdd, brush: Brush::Solid(Color::BLACK) },
    ] = paths.as_slice() else {
        panic!("expected two dividers and a degenerate rounded ring: {list:?}");
    };
    assert_eq!((v.width, h.width), (1., 1.));
    assert_eq!(vertical.bounds().unwrap().origin, Offset::new(400., 0.));
    assert_eq!(vertical.bounds().unwrap().size, Size::ZERO);
    assert_eq!(horizontal.bounds().unwrap().size, Size::new(800., 0.));
    for y in [-0.5, 0.5, 1.5] {
        assert!(!outer_ring.contains(Offset::new(300., y), FillRule::EvenOdd));
    }
}
