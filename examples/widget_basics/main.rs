//! Widget Basics Example
//! Demonstrates unstyled core primitives, constraint semantics, and decoupled styled components.

#[cfg(feature = "controls")]
use incular::controls_prelude::PrimaryButton;
#[cfg(feature = "material")]
use incular::material_prelude::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::SplitView;

#[cfg(feature = "material")]
fn raw_material_button(child: impl Into<Widget>, on_click: impl Fn() + 'static) -> Widget {
    RawMaterialButton::with_child(child)
        .on_click(on_click)
        .into()
}

#[cfg(feature = "material")]
fn neutral_button() -> Widget {
    raw_material_button(
        Text::new("Neutral Button (Unstyled)")
            .style(TextStyle::new().font_size(13.0).color(DEMO_TEXT)),
        || println!("Clicked unstyled button"),
    )
}

#[cfg(all(not(feature = "material"), feature = "controls"))]
fn neutral_button() -> Widget {
    PrimaryButton::builder()
        .label("Neutral Button (Control)")
        .on_click(|| println!("Clicked control button"))
        .build()
        .into()
}

#[cfg(not(any(feature = "material", feature = "controls")))]
fn neutral_button() -> Widget {
    Text::new("Neutral Button (optional features disabled)").into()
}

fn styled_button_content() -> Container {
    Container::builder()
        .padding(EdgeInsets::symmetric(14.0, 8.0))
        .color(DEMO_ACTION)
        .child(
            Text::new("Styled Action Button").style(
                TextStyle::new()
                    .font_size(13.0)
                    .font_weight(FontWeight::BOLD)
                    .color(DEMO_ON_ACTION),
            ),
        )
        .build()
}

#[cfg(feature = "material")]
fn styled_button() -> Widget {
    raw_material_button(styled_button_content(), || {
        println!("Clicked styled action button")
    })
}

#[cfg(all(not(feature = "material"), feature = "controls"))]
fn styled_button() -> Widget {
    PrimaryButton::builder()
        .child(styled_button_content())
        .on_click(|| println!("Clicked styled control button"))
        .build()
        .into()
}

#[cfg(not(any(feature = "material", feature = "controls")))]
fn styled_button() -> Widget {
    styled_button_content().into()
}

#[cfg(feature = "controls")]
fn control_builder_demo() -> Widget {
    PrimaryButton::builder()
        .child(
            Text::new("Typed control button")
                .style(TextStyle::new().font_size(12.0).color(DEMO_ON_ACTION)),
        )
        .on_click(|| println!("Clicked typed control button"))
        .build()
        .into()
}

#[cfg(not(feature = "controls"))]
fn control_builder_demo() -> Widget {
    Text::new("Controls feature disabled").into()
}

#[cfg(feature = "material")]
fn material_widget_demo() -> Widget {
    raw_material_button(
        Text::new("Raw Material action").style(TextStyle::new().font_size(12.0).color(DEMO_TEXT)),
        || println!("Clicked raw Material action"),
    )
}

#[cfg(not(feature = "material"))]
fn material_widget_demo() -> Widget {
    Text::new("Material feature disabled").into()
}

// Local sky palette.
const DEMO_CANVAS: Color = Color::rgba(236, 245, 248, 255);
const DEMO_SURFACE: Color = Color::rgba(250, 254, 255, 255);
const DEMO_ELEVATED: Color = Color::rgba(219, 236, 241, 255);
const DEMO_BORDER: Color = Color::rgba(172, 201, 211, 255);
const DEMO_TEXT: Color = Color::rgba(23, 49, 61, 255);
const DEMO_MUTED: Color = Color::rgba(69, 104, 118, 255);
const DEMO_ACCENT: Color = Color::rgba(12, 96, 123, 255);
const DEMO_ACTION: Color = Color::rgba(12, 96, 123, 255);
const DEMO_ON_ACTION: Color = Color::rgba(245, 253, 255, 255);

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

fn main() {
    let app = Application::new(|_cx| {
        let content: Widget = {
            // 1. Unstyled neutral Button: sizes strictly to its child, transparent background.
            let neutral_button = neutral_button();

            // 2. Fluent button composition: the visual surface is a generic Container child.
            let styled_button = styled_button();

            // 3. A typed Container can contain Text, a Controls widget, and a Material widget.
            //    The child remains generic `Widget`; no component-specific child type is needed.
            let composition_demo = Container::builder()
                .padding(EdgeInsets::all(10.0))
                .color(DEMO_SURFACE)
                .child(
                    Column::builder()
                        .children(vec![
                            Text::new("Container::builder composition")
                                .style(TextStyle::new().font_size(12.0).bold())
                                .into(),
                            control_builder_demo(),
                            material_widget_demo(),
                        ])
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .spacing(6.0)
                        .build(),
                )
                .build();

            // 4. Multi-line typography with explicit LineHeight::Multiplier.
            let multi_line_text = Text::new(
                "Line 1: Typography respects multiplier line heights.\n\
             Line 2: Lines are properly spaced without overlap.\n\
             Line 3: 1.5x multiplier means 1.5 * font_size logical pixels.",
            )
            .style(
                TextStyle::new()
                    .font_size(14.0)
                    .line_height_multiplier(1.5)
                    .color(DEMO_TEXT),
            );

            // 5. SplitView with explicit second_extent.
            let left_pane = Container::new()
                .padding(EdgeInsets::all(16.0))
                .color(DEMO_SURFACE)
                .child(Column::new([
                    Widget::from(
                        Text::new("Left Pane (Expanded)").style(
                            TextStyle::new()
                                .font_size(16.0)
                                .font_weight(FontWeight::BOLD)
                                .color(DEMO_TEXT),
                        ),
                    ),
                    SizedBox::new().height(12.0).into(),
                    multi_line_text.into(),
                    SizedBox::new().height(16.0).into(),
                    composition_demo.into(),
                    SizedBox::new().height(12.0).into(),
                    Row::new([
                        neutral_button,
                        SizedBox::new().width(12.0).into(),
                        styled_button,
                    ])
                    .into(),
                ]));

            let right_pane = Container::new()
                .padding(EdgeInsets::all(16.0))
                .color(DEMO_ELEVATED)
                .child(Column::new([
                    Widget::from(
                        Text::new("Right Pane (Fixed 280px)").style(
                            TextStyle::new()
                                .font_size(14.0)
                                .font_weight(FontWeight::BOLD)
                                .color(DEMO_ACCENT),
                        ),
                    ),
                    SizedBox::new().height(8.0).into(),
                    Text::new("Uses SplitView::horizontal(...).second_extent(280.0)")
                        .style(TextStyle::new().font_size(12.0).color(DEMO_MUTED))
                        .into(),
                ]));

            let split_demo = SplitView::horizontal(left_pane, right_pane)
                .second_extent(280.0)
                .divider_thickness(6.0)
                .divider_color(DEMO_BORDER);

            Container::new().color(DEMO_CANVAS).child(split_demo).into()
        };
        #[cfg(feature = "controls")]
        let content: Widget = {
            let mut theme = incular::controls::ControlTheme::light();
            theme.colors.accent = DEMO_ACTION;
            theme.colors.accent_hover = Color::rgba(17, 117, 148, 255);
            theme.colors.accent_active = Color::rgba(9, 76, 101, 255);
            theme.colors.accent_foreground = DEMO_ON_ACTION;
            incular::controls::ControlThemeScope::new(theme, content).into()
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
    .expect("valid basics app");

    example_support::spawn_if_requested(app.simulation(), simulations::run);
    if let Err(err) = incular::run(app) {
        eprintln!("Error running widget basics example: {err:?}");
    }
}
