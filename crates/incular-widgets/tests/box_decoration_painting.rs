//! Flutter's `_BoxDecorationPainter` and `Border.paint` through `Container`
//! and `AnimatedContainer`: shadows, circles, borders whose sides differ,
//! and `foregroundDecoration`. Expected geometry is worked out from
//! `painting/box_decoration.dart` and `painting/box_border.dart`.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Lerp, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{
    AnimatedContainer, Border, BorderSide, BoxDecoration, BoxShadow, BoxShape, Container, Widget,
};

const BLACK: Color = Color::rgba(0, 0, 0, 255);
const ORANGE: Color = Color::rgba(200, 100, 0, 255);
const RED: Color = Color::rgba(255, 0, 0, 255);

#[derive(Debug, PartialEq)]
enum Painted {
    Fill(Rect, Color),
    Stroke(Rect, Color, f32),
    Blur(f32),
}

/// Fills, strokes and blurs in paint order, path bounds in root
/// coordinates.
fn painted(list: &DisplayList) -> Vec<Painted> {
    let mut transforms = vec![CoreTransform::IDENTITY];
    let mut found = Vec::new();
    for command in list.commands() {
        let current = *transforms.last().unwrap();
        match command {
            PaintCommand::PushTransform { transform } => transforms.push(current.then(*transform)),
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color } => {
                found.push(Painted::Fill(current.transform_rect_bbox(*rect), *color));
            }
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } => found.push(Painted::Fill(
                current.transform_rect_bbox(rrect.rect),
                *color,
            )),
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(color),
                ..
            } => found.push(Painted::Fill(
                current.transform_rect_bbox(path.bounds().expect("bounds")),
                *color,
            )),
            PaintCommand::StrokePath {
                path,
                brush: Brush::Solid(color),
                stroke,
            } => found.push(Painted::Stroke(
                current.transform_rect_bbox(path.bounds().expect("bounds")),
                *color,
                stroke.width,
            )),
            PaintCommand::PushBlur { blur, .. } => found.push(Painted::Blur(blur.sigma_x)),
            _ => {}
        }
    }
    found
}

