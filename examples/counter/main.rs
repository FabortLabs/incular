//! A declarative native counter: no element IDs, action IDs, or runtime wiring.
#[cfg(all(not(feature = "material"), feature = "controls"))]
use incular::controls_prelude::PrimaryButton;
#[cfg(feature = "material")]
use incular::material_prelude::RawMaterialButton;
use incular::prelude::*;

#[cfg(feature = "material")]
fn compact_increment_button(on_click: impl Fn() + 'static) -> Widget {
    // RawMaterialButton is only the hit/semantic surface. The visible child is
    // an explicit, compact Container with no border or focus-color layer.
    RawMaterialButton::with_child(
        Container::builder()
            .padding(EdgeInsets::symmetric(10.0, 5.0))
            .color(DEMO_ACTION)
            .child(
                Text::new("Increment")
                    .style(TextStyle::new().font_size(13.0).color(DEMO_ON_ACTION)),
            )
            .build(),
    )
    .label("Increment")
    .on_click(on_click)
    .into()
}

#[cfg(all(not(feature = "material"), feature = "controls"))]
fn compact_increment_button(on_click: impl Fn() + 'static) -> Widget {
    PrimaryButton::builder()
        .label("Increment")
        .on_click(on_click)
        .build()
        .into()
}

#[cfg(not(any(feature = "material", feature = "controls")))]
fn compact_increment_button(_on_click: impl Fn() + 'static) -> Widget {
    Text::new("Increment").into()
}

// Local paper palette.
const DEMO_CANVAS: Color = Color::rgba(247, 243, 235, 255);
const DEMO_TEXT: Color = Color::rgba(46, 38, 32, 255);
const DEMO_ACTION: Color = Color::rgba(158, 62, 34, 255);
const DEMO_ON_ACTION: Color = Color::rgba(255, 250, 242, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let count = Signal::new(0_u32);
    let app_count = count.clone();
    let app = Application::new(move |_cx| {
        let content: Widget = {
            let value = app_count.get();
            let callback_count = app_count.clone();
            let increment = compact_increment_button(move || {
                callback_count.update(|count| *count += 1);
            });

            // Both the Column and Container builders accept generic Widget
            // children, while the same components retain their fluent APIs.
            Container::builder()
                .child(
                    Column::builder()
                        .children(vec![
                            Text::new("Incular Counter")
                                .style(TextStyle::new().font_size(17.0).color(DEMO_TEXT))
                                .into(),
                            Text::new(format!("Count: {value}"))
                                .style(TextStyle::new().font_size(32.0).bold().color(DEMO_TEXT))
                                .into(),
                            increment,
                        ])
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .spacing(16.0)
                        .build(),
                )
                .width(320.0)
                .height(220.0)
                .color(Color::rgba(255, 253, 248, 255))
                .radius(20.0)
                .padding(EdgeInsets::all(28.0))
                .alignment(Alignment::CENTER)
                .build()
                .into()
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::CENTER)
            .child(DefaultTextStyle::new(
                TextStyle::new().color(DEMO_TEXT),
                content,
            ))
            .into()
    })
    .expect("valid application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native counter application");
}
