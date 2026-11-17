//! Shared elements: `shared(id)` starts a newly mounted element at the rect
//! its namesake showed, animating home on the compositor; `shared_bounds`
//! crossfades; `shared_scope()` namespaces ids; leaving hands back.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as CoreTransform};
use incular_rendering::LayerKind;
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition, ms};
use incular_widgets::{Column, Directionality, Row, SizedBox, Stack, TextDirection, Widget};

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    now: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(wrap(widget)).expect("mount");
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
        self.tree.update(self.root, wrap(widget)).expect("update");
        self.frame();
        self.frame();
    }

    fn transforms(&self) -> Vec<CoreTransform> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if let Some(transform) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.content_transform(render))
            {
                found.push(transform);
            }
            let children: Vec<ElementId> = self.tree.children(id).unwrap_or(&[]).to_vec();
            stack.extend(children.into_iter().rev());
        }
        found
    }

    fn translation(&self, n: usize) -> Offset {
        self.transforms()[n].translation_offset()
    }

    fn scale_x(&self, n: usize) -> f32 {
        let [a, ..] = self.transforms()[n].coefficients();
        a as f32
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
                stack.extend(children.iter().rev().copied());
            }
        }
        out
    }
}

fn wrap(widget: Widget) -> Widget {
    Directionality::new(TextDirection::Ltr, widget).into()
}

fn slow() -> Transition {
    Transition::linear(ms(300))
}

fn indicator() -> Widget {
    Widget::box_(Size::new(100., 4.), Color::BLACK)
        .motion(|m| m.shared("tab-indicator").transition(slow()))
}

/// Three 100 px tabs; only the selected one renders the indicator.
fn tabs(selected: usize) -> Widget {
    Row::new((0..3).map(|index| {
        let slot = SizedBox::new().width(100.).height(4.);
        if index == selected {
            Widget::from(slot.child(indicator()))
        } else {
            Widget::from(slot)
        }
    }))
    .into()
}

#[test]
fn a_mounting_element_starts_where_its_namesake_left() {
    let mut h = Harness::new(tabs(0));
    assert!(
        h.translation(0).x.abs() < 0.01,
        "nothing to match on first mount"
    );
    let before = h.tree.diagnostics();
    h.update(tabs(2));
    let start = h.translation(0).x;
    assert!(
        (start + 200.).abs() < 2.,
        "starts over the old tab: {start}"
    );
    h.run(ms(150));
    let mid = h.translation(0).x;
    assert!(mid > start + 10. && mid < -10., "glides: {mid}");
    let layouts = h.tree.diagnostics().layouts;
    h.run(ms(400));
    assert!(h.translation(0).x.abs() < 0.5, "settles in the new tab");
    assert_eq!(
        h.tree.diagnostics().layouts,
        layouts,
        "the glide is compositor-only"
    );
    let _ = before;
}

#[test]
fn a_size_change_animates_as_scale() {
    let small = || -> Widget {
        Widget::box_(Size::new(50., 50.), Color::WHITE)
            .with_key("small")
            .motion(|m| m.shared("card").transition(slow()))
    };
    let large = || -> Widget {
        Widget::box_(Size::new(200., 100.), Color::WHITE)
            .with_key("large")
            .motion(|m| m.shared("card").transition(slow()))
    };
    let mut h = Harness::new(Column::new([small()]).into());
    h.update(Column::new([SizedBox::new().height(10.).into(), large()]).into());
    let scale = h.scale_x(0);
    assert!(
        (scale - 0.25).abs() < 0.02,
        "starts at the old width: {scale}"
    );
    h.run(ms(500));
    assert!((h.scale_x(0) - 1.).abs() < 0.01);
    assert!(h.translation(0).y.abs() < 0.5);
}

#[test]
fn stale_ids_do_not_animate() {
    let mut h = Harness::new(tabs(0));
    h.update(Row::new([SizedBox::new().width(10.).height(4.)]).into());
    std::thread::sleep(Duration::from_millis(600));
    h.update(tabs(2));
    assert!(h.translation(0).x.abs() < 0.01, "the old rect is too old");
}

