//! Flutter's `StadiumBorder`, `StarBorder` and `RoundedSuperellipseBorder`
//! paths, with bounds derived by hand from the Dart definitions.

use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, PaintCommand, Path};
use incular_widgets::{
    BorderRadius, BorderSide, RoundedSuperellipseBorder, ShapeBorder, StadiumBorder, StarBorder,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn assert_bounds(path: &Path, expected: Rect) {
    assert_bounds_within(path, expected, 1e-3);
}

fn assert_bounds_within(path: &Path, expected: Rect, tolerance: f32) {
    let bounds = path.bounds().expect("non-empty path");
    for (actual, wanted) in [
        (bounds.origin.x, expected.origin.x),
        (bounds.origin.y, expected.origin.y),
        (bounds.size.width, expected.size.width),
        (bounds.size.height, expected.size.height),
    ] {
        assert!(
            (actual - wanted).abs() < tolerance,
            "{bounds:?} != {expected:?}"
        );
    }
}

#[test]
fn stadium_paths_and_stroke() {
    // Radius = shortestSide / 2 = 50. An inside stroke (strokeAlign -1) of
    // width 4 insets the inner path by strokeInset 4 and is drawn on the
    // outline inflated by strokeOffset / 2 = -2.
    let border = StadiumBorder::new(BorderSide::solid(Color::BLACK, 4.));
    let area = rect(0., 0., 200., 100.);
    let outer = border.outer_path(area);
    assert_bounds(&outer, area);
    assert!(outer.contains(Offset::new(100., 50.), FillRule::NonZero));
    assert!(!outer.contains(Offset::new(2., 2.), FillRule::NonZero));
    assert_bounds(&border.inner_path(area), rect(4., 4., 192., 92.));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, stroke, .. }] = list.commands() else {
        panic!("one stroke: {:?}", list.commands());
    };
    assert_eq!(stroke.width, 4.);
    assert_bounds(path, rect(2., 2., 196., 96.));
    let mut canvas = Canvas::default();
    StadiumBorder::default().scale(0.).paint(&mut canvas, area);
    assert!(
        canvas.finish().is_empty(),
        "BorderSide.scale(0) removes the side"
    );
}

#[test]
fn star_bounds() {
    // Five points of radius 50 around (50, 50), the first at the top: the
    // side points at -18° reach 50 ± 50·cos 18° = 2.4472 / 97.5528 and the
    // lower ones at 54° reach 50 + 50·sin 54° = 90.4508.
    let star = StarBorder::new().outer_path(rect(0., 0., 100., 100.));
    assert_bounds(&star, rect(2.4472, 0., 95.1057, 90.4508));
    assert!(star.contains(Offset::new(50., 50.), FillRule::NonZero));
    assert!(!star.contains(Offset::new(50., 95.), FillRule::NonZero));
}

