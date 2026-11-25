use incular_config::TextDirection;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, FillRule, PaintCommand};
use incular_widgets::{BorderDirectional, BorderSide, BoxShape};

fn bounds() -> Rect {
    Rect::from_origin_size(Offset::new(10.0, 20.0), Size::new(20.0, 20.0))
}

fn end_border() -> BorderDirectional {
    BorderDirectional::builder()
        .end(BorderSide::solid(Color::rgba(0, 255, 0, 255), 10.0))
        .build()
}

#[test]
fn directional_end_side_fills_the_correct_edge_at_nonzero_origin() {
    for (direction, includes, excludes) in [
        (TextDirection::Rtl, 15.0, 25.0),
        (TextDirection::Ltr, 25.0, 15.0),
    ] {
        let mut canvas = Canvas::default();
        assert!(end_border().paint(
            &mut canvas,
            bounds(),
            Some(direction),
            BoxShape::Rectangle,
            None
        ));
        let list = canvas.finish();
        let [
            PaintCommand::FillPath {
                path,
                brush,
                fill_rule,
            },
        ] = list.commands()
        else {
            panic!("expected one filled edge, got {:?}", list.commands());
        };
        assert_eq!(brush, &Brush::Solid(Color::rgba(0, 255, 0, 255)));
        assert_eq!(*fill_rule, FillRule::NonZero);
        assert!(path.contains(Offset::new(includes, 30.0), *fill_rule));
        assert!(!path.contains(Offset::new(excludes, 30.0), *fill_rule));
    }
}

#[test]
#[should_panic(expected = "require a text direction")]
fn a_visible_nonuniform_directional_border_requires_direction() {
    let _ = end_border().paint(
        &mut Canvas::default(),
        bounds(),
        None,
        BoxShape::Rectangle,
        None,
    );
}

#[test]
fn uniform_and_invisible_borders_do_not_require_direction() {
    let mut canvas = Canvas::default();
    assert!(!BorderDirectional::default().paint(
        &mut canvas,
        bounds(),
        None,
        BoxShape::Rectangle,
        None
    ));
    assert!(canvas.finish().commands().is_empty());
    let mut canvas = Canvas::default();
    let side = BorderSide::default();
    assert!(BorderDirectional::only(side, side, side, side).paint(
        &mut canvas,
        bounds(),
        None,
        BoxShape::Rectangle,
        None,
    ));
    assert!(matches!(
        canvas.finish().commands(),
        [PaintCommand::StrokePath { .. }]
    ));
}
