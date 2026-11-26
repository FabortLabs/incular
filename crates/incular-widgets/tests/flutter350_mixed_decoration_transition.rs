use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{
    BoxDecoration, BoxShape, Decoration, RoundedRectangleBorder, ShapeDecoration,
};

#[test]
fn a_circular_box_transitions_continuously_to_a_rounded_shape() {
    let box_value = Decoration::from(
        BoxDecoration::new()
            .color(Color::BLACK)
            .shape(BoxShape::Circle),
    );
    let shape_value = Decoration::from(
        ShapeDecoration::new(RoundedRectangleBorder::default())
            .color(Color::rgba(200, 100, 0, 255)),
    );
    let size = Size::new(200., 100.);
    let rect = Rect::from_origin_size(Offset::ZERO, size);
    assert!(matches!(
        box_value.lerp(&shape_value, 0.),
        Decoration::Box(_)
    ));
    assert!(matches!(
        shape_value.lerp(&box_value, 1.),
        Decoration::Box(_)
    ));
    assert!(matches!(
        box_value.lerp(&shape_value, 1.),
        Decoration::Shape(_)
    ));
    for (t, inset, red, green) in [
        (0.25, 37.5, 50, 25),
        (0.5, 25., 100, 50),
        (0.75, 12.5, 150, 75),
    ] {
        for value in [
            box_value.lerp(&shape_value, t),
            shape_value.lerp(&box_value, 1. - t),
        ] {
            let Decoration::Shape(shape) = value else {
                panic!("an intermediate needs a ShapeDecoration")
            };
            let expected = Color::rgba(red, green, 0, 255);
            assert_eq!(shape.color, Some(expected));
            let clip = shape.get_clip_path(rect, None);
            assert_eq!(
                clip.bounds(),
                Some(Rect::from_origin_size(
                    Offset::new(inset, 0.),
                    Size::new(200. - 2. * inset, 100.)
                ))
            );
            let painted = shape.paint_fill(size);
            let [
                PaintCommand::FillPath {
                    path,
                    brush: Brush::Solid(color),
                    ..
                },
            ] = painted.commands()
            else {
                panic!("a fitted intermediate contour paints as a filled path")
            };
            assert_eq!(*color, expected);
            assert_eq!(path.as_ref(), &clip);
        }
    }
}
