use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{
    BorderSide, DecoratedShapeBox, Positioned, RelativeRect, ShapeBorder, ShapeDecoration, Stack,
    StarBorder, StarBorderError, StarBorderUpdate, Widget, internal::WidgetTree,
};

fn hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn star_and_polygon_constructors_keep_the_reference_defaults() {
    let star = StarBorder::new();
    assert_eq!(star, StarBorder::default());
    assert_eq!(star.side, BorderSide::NONE);
    assert_eq!(star.point_count(), 5.);
    assert_eq!(star.inner_ratio(), 0.4);
    assert_eq!(star.rotation_degrees(), 0.);
    assert_eq!(star.point_rounding_factor(), 0.);
    assert_eq!(star.valley_rounding_factor(), 0.);
    assert_eq!(star.squash_factor(), 0.);

    let polygon = StarBorder::polygon();
    assert_eq!(polygon.point_count(), 5.);
    assert_eq!(polygon.point_rounding_factor(), 0.);
    assert_eq!(polygon.rotation_degrees(), 0.);
    assert_eq!(polygon.squash_factor(), 0.);
}

#[test]
fn copy_with_updates_each_property_and_keeps_polygon_identity_distinct() {
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 255), 10.);
    let default = StarBorder::new();
    let copy = default.copy_with(StarBorderUpdate {
        side: Some(side),
        points: Some(3.),
        inner_radius_ratio: Some(0.1),
        point_rounding: Some(0.2),
        valley_rounding: Some(0.3),
        rotation_degrees: Some(180.),
        squash: Some(0.4),
    });
    let expected = StarBorder::new()
        .side(side)
        .points(3.)
        .inner_radius_ratio(0.1)
        .point_rounding(0.2)
        .valley_rounding(0.3)
        .rotation(180.)
        .squash(0.4);
    assert_eq!(default.copy_with(StarBorderUpdate::default()), default);
    assert_eq!(copy, expected);
    assert_eq!(hash(copy), hash(expected));
    assert_eq!(hash(copy), hash(side));

    let star = StarBorder::new()
        .points(3.)
        .inner_radius_ratio(1.)
        .point_rounding(0.2)
        .rotation(180.)
        .squash(0.4);
    let polygon = StarBorder::polygon()
        .points(3.)
        .point_rounding(0.2)
        .rotation(180.)
        .squash(0.4);
    assert_ne!(star, polygon);
    let materialized = polygon.copy_with(StarBorderUpdate::default());
    assert_ne!(polygon, materialized);
    assert_eq!(materialized.inner_ratio(), polygon.inner_ratio());
    assert_ne!(
        StarBorder::polygon()
            .points(2.)
            .copy_with(StarBorderUpdate::default()),
        StarBorder::polygon().points(2.)
    );

    let unchanged = StarBorderUpdate::default();
    for update in [
        StarBorderUpdate {
            side: Some(BorderSide::default()),
            ..unchanged
        },
        StarBorderUpdate {
            points: Some(10.),
            ..unchanged
        },
        StarBorderUpdate {
            inner_radius_ratio: Some(0.5),
            ..unchanged
        },
        StarBorderUpdate {
            point_rounding: Some(0.5),
            ..unchanged
        },
        StarBorderUpdate {
            valley_rounding: Some(0.5),
            ..unchanged
        },
        StarBorderUpdate {
            rotation_degrees: Some(10.),
            ..unchanged
        },
        StarBorderUpdate {
            squash: Some(0.),
            ..unchanged
        },
    ] {
        assert_ne!(copy, copy.copy_with(update), "{update:?}");
    }
}

#[test]
fn replacing_both_roundings_checks_the_final_combination() {
    let source = StarBorder::new().valley_rounding(0.8);
    let update = StarBorderUpdate {
        point_rounding: Some(0.8),
        valley_rounding: Some(0.2),
        ..Default::default()
    };
    let copy = source.copy_with(update);
    assert_eq!(copy.point_rounding_factor(), 0.8);
    assert_eq!(copy.valley_rounding_factor(), 0.2);
    assert_eq!(
        source.try_copy_with(StarBorderUpdate {
            valley_rounding: None,
            ..update
        }),
        Err(StarBorderError::RoundingSumExceedsOne)
    );
    assert_eq!(
        source.try_points(f32::INFINITY),
        Err(StarBorderError::TooFewPoints)
    );
}

#[test]
fn degenerate_positioned_decoration_never_produces_nonfinite_path_geometry() {
    let color = Color::rgba(0, 255, 0, 255);
    for (insets, expected_size) in [(100., 0.), (20., 60.)] {
        // The original uses 100 from every edge of a 100x100 Stack. Keep a
        // nonempty control run to prove the same decoration reaches painting.
        let mut tree = WidgetTree::new();
        let root = tree
            .mount(
                Stack::new([Positioned::from_relative_rect(
                    RelativeRect::from_ltrb(insets, insets, insets, insets),
                    DecoratedShapeBox::new(
                        ShapeDecoration::new(StarBorder::new()).color(color),
                        Widget::box_(Size::ZERO, Color::TRANSPARENT),
                    ),
                )])
                .into(),
            )
            .expect("mount positioned decoration");
        tree.layout(Constraints::tight(Size::new(100., 100.)))
            .expect("layout");
        let child = tree.children(root).expect("stack children")[0];
        assert_eq!(
            tree.element_bounds(child).expect("positioned bounds").size,
            Size::new(expected_size, expected_size)
        );
        let list = tree.paint();
        let paths: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::FillPath {
                    brush: Brush::Solid(actual),
                    path,
                    ..
                } if *actual == color => Some(path),
                _ => None,
            })
            .collect();
        for path in &paths {
            assert!(path.bez_path().is_finite());
        }
        if expected_size > 0. {
            assert!(
                paths.iter().any(|p| p.tight_bounds().is_some()),
                "positive control didn't paint"
            );
        } else {
            assert!(paths.iter().all(|p| p.bez_path().is_empty()), "{paths:?}");
            let direct =
                StarBorder::new().outer_path(Rect::from_origin_size(Offset::ZERO, Size::ZERO));
            assert!(direct.bez_path().is_empty());
        }
    }
}
