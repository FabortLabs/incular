use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::Canvas;
use incular_widgets::{
    Border, BorderDirectional, BorderRadius, BorderSide, BoxBorderOutline, BoxShape,
    CustomBoxBorder, CustomBoxBorderHandle, ShapeBorder,
};

const BLACK: Color = Color::BLACK;
const YELLOW: Color = Color::rgba(255, 255, 0, 255);
const MAGENTA: Color = Color::rgba(255, 0, 255, 255);
const NONE: BorderSide = BorderSide::NONE;

fn top(width: f32, color: Color) -> BorderSide {
    BorderSide::solid(color, width)
}

fn physical(top: BorderSide, left: BorderSide, right: BorderSide) -> BoxBorderOutline {
    Border {
        top,
        left,
        right,
        bottom: NONE,
    }
    .into()
}

fn directional(top: BorderSide, start: BorderSide, end: BorderSide) -> BoxBorderOutline {
    BorderDirectional::only(top, start, end, NONE).into()
}

fn sides(width: f32, alpha: u8) -> (BorderSide, BorderSide) {
    let color = Color::rgba(0, 0, 0, alpha);
    (top(width, color), top(width * 2., color))
}

fn assert_outline(actual: BoxBorderOutline, expected: BoxBorderOutline) {
    fn compare(a: BorderSide, b: BorderSide) {
        assert_eq!(
            (a.width, a.style, a.stroke_align),
            (b.width, b.style, b.stroke_align)
        );
        assert!(a.color.red.abs_diff(b.color.red) <= 1, "{a:?} != {b:?}");
        assert!(a.color.green.abs_diff(b.color.green) <= 1, "{a:?} != {b:?}");
        assert!(a.color.blue.abs_diff(b.color.blue) <= 1, "{a:?} != {b:?}");
        assert!(a.color.alpha.abs_diff(b.color.alpha) <= 1, "{a:?} != {b:?}");
    }
    use BoxBorderOutline::{Directional, Physical};
    match (actual, expected) {
        (Physical(a), Physical(b)) => {
            for (a, b) in [
                (a.top, b.top),
                (a.left, b.left),
                (a.right, b.right),
                (a.bottom, b.bottom),
            ] {
                compare(a, b);
            }
        }
        (Directional(a), Directional(b)) => {
            for (a, b) in [
                (a.top, b.top),
                (a.start, b.start),
                (a.end, b.end),
                (a.bottom, b.bottom),
            ] {
                compare(a, b);
            }
        }
        (a, b) => panic!("outline kind differs: {a:?} != {b:?}"),
    }
}

#[derive(Debug)]
struct Silly;

impl CustomBoxBorder for Silly {
    fn top(&self) -> BorderSide {
        NONE
    }
    fn bottom(&self) -> BorderSide {
        NONE
    }
    fn is_uniform(&self) -> bool {
        false
    }
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::ZERO
    }
    fn paint(
        &self,
        _: &mut Canvas,
        _: Rect,
        _: Option<TextDirection>,
        _: BoxShape,
        _: Option<BorderRadius>,
    ) -> bool {
        false
    }
    fn scale(&self, _: f32) -> CustomBoxBorderHandle {
        CustomBoxBorderHandle::new(Self)
    }
}