#[test]
fn polygon_squash_and_rotation() {
    // A square polygon is a diamond with its points on the circle of radius
    // 50. In a 200x100 rectangle squash 0 keeps it round (x 50..150) and
    // squash 1 stretches it to the longer side.
    let diamond = StarBorder::polygon().points(4.);
    assert_bounds(
        &diamond.outer_path(rect(0., 0., 100., 100.)),
        rect(0., 0., 100., 100.),
    );
    assert_bounds(
        &diamond.outer_path(rect(0., 0., 200., 100.)),
        rect(50., 0., 100., 100.),
    );
    assert_bounds(
        &diamond.squash(1.).outer_path(rect(0., 0., 200., 100.)),
        rect(0., 0., 200., 100.),
    );
    // Rotated 45° the valleys lead: inner radius 50·(cos 45°·0.998 + 0.002)
    // = 35.3846 from the center.
    assert_bounds(
        &diamond.rotation(45.).outer_path(rect(0., 0., 100., 100.)),
        rect(14.6154, 14.6154, 70.7693, 70.7693),
    );
    assert!((diamond.inner_ratio() - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
}

#[test]
fn rounded_superellipse_border() {
    let area = rect(0., 0., 100., 100.);
    let square = RoundedSuperellipseBorder::default();
    assert_bounds(&square.outer_path(area), area);
    let rounded = RoundedSuperellipseBorder::new(
        BorderSide::solid(Color::BLACK, 2.),
        BorderRadius::circular(20.),
    );
    let outer = rounded.outer_path(area);
    assert!(outer.contains(Offset::new(5.9, 5.9), FillRule::NonZero));
    assert!(!outer.contains(Offset::new(5.8, 5.8), FillRule::NonZero));
    let inner = rounded.inner_path(area);
    assert_bounds(&inner, rect(2., 2., 96., 96.));
    let heights = [1.9, 1.95, 1.99, 2., 2.01, 2.05];
    for (x, expected) in [
        (20., [false, false, false, false, false, false]),
        (25., [false, false, false, false, false, false]),
        (30., [false, false, false, false, false, true]),
        (35., [false, false, false, false, true, true]),
        (40., [false, false, false, false, true, true]),
        (45., [false, false, false, false, true, true]),
        (50., [false, false, false, true, true, true]),
    ] {
        assert_eq!(
            heights.map(|y| inner.contains(Offset::new(x, y), FillRule::NonZero)),
            expected,
            "edge at x={x}"
        );
    }
    let mut canvas = Canvas::default();
    rounded.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, .. }] = list.commands() else {
        panic!("one stroke");
    };
    // (strokeOutset - strokeInset) / 2 = (0 - 2) / 2 = -1.
    assert_bounds_within(path, rect(1., 1., 98., 98.), 0.01);
}

#[test]
fn circle_and_oval_borders_follow_flutter_paths() {
    use incular_widgets::{CircleBorder, OvalBorder};

    // Circle radius = shortestSide / 2 = 50 centred at (100, 50); the inner
    // path deflates the shortest side by strokeInset 4 (radius 48, which is
    // radius 50 minus the inset) and the stroke outline inflates by
    // strokeOffset / 2 = -2 (radius 48, drawn at the stroke centre).
    let area = rect(0., 0., 200., 100.);
    let circle = CircleBorder::new(BorderSide::solid(Color::BLACK, 4.));
    let outer = circle.outer_path(area);
    assert_bounds(&outer, rect(50., 0., 100., 100.));
    assert!(outer.contains(Offset::new(100., 50.), FillRule::NonZero));
    assert!(
        !outer.contains(Offset::new(52., 10.), FillRule::NonZero),
        "a corner of the box lies outside the circle"
    );
    assert_bounds_within(&circle.inner_path(area), rect(54., 4., 92., 92.), 1e-2);
    let mut canvas = Canvas::default();
    circle.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::StrokePath { path, .. }] = list.commands() else {
        panic!("one circle stroke: {:?}", list.commands());
    };
    assert_bounds_within(path, rect(52., 2., 96., 96.), 1e-2);

    // Oval fills the rectangle: its outer bounds are the box itself, with
    // the same inset and stroke rules applied to both axes.
    let oval = OvalBorder::new(BorderSide::solid(Color::BLACK, 4.));
    assert_bounds(&oval.outer_path(area), area);
    assert_bounds_within(&oval.inner_path(area), rect(4., 4., 192., 92.), 1e-2);
    assert!(
        oval.outer_path(area)
            .contains(Offset::new(190., 50.), FillRule::NonZero),
        "an oval reaches the box's side midpoints"
    );
    assert!(
        !oval
            .outer_path(area)
            .contains(Offset::new(2., 2.), FillRule::NonZero),
        "an oval leaves the corners empty"
    );
    let mut canvas = Canvas::default();
    OvalBorder::default().scale(0.).paint(&mut canvas, area);
    assert!(canvas.finish().is_empty());
}

