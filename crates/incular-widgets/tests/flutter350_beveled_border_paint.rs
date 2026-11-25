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
