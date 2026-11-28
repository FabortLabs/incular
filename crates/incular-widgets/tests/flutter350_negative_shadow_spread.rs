use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_widgets::{
    Border, BorderRadius, BorderSide, BoxDecoration, BoxShadow, BoxShape, CircleBorder,
    DecoratedShapeBox, RoundedRectangleBorder, ShapeDecoration, SizedBox, internal::WidgetTree,
};

const BOX: Size = Size {
    width: 20.,
    height: 16.,
};
const INK: Color = Color::rgba(255, 0, 0, 255);

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn bounds(commands: &DisplayList) -> Vec<Rect> {
    commands
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, color } if *color == INK => Some(*rect),
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if *color == INK => Some(rrect.rect),
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(color),
                ..
            } if *color == INK => path.bounds(),
            _ => None,
        })
        .collect()
}

fn mounted(decoration: ShapeDecoration) -> DisplayList {
    let mut tree = WidgetTree::new();
    tree.mount(DecoratedShapeBox::new(decoration, SizedBox::expand()).into())
        .unwrap();
    tree.layout(Constraints::tight(BOX)).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree.paint()
}

fn shadow(spread: f32) -> BoxShadow {
    BoxShadow {
        color: INK,
        spread_radius: spread,
        ..Default::default()
    }
}

#[test]
fn box_shadows_sort_inverted_edges_and_omit_a_zero_extent() {
    for (spread, target, circle_target) in [
        (-7., Some(rect(7., 7., 6., 2.)), Some(rect(9., 7., 2., 2.))),
        (-9., Some(rect(9., 7., 2., 2.)), Some(rect(9., 7., 2., 2.))),
        (-8., None, None),
        (
            -15.,
            Some(rect(5., 1., 10., 14.)),
            Some(rect(5., 3., 10., 10.)),
        ),
    ] {
        let shadow = shadow(spread);
        let styles = [
            (BoxDecoration::new(), target),
            (BoxDecoration::new().shape(BoxShape::Circle), circle_target),
            (
                BoxDecoration::new().border_radius(BorderRadius::circular(2.)),
                target,
            ),
        ];
        for (decoration, expected) in styles {
            assert_eq!(
                bounds(&decoration.paint_shadow(BOX, &shadow)),
                expected.into_iter().collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn mounted_shape_shadows_sort_inverted_edges_before_painting_their_paths() {
    for (spread, target, circle_target) in [
        (-7., Some(rect(7., 7., 6., 2.)), Some(rect(9., 7., 2., 2.))),
        (-9., Some(rect(9., 7., 2., 2.)), Some(rect(9., 7., 2., 2.))),
        (-8., None, None),
        (
            -15.,
            Some(rect(5., 1., 10., 14.)),
            Some(rect(5., 3., 10., 10.)),
        ),
    ] {
        let shadow = shadow(spread);
        let styles = [
            (ShapeDecoration::new(Border::default()), target),
            (ShapeDecoration::new(CircleBorder::default()), circle_target),
            (
                ShapeDecoration::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::circular(2.),
                )),
                target,
            ),
        ];
        for (decoration, expected) in styles {
            assert_eq!(
                bounds(&mounted(decoration.shadows([shadow]))),
                expected.into_iter().collect::<Vec<_>>()
            );
        }
    }
    let shifted = BoxShadow {
        offset: Offset::new(3., -2.),
        ..shadow(-15.)
    };
    assert_eq!(
        ShapeDecoration::shadow_rect(rect(8., 8., 20., 16.), &shifted),
        rect(16., 7., 10., 14.)
    );
}
