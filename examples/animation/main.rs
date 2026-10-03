//! A text-containing card translated by retained compositor state.
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::TranslationController;
use std::time::{Duration, Instant};

// Local orchid palette.
const DEMO_CANVAS: Color = Color::rgba(35, 25, 39, 255);
const DEMO_TEXT: Color = Color::rgba(253, 240, 237, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let translation = TranslationController::new();
    let moving = translation.clone();
    let app = Application::new(move |_| {
        let content: Widget = {
            let trigger = moving.clone();
            Widget::from(Column::new(Vec::<Widget>::from([
                RawMaterialButton::new("Move")
                    .on_press(move || {
                        trigger.animate_to(
                            Offset::new(180., 0.),
                            Duration::from_millis(900),
                            Instant::now(),
                        )
                    })
                    .into(),
                Widget::translate(
                    moving.clone(),
                    Widget::from(Column::new(vec![
                        Widget::box_(Size::new(180., 70.), Color::rgba(130, 70, 200, 255)),
                        Text::new("Cached text moves with the card")
                            .color(Color::WHITE)
                            .into(),
                    ])),
                ),
            ])))
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
    .expect("valid animation application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native animation application");
}
