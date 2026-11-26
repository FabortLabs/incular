use incular_config::{AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::{Brush, Canvas, GradientStops, LinearGradient, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Border, BorderDirectional, BorderRadiusDirectional, BorderSide, BoxDecoration,
    BoxLinearGradient, BoxShadow, BoxShape, CircleBorder, DecoratedShapeBox, DecorationImage,
    Directionality, RoundedRectangleBorder, ShapeDecoration, Widget,
};

const SIZE: Size = Size {
    width: 200.,
    height: 140.,
};
const RED: Color = Color::rgba(255, 0, 0, 255);
const GREEN: Color = Color::rgba(0, 255, 0, 255);

fn compare(actual: &ShapeDecoration, expected: &ShapeDecoration) {
    let rect = Rect::from_origin_size(Offset::ZERO, SIZE);
    for dir in [Some(TextDirection::Ltr), Some(TextDirection::Rtl)] {
        assert_eq!(
            actual.get_clip_path(rect, dir),
            expected.get_clip_path(rect, dir)
        );
        assert_eq!(actual.shape.dimensions(dir), expected.shape.dimensions(dir));
        assert_eq!(
            actual.shape.inner_path_with_direction(rect, dir),
            expected.shape.inner_path_with_direction(rect, dir)
        );
        let (mut a, mut b) = (Canvas::default(), Canvas::default());
        actual.shape.paint_with_direction(&mut a, rect, dir);
        expected.shape.paint_with_direction(&mut b, rect, dir);
        assert_eq!(a.finish(), b.finish());
    }
    assert_eq!(actual.color, expected.color);
    assert_eq!(actual.gradient, expected.gradient);
    assert_eq!(actual.aligned_gradient, expected.aligned_gradient);
    assert_eq!(actual.shadows, expected.shadows);
    assert_eq!(actual.image, expected.image);
}

#[test]
fn box_to_shape_conversion_preserves_the_five_pinned_constructor_cases() {
    compare(
        &ShapeDecoration::new(Border::default()),
        &ShapeDecoration::new(Border::default()),
    );
    compare(
        &ShapeDecoration::from_box_decoration(&BoxDecoration::new().shape(BoxShape::Circle)),
        &ShapeDecoration::new(CircleBorder::default()),
    );
    let radii = BorderRadiusDirectional::circular(100.);
    compare(
        &ShapeDecoration::from_box_decoration(
            &BoxDecoration::new().directional_border_radius(radii),
        ),
        &ShapeDecoration::new(RoundedRectangleBorder::new(BorderSide::NONE, radii)),
    );
    let side = BorderSide::solid(GREEN, 1.);
    compare(
        &ShapeDecoration::from_box_decoration(
            &BoxDecoration::new()
                .shape(BoxShape::Circle)
                .border(Border::all(side)),
        ),
        &ShapeDecoration::new(CircleBorder::new(side)),
    );
    let border = Border::new(1., RED);
    compare(
        &ShapeDecoration::from_box_decoration(&BoxDecoration::new().border(border)),
        &ShapeDecoration::new(border),
    );
    let logical = BorderDirectional::only(
        BorderSide::NONE,
        BorderSide::default(),
        BorderSide::NONE,
        BorderSide::NONE,
    );
    compare(
        &ShapeDecoration::from_box_decoration(&BoxDecoration::new().directional_border(logical)),
        &ShapeDecoration::new(logical),
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "color and gradient are mutually exclusive")]
fn specifying_color_and_gradient_on_a_shape_is_rejected() {
    let gradient = LinearGradient::new(
        Offset::ZERO,
        Offset::new(20., 0.),
        GradientStops::even(&[RED, GREEN]),
    );
    let _ = ShapeDecoration::new(Border::default())
        .color(RED)
        .gradient(gradient);
}

#[test]
fn converted_shapes_keep_relative_shader_image_and_shadow_until_painting() {
    let image = DecorationImage::new(ImageHandle::from_rgba8(1, 1, vec![255; 4]).unwrap());
    let shadow = BoxShadow::new(Color::BLACK, Offset::new(3., 5.), 4., 2.);
    let shader = BoxLinearGradient::new(
        AlignmentDirectional::CENTER_START,
        AlignmentDirectional::CENTER_END,
        GradientStops::even(&[RED, GREEN]),
    );
    let box_value = BoxDecoration::new()
        .shape(BoxShape::Circle)
        .aligned_gradient(shader)
        .image(image.clone())
        .box_shadow([shadow]);
    let decoration = ShapeDecoration::from_box_decoration(&box_value);
    assert_eq!(decoration.image, Some(image));
    assert_eq!(decoration.shadows, [shadow]);
    assert!(decoration.gradient.is_none());
    let shape = DecoratedShapeBox::new(decoration, Widget::box_(SIZE, Color::TRANSPARENT));
    let mut tree = WidgetTree::new();
    let make = |dir| Directionality::new(dir, shape.clone()).into();
    let root = tree.mount(make(TextDirection::Ltr)).unwrap();
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        if direction == TextDirection::Rtl {
            tree.update(root, make(direction)).unwrap();
        }
        tree.layout(Constraints::tight(SIZE)).unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let list = tree.paint();
        let brush = list
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::FillPath {
                    brush: Brush::LinearGradient(value),
                    ..
                } => Some(value),
                _ => None,
            })
            .expect("box-relative shader paints the shape");
        assert_eq!(brush.start.y, 70.);
        assert_eq!(brush.end.y, 70.);
        let expected_x = if direction == TextDirection::Ltr {
            (0., 200.)
        } else {
            (200., 0.)
        };
        assert_eq!((brush.start.x, brush.end.x), expected_x);
    }
}
