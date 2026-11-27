use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size, Transform};
use incular_rendering::{Brush, FillRule, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Border, BorderDirectional, BorderSide, BoxDecoration, Container, Directionality, Widget,
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const GREEN: Color = Color::rgba(0, 255, 0, 255);

fn border() -> BorderDirectional {
    BorderDirectional::only(
        BorderSide::NONE,
        BorderSide::solid(RED, 6.),
        BorderSide::solid(BLUE, 2.),
        BorderSide::NONE,
    )
}

#[test]
fn mounted_box_uses_the_same_directional_edges_for_layout_and_paint() {
    let size = Size::new(40., 20.);
    let container = Container::new()
        .width(size.width)
        .height(size.height)
        .decoration(BoxDecoration::new().directional_border(border()))
        .child(Widget::box_(size, GREEN));
    let mut tree = WidgetTree::new();
    let direction = |direction| Directionality::new(direction, container.clone()).into();
    let root = tree.mount(direction(TextDirection::Ltr)).unwrap();
    for (ambient, start, end, child_x) in [
        (
            TextDirection::Ltr,
            Offset::new(1., 10.),
            Offset::new(39., 10.),
            6.,
        ),
        (
            TextDirection::Rtl,
            Offset::new(39., 10.),
            Offset::new(1., 10.),
            2.,
        ),
    ] {
        if ambient == TextDirection::Rtl {
            tree.update(root, direction(ambient)).unwrap();
        }
        tree.layout(Constraints::tight(size)).unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let list = tree.paint();
        let mut transforms = vec![Transform::IDENTITY];
        let mut child = None;
        let mut sides = 0;
        for command in list.commands() {
            let transform = *transforms.last().unwrap();
            match command {
                PaintCommand::PushTransform { transform: child } => {
                    transforms.push(transform.then(*child))
                }
                PaintCommand::PopTransform => {
                    transforms.pop();
                }
                PaintCommand::Rect { rect, color: GREEN } => {
                    child = Some(transform.transform_rect_bbox(*rect));
                }
                PaintCommand::RRect {
                    rrect,
                    brush: Brush::Solid(GREEN),
                    ..
                } => {
                    child = Some(transform.transform_rect_bbox(rrect.rect));
                }
                PaintCommand::FillPath {
                    path,
                    brush: Brush::Solid(color),
                    ..
                } if *color == RED || *color == BLUE => {
                    let (inside, outside) = if *color == RED {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    assert!(path.contains(inside, FillRule::NonZero));
                    assert!(!path.contains(outside, FillRule::NonZero));
                    sides += 1;
                }
                _ => {}
            }
        }
        assert_eq!(
            child,
            Some(Rect::from_origin_size(
                Offset::new(child_x, 0.),
                Size::new(32., 20.)
            ))
        );
        assert_eq!(sides, 2);
    }
}

#[test]
fn physical_to_directional_borders_switch_sides_only_after_they_fade_out() {
    let physical = BoxDecoration::new().border(Border {
        left: BorderSide::solid(GREEN, 8.),
        ..Border::default()
    });
    let directional = BoxDecoration::new().directional_border(border());
    let quarter = physical.lerp(&directional, 0.25);
    assert_eq!(quarter.border.unwrap().left.width, 4.);
    assert!(quarter.directional_border.is_none());
    let half = physical.lerp(&directional, 0.5);
    assert!(half.border.is_none());
    assert_eq!(half.directional_border.unwrap().start.width, 0.);
    let three_quarters = physical.lerp(&directional, 0.75);
    assert!(three_quarters.border.is_none());
    assert_eq!(
        three_quarters
            .directional_border
            .as_ref()
            .unwrap()
            .start
            .width,
        3.
    );
    assert_eq!(
        three_quarters
            .directional_border
            .as_ref()
            .unwrap()
            .end
            .width,
        1.
    );
    assert_eq!(three_quarters, directional.lerp(&physical, 0.25));
}

#[test]
fn a_decoration_preserves_directional_start_padding_and_interpolation() {
    use incular_config::{EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry};

    let decoration = |width| {
        BoxDecoration::new().directional_border(BorderDirectional::only(
            BorderSide::NONE,
            BorderSide::solid(Color::BLACK, width),
            BorderSide::NONE,
            BorderSide::NONE,
        ))
    };
    let base = decoration(2.);
    for (direction, sample, non_sample, expected_padding) in [
        (
            TextDirection::Rtl,
            49.,
            31.,
            EdgeInsets::only(0., 0., 2., 0.),
        ),
        (
            TextDirection::Ltr,
            31.,
            49.,
            EdgeInsets::only(2., 0., 0., 0.),
        ),
    ] {
        let list = base
            .paint_border_for_direction(Size::new(20., 20.), Some(direction))
            .unwrap();
        let origin = Offset::new(30., 0.);
        let mut paths = list.commands().iter().filter_map(|command| match command {
            PaintCommand::FillPath {
                path, fill_rule, ..
            } => Some((path, *fill_rule)),
            _ => None,
        });
        let (path, rule) = paths.next().unwrap();
        assert!(paths.next().is_none());
        assert!(path.contains(Offset::new(sample, 10.) - origin, rule));
        assert!(!path.contains(Offset::new(non_sample, 10.) - origin, rule));
        assert_eq!(base.padding().unwrap().resolve(direction), expected_padding);
    }

    assert_eq!(
        base.padding(),
        Some(EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(
            2., 0., 0., 0.
        )))
    );
    assert_eq!(base.scale(2.), decoration(4.));
    assert_eq!(base.lerp(&decoration(6.), 0.5), decoration(4.));
    assert_eq!(BoxDecoration::new().padding(), None);
}
