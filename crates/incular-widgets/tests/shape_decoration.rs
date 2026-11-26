//! Flutter's `ShapeDecoration` (`_ShapeDecorationPainter`) and
//! `ClipPath.shape` (`ShapeBorderClipper`).

use std::{cell::RefCell, rc::Rc};

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, PaintCommand, Path};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BoxShadow, ClipPath, ClipRect, DecoratedShapeBox, Directionality, ShapeBorder, ShapeDecoration,
    StadiumBorder, Widget,
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

#[test]
fn clip_path_shape_under_loose_constraints_uses_the_childs_size() {
    let mut tree = WidgetTree::new();
    tree.mount(ClipPath::shape(
        StadiumBorder::default(),
        Widget::box_(Size::new(100., 40.), RED),
    ))
    .unwrap();
    tree.layout(Constraints::loose(Size::new(300., 300.)))
        .unwrap();
    let list = tree.paint();
    let actual = list.commands().iter().find_map(|command| match command {
        PaintCommand::PushClipPath { path, .. } => path.bounds().map(rounded),
        _ => None,
    });
    assert_eq!(
        actual,
        Some(Rect::from_origin_size(Offset::ZERO, Size::new(100., 40.)))
    );
}

/// clip_test.dart:741, preserving the six original rebuild checkpoints.
#[test]
fn clip_path_shape_reclips_only_on_shape_or_inherited_direction_changes() {
    type Observations = Rc<RefCell<Vec<(Rect, Option<TextDirection>)>>>;
    struct TestBorder(Observations);

    impl ShapeBorder for TestBorder {
        fn outer_path(&self, rect: Rect) -> Path {
            self.outer_path_with_direction(rect, None)
        }
        fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
            self.0.borrow_mut().push((rect, direction));
            StadiumBorder::default().outer_path(rect)
        }
        fn inner_path(&self, rect: Rect) -> Path {
            StadiumBorder::default().inner_path(rect)
        }
        fn paint(&self, canvas: &mut Canvas, rect: Rect) {
            StadiumBorder::default().paint(canvas, rect);
        }
    }
    let observations = Observations::default();
    let shape: Rc<dyn ShapeBorder> = Rc::new(TestBorder(observations.clone()));
    let build = || {
        ClipPath::shape_with_options(
            shape.clone(),
            Clip::AntiAlias,
            Some(Widget::box_(Size::new(100., 100.), RED)),
        )
    };
    let original = build();
    let mut tree = WidgetTree::new();
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.));
    let bounds = Constraints::tight(rect.size);
    let mut retained_root = None;
    for (direction, widget, reclip) in [
        (None, original.clone(), true),
        (None, build(), false),
        (Some(TextDirection::Ltr), original.clone(), true),
        (Some(TextDirection::Rtl), original.clone(), true),
        (Some(TextDirection::Rtl), build(), false),
        (Some(TextDirection::Ltr), original, true),
    ] {
        let widget = match direction {
            Some(direction) => Directionality::new(direction, widget).into(),
            None => widget,
        };
        // Keep the root render object compatible while inserting Directionality.
        let scene = ClipRect::new(widget).into();
        if let Some(root) = retained_root {
            tree.update(root, scene).unwrap();
        } else {
            retained_root = Some(tree.mount(scene).unwrap());
        }
        tree.layout(bounds).unwrap();
        let _ = tree.paint();
        let calls = observations.take();
        assert_eq!(
            calls,
            if reclip {
                vec![(rect, direction)]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn shape_decoration_paints_its_image_between_fill_and_border() {
    // Flutter `_ShapeDecorationPainter.paint` order: the color, then the
    // image clipped to the shape, then the shape's border. The image has no
    // fit, so it scales down and centres: 40 x 20 at (30, 10) in the 100 x 40
    // child.
    let image =
        incular_image::ImageHandle::from_rgba8(40, 20, vec![255; 40 * 20 * 4]).expect("image");
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedShapeBox::new(
            ShapeDecoration::new(StadiumBorder::new(incular_widgets::BorderSide::solid(
                RED, 2.,
            )))
            .color(RED)
            .image(incular_widgets::DecorationImage::new(image)),
            Widget::box_(Size::new(100., 40.), Color::WHITE),
        )
        .into(),
    )
    .expect("mount");
    tree.layout(Constraints::loose(Size::new(300., 300.)))
        .expect("layout");
    let list = tree.paint();
    let position = |matches: &dyn Fn(&PaintCommand) -> bool| {
        list.commands()
            .iter()
            .position(matches)
            .unwrap_or_else(|| panic!("command missing from {:?}", list.commands()))
    };
    let fill = position(&|command| matches!(command, PaintCommand::FillPath { .. }));
    let image_at = position(&|command| matches!(command, PaintCommand::Image { .. }));
    let border = position(&|command| matches!(command, PaintCommand::StrokePath { .. }));
    assert!(
        fill < image_at && image_at < border,
        "fill {fill}, image {image_at}, border {border}"
    );
    let destination = list
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Image { destination, .. } => Some(*destination),
            _ => None,
        })
        .expect("image destination");
    assert_eq!(
        destination,
        Rect::from_origin_size(Offset::new(30., 10.), Size::new(40., 20.))
    );
}

#[test]
fn superellipse_clip_uses_actual_loose_child_and_inherited_direction() {
    use incular_rendering::FillRule;
    use incular_widgets::{
        BorderRadiusDirectional, ClipRSuperellipse, Directionality, Radius, TextDirection,
    };
    let build = |direction, clip| -> Widget {
        Directionality::new(
            direction,
            ClipRSuperellipse::new(0., Widget::box_(Size::new(60., 30.), RED))
                .border_radius(BorderRadiusDirectional::only(
                    Radius::elliptical(20., 14.),
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                ))
                .clip_behavior(clip),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(build(TextDirection::Ltr, Clip::AntiAlias))
        .unwrap();
    let loose = Constraints::loose(Size::new(300., 300.));
    for (direction, left_visible) in [(TextDirection::Ltr, false), (TextDirection::Rtl, true)] {
        if direction == TextDirection::Rtl {
            tree.update(root, build(direction, Clip::AntiAlias))
                .unwrap();
        }
        tree.layout(loose).unwrap();
        let list = tree.paint();
        let path = list
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushClipPath { path, .. } => Some(path),
                _ => None,
            })
            .expect("actual retained clip");
        assert_eq!(
            path.bounds().map(rounded),
            Some(Rect::from_origin_size(Offset::ZERO, Size::new(60., 30.)))
        );
        assert_eq!(
            path.contains(Offset::new(1., 1.), FillRule::NonZero),
            left_visible
        );
        assert_eq!(
            path.contains(Offset::new(59., 1.), FillRule::NonZero),
            !left_visible
        );
        assert!(tree.hit_test(Offset::new(1., 1.)).is_some());
        assert!(tree.hit_test(Offset::new(59., 1.)).is_some());
        assert!(tree.hit_test(Offset::new(61., 1.)).is_none());
    }
    tree.update(root, build(TextDirection::Rtl, Clip::None))
        .unwrap();
    tree.layout(loose).unwrap();
    assert!(
        !tree
            .paint()
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::PushClipPath { .. }))
    );
}

#[test]
fn zero_area_superellipse_clip_keeps_a_zero_layout_without_panicking() {
    use incular_widgets::ClipRSuperellipse;
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(ClipRSuperellipse::new(8., Widget::box_(Size::new(20., 20.), RED)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::ZERO);
    let _ = tree.paint();
}
