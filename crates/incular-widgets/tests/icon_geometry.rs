//! `Icon` maps its path into the icon square: by the path's own bounds by
//! default, or by an icon set's authoring canvas (`view_box`) so the set's
//! built-in padding survives. Outline icon sets paint with a stroke that
//! scales with the icon.

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{PaintCommand, Path, Stroke};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Icon, PathView, Widget};

/// A square authored on a 24px canvas with 3px of padding on every side.
fn padded_square() -> Path {
    let mut builder = Path::builder();
    builder
        .move_to(Offset::new(3., 3.))
        .line_to(Offset::new(21., 3.))
        .line_to(Offset::new(21., 21.))
        .line_to(Offset::new(3., 21.))
        .close();
    builder.build()
}

fn painted(widget: Widget) -> Vec<PaintCommand> {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    tree.paint().commands().to_vec()
}

fn assert_rect(got: Rect, want: Rect) {
    assert!(
        (got.origin.x - want.origin.x).abs() < 0.01
            && (got.origin.y - want.origin.y).abs() < 0.01
            && (got.size.width - want.size.width).abs() < 0.01
            && (got.size.height - want.size.height).abs() < 0.01,
        "{got:?} != {want:?}"
    );
}

#[test]
fn icon_without_a_view_box_fits_the_path_bounds() {
    let commands = painted(Icon::new(padded_square()).size(12.).into());
    let fill = commands
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath { path, .. } => path.bounds(),
            _ => None,
        })
        .expect("filled icon");
    assert_rect(
        fill,
        Rect::from_origin_size(Offset::ZERO, Size::new(12., 12.)),
    );
}

#[test]
fn icon_view_box_keeps_the_icon_sets_padding() {
    let commands = painted(
        Icon::new(padded_square())
            .size(12.)
            .view_box(Rect::from_origin_size(Offset::ZERO, Size::new(24., 24.)))
            .into(),
    );
    let fill = commands
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath { path, .. } => path.bounds(),
            _ => None,
        })
        .expect("filled icon");
    assert_rect(
        fill,
        Rect::from_origin_size(Offset::new(1.5, 1.5), Size::new(9., 9.)),
    );
}

#[test]
fn stroked_icon_scales_its_outline_and_does_not_fill() {
    let commands = painted(
        Icon::new(padded_square())
            .size(48.)
            .brush(Color::WHITE)
            .view_box(Rect::from_origin_size(Offset::ZERO, Size::new(24., 24.)))
            .stroke(Stroke {
                width: 2.,
                ..Stroke::default()
            })
            .into(),
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, PaintCommand::FillPath { .. })),
        "an outline icon is not filled"
    );
    let (bounds, width) = commands
        .iter()
        .find_map(|command| match command {
            PaintCommand::StrokePath { path, stroke, .. } => {
                Some((path.bounds().expect("bounds"), stroke.width))
            }
            _ => None,
        })
        .expect("stroked icon");
    assert_rect(
        bounds,
        Rect::from_origin_size(Offset::new(6., 6.), Size::new(36., 36.)),
    );
    assert!((width - 4.).abs() < 0.001, "stroke width {width}");
}

#[test]
fn path_view_is_public_and_paints_fill_and_stroke() {
    let commands = painted(
        PathView::new(padded_square())
            .fill(Color::BLACK)
            .stroke(Color::WHITE, Stroke::default())
            .size(Size::new(24., 24.))
            .into(),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, PaintCommand::FillPath { .. }))
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, PaintCommand::StrokePath { .. }))
    );
}
