use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::{Axis, AxisDirection};
use incular_scroll::{
    SliverConstraints, SliverGeometry, SliverGeometryValidationError, SliverGrowthDirection,
    apply_growth_direction_to_axis_direction,
};

fn constraints() -> SliverConstraints {
    SliverConstraints::new(Axis::Vertical, false, 0., 0., 0., 0., 0., 0., 0., 0.)
}

#[test]
fn all_four_physical_directions_flip_only_when_growth_reverses() {
    use AxisDirection::*;
    use SliverGrowthDirection::*;
    for (given, opposite) in [(Up, Down), (Down, Up), (Right, Left), (Left, Right)] {
        assert_eq!(
            apply_growth_direction_to_axis_direction(given, Forward),
            given
        );
        assert_eq!(
            apply_growth_direction_to_axis_direction(given, Reverse),
            opposite
        );
        let c = constraints().with_axis_direction(given);
        assert_eq!(c.axis_direction(), given);
    }
}

#[test]
fn constraints_copies_hash_and_describe_equally_and_normalize_growth() {
    fn hash(c: &SliverConstraints) -> u64 {
        let mut s = DefaultHasher::new();
        c.hash(&mut s);
        s.finish()
    }
    use SliverGrowthDirection::*;
    let a = constraints();
    let copy = a;
    assert_eq!(a, copy);
    assert_eq!(hash(&a), hash(&copy));
    assert_eq!(format!("{a:?}"), format!("{copy:?}"));
    assert!(!format!("{a:?}").contains('\n'));
    assert_eq!(a.normalized_growth_direction(), Forward);

    let changed = SliverConstraints {
        growth_direction: Reverse,
        scroll_offset: 10.,
        overlap: 20.,
        remaining_paint_extent: 30.,
        cross_axis_extent: 40.,
        viewport_main_axis_extent: 30.,
        ..a.with_axis_direction(AxisDirection::Up)
    };
    let d = SliverConstraints {
        growth_direction: Reverse,
        ..SliverConstraints::new(Axis::Vertical, true, 10., 0., 20., 30., 40., 30., 0., 0.)
    };
    assert_eq!(changed, d);
    for (c, expected) in [
        (changed, Forward),
        (d, Forward),
        (d.with_axis_direction(AxisDirection::Right), Reverse),
        (d.with_axis_direction(AxisDirection::Left), Forward),
        (
            SliverConstraints {
                growth_direction: Forward,
                ..d
            },
            Reverse,
        ),
    ] {
        assert_eq!(c.normalized_growth_direction(), expected);
    }
    assert_eq!(hash(&a), hash(&SliverConstraints { overlap: -0., ..a }));
}

#[test]
fn sliver_geometry_reports_invalid_paint_bounds_without_altering_them() {
    use SliverGeometryValidationError::*;
    assert_eq!(SliverGeometry::default(), SliverGeometry::ZERO);
    assert_eq!(SliverGeometry::default().validate(), Ok(()));
    assert_eq!(
        SliverGeometry {
            layout_extent: 10.,
            paint_extent: 9.,
            ..SliverGeometry::ZERO
        }
        .validate(),
        Err(LayoutExceedsPaint {
            layout: 10.,
            paint: 9.
        })
    );
    assert_eq!(
        SliverGeometry {
            paint_extent: 9.,
            max_paint_extent: 8.,
            ..SliverGeometry::ZERO
        }
        .validate(),
        Err(MaxPaintTooSmall {
            maximum: 8.,
            paint: 9.
        })
    );
    assert_eq!(
        SliverGeometry {
            paint_extent: 0.5e-10,
            ..SliverGeometry::ZERO
        }
        .validate(),
        Ok(())
    );
    assert_eq!(
        SliverGeometry {
            scroll_offset_correction: Some(-0.),
            ..SliverGeometry::ZERO
        }
        .validate(),
        Err(ZeroCorrection)
    );
}