#[test]
fn beveled_border_cuts_corners_between_radius_endpoints() {
    use incular_widgets::BeveledRectangleBorder;

    // Radius 20 on each corner: the bevel runs from (0, 20) to (20, 0), and
    // the outline's bounds are the whole box.
    let area = rect(0., 0., 200., 100.);
    let border = BeveledRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 4.),
        BorderRadius::circular(20.),
    );
    let outer = border.outer_path(area);
    assert_bounds(&outer, area);
    assert!(outer.contains(Offset::new(100., 50.), FillRule::NonZero));
    assert!(
        !outer.contains(Offset::new(2., 2.), FillRule::NonZero),
        "the cut corner is outside"
    );
    assert!(
        outer.contains(Offset::new(30., 30.), FillRule::NonZero),
        "the far side of the bevel is inside"
    );
    assert_bounds(&border.inner_path(area), rect(4., 4., 192., 92.));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    assert!(
        !canvas.finish().is_empty(),
        "a solid side strokes the outline"
    );
}

#[test]
fn compound_border_only_uses_the_outer_interior_hook_when_all_layers_prefer_it() {
    use incular_rendering::Paint;
    use incular_widgets::{CompoundBorder, ShapeDecoration, TextDirection};
    use std::{cell::Cell, rc::Rc, sync::Arc};

    struct Shape {
        preferred: bool,
        interior_calls: Rc<Cell<u8>>,
    }

    impl ShapeBorder for Shape {
        fn outer_path(&self, rect: Rect) -> Path {
            let mut builder = Path::builder();
            builder.rounded_rect(rect, incular_rendering::CornerRadii::ZERO);
            builder.build()
        }

        fn inner_path(&self, rect: Rect) -> Path {
            self.outer_path(rect)
        }

        fn prefer_paint_interior(&self) -> bool {
            self.preferred
        }

        fn paint_interior(
            &self,
            canvas: &mut Canvas,
            rect: Rect,
            paint: &Paint,
            _: Option<TextDirection>,
        ) {
            assert!(self.preferred);
            self.interior_calls.set(self.interior_calls.get() + 1);
            canvas.draw_path(Arc::new(self.outer_path(rect)), paint, FillRule::NonZero);
        }

        fn paint(&self, _: &mut Canvas, _: Rect) {}
    }

    for (outer_preferred, inner_preferred) in
        [(true, true), (true, false), (false, true), (false, false)]
    {
        let outer = Rc::new(Cell::new(0));
        let inner = Rc::new(Cell::new(0));
        let compound = CompoundBorder::new(
            Shape {
                preferred: outer_preferred,
                interior_calls: outer.clone(),
            },
            Shape {
                preferred: inner_preferred,
                interior_calls: inner.clone(),
            },
        );
        assert_eq!(
            compound.prefer_paint_interior(),
            outer_preferred && inner_preferred
        );
        let fill = ShapeDecoration::new(compound)
            .color(Color::WHITE)
            .paint_fill(Size::new(40., 30.));
        assert!(matches!(fill.commands(), [PaintCommand::FillPath { .. }]));
        assert_eq!(outer.get(), u8::from(outer_preferred && inner_preferred));
        assert_eq!(inner.get(), 0);
    }
}

