//! Flutter's `_ChipDefaultsM3`: 8 px corners, 8 px padding and 8 px of
//! label padding on each side.

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_material::Chip;
use incular_rendering::PaintCommand;
use incular_widgets::{Directionality, internal::WidgetTree};

#[test]
fn chip_shape_and_label_inset() {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Chip::new("Label")).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    let mut stack = vec![Offset::ZERO];
    let mut surface = None;
    let mut text = None;
    for command in tree.paint().commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let top = *stack.last().unwrap();
                stack.push(transform.transform_point(top));
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::RRect { rrect, .. } if surface.is_none() => {
                surface = Some((
                    *stack.last().unwrap() + rrect.rect.origin,
                    rrect.radii.top_left,
                ));
            }
            PaintCommand::GlyphRun { .. } if text.is_none() => {
                text = Some(*stack.last().unwrap());
            }
            _ => {}
        }
    }
    let (surface, radius) = surface.expect("chip surface");
    assert_eq!(radius, 8.);
    // padding 8 + label padding 8.
    assert_eq!(text.expect("label").x - surface.x, 16.);
}
