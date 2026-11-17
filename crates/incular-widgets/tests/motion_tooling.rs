//! Motion tooling: hot reload replays enter animations; DevTools shows each
//! site's cost tier and live values.

use std::time::Instant;

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::LayerKind;
use incular_widgets::Widget;
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition, ms, replay_enter_animations};

fn frame(tree: &mut WidgetTree, now: Instant) {
    tree.tick_animations(now);
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    tree.update_compositor(now).expect("compositor");
    let _ = tree.paint();
}

fn alpha(tree: &WidgetTree) -> Option<f32> {
    let layers = tree.compositor_layers();
    let mut stack: Vec<_> = layers.root().into_iter().collect();
    while let Some(id) = stack.pop() {
        if let Some((kind, children)) = layers.layer(id) {
            if let LayerKind::Opacity { alpha } = kind {
                return Some(*alpha);
            }
            stack.extend(children.iter().copied());
        }
    }
    None
}

fn fading() -> Widget {
    Widget::box_(Size::new(50., 50.), Color::WHITE).motion(|m| {
        m.enter(|p| p.opacity(0.))
            .transition(Transition::linear(ms(200)))
    })
}

#[test]
fn a_reassembly_replays_enter_animations() {
    let mut tree = WidgetTree::new();
    let root: ElementId = tree.mount(fading()).expect("mount");
    let mut now = Instant::now();
    for _ in 0..20 {
        now += ms(16);
        frame(&mut tree, now);
    }
    assert!(alpha(&tree).is_none_or(|alpha| alpha > 0.99), "entered");
    // An ordinary rebuild does not replay.
    tree.update(root, fading()).expect("update");
    frame(&mut tree, now);
    assert!(alpha(&tree).is_none_or(|alpha| alpha > 0.99));
    replay_enter_animations();
    tree.update(root, fading()).expect("update");
    frame(&mut tree, now);
    let replayed = alpha(&tree).expect("an opacity layer");
    assert!(
        replayed < 0.1,
        "plays again from the enter pose: {replayed}"
    );
}