#[test]
fn compound_border_paints_outside_to_inside_with_accumulated_insets() {
    use incular_config::EdgeInsets;
    use incular_widgets::{CompoundBorder, RoundedRectangleBorder};

    let outside = Color::rgba(0, 40, 255, 255);
    let inside = Color::rgba(255, 40, 0, 255);
    let layer = |color, width| {
        RoundedRectangleBorder::new(BorderSide::solid(color, width), BorderRadius::ZERO)
    };
    let compound = CompoundBorder::new(layer(outside, 2.), layer(inside, 3.));
    let area = rect(0., 0., 40., 30.);
    assert_eq!(compound.dimensions(None), EdgeInsets::all(5.));
    assert_bounds(&compound.outer_path(area), area);
    assert_bounds(&compound.inner_path(area), rect(5., 5., 30., 20.));

    let mut canvas = Canvas::default();
    compound.paint(&mut canvas, area);
    let painted = canvas.finish();
    let [
        PaintCommand::FillPath {
            path: outer_path,
            brush: incular_rendering::Brush::Solid(outer_color),
            fill_rule: FillRule::EvenOdd,
            ..
        },
        PaintCommand::FillPath {
            path: inner_path,
            brush: incular_rendering::Brush::Solid(inner_color),
            fill_rule: FillRule::EvenOdd,
            ..
        },
    ] = painted.commands()
    else {
        panic!("expected exactly two successive filled stroke bands");
    };
    assert_eq!((*outer_color, *inner_color), (outside, inside));
    assert_bounds(outer_path, area);
    assert_bounds(inner_path, rect(2., 2., 36., 26.));
    assert!(outer_path.contains(Offset::new(1., 15.), FillRule::EvenOdd));
    assert!(!outer_path.contains(Offset::new(3., 15.), FillRule::EvenOdd));
    assert!(inner_path.contains(Offset::new(3., 15.), FillRule::EvenOdd));
    assert!(!inner_path.contains(Offset::new(6., 15.), FillRule::EvenOdd));
}

#[test]
fn compound_border_passes_through_direction_for_stroke_and_inner_path() {
    use incular_rendering::Brush;
    use incular_widgets::{
        CompoundBorder, LinearBorder, LinearBorderEdge, RoundedRectangleBorder, TextDirection,
    };

    let area = rect(0., 0., 40., 30.);
    let compound = CompoundBorder::new(
        LinearBorder::start(
            BorderSide::solid(Color::BLACK, 2.),
            LinearBorderEdge::default(),
        ),
        RoundedRectangleBorder::new(BorderSide::solid(Color::WHITE, 3.), BorderRadius::ZERO),
    );
    for (direction, stroke_x, inner_x) in
        [(TextDirection::Ltr, 0., 5.), (TextDirection::Rtl, 38., 3.)]
    {
        assert_bounds(
            &compound.inner_path_with_direction(area, Some(direction)),
            rect(inner_x, 3., 32., 24.),
        );
        let mut canvas = Canvas::default();
        compound.paint_with_direction(&mut canvas, area, Some(direction));
        let painting = canvas.finish();
        let [
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(Color::BLACK),
                ..
            },
            PaintCommand::FillPath {
                fill_rule: FillRule::EvenOdd,
                ..
            },
        ] = painting.commands()
        else {
            panic!("expected the one directional outside side followed by the inner stroke");
        };
        assert_bounds(path, rect(stroke_x, 0., 2., 30.));
    }
}

#[test]
fn superellipse_and_circle_insets_follow_inside_center_and_outside_alignment() {
    use incular_config::EdgeInsets;
    use incular_widgets::CircleBorder;
    for (alignment, inset) in [(-1., 10.), (0., 5.), (1., 0.)] {
        let side = BorderSide {
            stroke_align: alignment,
            ..BorderSide::solid(Color::BLACK, 10.)
        };
        let border = RoundedSuperellipseBorder::new(side, BorderRadius::ZERO);
        assert_eq!(border.dimensions(None), EdgeInsets::all(inset));
        assert_eq!(
            CircleBorder::new(side).dimensions(None),
            EdgeInsets::all(inset)
        );
    }
}

#[test]
fn superellipse_interpolation_moves_width_and_stroke_alignment_together() {
    use incular_core::Lerp;
    let make = |width, stroke_align| {
        RoundedSuperellipseBorder::new(
            BorderSide {
                stroke_align,
                ..BorderSide::solid(Color::BLACK, width)
            },
            BorderRadius::ZERO,
        )
    };
    assert_eq!(make(10., -1.).lerp(&make(20., 1.), 0.5), make(15., 0.));
}