fn paint(widget: impl Into<Widget>) -> Vec<Painted> {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    painted(&tree.paint())
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn assert_rect(actual: Rect, expected: Rect) {
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(
        close(actual.origin.x, expected.origin.x)
            && close(actual.origin.y, expected.origin.y)
            && close(actual.size.width, expected.size.width)
            && close(actual.size.height, expected.size.height),
        "expected {expected:?}, got {actual:?}"
    );
}

fn fills(painted: &[Painted]) -> Vec<(Rect, Color)> {
    painted
        .iter()
        .filter_map(|item| match item {
            Painted::Fill(rect, color) => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_container_paints_its_shadows_blurred_behind_the_background() {
    let shadow = BoxShadow::new(Color::rgba(0, 0, 0, 128), Offset::new(2., 3.), 4., 1.);
    let painted = paint(
        Container::new()
            .width(40.)
            .height(40.)
            .decoration(BoxDecoration::new().color(BLACK).box_shadow([shadow])),
    );
    // `BoxShadow.blurSigma`: 4 * 0.57735 + 0.5.
    let Painted::Blur(sigma) = painted[0] else {
        panic!("the shadow blurs first: {painted:?}");
    };
    assert!((sigma - 2.809_4).abs() < 1e-4, "sigma {sigma}");
    let fills = fills(&painted);
    assert_eq!(fills.len(), 2, "{painted:?}");
    // `rect.shift(offset).inflate(spreadRadius)`: (2, 3) less 1 each way.
    assert_rect(fills[0].0, rect(1., 2., 42., 42.));
    assert_eq!(fills[0].1, Color::rgba(0, 0, 0, 128));
    assert_rect(fills[1].0, rect(0., 0., 40., 40.));
    assert_eq!(fills[1].1, BLACK);
}

#[test]
fn a_circle_is_inscribed_in_the_shortest_side_with_its_border_inside() {
    let painted = paint(
        Container::new().width(60.).height(40.).decoration(
            BoxDecoration::new()
                .color(ORANGE)
                .shape(BoxShape::Circle)
                .border(Border::new(2., BLACK)),
        ),
    );
    // The opaque inside border pulls the background in by
    // `strokeInset / 2 = 1`: a circle of radius (38 / 2) about (30, 20).
    let fills = fills(&painted);
    assert_eq!(fills.len(), 1, "{painted:?}");
    assert_rect(fills[0].0, rect(11., 1., 38., 38.));
    assert_eq!(fills[0].1, ORANGE);
    // `_paintUniformBorderWithCircle`: radius (40 + strokeOffset) / 2 with
    // `strokeOffset = 2 * -1`, a 2-wide stroke centred on radius 19.
    let stroke = painted
        .iter()
        .find_map(|item| match item {
            Painted::Stroke(rect, color, width) => Some((*rect, *color, *width)),
            _ => None,
        })
        .expect("border stroke");
    assert_rect(stroke.0, rect(11., 1., 38., 38.));
    assert_eq!((stroke.1, stroke.2), (BLACK, 2.));
}

#[test]
fn a_border_with_one_side_paints_that_side_as_a_trapezoid() {
    let border = Border::only(
        BorderSide::solid(RED, 4.),
        BorderSide::NONE,
        BorderSide::NONE,
        BorderSide::NONE,
    );
    let child = Widget::box_(Size::new(10., 10.), Color::WHITE);
    let painted = paint(
        Container::new()
            .width(40.)
            .height(40.)
            .alignment(incular_config::Alignment::TOP_LEFT)
            .decoration(BoxDecoration::new().color(BLACK).border(border))
            .child(child),
    );
    let fills = fills(&painted);
    // `_adjustedRectOnOutlinedBorder`: the top comes in by 4 / 2.
    assert_rect(fills[0].0, rect(0., 2., 40., 38.));
    assert_eq!(fills[0].1, BLACK);
    // `paintBorder`'s top side: (0, 0), (40, 0), (40, 4), (0, 4).
    assert_rect(fills[1].0, rect(0., 0., 40., 4.));
    assert_eq!(fills[1].1, RED);
    // `decoration.padding` is the border's stroke insets: the child starts
    // below the 4-wide top side.
    assert_rect(fills[2].0, rect(0., 4., 10., 10.));
    assert_eq!(fills[2].1, Color::WHITE);
}

#[test]
fn a_foreground_decoration_paints_over_the_child() {
    let tint = Color::rgba(0, 0, 255, 128);
    let painted = paint(
        Container::new()
            .decoration(BoxDecoration::new().color(BLACK))
            .foreground_decoration(BoxDecoration::new().color(tint))
            .child(Widget::box_(Size::new(20., 20.), Color::WHITE)),
    );
    let colors: Vec<Color> = fills(&painted)
        .into_iter()
        .map(|(_, color)| color)
        .collect();
    assert_eq!(colors, vec![BLACK, Color::WHITE, tint]);
}

#[test]
fn box_decorations_lerp_like_flutter() {
    let shadow = BoxShadow::new(BLACK, Offset::new(4., 8.), 10., 2.);
    let a = BoxDecoration::new().box_shadow([shadow]);
    let b = BoxDecoration::new().color(Color::rgba(10, 20, 30, 200));
    let mid = a.lerp(&b, 0.25);
    // `Color.lerp(null, c, t)` scales the alpha: 200 * 0.25.
    assert_eq!(mid.color, Some(Color::rgba(10, 20, 30, 50)));
    // `BoxShadow.lerpList`: the extra shadow is `scale(1 - t)`, its color
    // unchanged.
    assert_eq!(
        mid.box_shadow,
        vec![BoxShadow::new(BLACK, Offset::new(3., 6.), 7.5, 1.5)]
    );
    // `BoxBorder.lerp(null, b, t)` is `b.scale(t)`.
    let bordered = BoxDecoration::new().border(Border::new(4., RED));
    let half = BoxDecoration::new().lerp(&bordered, 0.5);
    assert_eq!(half.border, Some(Border::new(2., RED)));
    // The ends are exact.
    assert_eq!(a.lerp(&b, 0.0), a);
    assert_eq!(a.lerp(&b, 1.0), b);
}

#[test]
fn animated_container_animates_its_foreground_decoration() {
    const DURATION: Duration = Duration::from_millis(200);
    let container = |color: Color| -> Widget {
        AnimatedContainer::new(DURATION)
            .foreground_decoration(BoxDecoration::new().color(color))
            .child(Widget::box_(Size::new(20., 20.), Color::WHITE))
            .into()
    };
    let mut tree = WidgetTree::new();
    let root: ElementId = tree.mount(container(BLACK)).expect("mount");
    let base = Instant::now();
    let frame = |tree: &mut WidgetTree, millis: u64| {
        tree.update_compositor(base + Duration::from_millis(millis))
            .expect("frame");
        tree.layout(Constraints::loose(Size::new(200., 200.)))
            .expect("layout");
    };
    frame(&mut tree, 0);
    frame(&mut tree, 0);
    tree.update(root, container(ORANGE)).expect("update");
    frame(&mut tree, 0);
    frame(&mut tree, 0);
    frame(&mut tree, 100);
    // Halfway: (0 + 200) / 2 and (0 + 100) / 2.
    let colors: Vec<Color> = fills(&painted(&tree.paint()))
        .into_iter()
        .map(|(_, color)| color)
        .collect();
    assert_eq!(colors, vec![Color::WHITE, Color::rgba(100, 50, 0, 255)]);
}
