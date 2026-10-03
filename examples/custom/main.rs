use incular::prelude::*;
use incular_controls::Button;

// Local lagoon palette.
const DEMO_CANVAS: Color = Color::rgba(14, 32, 33, 255);
const DEMO_TEXT: Color = Color::rgba(235, 247, 236, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    // State must live OUTSIDE the builder: builders re-run on every invalidation,
    // so a Signal created inside would be recreated (reset) each time.
    let pressed = Signal::new(false);
    let app = Application::new(move |_cx| {
        let content: Widget = {
            let display_pressed = pressed.get();
            // This diagnostic runs once per builder evaluation. Do not put
            // external side effects such as network requests or file writes here.
            println!("Build: pressed");
            Container::new()
                .background(DEMO_CANVAS)
                .child(Widget::from(Column::new(Vec::<Widget>::from([
                    Text::new("Incular Custom Application")
                        .color(DEMO_TEXT)
                        .into(),
                    Text::new("This is a custom application example.").into(),
                    Button::new("Press me")
                        .on_click({
                            let pressed = pressed.clone();
                            move || {
                                pressed.update(|pressed| *pressed = !*pressed);
                                println!("Click: pressed={}", pressed.get());
                            }
                        })
                        .into(),
                    Text::new(display_pressed.to_string()).into(),
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
    .expect("valid application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native custom application");
}
