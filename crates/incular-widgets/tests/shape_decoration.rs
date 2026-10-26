//! Flutter's `ShapeDecoration` (`_ShapeDecorationPainter`) and
//! `ClipPath.shape` (`ShapeBorderClipper`).

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BoxShadow, ClipPath, DecoratedShapeBox, ShapeDecoration, StadiumBorder, Widget,
};

const RED: Color = Color::rgba(0xF4, 0x43, 0x36, 0xFF);
const SHADOW: Color = Color::rgba(0x00, 0x00, 0x00, 0x42);

/// Arc bounds carry floating-point noise.
fn rounded(rect: Rect) -> Rect {
    let r = |v: f32| (v * 1e3).round() / 1e3 + 0.;
    Rect::from_origin_size(
        Offset::new(r(rect.origin.x), r(rect.origin.y)),
        Size::new(r(rect.size.width), r(rect.size.height)),
    )
}

fn fills(tree: &mut WidgetTree) -> Vec<(Color, Rect)> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(color),
                ..
            } => Some((*color, rounded(path.bounds().expect("bounds")))),
            _ => None,
        })
        .collect()
}

#[test]
fn the_decoration_fills_the_shape_at_the_childs_size() {
    let mut tree = WidgetTree::new();
    let shadow = BoxShadow {
        color: SHADOW,
        offset: Offset::new(2., 3.),
        spread_radius: 1.,
        ..BoxShadow::default()
    };
    tree.mount(
        DecoratedShapeBox::new(
            ShapeDecoration::new(StadiumBorder::default())
                .color(RED)
                .shadows([shadow]),
            Widget::box_(Size::new(100., 40.), Color::WHITE),
        )
        .into(),
    )
    .expect("mount");
    // Loose constraints: the decoration takes the child's 100 x 40.
    tree.layout(Constraints::loose(Size::new(300., 300.)))
        .expect("layout");
    assert_eq!(
        fills(&mut tree),
        vec![
            // The shadow's outline: shifted by (2, 3), inflated by 1.
            (
                SHADOW,
                Rect::from_origin_size(Offset::new(1., 2.), Size::new(102., 42.))
            ),
            (
                RED,
                Rect::from_origin_size(Offset::ZERO, Size::new(100., 40.))
            ),
        ]
    );
}

#[test]
fn clip_path_shape_clips_to_the_outer_path() {
    let mut tree = WidgetTree::new();
    tree.mount(ClipPath::shape(
        StadiumBorder::default(),
        Widget::box_(Size::new(100., 40.), RED),
    ))
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 40.)))
        .expect("layout");
    let clip = tree
        .paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushClipPath { path, .. } => Some(path.clone()),
            _ => None,
        })
        .expect("clip path");
    assert_eq!(
        clip.bounds().map(rounded),
        Some(Rect::from_origin_size(Offset::ZERO, Size::new(100., 40.)))
    );
}
