use incular_core::{Color, Offset, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{Border, BorderSide, BoxDecoration};

#[test]
fn symmetric_border_does_not_draw_the_absent_pair_or_a_hairline() {
    let color = Color::rgba(220, 20, 60, 255);
    let side = BorderSide::solid(color, 2.);
    for vertical in [true, false] {
        let border = if vertical {
            Border::symmetric(side, BorderSide::NONE)
        } else {
            Border::symmetric(BorderSide::NONE, side)
        };
        let display = BoxDecoration::new()
            .border(border)
            .paint_border(Size::new(20., 16.))
            .expect("the supplied pair paints");

        let painted = |point: Offset| {
            display.commands().iter().any(|command| match command {
                PaintCommand::Rect { rect, color: fill } => {
                    assert_eq!(*fill, color);
                    rect.contains(point)
                }
                PaintCommand::FillPath {
                    path,
                    brush: Brush::Solid(fill),
                    fill_rule,
                } => {
                    assert_eq!(*fill, color);
                    path.contains(point, *fill_rule)
                }
                other => panic!("an absent edge should not emit this border command: {other:?}"),
            })
        };
        for (x, y, visible) in [
            (0.5, 8.5, vertical),
            (19.5, 8.5, vertical),
            (10.5, 0.5, !vertical),
            (10.5, 15.5, !vertical),
            (10.5, 8.5, false),
            (-0.5, 8.5, false),
            (10.5, -0.5, false),
        ] {
            assert_eq!(painted(Offset::new(x, y)), visible, "point ({x}, {y})");
        }
    }
}
