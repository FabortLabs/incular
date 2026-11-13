//! `Transform::transform_rect_bbox` answers axis-aligned transforms without
//! the general four-corner computation. Both must give the same box.

use incular_core::{Offset, Rect, Size, Transform};

/// The general computation, straight from Kurbo.
fn reference(world: Transform, rect: Rect) -> Rect {
    let mapped = world.to_kurbo().transform_rect_bbox(kurbo::Rect::new(
        f64::from(rect.origin.x),
        f64::from(rect.origin.y),
        f64::from(rect.origin.x + rect.size.width),
        f64::from(rect.origin.y + rect.size.height),
    ));
    Rect::from_origin_size(
        Offset::new(mapped.x0 as f32, mapped.y0 as f32),
        Size::new(
            (mapped.x1 - mapped.x0) as f32,
            (mapped.y1 - mapped.y0) as f32,
        ),
    )
}

fn rects() -> Vec<Rect> {
    let mut rects = Vec::new();
    for origin in [
        Offset::ZERO,
        Offset::new(12.5, -7.25),
        Offset::new(-1e6, 3e5),
        Offset::new(0.1, 0.7),
    ] {
        for size in [
            Size::ZERO,
            Size::new(100., 20.),
            Size::new(0.3, 1e7),
            // Inverted boxes still map to a normalized bounding box.
            Size {
                width: -40.,
                height: 15.,
            },
            Size {
                width: 8.,
                height: -3.5,
            },
        ] {
            rects.push(Rect::from_origin_size(origin, size));
        }
    }
    rects
}

#[test]
fn axis_aligned_transforms_match_the_general_computation() {
    let transforms = [
        Transform::IDENTITY,
        Transform::translation(Offset::new(17.25, -300.5)),
        Transform::translation(Offset::new(0.1, 1e6)),
        Transform::scale(2.5),
        Transform::scale_non_uniform(-1., 1.),
        Transform::scale_non_uniform(0.5, -3.),
        Transform::scale_non_uniform(0., 4.),
        Transform::translation(Offset::new(40., 8.)).then(Transform::scale(1.5)),
        Transform::scale_non_uniform(-2., 0.25).then(Transform::translation(Offset::new(-9., 3.))),
    ];
    for world in transforms {
        for rect in rects() {
            assert_eq!(
                world.transform_rect_bbox(rect),
                reference(world, rect),
                "{world:?} applied to {rect:?}"
            );
        }
    }
}

#[test]
fn rotated_and_skewed_transforms_keep_the_general_computation() {
    let transforms = [
        Transform::rotation(0.3),
        Transform::skew(0.5, 0.),
        Transform::translation(Offset::new(5., 5.)).then(Transform::rotation(1.2)),
    ];
    for world in transforms {
        for rect in rects() {
            assert_eq!(world.transform_rect_bbox(rect), reference(world, rect));
        }
    }
}
