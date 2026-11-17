//! `.transition()`: transitionable widgets animate their own properties
//! between builds with the motion `Transition` vocabulary.

use std::time::{Duration, Instant};

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::motion::{MotionConfig, ReducedMotion, Spring, Transition, ms};
use incular_widgets::{
    Align, AnimatedContainer, Container, Opacity, Padding, Positioned, Stack, TransitionExt, Widget,
};

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
        harness.frame(0);
        harness
    }

    fn frame(&mut self, millis: u64) {
        let now = self.base + Duration::from_millis(millis);
        self.tree.tick_animations(now);
        self.tree.update_compositor(now).expect("frame");
        self.tree
            .layout(Constraints::loose(Size::new(400., 400.)))
            .expect("layout");
    }

    fn update(&mut self, widget: Widget, millis: u64) {
        self.tree.update(self.root, widget).expect("update");
        self.frame(millis);
        self.frame(millis);
    }

    fn size(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }

    /// The first descendant's bounds relative to the tree root.
    fn leaf_offset(&self) -> incular_core::Offset {
        let mut id = self.root;
        while let Some(children) = self.tree.children(id) {
            let Some(first) = children.first().copied() else {
                break;
            };
            id = first;
        }
        self.tree.element_bounds(id).expect("bounds").origin
    }

    fn opacity(&self) -> f32 {
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if let Some(RenderKind::Opacity { alpha, .. }) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.render_object_kind(render))
            {
                return *alpha;
            }
            stack.extend(self.tree.children(id).unwrap_or_default());
        }
        panic!("no opacity render");
    }
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 0.5
}

fn square() -> Widget {
    Widget::box_(Size::new(50., 50.), Color::WHITE)
}

fn card(width: f32, transition: Transition) -> Widget {
    Container::new()
        .width(width)
        .height(50.)
        .color(Color::WHITE)
        .transition(transition)
}

#[test]
fn container_eases_its_size_with_a_transition() {
    let linear = Transition::linear(ms(200));
    let mut h = Harness::new(card(100., linear.clone()));
    assert_eq!(h.size(), Size::new(100., 50.));
    h.update(card(200., linear), 1_000);
    h.frame(1_050);
    assert!(
        close(h.size().width, 125.),
        "a quarter of the way: {:?}",
        h.size()
    );
    h.frame(1_200);
    h.frame(1_216);
    assert_eq!(h.size(), Size::new(200., 50.));
}

#[test]
fn container_springs_and_retargets_from_the_value_on_screen() {
    let spring = Transition::spring(Spring::snappy());
    let mut h = Harness::new(card(100., spring.clone()));
    h.update(card(300., spring.clone()), 1_000);
    for millis in (1_016..=1_080).step_by(16) {
        h.frame(millis);
    }
    let midway = h.size().width;
    assert!(midway > 110. && midway < 300., "moving: {midway}");
    // Interrupt: the new run starts where the box is now.
    h.update(card(100., spring), 1_096);
    let after = h.size().width;
    assert!((after - midway).abs() < 30., "no jump: {midway} -> {after}");
    for millis in (1_112..=2_500).step_by(16) {
        h.frame(millis);
    }
    assert!(close(h.size().width, 100.), "settles: {:?}", h.size());
}

#[test]
fn opacity_never_springs() {
    let spring = Transition::spring(Spring::bouncy());
    let faded = |alpha: f32| Opacity::new(alpha, square()).transition(spring.clone());
    let mut h = Harness::new(faded(1.));
    h.update(faded(0.), 1_000);
    let mut lowest = 1f32;
    for millis in (1_016..=1_400).step_by(16) {
        h.frame(millis);
        lowest = lowest.min(h.opacity());
    }
    assert!(lowest >= 0., "no overshoot below zero: {lowest}");
    assert!(h.opacity() < 0.01, "short fade finished");
}

#[test]
fn padding_align_and_positioned_transition() {
    let linear = Transition::linear(ms(100));
    let padded =
        |inset: f32| Padding::new(EdgeInsets::all(inset), square()).transition(linear.clone());
    let mut h = Harness::new(padded(0.));
    h.update(padded(40.), 1_000);
    h.frame(1_050);
    assert!(
        close(h.size().width, 90.),
        "padding halfway: {:?}",
        h.size()
    );

    let aligned = |alignment: Alignment| {
        Container::new()
            .width(200.)
            .height(200.)
            .child(Align::new(alignment, square()).transition(linear.clone()))
            .into()
    };
    let mut h = Harness::new(aligned(Alignment::TOP_LEFT));
    h.update(aligned(Alignment::TOP_RIGHT), 1_000);
    h.frame(1_050);
    assert!(
        close(h.leaf_offset().x, 75.),
        "align halfway: {:?}",
        h.leaf_offset()
    );

    let placed = |left: f32| {
        Container::new()
            .width(300.)
            .height(100.)
            .child(Stack::new([Positioned::new(square())
                .left(left)
                .top(0.)
                .transition(linear.clone())]))
            .into()
    };
    let mut h = Harness::new(placed(0.));
    h.update(placed(100.), 1_000);
    h.frame(1_050);
    assert!(
        close(h.leaf_offset().x, 50.),
        "positioned halfway: {:?}",
        h.leaf_offset()
    );
}

#[test]
fn reduced_motion_snaps_sizes() {
    let reduced = |width: f32| -> Widget {
        MotionConfig::new(card(width, Transition::linear(ms(500))))
            .reduced_motion(ReducedMotion::Always)
            .into()
    };
    let mut h = Harness::new(reduced(100.));
    h.update(reduced(200.), 1_000);
    h.frame(1_016);
    assert_eq!(h.size(), Size::new(200., 50.));
}

#[test]
fn animated_widgets_accept_a_transition() {
    let animated = |width: f32| -> Widget {
        AnimatedContainer::new(Duration::from_secs(5))
            .width(width)
            .height(50.)
            .transition(Transition::linear(ms(100)))
            .into()
    };
    let mut h = Harness::new(animated(100.));
    h.update(animated(200.), 1_000);
    h.frame(1_050);
    assert!(
        close(h.size().width, 150.),
        "transition replaces the duration: {:?}",
        h.size()
    );
}
