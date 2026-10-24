//! Flutter's `AnimatedSwitcher`: a child that cannot update the current one
//! fades in over `duration` while the old child fades out and then leaves.

use std::time::{Duration, Instant};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{AnimatedSwitcher, Widget};

fn card(key: &str) -> Widget {
    Widget::box_(Size::new(50., 50.), Color::WHITE).with_key(key)
}

fn switcher(key: &str) -> Widget {
    AnimatedSwitcher::new(Duration::from_millis(300), Some(card(key))).into()
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
        harness
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree
            .layout(Constraints::tight(Size::new(100., 100.)))
            .expect("layout");
    }

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.root, widget).expect("update");
    }

    /// The opacity of each transition, bottom to top.
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

fn close(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
}

#[test]
fn the_first_child_shows_without_a_transition() {
    let harness = Harness::new(switcher("a"));
    assert_eq!(harness.opacities(), [1.0]);
}

#[test]
fn a_new_child_cross_fades_with_the_old_one() {
    let mut harness = Harness::new(switcher("a"));
    harness.update(switcher("b"));
    // The rebuild creates the transitions; their animations start on the
    // following frame, as Flutter's tickers do.
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    // Bottom: the outgoing child at 1 - 0.5; top: the new child at 0.5.
    assert!(
        close(&harness.opacities(), &[0.5, 0.5]),
        "{:?}",
        harness.opacities()
    );
    harness.frame(1_300);
    harness.frame(1_316);
    // The dismissed outgoing child is gone.
    assert_eq!(harness.opacities(), [1.0]);
}

#[test]
fn a_child_of_the_same_type_and_key_updates_in_place() {
    let mut harness = Harness::new(switcher("a"));
    harness.update(
        AnimatedSwitcher::new(
            Duration::from_millis(300),
            Some(Widget::box_(Size::new(60., 60.), Color::BLACK).with_key("a")),
        )
        .into(),
    );
    harness.frame(1_000);
    assert_eq!(harness.opacities(), [1.0]);
}

#[test]
fn curves_follow_the_direction_of_each_transition() {
    let configure = |key: &str| -> Widget {
        AnimatedSwitcher::new(Duration::from_millis(300), Some(card(key)))
            .switch_in_curve(Curve::EaseIn)
            .switch_out_curve(Curve::EaseOut)
            .into()
    };
    let mut harness = Harness::new(configure("a"));
    harness.update(configure("b"));
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    // The outgoing child reverses with the out curve, the new one with the
    // in curve, both at t = 0.5.
    let expected = [Curve::EaseOut.apply(0.5), Curve::EaseIn.apply(0.5)];
    assert!(
        close(&harness.opacities(), &expected),
        "{:?}",
        harness.opacities()
    );
}

#[test]
fn removing_the_child_fades_it_out() {
    let mut harness = Harness::new(switcher("a"));
    harness.update(AnimatedSwitcher::new(Duration::from_millis(300), None).into());
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    assert!(close(&harness.opacities(), &[0.5]));
    harness.frame(1_300);
    harness.frame(1_316);
    assert!(harness.opacities().is_empty());
}
