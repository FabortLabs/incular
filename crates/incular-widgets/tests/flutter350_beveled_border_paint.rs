//! Strict BeveledRectangleBorder geometry and stroke operation.
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand};
use incular_widgets::{BeveledRectangleBorder, BorderRadius, BorderSide, ShapeBorder};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

#[test]
fn a_beveled_border_strokes_both_outer_and_inner_contours() {
    let side = BorderSide::solid(Color::BLACK, 4.).copy_with(None, None, None, Some(0.));
    let border = BeveledRectangleBorder::new(side, BorderRadius::circular(10.));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, rect(0., 0., 100., 80.));
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, stroke, .. }] = list.commands() else {
        panic!("expected one stroked path, got {:?}", list.commands());
    };
    assert_eq!(stroke.width, 4.);
    let contours = path
        .bez_path()
        .elements()
        .iter()
        .filter(|element| element.end_point().is_none())
        .count();
    assert_eq!(contours, 2);
    assert_eq!(path.tight_bounds(), Some(rect(-2., -2., 104., 84.)));
}

#[test]
fn a_zero_width_rect_does_not_emit_a_beveled_border() {
    let border =
        BeveledRectangleBorder::new(BorderSide::solid(Color::BLACK, 4.), BorderRadius::ZERO);
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, rect(0., 0., 0., 80.));
    assert!(canvas.finish().commands().is_empty());
}

#[test]
fn a_stroke_thicker_than_the_box_can_still_emit_its_contours() {
    let border =
        BeveledRectangleBorder::new(BorderSide::solid(Color::BLACK, 10.), BorderRadius::ZERO);
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, rect(0., 0., 2., 3.));
    assert!(matches!(
        canvas.finish().commands(),
        [PaintCommand::StrokePath { .. }]
    ));
}

#[test]
fn zero_radius_inside_and_outside_paths_include_rectangular_boundaries() {
    let area = rect(10., 20., 20., 20.);
    let plain = BeveledRectangleBorder::default();
    let bordered =
        BeveledRectangleBorder::new(BorderSide::solid(Color::BLACK, 4.), BorderRadius::ZERO);
    for path in [
        plain.outer_path(area),
        plain.inner_path(area),
        bordered.outer_path(area),
    ] {
        for p in [Offset::new(10., 20.), Offset::new(20., 30.)] {
            assert!(path.contains(p, FillRule::NonZero));
        }
        for p in [Offset::new(9., 19.), Offset::new(31., 41.)] {
            assert!(!path.contains(p, FillRule::NonZero));
        }
    }
    let inner = bordered.inner_path(area);
    for p in [Offset::new(14., 24.), Offset::new(16., 26.)] {
        assert!(inner.contains(p, FillRule::NonZero));
    }
    for p in [Offset::new(9., 23.), Offset::new(27., 37.)] {
        assert!(!inner.contains(p, FillRule::NonZero));
    }
}

fn assert_samples(path: &incular_rendering::Path, include: &[(f32, f32)], exclude: &[(f32, f32)]) {
    for &(x, y) in include {
        assert!(
            path.contains(Offset::new(x, y), FillRule::NonZero),
            "missing ({x},{y})"
        );
    }
    for &(x, y) in exclude {
        assert!(
            !path.contains(Offset::new(x, y), FillRule::NonZero),
            "covered ({x},{y})"
        );
    }
}

#[test]
fn positive_physical_and_directional_radii_cut_the_correct_corners() {
    use incular_config::TextDirection;
    use incular_widgets::{BorderRadiusDirectional, Radius};
    let target = rect(10., 20., 20., 20.);
    let plain = BeveledRectangleBorder::new(BorderSide::NONE, BorderRadius::circular(5.));
    for path in [plain.outer_path(target), plain.inner_path(target)] {
        assert_samples(&path, &[(15., 25.), (20., 30.)], &[(10., 20.), (30., 40.)]);
    }
    let radius = Radius::circular(5.);
    let border = BeveledRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusDirectional {
            top_start: radius,
            bottom_start: radius,
            ..BorderRadiusDirectional::ZERO
        },
    );
    for (direction, include, exclude) in [
        (
            TextDirection::Ltr,
            [(15., 25.), (20., 30.)],
            [(10., 20.), (10., 40.)],
        ),
        (
            TextDirection::Rtl,
            [(25., 35.), (25., 25.)],
            [(30., 20.), (30., 40.)],
        ),
    ] {
        for path in [
            border.inner_path_with_direction(target, Some(direction)),
            border.outer_path_with_direction(target, Some(direction)),
        ] {
            assert_samples(&path, &include, &exclude);
        }
    }
}