#[test]
fn circle_eccentricity_adjusts_the_long_axis_for_paths_and_strokes() {
    use incular_widgets::CircleBorder;

    let circle = CircleBorder::default();
    let wide = rect(0., 0., 200., 100.);
    assert_eq!(circle.eccentricity(), 0.);
    assert_eq!(circle.side, BorderSide::NONE);
    assert_bounds(&circle.outer_path(wide), rect(50., 0., 100., 100.));
    assert_bounds(&circle.inner_path(wide), rect(50., 0., 100., 100.));

    let oval = circle.with_eccentricity(1.);
    assert_bounds(&oval.outer_path(wide), wide);
    assert_bounds(&oval.inner_path(wide), wide);
    let stroke = BorderSide::solid(Color::BLACK, 10.);
    let edged = oval.copy_with(Some(stroke), None);
    assert_bounds(&edged.outer_path(wide), wide);
    assert_bounds(&edged.inner_path(wide), rect(10., 10., 180., 80.));

    let medium = edged.copy_with(None, Some(0.5));
    assert_bounds(&medium.outer_path(wide), rect(25., 0., 150., 100.));
    assert_bounds(
        &medium.outer_path(rect(10., 20., 100., 200.)),
        rect(10., 45., 100., 150.),
    );
    let mut canvas = Canvas::default();
    medium.paint(&mut canvas, wide);
    let paint = canvas.finish();
    let [
        PaintCommand::StrokePath {
            path,
            stroke: drawn,
            ..
        },
    ] = paint.commands()
    else {
        panic!("expected one oval stroke");
    };
    assert_eq!(drawn.width, 10.);
    assert_bounds(path, rect(30., 5., 140., 90.));
}

#[test]
fn circle_eccentricity_survives_scaling_and_clamps_on_extrapolation() {
    use incular_core::Lerp;
    use incular_widgets::CircleBorder;

    let start = CircleBorder::new(BorderSide::solid(Color::BLACK, 10.));
    let finish = CircleBorder::new(BorderSide::solid(Color::BLACK, 20.)).with_eccentricity(1.);
    assert_eq!(finish.scale(0.5), start.with_eccentricity(1.));
    assert_eq!(start.lerp(&finish, 0.5).eccentricity(), 0.5);
    assert_eq!(start.lerp(&finish, -1.).eccentricity(), 0.);
    assert_eq!(start.lerp(&finish, 2.).eccentricity(), 1.);
    assert_eq!(finish.copy_with(None, None), finish);
}

#[test]
fn oval_border_scale_interpolation_and_edge_samples_match_the_reference() {
    use incular_config::EdgeInsets;
    use incular_core::Lerp;
    use incular_widgets::OvalBorder;

    let clean = |width| OvalBorder::new(BorderSide::solid(Color::BLACK, width));
    let (thin, middle, thick) = (clean(10.), clean(15.), clean(20.));
    assert_eq!(thin.dimensions(None), EdgeInsets::all(10.));
    assert_eq!(thin.scale(2.), thick);
    assert_eq!(thick.scale(0.5), thin);
    assert_eq!(thin.lerp(&thick, 0.), thin);
    assert_eq!(thin.lerp(&thick, 0.5), middle);
    assert_eq!(thin.lerp(&thick, 1.), thick);
    for (path, included, excluded) in [
        (
            thin.inner_path(rect(0., 0., 100., 40.)),
            [(12., 19.), (50., 10.), (88., 19.), (50., 29.)],
            [(17., 26.), (15., 15.), (74., 10.), (76., 28.)],
        ),
        (
            thin.outer_path(rect(0., 0., 100., 20.)),
            [(2., 9.), (50., 0.), (98., 9.), (50., 19.)],
            [(7., 16.), (10., 2.), (84., 1.), (86., 18.)],
        ),
    ] {
        for (x, y) in included {
            assert!(
                path.contains(Offset::new(x, y), FillRule::NonZero),
                "inside ({x}, {y})"
            );
        }
        for (x, y) in excluded {
            assert!(
                !path.contains(Offset::new(x, y), FillRule::NonZero),
                "outside ({x}, {y})"
            );
        }
    }
}