#[test]
fn nullable_mixed_and_custom_box_border_timeline_matches_flutter() {
    let solid: BoxBorderOutline = Border::all(top(10., BLACK)).into();
    let no_border: BoxBorderOutline = Border::default().into();
    let silly: BoxBorderOutline = CustomBoxBorderHandle::new(Silly).into();
    let logical_top = directional(top(10., BLACK), NONE, NONE);
    let physical_top = physical(top(100., BLACK), NONE, NONE);
    let (left, right) = sides(10., 255);
    let physical_sides = physical(NONE, left, right);
    let logical_sides = directional(NONE, left, right);
    let logical_magenta = directional(top(5., MAGENTA), NONE, NONE);
    let physical_yellow = physical(top(5., YELLOW), NONE, NONE);
    let lerp = |a: &BoxBorderOutline, b: &BoxBorderOutline, t| {
        BoxBorderOutline::lerp(Some(a), Some(b), t).unwrap()
    };
    for (index, t) in [-1.0_f32, 0., 0.25, 0.75, 1., 2.].into_iter().enumerate() {
        assert_eq!(BoxBorderOutline::lerp(None, None, t), None);
        for (scale, from) in [(1. - t, true), (t, false)] {
            let expected = |width| if width > 0. { top(width, BLACK) } else { NONE };
            let endpoint = |value| {
                if from {
                    (Some(value), None)
                } else {
                    (None, Some(value))
                }
            };
            let (a, b) = endpoint(&solid);
            assert_outline(
                BoxBorderOutline::lerp(a, b, t).unwrap(),
                Border::all(expected(10. * scale)).into(),
            );
            let (a, b) = endpoint(&logical_top);
            assert_outline(
                BoxBorderOutline::lerp(a, b, t).unwrap(),
                directional(expected(10. * scale), NONE, NONE),
            );
        }
        let width = [0., 10., 32.5, 77.5, 100., 190.][index];
        assert_outline(
            lerp(&logical_top, &physical_top, t),
            physical(
                if width > 0. { top(width, BLACK) } else { NONE },
                NONE,
                NONE,
            ),
        );

        let (fade_width, fade_alpha) = [
            (20., 255),
            (10., 255),
            (7.5, 191),
            (2.5, 63),
            (0., 0),
            (0., 0),
        ][index];
        let (l, r) = if fade_width > 0. {
            sides(fade_width, fade_alpha)
        } else {
            (NONE, NONE)
        };
        let (ink_width, ink) = [
            (0., NONE.color),
            (0., NONE.color),
            (1.25, Color::rgba(63, 0, 63, 63)),
            (3.75, Color::rgba(191, 0, 191, 191)),
            (5., MAGENTA),
            (10., MAGENTA),
        ][index];
        let ink = if ink_width > 0. {
            top(ink_width, ink)
        } else {
            NONE
        };
        assert_outline(
            lerp(&physical_sides, &logical_magenta, t),
            physical(ink, l, r),
        );

        let tone = [
            YELLOW,
            YELLOW,
            Color::rgba(255, 191, 64, 255),
            Color::rgba(255, 64, 191, 255),
            MAGENTA,
            MAGENTA,
        ][index];
        assert_outline(
            lerp(&physical_yellow, &logical_magenta, t),
            physical(top(5., tone), NONE, NONE),
        );

        let (width, alpha) = [
            (30., 255),
            (10., 255),
            (5., 127),
            (5., 127),
            (10., 255),
            (30., 255),
        ][index];
        let (l, r) = sides(width, alpha);
        let expected = if t < 0.5 {
            physical(NONE, l, r)
        } else {
            directional(NONE, l, r)
        };
        assert_outline(lerp(&physical_sides, &logical_sides, t), expected);

        let (width, alpha) = [
            (0., 0),
            (0., 0),
            (2.5, 63),
            (7.5, 191),
            (10., 255),
            (20., 255),
        ][index];
        let (start, end) = if width > 0. {
            sides(width, alpha)
        } else {
            (NONE, NONE)
        };
        let (width, component) = [
            (10., 255),
            (5., 255),
            (3.75, 191),
            (1.25, 63),
            (0., 0),
            (0., 0),
        ][index];
        let top = if width > 0. {
            top(width, Color::rgba(component, component, 0, component))
        } else {
            NONE
        };
        assert_outline(
            lerp(&physical_yellow, &logical_sides, t),
            directional(top, start, end),
        );
        assert_eq!(
            lerp(&silly, &no_border, t),
            if t < 0.5 {
                silly.clone()
            } else {
                no_border.clone()
            }
        );
    }
}

#[test]
fn midpoint_switches_orientation_without_painting_a_left_or_right_stroke() {
    let (a, b) = sides(10., 255);
    let fixed = physical(NONE, a, b);
    let relative = directional(NONE, a, b);
    let Some(BoxBorderOutline::Directional(center)) =
        BoxBorderOutline::lerp(Some(&fixed), Some(&relative), 0.5)
    else {
        panic!("the midpoint chooses the directional representation");
    };
    assert_eq!(center.start, NONE);
    assert_eq!(center.end, NONE);
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let rect = Rect::from_origin_size(Offset::ZERO, Size::new(80., 50.));
        let mut canvas = Canvas::default();
        center.paint_with_direction(&mut canvas, rect, Some(direction));
        assert!(canvas.finish().commands().is_empty());
    }
}
