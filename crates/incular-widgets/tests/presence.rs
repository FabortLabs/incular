//! Presence: `enter` on mount, `exit` through departing elements, re-entry,
//! exit layouts and keyed replacement.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_rendering::LayerKind;
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{ExitLayout, MotionExt, Presence, Transition, ms};
use incular_widgets::{Column, SizedBox, Widget};

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
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::loose(Size::new(200., 400.)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        let _ = self.tree.paint();
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
    }

    fn children(&self) -> Vec<ElementId> {
        self.tree.children(self.root).unwrap_or(&[]).to_vec()
    }

    fn opacities(&self) -> Vec<f32> {
        let layers = self.tree.compositor_layers();
        let mut out = Vec::new();
        let mut stack: Vec<_> = layers.root().into_iter().collect();
        while let Some(id) = stack.pop() {
            if let Some((kind, children)) = layers.layer(id) {
                if let LayerKind::Opacity { alpha } = kind {
                    out.push(*alpha);
                }
                stack.extend(children.iter().copied());
            }
        }
        out
    }
}

fn block(color: Color) -> Widget {
    Widget::box_(Size::new(100., 100.), color)
}

fn fading(key: &'static str) -> Widget {
    block(Color::WHITE).with_key(key).motion(|m| {
        m.enter(|p| p.opacity(0.))
            .exit(|p| p.opacity(0.))
            .transition(Transition::linear(ms(200)))
    })
}

#[test]
fn enter_plays_on_mount_only() {
    let list = |extra: bool| -> Widget {
        let mut children = vec![fading("a")];
        if extra {
            children.push(block(Color::BLACK));
        }
        Column::new(children).into()
    };
    let mut h = Harness::new(list(false));
    assert!(h.opacities().iter().any(|alpha| *alpha < 0.01), "entering");
    h.run(ms(300));
    assert!(h.opacities().iter().all(|alpha| *alpha > 0.99));
    h.update(list(true));
    h.run(ms(32));
    assert!(
        h.opacities().iter().all(|alpha| *alpha > 0.99),
        "a rebuild does not replay"
    );
}

#[test]
fn removed_elements_exit_inert_then_leave() {
    let list = |with_b: bool| -> Widget {
        let mut children = vec![block(Color::BLACK).with_key("top")];
        if with_b {
            children.push(fading("b"));
        }
        Column::new(children).into()
    };
    let mut h = Harness::new(list(true));
    h.run(ms(300));
    let inside_b = Offset::new(100., 150.);
    let hit_before = h.tree.hit_test(inside_b);
    assert_eq!(h.children().len(), 2);
    h.update(list(false));
    assert_eq!(h.children().len(), 2, "kept while exiting");
    assert_ne!(
        h.tree.hit_test(inside_b),
        hit_before,
        "departing elements take no hits"
    );
    h.run(ms(96));
    let alpha = h.opacities().into_iter().fold(1f32, f32::min);
    assert!(alpha > 0.1 && alpha < 0.9, "fading out: {alpha}");
    h.run(ms(300));
    assert_eq!(h.children().len(), 1, "gone once the exit settles");
}

#[test]
fn reinserting_mid_exit_reverses_without_remounting() {
    let list = |with_b: bool| -> Widget {
        let mut children = vec![block(Color::BLACK).with_key("top")];
        if with_b {
            children.push(fading("b"));
        }
        Column::new(children).into()
    };
    let mut h = Harness::new(list(true));
    h.run(ms(300));
    let b = h.children()[1];
    h.update(list(false));
    h.run(ms(96));
    h.update(list(true));
    assert_eq!(
        h.children(),
        vec![h.children()[0], b],
        "same element revived"
    );
    h.run(ms(400));
    assert!(
        h.opacities().iter().all(|alpha| *alpha > 0.99),
        "back at rest"
    );
    assert_eq!(h.children().len(), 2);
}

#[test]
fn pop_frees_the_slot_at_once() {
    let popping = block(Color::WHITE).with_key("b").motion(|m| {
        m.exit(|p| p.opacity(0.))
            .exit_layout(ExitLayout::Pop)
            .transition(Transition::linear(ms(200)))
    });
    let list = |with_b: bool| -> Widget {
        let mut children = Vec::new();
        if with_b {
            children.push(popping.clone());
        }
        children.push(Widget::from(SizedBox::new().width(50.).height(50.)).with_key("c"));
        Column::new(children).into()
    };
    let mut h = Harness::new(list(true));
    let c_before = h
        .tree
        .element_bounds(*h.children().last().unwrap())
        .unwrap();
    assert!((c_before.origin.y - 100.).abs() < 0.5);
    h.update(list(false));
    h.frame();
    let children = h.children();
    assert_eq!(children.len(), 2, "b still paints while leaving");
    let c_now = h.tree.element_bounds(*children.last().unwrap()).unwrap();
    assert!(c_now.origin.y.abs() < 0.5, "c moved up at once: {c_now:?}");
    h.run(ms(300));
    assert_eq!(h.children().len(), 1);
}

#[test]
fn presence_wait_swaps_after_the_exit() {
    let page = |key: &'static str| -> Widget {
        Widget::from(SizedBox::new().width(80.).height(80.))
            .with_key(key)
            .motion(|m| {
                m.enter(|p| p.opacity(0.))
                    .exit(|p| p.opacity(0.))
                    .transition(Transition::linear(ms(100)))
            })
    };
    let host = |key: &'static str, mode: Presence| -> Widget {
        Column::new([page(key).motion(move |m| m.presence(mode))]).into()
    };
    // Sync: both pages are present during the swap.
    let mut h = Harness::new(host("one", Presence::Sync));
    h.run(ms(200));
    h.update(host("two", Presence::Sync));
    h.run(ms(32));
    assert_eq!(stack_children(&h), 2, "sync overlaps");
    h.run(ms(300));
    assert_eq!(stack_children(&h), 1);

    // Wait: the new page mounts only after the old one has left.
    let mut h = Harness::new(host("one", Presence::Wait));
    h.run(ms(200));
    h.update(host("two", Presence::Wait));
    h.run(ms(48));
    assert_eq!(stack_children(&h), 1, "old page still exiting alone");
    let alpha = h.opacities()[0];
    assert!(alpha < 0.9, "the old page is fading out: {alpha}");
    h.run(ms(400));
    assert_eq!(stack_children(&h), 1, "new page shown");
}

/// The pages mounted under the presence parent (one opacity layer each).
fn stack_children(h: &Harness) -> usize {
    h.opacities().len()
}

#[test]
fn only_the_topmost_site_decides() {
    // The removed element's topmost motion site has no exit, so the inner
    // site's exit does not run: the element leaves at once.
    let nested = fading("inner").motion(|m| m.hovered(|p| p.scale(1.1)));
    let list = |with: bool| -> Widget {
        let mut children = vec![block(Color::BLACK).with_key("top")];
        if with {
            children.push(nested.clone());
        }
        Column::new(children).into()
    };
    let mut h = Harness::new(list(true));
    h.run(ms(300));
    h.update(list(false));
    assert_eq!(h.children().len(), 1);
}
