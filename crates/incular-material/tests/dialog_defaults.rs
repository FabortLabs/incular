//! Flutter's `_DialogDefaultsM3` (radius 28) and AlertDialog's Material 3
//! paddings.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_material::{AlertDialog, SimpleDialog};
use incular_rendering::{PaintCommand, RRect};
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

fn painted(widget: Widget) -> Vec<(Offset, RRect)> {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(600., 600.)))
        .expect("layout");
    let mut stack = vec![Offset::ZERO];
    let mut rects = Vec::new();
    for command in tree.paint().commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let top = *stack.last().unwrap();
                stack.push(transform.transform_point(top));
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::RRect { rrect, .. } => rects.push((*stack.last().unwrap(), *rrect)),
            _ => {}
        }
    }
    rects
}

#[test]
fn dialogs_are_rounded_by_28() {
    for dialog in [
        Widget::from(AlertDialog::new().content(Widget::box_(Size::new(100., 60.), Color::WHITE))),
        Widget::from(SimpleDialog::new().child(Widget::box_(Size::new(100., 60.), Color::WHITE))),
    ] {
        let rects = painted(dialog);
        assert!(
            rects.iter().any(|(_, rrect)| rrect.radii.top_left == 28.),
            "{rects:?}"
        );
    }
}

#[test]
fn alert_dialog_content_padding() {
    // Content alone: (24, 16, 24, 24) around a 100x20 box, 148 wide, within
    // Dialog's 280 minimum width.
    let content = Widget::box_(Size::new(100., 20.), Color::rgba(1, 2, 3, 255));
    let rects = painted(AlertDialog::new().content(content).into());
    let surface = rects
        .iter()
        .find(|(_, rrect)| rrect.radii.top_left == 28.)
        .expect("surface");
    assert_eq!(surface.1.rect.size, Size::new(280., 60.));
}
