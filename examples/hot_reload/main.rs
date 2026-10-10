//! Hot reload playground: edit this file while the app is running.
//!
//! ```text
//! dx serve --hot-patch --windows -p incular-hot-reload-example
//! ```
//!
//! Click Increment a few times, then change anything marked "try editing" and
//! save. The window updates in place and the count is kept.
use incular::controls_prelude::PrimaryButton;
use incular::prelude::*;

// Try editing: the palette, the copy and the layout numbers.
const CANVAS: Color = Color::rgba(244, 241, 250, 255);
const CARD: Color = Color::rgba(255, 255, 255, 255);
const INK: Color = Color::rgba(38, 32, 54, 255);
const ACCENT: Color = Color::rgba(98, 70, 190, 255);
const TITLE: &str = "Hot reload";
const HINT: &str = "Edit examples/hot_reload/main.rs and save";
const CARD_PADDING: f32 = 28.0;
const SPACING: f32 = 14.0;

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

/// Try editing: how the retained count is described.
fn describe(count: u32) -> String {
    match count {
        0 => "Nothing counted yet".to_owned(),
        1 => "Counted once".to_owned(),
        count => format!("Counted {count} times"),
    }
}

/// The whole interface. Keeping it in a named function gives every patch the
/// same entry point however much the body changes.
fn view(count: &Signal<u32>) -> Widget {
    let increment_count = count.clone();
    // Try editing: reorder, remove or add rows.
    let rows: Vec<Widget> = vec![
        Text::new(TITLE)
            .style(TextStyle::new().font_size(17.0).color(INK))
            .into(),
        Text::new(describe(count.get()))
            .style(TextStyle::new().font_size(28.0).bold().color(ACCENT))
            .into(),
        PrimaryButton::builder()
            .label("Increment")
            .on_click(move || increment_count.update(|count| *count += 1))
            .build()
            .into(),
        Text::new(HINT)
            .style(TextStyle::new().font_size(12.0).color(INK))
            .into(),
    ];
    let card: Widget = Container::builder()
        .child(
            Column::builder()
                .children(rows)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .spacing(SPACING)
                .build(),
        )
        .color(CARD)
        .radius(20.0)
        .padding(EdgeInsets::all(CARD_PADDING))
        .build()
        .into();
    Container::new()
        .background(CANVAS)
        .alignment(Alignment::CENTER)
        .child(DefaultTextStyle::new(TextStyle::new().color(INK), card))
        .into()
}

fn main() {
    // State lives outside the build function, so a patch never resets it.
    let count = Signal::new(0_u32);
    let app = Application::new(move |_cx| view(&count)).expect("valid application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native hot reload example");
}
