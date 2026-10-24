//! Flutter's `AnimatedCrossFade`: the shown child fades in on top while the
//! other fades out below it, and the size animates between theirs.

use std::time::{Duration, Instant};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{AnimatedCrossFade, CrossFadeState, Widget};

fn fade(state: CrossFadeState) -> AnimatedCrossFade {
    AnimatedCrossFade::new(
        Widget::box_(Size::new(50., 50.), Color::WHITE),
        Widget::box_(Size::new(100., 80.), Color::BLACK),
        state,
        Duration::from_millis(300),
    )
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    base: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            base: Instant::now(),
        };
        harness.frame(0);
        harness.frame(16);
        harness
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree
            .layout(Constraints::loose(Size::new(200., 200.)))
            .expect("layout");
    }

    fn size(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }

    /// Opacities in paint order: bottom child, then top child.
    fn opacities(&self) -> Vec<f32> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if let Some(RenderKind::Opacity { alpha, .. }) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.render_object_kind(render))
            {
                found.push(*alpha);
                continue;
            }
            let mut children = self.tree.children(id).unwrap_or_default().to_vec();
            children.reverse();
            stack.extend(children);
        }
        found
    }
}

#[test]
fn the_first_child_shows_at_rest() {
    let harness = Harness::new(fade(CrossFadeState::ShowFirst).into());
    // Bottom: the second child, invisible; top: the first, opaque.
    assert_eq!(harness.opacities(), [0.0, 1.0]);
    assert_eq!(harness.size(), Size::new(50., 50.));
    let harness = Harness::new(fade(CrossFadeState::ShowSecond).into());
    assert_eq!(harness.opacities(), [0.0, 1.0]);
    assert_eq!(harness.size(), Size::new(100., 80.));
}

#[test]
fn switching_cross_fades_and_resizes() {
    let mut harness = Harness::new(fade(CrossFadeState::ShowFirst).into());
    harness
        .tree
        .update(harness.root, fade(CrossFadeState::ShowSecond).into())
        .expect("update");
    // The rebuild at 1000 ms starts the controller on the next frame.
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    // Running forward, the second child is on top at t = 0.5 and the first
    // below at 1 - 0.5.
    assert_eq!(harness.opacities(), [0.5, 0.5]);
    for millis in [1_300, 1_316, 1_450, 1_600, 1_616] {
        harness.frame(millis);
    }
    assert_eq!(harness.opacities(), [0.0, 1.0]);
    assert_eq!(harness.size(), Size::new(100., 80.));
}

#[test]
fn each_child_has_its_own_curve() {
    let mut harness = Harness::new(
        fade(CrossFadeState::ShowFirst)
            .first_curve(Curve::EaseOut)
            .second_curve(Curve::EaseIn)
            .into(),
    );
    harness
        .tree
        .update(
            harness.root,
            fade(CrossFadeState::ShowSecond)
                .first_curve(Curve::EaseOut)
                .second_curve(Curve::EaseIn)
                .into(),
        )
        .expect("update");
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    let opacities = harness.opacities();
    assert!((opacities[0] - (1. - Curve::EaseOut.apply(0.5))).abs() < 1e-4);
    assert!((opacities[1] - Curve::EaseIn.apply(0.5)).abs() < 1e-4);
}