#[test]
fn rounded_outline_value_methods_keep_physical_and_directional_radii() {
    use incular_core::Lerp;
    use incular_widgets::{BorderRadiusDirectional, Radius, RoundedRectangleBorder, TextDirection};
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let default = RoundedRectangleBorder::<BorderRadius>::default();
    assert_eq!(default.copy_with(None, None), default);
    assert_eq!(hash(default.copy_with(None, None)), hash(default));
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 255), 10.);
    let radius = BorderRadius::circular(16.);
    assert_eq!(
        default.copy_with(Some(side), Some(radius)),
        RoundedRectangleBorder::new(side, radius)
    );
    let default = RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusDirectional::ZERO);
    let radius = BorderRadiusDirectional::circular(16.);
    assert_eq!(
        default.copy_with(Some(side), Some(radius)),
        RoundedRectangleBorder::new(side, radius)
    );

    let directional = RoundedRectangleBorder::new(
        side,
        BorderRadiusDirectional::only(
            Radius::circular(20.),
            Radius::ZERO,
            Radius::ZERO,
            Radius::ZERO,
        ),
    );
    let area = rect(0., 0., 80., 70.);
    let includes_top_left = |direction| {
        directional
            .outer_path_with_direction(area, Some(direction))
            .contains(Offset::new(1., 1.), FillRule::NonZero)
    };
    assert!(!includes_top_left(TextDirection::Ltr));
    assert!(includes_top_left(TextDirection::Rtl));

    let physical = RoundedRectangleBorder::new(side, BorderRadius::circular(100.));
    let directional = directional.into_radius_geometry();
    let physical = physical.into_radius_geometry();
    assert_eq!(
        directional.lerp(&physical, 1.),
        physical.lerp(&directional, 0.)
    );
    assert_eq!(directional.lerp(&physical, 0.), directional);
}

#[test]
fn rounded_rectangle_and_circle_budget_only_the_inner_stroke() {
    use incular_config::EdgeInsets;
    use incular_core::Lerp;
    use incular_widgets::{CircleBorder, RoundedRectangleBorder};
    for (alignment, inset) in [(-1., 10.), (0., 5.), (1., 0.)] {
        let side = BorderSide {
            stroke_align: alignment,
            ..BorderSide::solid(Color::BLACK, 10.)
        };
        let rectangle = RoundedRectangleBorder::new(side, BorderRadius::ZERO);
        assert_eq!(rectangle.dimensions(None), EdgeInsets::all(inset));
        assert_eq!(
            CircleBorder::new(side).dimensions(None),
            EdgeInsets::all(inset)
        );
    }

    let make = |width, align| {
        RoundedRectangleBorder::new(
            BorderSide {
                stroke_align: align,
                ..BorderSide::solid(Color::BLACK, width)
            },
            BorderRadius::ZERO,
        )
    };
    assert_eq!(make(10., -1.).lerp(&make(20., 1.), 0.5), make(15., 0.));
}

