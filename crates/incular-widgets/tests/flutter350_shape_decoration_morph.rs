use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{FillRule, PaintCommand};
use incular_widgets::{
    BorderSide, CircleBorder, Decoration, OvalBorder, RoundedRectangleBorder, ShapeDecoration,
};

fn shape(decoration: Decoration) -> ShapeDecoration {
    let Decoration::Shape(value) = decoration else {
        panic!("a shape-to-shape transition must stay a shape");
    };
    value
}

#[test]
fn animated_shape_decoration_hit_tests_follow_the_interpolated_outline() {
    let shapes = [
        ShapeDecoration::new(CircleBorder::default()),
        ShapeDecoration::new(RoundedRectangleBorder::default()),
        ShapeDecoration::new(OvalBorder::default()),
    ];
    let size = Size::new(200., 100.);
    let cross = |from: usize, to: usize, t: f32| {
        shape(Decoration::from(shapes[from].clone()).lerp(&shapes[to].clone().into(), t))
    };
    for (from, to) in [(0, 1), (0, 2), (1, 2)] {
        assert!(std::rc::Rc::ptr_eq(
            &cross(from, to, 0.).shape,
            &shapes[from].shape
        ));
        assert!(std::rc::Rc::ptr_eq(
            &cross(from, to, 1.).shape,
            &shapes[to].shape
        ));
    }
    for (from, t, point, hit) in [
        (0, 0.0, (20., 50.), false),
        (2, 0.0, (50., 5.), false),
        (2, 0.0, (5., 30.), false),
        (1, 0.0, (20., 50.), true),
    ] {
        let value = cross(from, 0, t);
        assert_eq!(
            value.hit_test(size, Offset::new(point.0, point.1), None),
            hit
        );
    }
    for (from, to, t, (x, y), hit) in [
        (0, 1, 0.1, (20., 50.), false),
        (0, 1, 0.5, (20., 50.), false),
        (0, 1, 0.9, (20., 50.), true),
        (0, 2, 0.1, (30., 50.), false),
        (0, 2, 0.5, (30., 50.), true),
        (0, 2, 0.9, (30., 50.), true),
        (1, 2, 0.1, (45., 10.), true),
        (1, 2, 0.5, (30., 10.), true),
        (1, 2, 0.9, (10., 30.), true),
    ] {
        let sample = Offset::new(x, y);
        for (from, to, t) in [(from, to, t), (to, from, 1. - t)] {
            let value = cross(from, to, t);
            assert_eq!(
                value.hit_test(size, sample, None),
                hit,
                "{from}->{to} at {t} ({x}, {y})"
            );
            assert_eq!(
                value
                    .get_clip_path(Rect::from_origin_size(Offset::ZERO, size), None)
                    .contains(sample, FillRule::NonZero),
                hit,
                "clip {from}->{to} at {t} ({x}, {y})"
            );
        }
    }
}

#[test]
fn circle_to_rectangle_paints_the_same_intermediate_contour_as_the_clip() {
    let a = Decoration::from(
        ShapeDecoration::new(CircleBorder::new(BorderSide::solid(Color::BLACK, 2.)))
            .color(Color::WHITE),
    );
    let b = Decoration::from(
        ShapeDecoration::new(RoundedRectangleBorder::default()).color(Color::WHITE),
    );
    let size = Size::new(200., 100.);
    let value = shape(a.lerp(&b, 0.5));
    let frame = Rect::from_origin_size(Offset::ZERO, size);
    let clip = value.get_clip_path(frame, None);
    let painted = value.paint_fill(size);
    let [PaintCommand::FillPath { path, .. }] = painted.commands() else {
        panic!(
            "expected a filled intermediate contour, got {:?}",
            painted.commands()
        );
    };
    assert_eq!(path.as_ref(), &clip);
    assert_eq!(
        clip.bounds(),
        Some(Rect::from_origin_size(
            Offset::new(25., 0.),
            Size::new(150., 100.)
        ))
    );
    assert_eq!(value.shape.dimensions(None).left, 1.);
}
