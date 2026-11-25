//! Flutter 3.50 continuous border paths, scale, copy, and paint bounds.
use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand, Path};
use incular_widgets::{BorderRadius, BorderSide, ContinuousRectangleBorder, Radius, ShapeBorder};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

fn samples(path: &Path, includes: &[(f32, f32)], excludes: &[(f32, f32)]) {
    for &(x, y) in includes {
        assert!(
            path.contains(Offset::new(x, y), FillRule::NonZero),
            "did not include ({x},{y})"
        );
    }
    for &(x, y) in excludes {
        assert!(
            !path.contains(Offset::new(x, y), FillRule::NonZero),
            "included ({x},{y})"
        );
    }
}

#[test]
fn zero_radius_paths_include_rectangular_boundaries() {
    let area = rect(10., 20., 20., 20.);
    let plain = ContinuousRectangleBorder::default();
    let bordered =
        ContinuousRectangleBorder::new(BorderSide::solid(Color::BLACK, 4.), BorderRadius::ZERO);
    for path in [
        plain.outer_path(area),
        plain.inner_path(area),
        bordered.outer_path(area),
    ] {
        samples(&path, &[(10., 20.), (20., 30.)], &[(9., 19.), (31., 41.)]);
    }
    samples(
        &bordered.inner_path(area),
        &[(14., 24.), (16., 26.)],
        &[(9., 23.), (27., 37.)],
    );
}

#[test]
fn positive_radii_cut_the_sharp_corners_without_touching_the_center() {
    let area = rect(10., 20., 20., 20.);
    let plain = ContinuousRectangleBorder::new(BorderSide::NONE, BorderRadius::circular(5.));
    for path in [plain.outer_path(area), plain.inner_path(area)] {
        samples(&path, &[(15., 25.), (20., 30.)], &[(10., 20.), (30., 40.)]);
    }
}

#[test]
fn a_thick_stroke_does_not_require_a_negative_size_for_its_inner_path() {
    let border = ContinuousRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 10.),
        BorderRadius::circular(5.),
    );
    let area = rect(0., 0., 2., 3.);
    let inner = border.inner_path(area);
    assert_eq!(inner.tight_bounds(), Some(rect(-8., -7., 18., 17.)));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, stroke, .. }] = list.commands() else {
        panic!("expected the outer stroke: {:?}", list.commands());
    };
    assert_eq!(path.tight_bounds(), Some(area));
    assert_eq!(stroke.width, 10.);
}

#[test]
fn directional_curves_mirror_the_start_corners_onto_the_right() {
    use incular_widgets::BorderRadiusDirectional;
    let area = rect(10., 20., 20., 20.);
    let r = Radius::circular(5.);
    let border = ContinuousRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusDirectional {
            top_start: r,
            bottom_start: r,
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
            border.inner_path_with_direction(area, Some(direction)),
            border.outer_path_with_direction(area, Some(direction)),
        ] {
            samples(&path, &include, &exclude);
        }
    }
}

#[test]
fn scaling_and_interpolating_continuous_borders_match_the_reference_values() {
    let border = |width| {
        ContinuousRectangleBorder::new(
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
fn copy_can_mix_geometry_while_preserving_equal_hashes_for_unchanged_values() {
    use incular_widgets::{BorderRadiusDirectional, BorderRadiusGeometry};
    use std::hash::{DefaultHasher, Hash, Hasher};
    let original = ContinuousRectangleBorder::default();
    let copy = original.copy_with(None, None);
    let hash = |border: ContinuousRectangleBorder| {
        let mut state = DefaultHasher::new();
        border.hash(&mut state);
        state.finish()
    };
    assert_eq!(original, copy);
    assert_eq!(hash(original), hash(copy));
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 0xff), 10.);
    let radius = BorderRadius::circular(16.);
    assert_eq!(
        copy.copy_with(Some(side), Some(radius)),
        ContinuousRectangleBorder::new(side, radius)
    );
    let geometry = BorderRadiusGeometry::from(BorderRadiusDirectional::all(Radius::circular(16.)));
    let copy = ContinuousRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::ZERO);
    assert_eq!(
        copy.copy_with(Some(side), Some(geometry)),
        ContinuousRectangleBorder::new(side, geometry)
    );
}

#[test]
fn continuous_elliptical_top_left_matches_flutters_axis_mapping() {
    let radius = BorderRadius::only(
        Radius::elliptical(10., 3.),
        Radius::ZERO,
        Radius::ZERO,
        Radius::ZERO,
    );
    let border = ContinuousRectangleBorder::new(BorderSide::NONE, radius);
    samples(
        &border.outer_path(rect(0., 0., 100., 80.)),
        &[(3., 0.), (0., 10.), (4., 0.25)],
        &[(0., 5.), (2., 0.)],
    );
}
