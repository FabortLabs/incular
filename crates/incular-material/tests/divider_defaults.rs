//! Flutter's `_DividerDefaultsM3`: a 1 px line centered in 16 px of space.

use incular_config::{Axis, Constraints, CrossAxisAlignment, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_material::{Divider, VerticalDivider};
use incular_rendering::PaintCommand;
use incular_widgets::{Column, Directionality, Row, Widget, internal::WidgetTree};

/// The painted rects of `widget` followed by a marker, in a `Flex` along
/// `axis` that stretches across it.
fn rects(widget: Widget, axis: Axis) -> Vec<Rect> {
    let marker = Widget::box_(Size::new(10., 10.), MARKER);
    let flex: Widget = match axis {
        Axis::Vertical => Column::new([widget, marker])
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .into(),
        Axis::Horizontal => Row::new([widget, marker])
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .into(),
    };
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, flex).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
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
            PaintCommand::Rect { rect, .. } => {
                found.push(Rect::from_origin_size(
                    *stack.last().unwrap() + rect.origin,
                    rect.size,
                ));
            }
            PaintCommand::RRect { rrect, .. } => {
                found.push(Rect::from_origin_size(
                    *stack.last().unwrap() + rrect.rect.origin,
                    rrect.rect.size,
                ));
            }
            _ => {}
        }
    }
    found
}

const MARKER: Color = Color::rgba(1, 2, 3, 255);

#[test]
fn divider_line_is_centered_in_its_space() {
    // The line is centered in 16 px, so the next child starts at 16.
    let painted = rects(Divider::new().color(Color::BLACK).into(), Axis::Vertical);
    assert_eq!(
        painted[0],
        Rect::from_origin_size(Offset::new(0., 7.5), Size::new(200., 1.))
    );
    assert_eq!(painted[1].origin.y, 16.);
    let painted = rects(
        VerticalDivider::new().color(Color::BLACK).into(),
        Axis::Horizontal,
    );
    assert_eq!(
        painted[0],
        Rect::from_origin_size(Offset::new(7.5, 0.), Size::new(1., 200.))
    );
    assert_eq!(painted[1].origin.x, 16.);
}
