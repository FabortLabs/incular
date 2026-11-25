//! Flutter 3.50 TableBorder construction, interpolation and paint order.
use incular_config::EdgeInsets;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, FillRule, PaintCommand};
use incular_widgets::{BorderRadius, BorderSide, BorderStyle, Radius, TableBorder};

const BLUE: Color = Color::rgba(0, 0, 255, 255);
const RED: Color = Color::rgba(255, 0, 0, 255);
const GREEN: Color = Color::rgba(0, 255, 0, 255);

#[test]
fn standard_constructors_and_dimensions_describe_all_six_sides() {
    let border = TableBorder {
        left: BorderSide::default(),
        right: BorderSide::solid(GREEN, 1.),
        vertical_inside: BorderSide::default(),
        ..TableBorder::default()
    };
    assert_eq!(border.top, BorderSide::NONE);
    assert_eq!(border.bottom, BorderSide::NONE);
    assert_eq!(border.horizontal_inside, BorderSide::NONE);
    assert_eq!(border.dimensions(), EdgeInsets::only(1., 0., 1., 0.));
    assert!(!border.is_uniform());
    let scaled = border.scale(2.);
    assert_eq!(scaled.right, BorderSide::solid(GREEN, 2.));
    assert_eq!(scaled.left, BorderSide::solid(Color::BLACK, 2.));
    assert_eq!(scaled.vertical_inside, scaled.left);

    let uniform = TableBorder::all(BorderSide::solid(BLUE, 2.));
    assert!(uniform.is_uniform());
    assert_eq!(uniform.dimensions(), EdgeInsets::all(2.));
    assert_eq!(
        uniform.scale(0.5),
        TableBorder::all(BorderSide::solid(BLUE, 1.))
    );
    let symmetric = TableBorder::symmetric(
        BorderSide::solid(Color::BLACK, 3.),
        BorderSide::solid(RED, 1.),
    );
    assert!(!symmetric.is_uniform());
    assert_eq!(symmetric.dimensions(), EdgeInsets::all(1.));
    assert_eq!(symmetric.vertical_inside.width, 3.);
    assert_eq!(symmetric.horizontal_inside.width, 3.);
    assert_eq!(
        symmetric.scale(0.),
        TableBorder::symmetric(
            BorderSide::NONE,
            BorderSide::new(RED, 0., BorderStyle::None)
        )
    );
}

#[test]
fn lerp_interpolates_each_side_and_supports_absent_endpoints() {
    let side = |width| BorderSide::solid(Color::rgba(0, 0, width as u8, 0), width);
    let a = TableBorder {
        top: side(1.),
        right: side(2.),
        bottom: side(3.),
        left: side(4.),
        horizontal_inside: side(5.),
        vertical_inside: side(6.),
        ..TableBorder::default()
    };
    let b = a.scale(2.);
    let c = a.scale(3.);
    assert_eq!(TableBorder::lerp(Some(a), Some(c), 0.5), Some(b));
    assert_eq!(TableBorder::lerp(Some(a), Some(b), 2.), Some(c));
    assert_eq!(TableBorder::lerp(Some(b), Some(c), -1.), Some(a));
    let fraction = TableBorder::lerp(Some(a), Some(c), 0.9195).unwrap();
    assert!(!fraction.is_uniform());
    assert_eq!(
        fraction.dimensions(),
        incular_core::Lerp::lerp(&a.dimensions(), &c.dimensions(), 0.9195)
    );
    assert_eq!(TableBorder::lerp(None, None, 0.5), None);
    assert_eq!(TableBorder::lerp(Some(a), Some(a), 0.5), Some(a));
    assert_eq!(TableBorder::lerp(Some(b), None, 0.5), Some(a));
    assert_eq!(TableBorder::lerp(None, Some(b), 0.5), Some(a));
}

#[test]
fn border_defaults_to_no_paint_then_draws_vertical_horizontal_and_outer_in_order() {
    let rect = Rect::from_origin_size(Offset::new(10., 20.), Size::new(80., 50.));
    let mut canvas = Canvas::default();
    TableBorder::default().paint(&mut canvas, rect, &[25.], &[40.]);
    assert!(canvas.finish().commands().is_empty());

    let border = TableBorder {
        top: BorderSide::solid(GREEN, 4.),
        horizontal_inside: BorderSide::solid(RED, 3.),
        vertical_inside: BorderSide::solid(BLUE, 2.),
        ..TableBorder::default()
    };
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, rect, &[25.], &[40.]);
    let list = canvas.finish();
    let [vertical, horizontal, outside] = list.commands() else {
        panic!("{list:?}");
    };
    assert!(
        matches!(vertical, PaintCommand::StrokePath { path, brush: Brush::Solid(BLUE), stroke }
        if stroke.width == 2. && path.bounds().is_some_and(|bounds| bounds.origin == Offset::new(50., 20.)))
    );
    assert!(
        matches!(horizontal, PaintCommand::StrokePath { path, brush: Brush::Solid(RED), stroke }
        if stroke.width == 3. && path.bounds().is_some_and(|bounds| bounds.origin == Offset::new(10., 45.)))
    );
    assert!(matches!(
        outside,
        PaintCommand::FillPath {
            brush: Brush::Solid(GREEN),
            ..
        }
    ));
}