#[test]
fn independent_x_and_y_radii_cut_elliptical_corners_at_different_offsets() {
    use incular_widgets::Radius;
    let border = BeveledRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::only(
            Radius::elliptical(10., 3.),
            Radius::ZERO,
            Radius::ZERO,
            Radius::ZERO,
        ),
    );
    assert_samples(
        &border.outer_path(rect(0., 0., 100., 80.)),
        &[(10., 0.), (0., 3.), (2., 4.)],
        &[(9., 0.), (0., 2.)],
    );
}

#[test]
fn stroke_alignment_uses_the_upstream_inner_path_samples() {
    use incular_config::EdgeInsets;
    let target = rect(0., 0., 120., 40.);
    let radius = BorderRadius::circular(10.);
    let side = BorderSide::solid(Color::BLACK, 10.);
    for (align, inset, include, exclude) in [
        (
            -1.,
            10.,
            [(10., 20.), (100., 10.), (50., 30.), (50., 20.)],
            [(9., 9.), (100., 0.), (110., 31.), (9., 31.)],
        ),
        (
            0.,
            5.,
            [(9., 9.), (100., 10.), (110., 31.), (9., 31.)],
            [(4., 4.), (100., 0.), (116., 31.), (4., 31.)],
        ),
        (
            1.,
            0.,
            [(5., 5.), (110., 0.), (116., 31.), (4., 31.)],
            [(0., 0.), (120., 0.), (120., 31.), (0., 31.)],
        ),
    ] {
        let shape =
            BeveledRectangleBorder::new(side.copy_with(None, None, None, Some(align)), radius);
        assert_eq!(shape.dimensions(None), EdgeInsets::all(inset));
        assert_samples(&shape.inner_path(target), &include, &exclude);
    }
}

#[test]
fn scaling_and_interpolation_preserve_radius_and_stroke_proportions() {
    use incular_config::EdgeInsets;
    use incular_core::Lerp;
    let border = |width| {
        BeveledRectangleBorder::new(
            BorderSide::solid(Color::BLACK, width),
            BorderRadius::circular(width * 10.),
        )
    };
    let a = border(10.);
    let b = border(20.);
    assert_eq!(a.dimensions(None), EdgeInsets::all(10.));
    assert_eq!(a.scale(2.), b);
    assert_eq!(b.scale(0.5), a);
    assert_eq!(a.lerp(&b, 0.), a);
    assert_eq!(a.lerp(&b, 0.5), border(15.));
    assert_eq!(a.lerp(&b, 1.), b);
}

#[test]
fn copying_beveled_borders_preserves_equality_hash_and_geometry_type() {
    use incular_widgets::{BorderRadiusDirectional, BorderRadiusGeometry, Radius};
    use std::hash::{DefaultHasher, Hash, Hasher};
    let hash = |value: BeveledRectangleBorder| {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    };
    let original = BeveledRectangleBorder::default();
    assert_eq!(original, original.copy_with(None, None));
    assert_eq!(hash(original), hash(original.copy_with(None, None)));
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 0xff), 10.);
    let circular = BorderRadius::circular(16.);
    assert_eq!(
        original.copy_with(Some(side), Some(circular)),
        BeveledRectangleBorder::new(side, circular)
    );
    let directional =
        BorderRadiusGeometry::from(BorderRadiusDirectional::all(Radius::circular(16.)));
    let original = BeveledRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::ZERO);
    assert_eq!(
        original.copy_with(Some(side), Some(directional)),
        BeveledRectangleBorder::new(side, directional)
    );
}
