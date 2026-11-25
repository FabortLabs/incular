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
        PaintCommand::StrokePath {
            path: outer_path,
            stroke: outer_stroke,
            ..
        },
        PaintCommand::StrokePath {
            path: inner_path,
            stroke: inner_stroke,
            ..
        },
    ] = painted.commands()
    else {
        panic!("expected exactly two successive strokes");
    };
    assert_eq!((outer_stroke.width, inner_stroke.width), (2., 3.));
    assert_bounds(outer_path, rect(1., 1., 38., 28.));
    assert_bounds(inner_path, rect(3.5, 3.5, 33., 23.));
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
            PaintCommand::StrokePath { .. },
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
