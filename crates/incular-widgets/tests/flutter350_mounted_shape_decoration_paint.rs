use std::{cell::RefCell, rc::Rc};

use incular_config::{AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_rendering::{Brush, Canvas, DisplayList, FillRule, GradientStops, PaintCommand, Path};
use incular_widgets::{
    AlignedShapeGradient, Border, BorderSide, BoxRadialGradient, DecoratedShapeBox,
    DecorationImage, Directionality, RoundedRectangleBorder, ShapeBorder, ShapeDecoration,
    SizedBox, Widget, internal::WidgetTree,
};

const SIZE: Size = Size {
    width: 800.,
    height: 600.,
};
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const RED: Color = Color::rgba(255, 0, 0, 255);

fn frame() -> Rect {
    Rect::from_origin_size(Offset::ZERO, SIZE)
}

fn mounted(decoration: ShapeDecoration, direction: Option<TextDirection>) -> DisplayList {
    let widget = DecoratedShapeBox::new(decoration, SizedBox::expand());
    let root: Widget = match direction {
        Some(direction) => Directionality::new(direction, widget).into(),
        None => widget.into(),
    };
    let mut tree = WidgetTree::new();
    tree.mount(root).unwrap();
    tree.layout(Constraints::tight(SIZE)).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree.paint()
}

fn compound() -> impl ShapeBorder {
    Border::new(1., Color::WHITE) + Border::new(1., Color::BLACK)
}

fn transparent_image() -> ImageHandle {
    ImageHandle::from_rgba8(1, 1, vec![0; 4]).unwrap()
}

#[test]
fn color_paints_first_then_a_compound_border_paints_outside_to_inside() {
    let display = mounted(ShapeDecoration::new(compound()).color(BLUE), None);
    let colors: Vec<_> = display
        .commands()
        .iter()
        .filter_map(|item| match item {
            PaintCommand::Rect { color, .. } => Some(*color),
            PaintCommand::Border { border, .. } => Some(border.color),
            PaintCommand::StrokePath {
                brush: Brush::Solid(color),
                ..
            } => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(colors, [BLUE, Color::BLACK, Color::WHITE]);
}

#[test]
fn image_draws_before_both_borders_and_is_clipped_inside_the_compound_stroke() {
    let image = transparent_image();
    let display = mounted(
        ShapeDecoration::new(compound()).image(DecorationImage::new(image.clone())),
        Some(TextDirection::Ltr),
    );
    let mut operations = vec![];
    let mut clip_depth = 0;
    for item in display.commands() {
        match item {
            PaintCommand::PushClipPath { path, .. } => {
                assert!(!path.contains(Offset::new(1.5, 20.), FillRule::NonZero));
                assert!(path.contains(Offset::new(2.5, 20.), FillRule::NonZero));
                clip_depth += 1;
            }
            PaintCommand::PopClip => clip_depth -= 1,
            PaintCommand::Image { image: drawn, .. } => {
                assert_eq!(drawn.id(), image.id());
                assert_eq!(clip_depth, 1);
                operations.push("image");
            }
            PaintCommand::Border { border, .. } => {
                assert_eq!(clip_depth, 0);
                operations.push(if border.color == Color::BLACK {
                    "black"
                } else {
                    "white"
                });
            }
            PaintCommand::StrokePath {
                brush: Brush::Solid(color),
                ..
            } => {
                assert_eq!(clip_depth, 0);
                operations.push(if *color == Color::BLACK {
                    "black"
                } else {
                    "white"
                });
            }
            _ => {}
        }
    }
    assert_eq!(clip_depth, 0);
    assert_eq!(operations, ["image", "black", "white"]);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Call {
    OuterPath,
    InnerPath,
    Paint,
}

type Calls = Rc<RefCell<Vec<(Call, Rect, Option<TextDirection>)>>>;

struct LoggingBorder(Calls);

impl ShapeBorder for LoggingBorder {
    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.0.borrow_mut().push((Call::OuterPath, rect, direction));
        Path::builder().build()
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.0.borrow_mut().push((Call::InnerPath, rect, direction));
        Path::builder().build()
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(&self, _: &mut Canvas, rect: Rect, direction: Option<TextDirection>) {
        self.0.borrow_mut().push((Call::Paint, rect, direction));
    }
}

#[test]
fn mounted_custom_border_requests_the_outer_path_for_color_and_inner_path_for_image() {
    for (direction, expected) in [
        (TextDirection::Ltr, Call::OuterPath),
        (TextDirection::Rtl, Call::InnerPath),
    ] {
        let calls = Calls::default();
        let decoration = ShapeDecoration::new(LoggingBorder(calls.clone()));
        let decoration = if expected == Call::OuterPath {
            decoration.color(RED)
        } else {
            decoration.image(DecorationImage::new(transparent_image()))
        };
        let _ = mounted(decoration, Some(direction));
        assert_eq!(
            calls.borrow().as_slice(),
            [
                (expected, frame(), Some(direction)),
                (Call::Paint, frame(), Some(direction)),
            ]
        );
    }
}

#[test]
fn directional_radial_gradient_resolves_the_radius_and_focal_point_inside_the_shape() {
    let stops = GradientStops::from_colors(&[RED, Color::BLACK], Some(&[0., 0.4])).unwrap();
    let gradient = BoxRadialGradient::new(stops.clone())
        .focal(AlignmentDirectional::BOTTOM_CENTER, 5.)
        .radius(2.);
    let border = RoundedRectangleBorder::new(
        BorderSide::NONE,
        incular_widgets::BorderRadius::circular(8.),
    );
    let display = mounted(
        ShapeDecoration::new(border)
            .aligned_gradient(AlignedShapeGradient::Radial(gradient.into())),
        Some(TextDirection::Rtl),
    );
    let shader = display
        .commands()
        .iter()
        .find_map(|item| match item {
            PaintCommand::FillPath {
                brush: Brush::RadialGradient(gradient),
                ..
            }
            | PaintCommand::RRect {
                brush: Brush::RadialGradient(gradient),
                ..
            } => Some(gradient),
            _ => None,
        })
        .expect("painted radial gradient");
    assert_eq!(shader.center, Offset::new(400., 300.));
    assert_eq!(shader.radius, 1200.);
    assert_eq!(shader.focal, Some(Offset::new(400., 600.)));
    assert_eq!(shader.focal_radius, 3000.);
    assert_eq!(shader.stops, stops);
}

#[test]
fn shape_padding_remains_directional_until_the_caller_supplies_direction() {
    use incular_config::{EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry};
    use incular_widgets::{BorderDirectional, CompoundBorder};

    let directional = BorderDirectional::only(
        BorderSide::NONE,
        BorderSide::solid(RED, 3.),
        BorderSide::NONE,
        BorderSide::NONE,
    );
    let padding = ShapeDecoration::new(directional).padding();
    assert!(matches!(padding, EdgeInsetsGeometry::Directional(insets) if
        insets == EdgeInsetsDirectional::only(3., 0., 0., 0.)));
    assert!(padding.try_resolve(None).is_err());
    assert_eq!(
        padding.resolve(TextDirection::Ltr),
        EdgeInsets::only(3., 0., 0., 0.)
    );
    assert_eq!(
        padding.resolve(TextDirection::Rtl),
        EdgeInsets::only(0., 0., 3., 0.)
    );

    let compound = ShapeDecoration::new(CompoundBorder::new(
        Border::new(1., Color::BLACK),
        directional,
    ))
    .padding();
    assert!(matches!(compound, EdgeInsetsGeometry::Mixed { .. }));
    assert_eq!(
        compound.resolve(TextDirection::Ltr),
        EdgeInsets::only(4., 1., 1., 1.)
    );
    assert_eq!(
        compound.resolve(TextDirection::Rtl),
        EdgeInsets::only(1., 1., 4., 1.)
    );
    assert_eq!(
        ShapeDecoration::new(Border::default()).padding(),
        EdgeInsetsGeometry::ZERO
    );
}

#[test]
fn separately_constructed_shape_decorations_have_value_equality_and_matching_hashes() {
    use incular_widgets::BoxShadow;
    use std::hash::{DefaultHasher, Hash, Hasher};

    let make = || {
        ShapeDecoration::new(Border::default())
            .color(Color::WHITE)
            .shadows([BoxShadow::default()])
    };
    let hash = |shape: &ShapeDecoration| {
        let mut hasher = DefaultHasher::new();
        shape.hash(&mut hasher);
        hasher.finish()
    };
    let a = make();
    let b = make();
    assert!(!Rc::ptr_eq(&a.shape, &b.shape));
    assert!(a == b);
    assert_eq!(hash(&a), hash(&b));
    assert!(
        a != ShapeDecoration::new(Border::new(1., Color::BLACK))
            .color(Color::WHITE)
            .shadows([BoxShadow::default()])
    );
    assert!(a != make().color(RED));

    let separately_compound = || {
        ShapeDecoration::new(incular_widgets::CompoundBorder::new(
            Border::new(1., Color::BLACK),
            RoundedRectangleBorder::new(
                BorderSide::NONE,
                incular_widgets::BorderRadius::circular(8.),
            ),
        ))
    };
    let (first, second) = (separately_compound(), separately_compound());
    assert!(first == second);
    assert_eq!(hash(&first), hash(&second));

    let custom = ShapeDecoration::new(LoggingBorder(Calls::default()));
    assert!(custom == custom.clone());
    assert!(custom != ShapeDecoration::new(LoggingBorder(Calls::default())));
}