#[test]
fn rounded_outline_uses_independent_contours_and_only_a_zero_width_hairline() {
    use incular_config::EdgeInsets;
    use incular_core::Lerp;
    use incular_widgets::RoundedRectangleBorder;
    let shape = |width, radius| {
        RoundedRectangleBorder::new(
            BorderSide::solid(Color::BLACK, width),
            BorderRadius::circular(radius),
        )
    };
    let [a, b, c] = [shape(10., 100.), shape(15., 150.), shape(20., 200.)];
    assert_eq!(a.dimensions(None), EdgeInsets::all(10.));
    assert_eq!(a.scale(2.), c);
    assert_eq!(c.scale(0.5), a);
    for (factor, result) in [(0., a), (0.5, b), (1., c)] {
        assert_eq!(a.lerp(&c, factor), result);
    }
    for path in [
        shape(1., 2.).inner_path(rect(-2., -2., 4., 4.)),
        shape(1., 1.).outer_path(rect(-1., -1., 2., 2.)),
    ] {
        assert_bounds(&path, rect(-1., -1., 2., 2.));
        assert!(path.contains(Offset::new(0.9, 0.), FillRule::NonZero));
        assert!(path.contains(Offset::new(0., 0.9), FillRule::NonZero));
        assert!(!path.contains(Offset::new(0.9, 0.9), FillRule::NonZero));
    }
    let asymmetric = RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(incular_widgets::Radius::elliptical(24., 8.)),
    );
    let square = rect(0., 0., 64., 64.);
    assert!(
        asymmetric
            .outer_path(square)
            .contains(Offset::new(1., 10.), FillRule::NonZero)
    );
    assert!(
        !shape(0., 24.)
            .outer_path(square)
            .contains(Offset::new(1., 10.), FillRule::NonZero)
    );

    let area = rect(10., 20., 70., 170.);
    let mut canvas = Canvas::default();
    a.paint(&mut canvas, area);
    let drawing = canvas.finish();
    let [
        PaintCommand::FillPath {
            path,
            fill_rule: FillRule::EvenOdd,
            ..
        },
    ] = drawing.commands()
    else {
        panic!("positive-width rounded rectangle must fill a ring");
    };
    assert_bounds(path, area);
    assert!(path.contains(Offset::new(45., 25.), FillRule::EvenOdd));
    assert!(!path.contains(Offset::new(45., 40.), FillRule::EvenOdd));

    for (width, height, hole) in [
        (5., 4., true),
        (6., 6., false),
        (6., 7., false),
        (7., 7., true),
    ] {
        let mut canvas = Canvas::default();
        shape(3., 0.).paint(&mut canvas, rect(0., 0., width, height));
        let drawing = canvas.finish();
        let [
            PaintCommand::FillPath {
                path,
                fill_rule: FillRule::EvenOdd,
                ..
            },
        ] = drawing.commands()
        else {
            panic!("expected a ring")
        };
        assert!(!path.contains(Offset::new(width * 0.5, height * 0.5), FillRule::EvenOdd) == hole);
        assert!(path.contains(Offset::new(0.5, 0.5), FillRule::EvenOdd));
    }
    let mut canvas = Canvas::default();
    shape(0., 7.).paint(&mut canvas, area);
    assert!(
        matches!(canvas.finish().commands(), [PaintCommand::StrokePath { stroke, .. }] if stroke.width == 0.)
    );
}

#[test]
fn circle_oval_and_stadium_copy_with_preserves_identity_and_hash() {
    use incular_widgets::{CircleBorder, OvalBorder};
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    let side = BorderSide::solid(Color::rgba(0x12, 0x34, 0x56, 255), 10.);
    let circle = CircleBorder::default();
    assert_eq!(circle.copy_with(None, None), circle);
    assert_eq!(hash(circle.copy_with(None, None)), hash(circle));
    assert_eq!(
        hash(circle.copy_with(None, Some(0.5))),
        hash(circle.with_eccentricity(0.5))
    );
    assert_eq!(circle.copy_with(Some(side), None), CircleBorder::new(side));
    let oval = OvalBorder::default();
    assert_eq!(oval.copy_with(None), oval);
    assert_eq!(hash(oval.copy_with(None)), hash(oval));
    assert_eq!(oval.copy_with(Some(side)), OvalBorder::new(side));
    let stadium = StadiumBorder::default();
    assert_eq!(stadium.copy_with(None), stadium);
    assert_eq!(hash(stadium.copy_with(None)), hash(stadium));
    assert_eq!(stadium.copy_with(Some(side)), StadiumBorder::new(side));
}

