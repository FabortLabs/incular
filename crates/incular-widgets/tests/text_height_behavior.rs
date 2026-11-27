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
    // Trimming only the first ascent removes half as much, up to the
    // whole-pixel rounding of each line height (SkParagraph
    // `InternalLineMetrics::height`).
    let top = height(
        text()
            .text_height_behavior(TextHeightBehavior::new(
                false,
                true,
                TextLeadingDistribution::Proportional,
            ))
            .into(),
    );
    assert!((top - (30. + trimmed) / 2.).abs() <= 1., "{top} {trimmed}");
    assert_eq!(top.fract(), 0.);
}

fn layout_text(
    tree: &mut WidgetTree,
    key: &incular_widgets::internal::Key,
) -> (TextHeightBehavior, Size) {
    use incular_widgets::internal::RenderKind;

    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .unwrap();
    let element = tree.element_with_key(key).unwrap();
    let render = tree.render_id(element).unwrap();
    let Some(RenderKind::Text {
        height_behavior, ..
    }) = tree.render_object_kind(render)
    else {
        panic!("expected the resolved text paragraph");
    };
    (*height_behavior, tree.render_size(render).unwrap())
}

#[test]
fn style_height_behavior_wins_over_standalone_scope_in_both_orders() {
    use incular_widgets::{DefaultTextStyle, internal::Key};
    let key = Key::from("controlled paragraph");
    let leaf =
        Widget::from(Text::new("Hello").text_direction(TextDirection::Ltr)).with_key(key.clone());
    let asym = TextHeightBehavior::new(false, true, TextLeadingDistribution::Proportional);
    let make_style = |child| DefaultTextStyle::new(style(), child).text_height_behavior(TRIMMED);
    let make_scope = |child| DefaultTextHeightBehavior::new(asym, child);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(make_style(make_scope(leaf.clone()).into()).into())
        .unwrap();
    let (behavior, measured) = layout_text(&mut tree, &key);
    assert_eq!(behavior, TRIMMED);
    assert!(measured.height > 0. && measured.height < 30.);

    tree.update(root, make_scope(make_style(leaf.clone()).into()).into())
        .unwrap();
    assert_eq!(layout_text(&mut tree, &key), (TRIMMED, measured));

    let full = TextHeightBehavior::default();
    let fallback = DefaultTextHeightBehavior::new(
        full,
        make_style(DefaultTextStyle::new(style(), leaf).into()),
    );
    tree.update(root, fallback.into()).unwrap();
    let (behavior, size) = layout_text(&mut tree, &key);
    assert_eq!(
        behavior, full,
        "an unset inner DefaultTextStyle leaves the ambient scope in control"
    );
    assert_eq!(size.height, 30.);
}

#[test]
fn explicit_text_height_behavior_overrides_both_inherited_sources() {
    use incular_widgets::{DefaultTextStyle, internal::Key};
    let key = Key::from("explicit paragraph");
    let explicit = TextHeightBehavior::default();
    let text = Widget::from(
        Text::new("Hello")
            .text_direction(TextDirection::Ltr)
            .text_height_behavior(explicit),
    )
    .with_key(key.clone());
    let mut tree = WidgetTree::new();
    tree.mount(
        DefaultTextHeightBehavior::new(
            TRIMMED,
            DefaultTextStyle::new(style(), text).text_height_behavior(TRIMMED),
        )
        .into(),
    )
    .unwrap();
    let (observed, size) = layout_text(&mut tree, &key);
    assert_eq!(observed, explicit);
    assert_eq!(size.height, 30.);
}

#[test]
fn same_inherited_value_stays_quiet_but_changed_value_relayouts_existing_text() {
    use incular_widgets::{Builder, internal::Key};
    use std::{cell::Cell, rc::Rc};

    let key = Key::from("reused paragraph");
    let leaf = Widget::from(
        Text::new("Hg")
            .text_direction(TextDirection::Ltr)
            .style(style()),
    )
    .with_key(key.clone());
    let builds = Rc::new(Cell::new(0));
    let listener: Widget = Builder::new({
        let builds = builds.clone();
        move |context| {
            builds.set(builds.get() + 1);
            let _ = DefaultTextHeightBehavior::maybe_of(context).unwrap();
            leaf.clone()
        }
    })
    .into();
    let make = |value| DefaultTextHeightBehavior::new(value, listener.clone()).into();

    let mut tree = WidgetTree::new();
    let root = tree.mount(make(TRIMMED)).unwrap();
    let trimmed = layout_text(&mut tree, &key);
    let paragraph = tree.element_with_key(&key).unwrap();
    assert_eq!(builds.get(), 1);
    tree.update(root, make(TRIMMED)).unwrap();
    assert_eq!(layout_text(&mut tree, &key), trimmed);
    assert_eq!(builds.get(), 1);

    let full = TextHeightBehavior::default();
    tree.update(root, make(full)).unwrap();
    let (observed, size) = layout_text(&mut tree, &key);
    assert_eq!((observed, size.height), (full, 30.));
    assert_eq!(tree.element_with_key(&key), Some(paragraph));
    assert_eq!(builds.get(), 2);
}

#[test]
fn no_height_provider_returns_none_and_text_uses_default_metrics() {
    use incular_widgets::{Builder, internal::Key};

    let key = Key::from("unscoped paragraph");
    let text = Widget::from(
        Text::new("Hello")
            .style(style())
            .text_direction(TextDirection::Ltr),
    )
    .with_key(key.clone());
    let mut tree = WidgetTree::new();
    let scope = Builder::new(move |context| {
        assert_eq!(DefaultTextHeightBehavior::maybe_of(context), None);
        text.clone()
    });
    tree.mount(scope.into()).unwrap();
    let (behavior, size) = layout_text(&mut tree, &key);
    assert_eq!(
        (behavior, size.height),
        (TextHeightBehavior::default(), 30.)
    );
}

#[test]
fn animated_style_applies_height_behavior_before_the_style_tween_advances() {
    use incular_widgets::{AnimatedDefaultTextStyle, internal::Key};
    use std::time::{Duration, Instant};

    let key = Key::from("animated height defaults");
    let make = |behavior, spacing| {
        let base = style().letter_spacing(spacing);
        AnimatedDefaultTextStyle::new(
            base,
            Duration::from_millis(200),
            Widget::from(Text::new("Hg").text_direction(TextDirection::Ltr)).with_key(key.clone()),
        )
        .text_height_behavior(behavior)
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(TRIMMED, 0.)).unwrap();
    let base = Instant::now();
    let (_, start) = layout_text(&mut tree, &key);
    let full = TextHeightBehavior::default();
    tree.update(root, make(full, 10.)).unwrap();
    for millis in [0, 0] {
        tree.update_compositor(base + Duration::from_millis(millis))
            .unwrap();
        let (observed, size) = layout_text(&mut tree, &key);
        assert_eq!(observed, full);
        assert_eq!(size, Size::new(start.width, 30.));
    }
    tree.update_compositor(base + Duration::from_millis(100))
        .unwrap();
    let (observed, size) = layout_text(&mut tree, &key);
    assert_eq!((observed, size.height), (full, 30.));
    assert!(size.width > start.width, "{size:?} after {start:?}");
}
