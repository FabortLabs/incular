//! Flutter's `DefaultTextHeightBehavior` and `Text.textHeightBehavior`.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::{LineHeight, TextLeadingDistribution, TextStyle};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    DefaultTextHeightBehavior, Directionality, Text, TextHeightBehavior, Widget,
};

fn style() -> TextStyle {
    TextStyle {
        size: 10.,
        line_height: Some(LineHeight::Absolute(30.)),
        leading_distribution: Some(TextLeadingDistribution::Even),
        ..TextStyle::default()
    }
}

fn height(widget: Widget) -> f32 {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    tree.element_bounds(root).expect("bounds").size.height
}

const TRIMMED: TextHeightBehavior =
    TextHeightBehavior::new(false, false, TextLeadingDistribution::Proportional);

#[test]
fn text_takes_the_ambient_behavior_unless_it_sets_its_own() {
    let text = || Text::new("Hg").style(style());
    assert_eq!(height(text().into()), 30.);
    let trimmed = height(DefaultTextHeightBehavior::new(TRIMMED, text()).into());
    // Both halves of the leading are gone, leaving the font's own extent.
    assert!(trimmed < 30. && trimmed > 0., "{trimmed}");
    // The text's own behavior wins over the ambient one.
    assert_eq!(
        height(
            DefaultTextHeightBehavior::new(
                TRIMMED,
                text().text_height_behavior(TextHeightBehavior::default()),
            )
            .into()
        ),
        30.
    );
    // Trimming only the first ascent removes half as much.
    let top = height(
        text()
            .text_height_behavior(TextHeightBehavior::new(
                false,
                true,
                TextLeadingDistribution::Proportional,
            ))
            .into(),
    );
    assert!((top - (30. + trimmed) / 2.).abs() < 1e-3, "{top} {trimmed}");
}