#[test]
fn circle_and_stadium_scales_lerps_and_stroked_centerlines_match_originals() {
    use incular_config::EdgeInsets;
    use incular_core::Lerp;
    use incular_widgets::CircleBorder;
    let sides = [10., 15., 20.].map(|width| BorderSide::solid(Color::BLACK, width));
    let circles = sides.map(CircleBorder::new);
    let stadiums = sides.map(StadiumBorder::new);
    assert_eq!(circles[0].dimensions(None), EdgeInsets::all(10.));
    assert_eq!(stadiums[0].dimensions(None), EdgeInsets::all(10.));
    assert_eq!(circles[0].scale(2.), circles[2]);
    assert_eq!(circles[2].scale(0.5), circles[0]);
    assert_eq!(stadiums[0].scale(2.), stadiums[2]);
    assert_eq!(stadiums[2].scale(0.5), stadiums[0]);
    for (i, t) in [0., 0.5, 1.].into_iter().enumerate() {
        assert_eq!(circles[0].lerp(&circles[2], t), circles[i]);
        assert_eq!(stadiums[0].lerp(&stadiums[2], t), stadiums[i]);
    }
    for circle in [
        circles[0].inner_path(rect(-11., -11., 22., 22.)),
        circles[0].outer_path(rect(-1., -1., 2., 2.)),
        StadiumBorder::new(BorderSide::solid(Color::BLACK, 1.)).inner_path(rect(-2., -2., 4., 4.)),
        StadiumBorder::new(BorderSide::solid(Color::BLACK, 1.)).outer_path(rect(-1., -1., 2., 2.)),
    ] {
        assert_bounds(&circle, rect(-1., -1., 2., 2.));
        assert!(circle.contains(Offset::new(0.9, 0.), FillRule::NonZero));
        assert!(!circle.contains(Offset::new(0.9, 0.9), FillRule::NonZero));
    }

    let stroked_bounds = |border: &dyn ShapeBorder, area| {
        let mut canvas = Canvas::default();
        border.paint(&mut canvas, area);
        let drawing = canvas.finish();
        let [PaintCommand::StrokePath { path, stroke, .. }] = drawing.commands() else {
            panic!("expected one real stroke");
        };
        assert_eq!(stroke.width, 10.);
        path.bounds().unwrap()
    };
    assert_eq!(
        stroked_bounds(&circles[0], rect(10., 20., 30., 40.)),
        rect(15., 30., 20., 20.)
    );
    let area = rect(10., 20., 90., 180.);
    assert_eq!(
        stroked_bounds(&stadiums[0], area),
        rect(15., 25., 80., 170.)
    );
    for (align, inset, expected) in [(0., 5., area), (1., 0., rect(5., 15., 100., 190.))] {
        let stadium = StadiumBorder::new(BorderSide {
            stroke_align: align,
            ..sides[0]
        });
        assert_eq!(stadium.dimensions(None), EdgeInsets::all(inset));
        assert_eq!(stroked_bounds(&stadium, area), expected);
    }
}

#[test]
fn oversized_circle_oval_and_stadium_distinguish_query_bounds_from_paint() {
    use incular_widgets::{CircleBorder, OvalBorder};
    let area = rect(4., 4., 4., 4.);
    let center = Offset::new(6., 6.);
    for width in [2., 3., 4., 8.] {
        let side = BorderSide::solid(Color::BLACK, width);
        let radius = (2. - width).abs();
        let expected = rect(6. - radius, 6. - radius, 2. * radius, 2. * radius);
        for (name, border, draws) in [
            (
                "circle",
                &CircleBorder::new(side) as &dyn ShapeBorder,
                width < 4.,
            ),
            ("oval", &OvalBorder::new(side), width != 4.),
            ("stadium", &StadiumBorder::new(side), width != 4.),
        ] {
            let inner = border.inner_path(area);
            assert_bounds(&inner, expected);
            assert_eq!(
                inner.contains(center, FillRule::NonZero),
                radius > 0.,
                "{name}, {width}"
            );
            let mut canvas = Canvas::default();
            border.paint(&mut canvas, area);
            let drawing = canvas.finish();
            assert_eq!(!drawing.is_empty(), draws, "{name}, {width}");
            if draws {
                let [PaintCommand::StrokePath { path, stroke, .. }] = drawing.commands() else {
                    panic!("one stroke")
                };
                let radius = (2. - width * 0.5).abs();
                assert_bounds(
                    path,
                    rect(6. - radius, 6. - radius, radius * 2., radius * 2.),
                );
                assert_eq!(stroke.width, width);
            }
        }
    }
}
