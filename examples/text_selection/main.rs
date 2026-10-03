//! Drag across read-only labels, use Shift+Arrow to extend, and Ctrl/Cmd+C to copy.
use incular::material_prelude::{SelectableText, SelectionArea};
use incular::prelude::*;

// Local sky palette.
const DEMO_CANVAS: Color = Color::rgba(236, 245, 248, 255);
const DEMO_TEXT: Color = Color::rgba(23, 49, 61, 255);
const DEMO_MUTED: Color = Color::rgba(69, 104, 118, 255);
const DEMO_ACCENT: Color = Color::rgba(12, 96, 123, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let app = Application::new(|_| {
        let content: Widget = {
            SelectionArea::new(Widget::from(Column::new(Vec::<Widget>::from([
                Text::new("Read-only selection")
                    .style(TextStyle {
                        inherit: false,
                        size: 28.,
                        color: DEMO_TEXT,
                        ..TextStyle::default()
                    })
                    .into(),
                SelectableText::new("Drag from this Latin text…")
                    .style(TextStyle {
                        inherit: false,
                        size: 20.,
                        color: DEMO_ACCENT,
                        ..TextStyle::default()
                    })
                    .into(),
                SelectableText::new("…through this mixed bidi line: עברית / English / 世界")
                    .style(TextStyle {
                        inherit: false,
                        size: 20.,
                        color: DEMO_MUTED,
                        ..TextStyle::default()
                    })
                    .into(),
                Text::new("Selection is read-only: there is no caret or IME session.").into(),
            ]))))
            .into()
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(
                TextStyle::new().color(DEMO_TEXT),
                content,
            ))
            .into()
    })
    .expect("valid selection application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native selection application");
}
