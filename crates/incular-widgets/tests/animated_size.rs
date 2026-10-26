//! Flutter's `AnimatedSize` (`RenderAnimatedSize`): the box animates from
//! its laid-out size to its child's new size, aligning the child meanwhile.

use std::time::{Duration, Instant};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{AnimatedSize, Widget};

fn sized(width: f32) -> Widget {
    AnimatedSize::new(
        Duration::from_millis(300),
        Widget::box_(Size::new(width, 50.), Color::WHITE),
    )
    .into()
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    base: Instant,
    constraints: Constraints,
}

impl Harness {
    fn new(widget: Widget, constraints: Constraints) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            base: Instant::now(),
            constraints,
        };
        harness.frame(0);
        harness.frame(16);
        harness
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree.layout(self.constraints).expect("layout");
    }

    fn size(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }
}

#[test]
fn the_box_animates_to_the_childs_new_size() {
    let mut harness = Harness::new(sized(50.), Constraints::loose(Size::new(200., 200.)));
    assert_eq!(harness.size(), Size::new(50., 50.));
    harness
        .tree
        .update(harness.root, sized(100.))
        .expect("update");
    // The rebuild lays out the larger child and starts the animation from
    // the current size; the ticker starts on the next frame.
    harness.frame(1_000);
    assert_eq!(harness.size(), Size::new(50., 50.));
    harness.frame(1_000);
    harness.frame(1_150);
    // Linear: halfway from 50 to 100.
    assert_eq!(harness.size(), Size::new(75., 50.));
    harness.frame(1_300);
    harness.frame(1_316);
    assert_eq!(harness.size(), Size::new(100., 50.));
}

#[test]
fn the_curve_shapes_the_size() {
    let make = |width: f32| -> Widget {
        AnimatedSize::new(
            Duration::from_millis(300),
            Widget::box_(Size::new(width, 50.), Color::WHITE),
        )
        .curve(Curve::EaseIn)
        .into()
    };
    let mut harness = Harness::new(make(50.), Constraints::loose(Size::new(200., 200.)));
    harness
        .tree
        .update(harness.root, make(100.))
        .expect("update");
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    let expected = 50. + 50. * Curve::EaseIn.apply(0.5);
    assert!((harness.size().width - expected).abs() < 1e-3);
}

#[test]
fn tight_constraints_decide_the_size() {
    let mut harness = Harness::new(sized(50.), Constraints::tight(Size::new(80., 60.)));
    assert_eq!(harness.size(), Size::new(80., 60.));
    harness
        .tree
        .update(harness.root, sized(100.))
        .expect("update");
    harness.frame(1_000);
    harness.frame(1_150);
    assert_eq!(harness.size(), Size::new(80., 60.));
}

#[test]
fn the_child_is_aligned_while_the_size_differs() {
    let mut harness = Harness::new(sized(50.), Constraints::loose(Size::new(200., 200.)));
    harness
        .tree
        .update(harness.root, sized(100.))
        .expect("update");
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    // `alignChild` with the default center alignment: the 100 px child
    // sits at (75 - 100) / 2 = -12.5 in the 75 px box.
    let mut leaf = harness.root;
    while let Some(&child) = harness.tree.children(leaf).and_then(|kids| kids.first()) {
        leaf = child;
    }
    let bounds = harness.tree.element_bounds(leaf).expect("bounds");
    assert_eq!(bounds.size, Size::new(100., 50.));
    assert_eq!(bounds.origin, Offset::new(-12.5, 0.));
}
