//! Native editing: type `abc`, press Backspace, and watch it become `ab`.
use incular::material::TextField;
use incular::prelude::*;
use incular::widgets::internal::TextEditingController;

// Local orchid palette.
const DEMO_CANVAS: Color = Color::rgba(35, 25, 39, 255);
const DEMO_TEXT: Color = Color::rgba(253, 240, 237, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let name = TextEditingController::with_text("Ada");
    let note = TextEditingController::with_text("First line\nSecond line\n");
    let form = incular::widgets::Form::new();
    let name_field = form
        .register(name.clone())
        .validator(|value| {
            value
                .trim()
                .is_empty()
                .then(|| "Name is required".to_owned())
        })
        .autovalidate(incular::widgets::AutovalidateMode::Always);
    let app = Application::new(move |_cx| {
        let content: Widget = {
            Widget::from(Column::new(Vec::<Widget>::from([
                Text::new("Incular editable text").color(DEMO_TEXT).into(),
                TextField::new(name.clone())
                    .placeholder("Name")
                    .size(Size::new(320., 42.))
                    .into(),
                Text::new(
                    name_field
                        .error()
                        .unwrap_or_else(|| format!("Hello, {}", name.text())),
                )
                .into(),
                Text::new("Multiline (Enter and Shift+Enter add a line)").into(),
                TextField::new(note.clone())
                    .multiline(true)
                    .placeholder("Type Unicode or use an IME")
                    .size(Size::new(320., 180.))
                    .into(),
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
    .expect("valid text field application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native text field application");
}
