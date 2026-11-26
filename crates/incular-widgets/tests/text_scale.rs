//! The system text scale reaches `Text` unless the text sets its own scaler.

use incular_config::{Constraints, RuntimeEnvironment, TextDirection};
use incular_core::Size;
use incular_text::TextScaler;
use incular_widgets::{Text, internal::WidgetTree};

fn environment(text_scale: f32) -> RuntimeEnvironment {
    RuntimeEnvironment {
        text_scale,
        ..RuntimeEnvironment::default()
    }
}

fn label() -> Text {
    Text::new("Scaled label").text_direction(TextDirection::Ltr)
}

/// Mounts `text` under the given text scale and returns the tree with its
/// root size after a loose layout.
fn mounted(text_scale: f32, text: Text) -> (WidgetTree, Size) {
    let mut tree = WidgetTree::new();
    tree.set_environment(environment(text_scale));
    tree.mount(text.into()).expect("mount");
    let size = laid_out(&mut tree);
    (tree, size)
}

fn laid_out(tree: &mut WidgetTree) -> Size {
    tree.layout(Constraints::loose(Size::new(800., 800.)))
        .expect("layout");
    tree.root_layout_size().expect("root size")
}

#[test]
fn text_without_a_scaler_follows_the_environment_text_scale() {
    let (_, base) = mounted(1.0, label());
    let (_, large) = mounted(2.0, label());
    assert!(large.width > base.width, "{large:?} vs {base:?}");
    assert!(large.height > base.height, "{large:?} vs {base:?}");
}

#[test]
fn explicit_text_scaler_ignores_the_environment() {
    let (_, base) = mounted(1.0, label().text_scaler(TextScaler::no_scaling()));
    let (_, large) = mounted(2.0, label().text_scaler(TextScaler::no_scaling()));
    assert_eq!(base, large);
    let (_, linear) = mounted(2.0, label().text_scaler(TextScaler::linear(1.0)));
    assert_eq!(base, linear);
}

#[test]
fn changing_the_environment_text_scale_relayouts_mounted_text() {
    let mut tree = WidgetTree::new();
    tree.set_environment(environment(1.0));
    tree.mount(label().into()).expect("mount");
    let before = laid_out(&mut tree);
    assert!(tree.set_environment(environment(1.5)));
    let after = laid_out(&mut tree);
    assert!(after.height > before.height, "{after:?} vs {before:?}");
    assert!(after.width > before.width, "{after:?} vs {before:?}");
}

#[test]
fn retained_text_uses_the_clamped_factor_and_can_reroute_to_new_bounds() {
    let original = TextScaler::linear(5.);
    let clamped = original.try_clamp(0., 3.).unwrap();
    let (mut tree, measured) = mounted(2., label().text_scaler(clamped));
    let (_, direct) = mounted(2., label().text_scaler(TextScaler::linear(3.)));
    assert_eq!(measured, direct);

    let leaf = tree.root().unwrap();
    let render = tree.render_id(leaf);
    let next = clamped.try_clamp(4., 6.).unwrap();
    tree.update(leaf, label().text_scaler(next).into()).unwrap();
    let renewed = laid_out(&mut tree);
    let (_, direct_next) = mounted(2., label().text_scaler(TextScaler::linear(4.)));
    assert_eq!(renewed, direct_next);
    assert_eq!(tree.render_id(leaf), render);
    assert!(renewed.width > measured.width && renewed.height > measured.height);
}
