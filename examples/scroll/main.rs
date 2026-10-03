//! Retained vertical scrolling: wheel input changes only the viewport transform.
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::ScrollView;

// Local lagoon palette.
const DEMO_CANVAS: Color = Color::rgba(14, 32, 33, 255);
const DEMO_TEXT: Color = Color::rgba(235, 247, 236, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let controller = ScrollController::new();
    let items = (0..100)
        .map(|index| {
            RawMaterialButton::new(format!("Item {index}"))
                .color(Color::rgba(55, 90 + (index % 4) as u8 * 25, 155, 255))
                .on_press(move || eprintln!("clicked Item {index}"))
                .into()
        })
        .collect::<Vec<Widget>>();
    let app = Application::new(move |_| {
        let content =
            ScrollView::vertical(controller.clone(), Widget::from(Column::new(items.clone())));
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(
                TextStyle::new().color(DEMO_TEXT),
                content,
            ))
            .into()
    })
    .expect("valid scrolling application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native scroll application");
}
