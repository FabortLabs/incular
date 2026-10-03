//! Resize the window or change desktop scale preference to inspect the typed
//! runtime environment and its logical SafeArea composition.
use incular::prelude::*;

// Local lagoon palette.
const DEMO_CANVAS: Color = Color::rgba(14, 32, 33, 255);
const DEMO_SURFACE: Color = Color::rgba(24, 49, 49, 255);
const DEMO_TEXT: Color = Color::rgba(235, 247, 236, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let app = Application::new(move |cx| {
        let content: Widget = {
        let environment = cx.environment();
        let detail = format!(
            "logical viewport: {:.0} × {:.0}\nphysical viewport: {} × {}\nscale: {:.2}\ntext scale: {:.2}\nbrightness: {:?}\nlocale: {}\ndirection: {:?}\nsafe insets: {:?}",
            environment.viewport.width,
            environment.viewport.height,
            environment.physical_width,
            environment.physical_height,
            environment.scale_factor,
            environment.text_scale,
            environment.brightness,
            environment
                .primary_locale()
                .map(ToString::to_string)
                .unwrap_or_else(|| "platform default".into()),
            environment.text_direction,
            environment.safe_insets,
        );
        cx.safe_area(
            SafeArea::new(
                Padding::all(
                    24.,
                    DecoratedBox::new(Padding::all(20., Text::new(detail)))
                        .background(DEMO_SURFACE)
                        .radius(16.),
                ),
            )
            .minimum(EdgeInsets::all(12.)),
        )
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(TextStyle::new().color(DEMO_TEXT), content))
            .into()
    })
    .expect("valid environment application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native environment application");
}
