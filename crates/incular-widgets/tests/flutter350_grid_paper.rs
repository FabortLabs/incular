use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{Center, GridPaper, SizedBox, Widget, internal::WidgetTree};

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn identical_grid_rebuild_keeps_the_published_paint_buffer() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(GridPaper::new().into()).unwrap();
    frame(&mut tree);
    let before = tree.paint();
    assert!(
        before
            .commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::StrokePath { .. }))
    );

    tree.update(root, GridPaper::new().into()).unwrap();
    frame(&mut tree);
    let after = tree.paint();
    assert!(before.shares_buffer(&after));

    tree.update(root, GridPaper::new().interval(50.).into())
        .unwrap();
    frame(&mut tree);
    let changed = tree.paint();
    assert!(!changed.shares_buffer(&after));
    assert!(changed.commands().len() > after.commands().len());
}

#[test]
fn zero_area_grid_does_not_crash_or_expand() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Center::new(SizedBox::shrink().child(GridPaper::new())).into())
        .unwrap();
    frame(&mut tree);
    let tight_box = tree.children(root).unwrap()[0];
    let grid = tree.children(tight_box).unwrap()[0];
    assert_eq!(tree.element_bounds(grid).unwrap().size, Size::ZERO);
    let _ = tree.paint();
}

fn check_grid(
    grid: GridPaper,
    size: Size,
    color: Color,
    x_widths: &[f32],
    y_widths: &[f32],
    step: f32,
) {
    let background = Color::BLACK;
    let mut tree = WidgetTree::new();
    let with_child = grid.child(Widget::box_(size, background));
    tree.mount(Center::new(with_child).into()).unwrap();
    frame(&mut tree);
    let paint = tree.paint();
    let mut saw_background = false;
    let mut actual = Vec::new();
    for command in paint.commands() {
        match command {
            PaintCommand::Rect { color, .. } if *color == background => saw_background = true,
            PaintCommand::StrokePath {
                path,
                brush: Brush::Solid(actual_color),
                stroke,
            } => {
                assert!(saw_background, "grid paints over its child");
                assert_eq!(*actual_color, color);
                actual.push((path.bounds().unwrap(), stroke.width));
            }
            _ => {}
        }
    }
    let expected: Vec<_> = x_widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            (
                Rect::from_origin_size(
                    Offset::new(index as f32 * step, 0.),
                    Size::new(0., size.height),
                ),
                *width,
            )
        })
        .chain(y_widths.iter().enumerate().map(|(index, width)| {
            (
                Rect::from_origin_size(
                    Offset::new(0., index as f32 * step),
                    Size::new(size.width, 0.),
                ),
                *width,
            )
        }))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn grid_lines_match_the_reference_stroke_classes_and_draw_order() {
    check_grid(
        GridPaper::new(),
        Size::new(100., 50.),
        Color::rgba(195, 232, 243, 127),
        &[1., 0.25, 0.5, 0.25, 0.5, 0.25, 0.5, 0.25, 0.5, 0.25, 1.],
        &[1., 0.25, 0.5, 0.25, 0.5, 0.25],
        10.,
    );

    let green = Color::rgba(17, 255, 32, 191);
    check_grid(
        GridPaper::builder()
            .interval(12.)
            .divisions(3)
            .subdivisions(2)
            .color(green)
            .build(),
        Size::new(12., 6.),
        green,
        &[1., 0.25, 0.25, 0.5, 0.25, 0.25, 1.],
        &[1., 0.25, 0.25, 0.5],
        2.,
    );
    check_grid(
        GridPaper::new()
            .interval(0.75)
            .divisions(3)
            .subdivisions(2)
            .color(green),
        Size::new(0.75, 0.375),
        green,
        &[1., 0.25, 0.25, 0.5, 0.25, 0.25, 1.],
        &[1., 0.25, 0.25, 0.5],
        0.125,
    );
}

#[test]
fn malformed_or_unreasonably_dense_grids_keep_their_child_visible() {
    for grid in [
        GridPaper::new().interval(0.),
        GridPaper::new().interval(f32::INFINITY),
        GridPaper::new().subdivisions(0),
        GridPaper::new().divisions(usize::MAX),
        GridPaper::new().interval(f32::MIN_POSITIVE),
    ] {
        let mut tree = WidgetTree::new();
        tree.mount(
            grid.child(Widget::box_(Size::new(20., 20.), Color::WHITE))
                .into(),
        )
        .unwrap();
        frame(&mut tree);
        let paint = tree.paint();
        assert!(paint.commands().iter().any(|command| matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));
        assert!(
            !paint
                .commands()
                .iter()
                .any(|command| matches!(command, PaintCommand::StrokePath { .. }))
        );
    }
}
