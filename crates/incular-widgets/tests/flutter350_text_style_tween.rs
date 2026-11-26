use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_text::TextStyle;
use incular_widgets::{
    AnimatedDefaultTextStyle, Directionality, Text, Widget, internal::WidgetTree,
};

#[test]
fn animated_default_text_style_changes_painted_glyphs_without_inventing_metric_values() {
    let mut tree = WidgetTree::new();
    let next = Rc::new(Cell::new(false));
    let revision = Rc::new(Cell::new(0));
    let read_next = next.clone();
    tree.mount(Widget::stateful_builder(revision.clone(), move |_| {
        let style = if read_next.get() {
            TextStyle::new()
                .font_size(26.)
                .color(Color::rgba(17, 51, 85, 255))
        } else {
            TextStyle::new()
        };
        Directionality::new(
            TextDirection::Ltr,
            AnimatedDefaultTextStyle::new(style, Duration::from_secs(1), Text::new("Color A")),
        )
        .into()
    }))
    .unwrap();

    let epoch = Instant::now();
    let frame = |tree: &mut WidgetTree, milliseconds| {
        tree.update_compositor(epoch + Duration::from_millis(milliseconds))
            .unwrap();
        tree.layout(Constraints::loose(Size::new(240., 80.)))
            .unwrap();
        tree.paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::GlyphRun { run, color } => Some((run.font_size, *color)),
                _ => None,
            })
            .unwrap()
    };
    assert_ne!(frame(&mut tree, 0).0, 26.);
    next.set(true);
    revision.set(1);
    frame(&mut tree, 16);
    frame(&mut tree, 16);
    for (elapsed, alpha) in [(250, 64), (500, 128), (750, 191), (1000, 255)] {
        assert_eq!(
            frame(&mut tree, 16 + elapsed),
            (26., Color::rgba(17, 51, 85, alpha))
        );
    }
}