#[test]
fn same_color_nonuniform_outer_sides_can_paint_a_rounded_ring() {
    let border = TableBorder {
        top: BorderSide::solid(GREEN, 2.),
        right: BorderSide::solid(GREEN, 5.),
        bottom: BorderSide::solid(GREEN, 3.),
        left: BorderSide::solid(GREEN, 4.),
        ..TableBorder::default()
    }
    .border_radius(BorderRadius::all(Radius::circular(8.)));
    assert_eq!(
        border.border_radius,
        BorderRadius::all(Radius::circular(8.))
    );
    let mut canvas = Canvas::default();
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(80., 50.));
    border.paint(&mut canvas, rect, &[], &[]);
    assert!(matches!(
        canvas.finish().commands(),
        [PaintCommand::FillPath {
            brush: Brush::Solid(GREEN),
            fill_rule: FillRule::EvenOdd,
            ..
        }]
    ));
}

#[test]
fn retained_table_adds_only_explicit_strokes_over_cells_and_clears_them_on_removal() {
    use incular_config::{Constraints, TextDirection};
    use incular_widgets::{Table, TableColumnWidth, Widget, internal::WidgetTree};

    const CELL: Color = Color::rgba(40, 40, 40, 255);
    let scene = |direction, border| {
        let mut table = Table::new(
            2,
            (0..4).map(|i| Widget::box_(Size::new(20., if i < 2 { 20. } else { 10. }), CELL)),
        )
        .column_width(0, TableColumnWidth::Fixed(60.))
        .column_width(1, TableColumnWidth::Fixed(40.))
        .text_direction(direction);
        if let Some(border) = border {
            table = table.border(border);
        }
        Widget::from(table)
    };
    let explicit = TableBorder {
        vertical_inside: BorderSide::solid(BLUE, 2.),
        horizontal_inside: BorderSide::solid(RED, 3.),
        top: BorderSide::solid(GREEN, 4.),
        ..TableBorder::default()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr, None)).unwrap();
    let original = tree.render_id(root);
    let child = tree.children(root).unwrap()[0];
    let paint = |tree: &mut WidgetTree| {
        tree.layout(Constraints::tight(Size::new(100., 80.)))
            .unwrap();
        tree.paint()
    };
    let default = paint(&mut tree);
    assert!(default.commands().iter().all(|command| {
        !matches!(
            command,
            PaintCommand::StrokePath { .. } | PaintCommand::FillPath { .. }
        )
    }));

    for (direction, divider) in [(TextDirection::Ltr, 60.), (TextDirection::Rtl, 40.)] {
        tree.update(root, scene(direction, Some(explicit))).unwrap();
        let list = paint(&mut tree);
        let last_cell = list
            .commands()
            .iter()
            .rposition(|op| matches!(op, PaintCommand::Rect { color: CELL, .. }))
            .unwrap();
        let first_line = list
            .commands()
            .iter()
            .position(|op| matches!(op, PaintCommand::StrokePath { .. }))
            .unwrap();
        assert!(
            last_cell < first_line,
            "table border must overlay cell paint"
        );
        let lines: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|op| match op {
                PaintCommand::StrokePath {
                    brush: Brush::Solid(color),
                    path,
                    ..
                } => Some((*color, path.bounds().unwrap())),
                _ => None,
            })
            .collect();
        assert_eq!(
            lines,
            [
                (
                    BLUE,
                    Rect::from_origin_size(Offset::new(divider, 0.), Size::new(0., 30.))
                ),
                (
                    RED,
                    Rect::from_origin_size(Offset::new(0., 20.), Size::new(100., 0.))
                ),
            ]
        );
        assert_eq!(tree.render_id(root), original);
        assert_eq!(tree.children(root).unwrap()[0], child);
    }

    tree.update(
        root,
        scene(TextDirection::Rtl, Some(TableBorder::default())),
    )
    .unwrap();
    let list = paint(&mut tree);
    assert!(list.commands().iter().all(|op| !matches!(
        op,
        PaintCommand::StrokePath { .. } | PaintCommand::FillPath { .. }
    )));
}
