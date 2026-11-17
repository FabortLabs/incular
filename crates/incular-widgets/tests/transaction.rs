//! `with_transition` / `without_transition`: the animations an update
//! causes use the transaction's transition instead of their local ones,
//! until the frame that consumes the update ends.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition, ms, with_transition, without_transition};
use incular_widgets::{Column, Container, TransitionExt, Widget};

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    now: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            now: Instant::now(),
        };
        harness.frame();
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::loose(Size::new(400., 400.)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
    }

    fn run(&mut self, duration: Duration) {
        let end = self.now + duration;
        while self.now < end {
            self.now += ms(16);
            self.frame();
        }
    }

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.root, widget).expect("update");
        self.frame();
        self.frame();
    }

    /// Each motion site's vertical offset, in tree order.
    fn offsets(&self) -> Vec<f32> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if let Some(transform) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.content_transform(render))
            {
                found.push(transform.translation_offset().y);
            }
            let children: Vec<ElementId> = self.tree.children(id).unwrap_or(&[]).to_vec();
            stack.extend(children.into_iter().rev());
        }
        found
    }

    fn width(&self) -> f32 {
        self.tree
            .element_bounds(self.root)
            .expect("bounds")
            .size
            .width
    }
}

fn slow() -> Transition {
    Transition::linear(ms(1000))
}

fn list(order: &[&'static str]) -> Widget {
    Column::new(order.iter().map(|key| {
        Widget::box_(Size::new(100., 50.), Color::WHITE)
            .with_key(*key)
            .motion(|m| m.layout_position().transition(slow()))
    }))
    .into()
}

fn bar(width: f32) -> Widget {
    Container::new()
        .width(width)
        .height(10.)
        .color(Color::WHITE)
        .transition(slow())
}

#[test]
fn with_transition_overrides_the_local_layout_transition() {
    let mut h = Harness::new(list(&["a", "b"]));
    with_transition(Transition::linear(ms(100)), || h.update(list(&["b", "a"])));
    assert!(h.offsets().iter().any(|y| y.abs() > 40.), "it animates");
    h.run(ms(150));
    assert!(
        h.offsets().iter().all(|y| y.abs() < 0.5),
        "on the transaction's 100 ms, not the site's 1 s: {:?}",
        h.offsets()
    );
}

#[test]
fn without_transition_jumps_and_ends_with_the_frame() {
    let mut h = Harness::new(list(&["a", "b"]));
    without_transition(|| h.update(list(&["b", "a"])));
    assert!(
        h.offsets().iter().all(|y| y.abs() < 0.01),
        "{:?}",
        h.offsets()
    );
    // The next update is not part of the transaction.
    h.update(list(&["a", "b"]));
    assert!(h.offsets().iter().any(|y| y.abs() > 40.), "animates again");
}

#[test]
fn transactions_reach_dot_transition() {
    let mut h = Harness::new(bar(100.));
    without_transition(|| h.update(bar(300.)));
    assert!((h.width() - 300.).abs() < 0.01, "jumps: {}", h.width());
    h.update(bar(100.));
    h.run(ms(200));
    let width = h.width();
    assert!(
        width > 150. && width < 290.,
        "the local 1 s transition again: {width}"
    );
}
