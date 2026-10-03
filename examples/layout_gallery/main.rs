//! A visual gallery for the retained layout primitives.
use incular::prelude::*;
use incular::widgets::internal::ScrollView;

const SURFACE: Color = DEMO_SURFACE;
const MUTED: Color = DEMO_MUTED;

fn text(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value)
        .style(TextStyle {
            inherit: false,
            size,
            color,
            ..TextStyle::default()
        })
        .into()
}

fn heading(value: &str) -> Widget {
    Padding::all(8., text(value, 20., Color::WHITE)).into()
}

fn card(child: impl Into<Widget>) -> Widget {
    DecoratedBox::new(Padding::all(12., child))
        .background(SURFACE)
        .border(Border::new(1., DEMO_BORDER))
        .radius(10.)
        .into()
}

fn chip(label: &str, color: Color) -> Widget {
    DecoratedBox::new(Padding::all(8., text(label, 15., Color::WHITE)))
        .background(color)
        .radius(14.)
        .into()
}

// Local lagoon palette.
const DEMO_CANVAS: Color = Color::rgba(14, 32, 33, 255);
const DEMO_SURFACE: Color = Color::rgba(24, 49, 49, 255);
const DEMO_BORDER: Color = Color::rgba(65, 104, 99, 255);
const DEMO_TEXT: Color = Color::rgba(235, 247, 236, 255);
const DEMO_MUTED: Color = Color::rgba(167, 197, 182, 255);
const DEMO_ACCENT: Color = Color::rgba(125, 224, 188, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let controller = ScrollController::new();
    let app = Application::new(move |_| {
        let content: Widget = {
            ScrollView::vertical(
                controller.clone(),
                Padding::all(
                    16.,
                    Column::new([
                        text("Incular layout gallery", 30., DEMO_ACCENT),
                        text(
                            "Scroll through concrete retained layout primitives",
                            16.,
                            MUTED,
                        ),
                        heading("Wrap · flowing tags"),
                        card(
                            Wrap::new([
                                chip("layout", Color::rgba(32, 111, 89, 255)),
                                chip("painting", Color::rgba(139, 67, 100, 255)),
                                chip("semantics", Color::rgba(51, 125, 92, 255)),
                                chip("animation", Color::rgba(181, 78, 67, 255)),
                                chip("native", Color::rgba(146, 97, 34, 255)),
                                chip("widgets", Color::rgba(45, 111, 137, 255)),
                            ])
                            .spacing(8.)
                            .run_spacing(8.),
                        ),
                        heading("Table · measured grid"),
                        card(
                            Table::new(
                                3,
                                [
                                    text("Primitive", 16., Color::WHITE),
                                    text("Purpose", 16., Color::WHITE),
                                    text("Status", 16., Color::WHITE),
                                    text("Stack", 15., MUTED),
                                    text("Layering", 15., MUTED),
                                    text("Ready", 15., Color::rgba(110, 225, 168, 255)),
                                    text("AspectRatio", 15., MUTED),
                                    text("Proportions", 15., MUTED),
                                    text("Ready", 15., Color::rgba(110, 225, 168, 255)),
                                ],
                            )
                            .column_spacing(26.)
                            .row_spacing(10.),
                        ),
                        heading("Stack · overlay paint order"),
                        card(
                            SizedBox::from_size(Size::new(360., 150.)).child(
                                Stack::new([
                                    DecoratedBox::new(Widget::box_(
                                        Size::new(300., 112.),
                                        Color::TRANSPARENT,
                                    ))
                                    .background(Color::rgba(32, 111, 89, 255))
                                    .radius(20.)
                                    .into(),
                                    Align::new(
                                        Alignment::TOP_LEFT,
                                        Padding::all(18., text("top-left", 15., Color::WHITE)),
                                    )
                                    .into(),
                                    Align::new(
                                        Alignment::BOTTOM_RIGHT,
                                        Padding::all(18., text("front-most", 15., Color::WHITE)),
                                    )
                                    .into(),
                                    text("Stack", 30., Color::WHITE),
                                ])
                                .alignment(Alignment::CENTER),
                            ),
                        ),
                        heading("Fractional sizing + AspectRatio"),
                        card(
                            SizedBox::from_size(Size::new(400., 140.)).child(
                                FractionallySizedBox::new(AspectRatio::new(
                                    16. / 9.,
                                    DecoratedBox::new(Center::new(text(
                                        "16 : 9",
                                        28.,
                                        Color::WHITE,
                                    )))
                                    .background(Color::rgba(139, 67, 100, 255))
                                    .radius(12.),
                                ))
                                .width_factor(0.70)
                                .height_factor(0.90),
                            ),
                        ),
                        heading("Baseline + constraints"),
                        card(Row::new(vec![
                            Widget::from(Baseline::new(
                                38.,
                                text("Baseline", 28., Color::rgba(255, 203, 102, 255)),
                            )),
                            Widget::from(Baseline::new(38., text("aligned", 16., Color::WHITE))),
                            Widget::from(ConstrainedBox::new(
                                Constraints::tight(Size::new(112., 44.)),
                                DecoratedBox::new(Center::new(text("tight", 15., Color::WHITE)))
                                    .background(Color::rgba(37, 126, 111, 255))
                                    .radius(8.),
                            )),
                        ])),
                        heading("UnconstrainedBox + Visibility"),
                        card(Row::new(vec![
                            Widget::from(UnconstrainedBox::new(
                                DecoratedBox::new(text("natural width", 16., Color::WHITE))
                                    .background(Color::rgba(154, 73, 39, 255))
                                    .radius(8.),
                            )),
                            Widget::from(
                                Visibility::new(text("This stays hidden", 16., Color::WHITE))
                                    .visible(false),
                            ),
                            text("Hidden sibling omitted", 16., MUTED),
                        ])),
                    ]),
                ),
            )
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
    .expect("valid layout gallery application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native layout gallery application");
}