#[test]
fn scopes_keep_equal_ids_apart() {
    let scoped =
        |child: Widget| -> Widget { SizedBox::new().width(100.).height(50.).child(child).into() };
    let item = || -> Widget {
        Widget::box_(Size::new(20., 20.), Color::WHITE).motion(|m| m.shared("x").transition(slow()))
    };
    let lists = |second: bool| -> Widget {
        let left: Widget = scoped(item()).motion(|m| m.shared_scope());
        let right: Widget = if second {
            scoped(item()).motion(|m| m.shared_scope())
        } else {
            scoped(SizedBox::new().into()).motion(|m| m.shared_scope())
        };
        Row::new([left, right]).into()
    };
    let mut h = Harness::new(lists(false));
    h.update(lists(true));
    assert!(
        h.transforms()
            .iter()
            .all(|t| t.translation_offset().x.abs() < 0.01),
        "the right list's x does not fly from the left list's x"
    );
}

#[test]
fn a_second_place_takes_over_and_hands_back() {
    // A thumbnail stays mounted while a detail view with the same id opens
    // above it; closing the detail hands the element back.
    let thumb = || -> Widget {
        Widget::box_(Size::new(50., 50.), Color::WHITE)
            .motion(|m| m.shared("album").transition(slow()))
    };
    let detail = || -> Widget {
        Widget::box_(Size::new(50., 50.), Color::BLACK)
            .motion(|m| m.shared("album").transition(slow()))
    };
    let scene = |open: bool| -> Widget {
        let mut layers = vec![Column::new([SizedBox::new().height(200.).into(), thumb()]).into()];
        if open {
            layers.push(detail());
        }
        Stack::new(layers).into()
    };
    let mut h = Harness::new(scene(false));
    h.update(scene(true));
    // [thumb, detail]: the detail flies from the thumb (200 px lower).
    assert!(
        (h.translation(1).y - 200.).abs() < 2.,
        "{:?}",
        h.translation(1)
    );
    assert_eq!(h.opacities().first().copied(), Some(0.), "the thumb hides");
    h.run(ms(500));
    h.update(scene(false));
    let back = h.translation(0).y;
    assert!(
        (back + 200.).abs() < 2.,
        "the thumb flies back from the detail: {back}"
    );
    assert!(
        h.opacities().iter().all(|alpha| *alpha > 0.99),
        "visible again"
    );
    h.run(ms(500));
    assert!(h.translation(0).y.abs() < 0.5);
}

#[test]
fn shared_bounds_crossfades_the_newcomer() {
    let a = || -> Widget {
        Widget::box_(Size::new(50., 50.), Color::WHITE)
            .with_key("a")
            .motion(|m| m.shared_bounds("pill").transition(slow()))
    };
    let b = || -> Widget {
        Widget::box_(Size::new(200., 200.), Color::BLACK)
            .with_key("b")
            .motion(|m| m.shared_bounds("pill").transition(slow()))
    };
    let mut h = Harness::new(Column::new([a()]).into());
    h.update(Column::new([b()]).into());
    let alpha = h.opacities()[0];
    assert!(alpha < 0.2, "fades in: {alpha}");
    h.run(ms(500));
    assert!((h.opacities()[0] - 1.).abs() < 0.01);
}

#[test]
fn an_exiting_holder_morphs_into_the_new_bounds_and_fades() {
    let a = || -> Widget {
        Widget::box_(Size::new(50., 50.), Color::WHITE)
            .with_key("a")
            .motion(|m| {
                m.shared_bounds("pill")
                    .exit(|p| p.blur(4.))
                    .transition(slow())
            })
    };
    let b = || -> Widget {
        Widget::box_(Size::new(200., 200.), Color::BLACK)
            .with_key("b")
            .motion(|m| m.shared_bounds("pill").transition(slow()))
    };
    let mut h = Harness::new(Stack::new([a()]).into());
    h.update(Stack::new([b()]).into());
    h.run(ms(150));
    // [a (exiting), b]: a grows toward b's size and fades while b fades in.
    assert_eq!(h.transforms().len(), 2, "a is still mounted while it exits");
    let grow = h.scale_x(0);
    assert!(grow > 1.5 && grow < 4., "a morphs toward b: {grow}");
    let alphas = h.opacities();
    assert!(alphas.len() >= 2, "{alphas:?}");
    assert!(alphas[0] < 0.9 && alphas[1] > 0.1, "crossfade: {alphas:?}");
}
