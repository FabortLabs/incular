//! Resize this window to inspect the public configuration values flowing
//! through retained layout widgets: constraints, insets, alignment and wraps.
use incular::prelude::*;

fn swatch(name: &str, color: Color) -> Widget {
    ConstrainedBox::new(
        Constraints::tight(Size::new(108., 52.)),
        DecoratedBox::new(Center::new(Text::new(name).color(Color::WHITE)))
            .background(color)
            .radius(12.)
            .border(Border::new(1., Color::rgba(255, 255, 255, 105))),
    )
    .into()
}

// Local ember palette.
const DEMO_CANVAS: Color = Color::rgba(30, 27, 25, 255);
const DEMO_TEXT: Color = Color::rgba(251, 243, 227, 255);
const DEMO_ACCENT: Color = Color::rgba(255, 193, 112, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let app = Application::new(move |_| {
        let content: Widget = {
        let table_cells: Vec<Widget> = vec![
            Text::new("Constraints")
                .color(DEMO_ACCENT)
                .into(),
            Text::new("max width: 560 logical pixels").into(),
            Text::new("Insets")
                .color(DEMO_ACCENT)
                .into(),
            Text::new("24 horizontal / 20 vertical").into(),
            Text::new("Alignment")
                .color(DEMO_ACCENT)
                .into(),
            Text::new("bottom-right media caption").into(),
        ];
        Align::new(
            Alignment::TOP_CENTER,
            ConstrainedBox::new(
            Constraints::new(0., 560., 0., 760.),
            Padding::new(
                EdgeInsets::symmetric(24., 20.),
                Widget::from(Column::new(Vec::<Widget>::from([
                    Text::new("Responsive configuration gallery")
                        .style(TextStyle { inherit: false,
                            size: 26.,
                            color: Color::rgba(255, 230, 165, 255),
                            ..TextStyle::default()
                        })
                        .into(),
                    Text::new("A bounded Wrap reflows swatches; the next card uses a fractional width and a 16:9 aspect ratio.")
                        .color(DEMO_TEXT)
                        .into(),
                    Padding::all(
                        12.,
                        Wrap::new([
                            swatch("ocean", Color::rgba(48, 118, 220, 255)),
                            swatch("mint", Color::rgba(45, 184, 151, 255)),
                            swatch("sun", Color::rgba(242, 166, 55, 255)),
                            swatch("plum", Color::rgba(151, 79, 202, 255)),
                            swatch("rose", Color::rgba(222, 83, 119, 255)),
                        ])
                        .spacing(12.)
                        .run_spacing(12.),
                    )
                    .into(),
                    FractionallySizedBox::new(AspectRatio::new(
                        16. / 9.,
                        DecoratedBox::new(Align::new(
                            Alignment::BOTTOM_RIGHT,
                            Padding::all(
                                14.,
                                Text::new("fractionally sized media panel")
                                    .color(Color::WHITE),
                            ),
                        ))
                        .background(LinearGradient {
                            start: Offset::ZERO,
                            end: Offset::new(420., 190.),
                            stops: GradientStops::new(vec![
                                GradientStop { offset: 0., color: Color::rgba(42, 72, 154, 255) },
                                GradientStop { offset: 1., color: Color::rgba(170, 72, 158, 255) },
                            ]),
                        })
                        .radius(18.),
                    ))
                    .width_factor(0.9)
                    .into(),
                    Padding::new(
                        EdgeInsets::only(0., 18., 0., 0.),
                        Table::new(2, table_cells)
                        .column_spacing(26.)
                        .row_spacing(8.),
                    )
                    .into(),
                ]))),
            ),
            ),
        )
        .into()
        };
        Container::new()
            .background(DEMO_CANVAS)
            .alignment(Alignment::TOP_LEFT)
            .child(DefaultTextStyle::new(TextStyle::new().color(DEMO_TEXT), content))
            .into()
    })
    .expect("valid responsive layout application");
    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app).expect("native responsive layout application");
}
