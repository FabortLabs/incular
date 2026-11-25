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

#[test]
fn media_query_scopes_accessibility_flags_and_selectively_rebuilds_consumers() {
    use incular_widgets::{Builder, Column, MediaQuery, SizedBox};
    use std::cell::RefCell;

    let seen = Rc::new(RefCell::new([
        Vec::<bool>::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ]));
    let child = Widget::from(Column::new(
        (0..4)
            .map(|slot| {
                let seen = seen.clone();
                Builder::new(move |context| {
                    seen.borrow_mut()[slot].push(match slot {
                        0 => context.high_contrast(),
                        1 => context.on_off_switch_labels(),
                        2 => context.supports_announce(),
                        _ => context.bold_text(),
                    });
                    SizedBox::shrink().into()
                })
                .into()
            })
            .collect::<Vec<Widget>>(),
    ));
    let outer = Builder::new({
        let child = child.clone();
        move |context| {
            assert!(!context.high_contrast());
            assert!(!context.on_off_switch_labels());
            assert!(!context.supports_announce());
            assert!(!context.bold_text());
            child.clone()
        }
    });
    let mut outside_tree = WidgetTree::new();
    outside_tree.mount(outer.into()).unwrap();
    let constraints = Constraints::loose(Size::new(300., 200.));
    outside_tree.layout(constraints).unwrap();
    assert!(seen.borrow().iter().all(|values| values == &[false]));

    let scoped = |high_contrast| {
        MediaQuery::new(
            RuntimeEnvironment {
                high_contrast,
                on_off_switch_labels: true,
                supports_announce: true,
                bold_text: true,
                ..Default::default()
            },
            child.clone(),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scoped(true)).unwrap();
    tree.layout(constraints).unwrap();
    assert!(seen.borrow().iter().all(|values| values == &[false, true]));

    tree.update(root, scoped(false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(seen.borrow()[0], [false, true, false]);
    assert!(
        seen.borrow()[1..]
            .iter()
            .all(|values| values == &[false, true])
    );
}

#[test]
fn nested_text_overrides_replace_and_clear_outer_values_in_actual_render_style() {
    use incular_widgets::{Builder, MediaQuery};

    let key = Key::from("override text");
    let seen = Rc::new(Cell::new(None));
    let child: Widget = Builder::new({
        let key = key.clone();
        let seen = seen.clone();
        move |context| {
            seen.set(Some([
                context.line_height_scale_factor_override(),
                context.letter_spacing_override(),
                context.word_spacing_override(),
                context.paragraph_spacing_override(),
            ]));
            text().with_key(key.clone())
        }
    })
    .into();
    let scoped = |value, bold_text| {
        MediaQuery::new(
            RuntimeEnvironment {
                bold_text,
                line_height_scale_factor_override: Some(9.),
                letter_spacing_override: Some(9.),
                word_spacing_override: Some(9.),
                paragraph_spacing_override: Some(9.),
                ..Default::default()
            },
            MediaQuery::apply_text_style_overrides(child.clone(), value, value, value, value),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let constraints = Constraints::loose(Size::new(300., 200.));
    let root = tree.mount(scoped(Some(2.), true)).unwrap();
    tree.layout(constraints).unwrap();
    let element = tree.element_with_key(&key).unwrap();

    let style = |tree: &WidgetTree| {
        let RenderKind::Text { style, .. } = tree
            .render_object_kind(tree.render_id(element).unwrap())
            .unwrap()
        else {
            panic!("expected text render object")
        };
        (
            style.weight,
            style.line_height,
            style.letter_spacing,
            style.word_spacing,
        )
    };
    assert_eq!(seen.get(), Some([Some(2.); 4]));
    assert_eq!(
        style(&tree),
        (
            FontWeight::BOLD,
            Some(LineHeight::Multiplier(2.)),
            2.,
            Some(2.)
        )
    );

    tree.update(root, scoped(None, false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.element_with_key(&key), Some(element));
    assert_eq!(seen.get(), Some([None; 4]));
    assert_eq!(style(&tree), (FontWeight::NORMAL, None, 1., None));
}
