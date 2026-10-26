//! Flutter's `_ListTileDefaultsM3`: content padding 16 at the start and 24
//! at the end, an 8 px minimum vertical padding, a 24 px minimum leading
//! width and a 16 px title gap, in a 56 px one-line tile.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_material::ListTile;
use incular_rendering::PaintCommand;
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

const LEADING: Color = Color::rgba(1, 1, 1, 255);
const TITLE: Color = Color::rgba(2, 2, 2, 255);
const TRAILING: Color = Color::rgba(3, 3, 3, 255);

fn rects(direction: TextDirection) -> Vec<(Color, Rect)> {
    let tile = ListTile::new(Widget::box_(Size::new(100., 20.), TITLE))
        .leading(Widget::box_(Size::new(20., 20.), LEADING))
        .trailing(Widget::box_(Size::new(10., 10.), TRAILING));
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(direction, tile).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(400., 56.)))
        .expect("layout");
    let mut stack = vec![Offset::ZERO];
    let mut found = Vec::new();
    for command in tree.paint().commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let top = *stack.last().unwrap();
                stack.push(transform.transform_point(top));
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::Rect { rect, color } => found.push((
                *color,
                Rect::from_origin_size(*stack.last().unwrap() + rect.origin, rect.size),
            )),
            _ => {}
        }
    }
    found
}

fn find(rects: &[(Color, Rect)], color: Color) -> Rect {
    rects
        .iter()
        .find(|(c, _)| *c == color)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("{color:?} in {rects:?}"))
}

#[test]
fn one_line_tile_layout() {
    let painted = rects(TextDirection::Ltr);
    // Leading at the 16 start padding, then a 24 wide slot and the 16 gap.
    assert_eq!(find(&painted, LEADING).origin.x, 16.);
    assert_eq!(find(&painted, TITLE).origin.x, 16. + 24. + 16.);
    // The trailing widget ends 24 from the end.
    let trailing = find(&painted, TRAILING);
    assert_eq!(trailing.origin.x + trailing.size.width, 400. - 24.);
}
