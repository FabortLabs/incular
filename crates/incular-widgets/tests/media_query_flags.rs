//! Flutter `MediaQueryData` accessibility values: defaults, build-context
//! access, and their effect on `Text` styles (`EditableText` resolves its
//! style through the same path).

use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, NavigationMode, RuntimeEnvironment};
use incular_core::Size;
use incular_text::{FontWeight, LineHeight, TextStyle};
use incular_widgets::internal::*;

#[test]
fn defaults_match_flutter() {
    let data = RuntimeEnvironment::default();
    assert!(!data.always_use_24_hour_format);
    assert!(!data.accessible_navigation);
    assert!(!data.invert_colors);
    assert!(!data.high_contrast);
    assert!(!data.on_off_switch_labels);
    assert!(!data.reduced_motion);
    assert!(!data.bold_text);
    assert!(!data.supports_announce);
    assert_eq!(data.navigation_mode, NavigationMode::Traditional);
    assert!(!data.supports_showing_system_context_menu);
    assert_eq!(data.line_height_scale_factor_override, None);
    assert_eq!(data.letter_spacing_override, None);
    assert_eq!(data.word_spacing_override, None);
    assert_eq!(data.paragraph_spacing_override, None);
}

fn text_style(environment: RuntimeEnvironment, widget: Widget) -> TextStyle {
    let mut tree = WidgetTree::new();
    tree.set_environment(environment);
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(300., 100.)))
        .expect("layout");
    match tree
        .render_object_kind(tree.render_id(root).expect("render"))
        .expect("render object")
    {
        RenderKind::Text { style, .. } => (**style).clone(),
        _ => panic!("expected a text render object"),
    }
}

fn text() -> Widget {
    Text::new("hello")
        .style(TextStyle::default().letter_spacing(1.0))
        .text_direction(incular_widgets::TextDirection::Ltr)
        .into()
}

#[test]
fn bold_text_makes_text_bold() {
    let plain = text_style(RuntimeEnvironment::default(), text());
    assert_ne!(plain.weight, FontWeight::BOLD);
    let bold = text_style(
        RuntimeEnvironment {
            bold_text: true,
            ..RuntimeEnvironment::default()
        },
        text(),
    );
    assert_eq!(bold.weight, FontWeight::BOLD);
}

#[test]
fn spacing_overrides_replace_the_style_values() {
    let style = text_style(
        RuntimeEnvironment {
            line_height_scale_factor_override: Some(5.0),
            letter_spacing_override: Some(3.0),
            word_spacing_override: Some(4.0),
            ..RuntimeEnvironment::default()
        },
        text(),
    );
    // `TextStyle(height: 5.0)` is a multiplier even above 4.
    assert_eq!(style.line_height, Some(LineHeight::Multiplier(5.0)));
    assert_eq!(style.letter_spacing, 3.0);
    assert_eq!(style.word_spacing, Some(4.0));
}

#[test]
fn build_context_reads_the_flags() {
    let seen = Rc::new(Cell::new(None));
    let mut tree = WidgetTree::new();
    tree.set_environment(RuntimeEnvironment {
        always_use_24_hour_format: true,
        invert_colors: true,
        navigation_mode: NavigationMode::Directional,
        paragraph_spacing_override: Some(2.0),
        reduced_motion: true,
        ..RuntimeEnvironment::default()
    });
    tree.mount(
        incular_widgets::Builder::new({
            let seen = seen.clone();
            move |context| {
                seen.set(Some((
                    context.always_use_24_hour_format(),
                    context.invert_colors(),
                    context.bold_text(),
                    context.navigation_mode(),
                    context.paragraph_spacing_override(),
                    context.disable_animations(),
                )));
                Widget::box_(Size::new(1., 1.), incular_core::Color::WHITE)
            }
        })
        .into(),
    )
    .expect("mount");
    tree.layout(Constraints::loose(Size::new(10., 10.)))
        .expect("layout");
    assert_eq!(
        seen.get(),
        Some((
            true,
            true,
            false,
            NavigationMode::Directional,
            Some(2.0),
            true
        ))
    );
}
