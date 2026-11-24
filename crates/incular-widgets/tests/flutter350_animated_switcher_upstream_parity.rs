//! Mounted Flutter 3.50.0 AnimatedSwitcher transition lifecycle.
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::{
    AnimatedSwitcher, Column, Widget,
    internal::{ElementId, Key, RenderKind, WidgetTree},
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const DURATION: Duration = Duration::from_millis(100);

fn card(key: Option<u64>, color: Color) -> Widget {
    let widget = Widget::box_(Size::new(50., 50.), color);
    key.map_or_else(
        || widget.clone(),
        |id| widget.clone().with_key(Key::Value(id)),
    )
}
fn switcher(child: Option<Widget>) -> Widget {
    AnimatedSwitcher::new(DURATION, child).into()
}
struct Scene {
    tree: WidgetTree,
    root: ElementId,
    zero: Instant,
}
impl Scene {
    fn new(child: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(child).unwrap();
        let mut scene = Self {
            tree,
            root,
            zero: Instant::now(),
        };
        scene.frame(0);
        scene
    }
    fn frame(&mut self, ms: u64) {
        self.tree
            .update_compositor(self.zero + Duration::from_millis(ms))
            .unwrap();
        self.tree
            .layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    }
    fn replace(&mut self, child: Option<Widget>, ms: u64) {
        self.tree.update(self.root, switcher(child)).unwrap();
        self.frame(ms);
        self.frame(ms);
    }
    fn alphas(&self) -> Vec<f32> {
        let mut rem = vec![self.root];
        let mut out = Vec::new();
        while let Some(id) = rem.pop() {
            if let Some(RenderKind::Opacity { alpha, .. }) = self
                .tree
                .render_id(id)
                .and_then(|id| self.tree.render_object_kind(id))
            {
                out.push(*alpha);
            } else if let Some(children) = self.tree.children(id) {
                rem.extend(children.iter().rev().copied());
            }
        }
        out
    }
    fn mounted(&self) -> [bool; 3] {
        [1, 2, 3].map(|id| self.tree.element_with_key(&Key::Value(id)).is_some())
    }
}
fn assert_approx(actual: &[f32], expected: &[f32]) {
    assert!(
        actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() <= 0.01),
        "wanted {expected:?}, observed {actual:?}"
    );
}

#[test]
fn interrupted_fade_samples_all_three_mounted_transitions() {
    let mut s = Scene::new(switcher(Some(card(Some(1), Color::TRANSPARENT))));
    assert_eq!(s.alphas(), [1.]);
    s.replace(Some(card(Some(2), Color::BLACK)), 1000);
    s.frame(1050);
    let half = s.alphas();
    assert_eq!(half.len(), 2);
    assert_eq!(half[0], 0.5);
    s.replace(Some(card(Some(3), Color::rgba(255, 0, 0, 255))), 1050);
    s.frame(1060);
    let interrupted = s.alphas();
    s.frame(1200);
    s.frame(1216);
    assert_approx(&interrupted, &[0.4, 0.4, 0.1]);
    assert_eq!(s.mounted(), [false, false, true]);
}

#[test]
fn back_to_back_changes_discard_a_never_visible_intermediate_child() {
    let mut s = Scene::new(switcher(Some(card(Some(1), Color::WHITE))));
    let initial = s.mounted();
    s.replace(Some(card(Some(2), Color::WHITE)), 1000);
    let next = s.mounted();
    s.replace(Some(card(Some(3), Color::WHITE)), 1000);
    let last = s.mounted();
    assert_eq!(
        [initial, next, last],
        [
            [true, false, false],
            [true, true, false],
            [true, false, true]
        ]
    );
}

#[test]
fn same_type_unkeyed_edit_does_not_transition() {
    let mut s = Scene::new(switcher(Some(card(None, Color::TRANSPARENT))));
    assert_eq!(s.alphas(), [1.]);
    s.replace(Some(card(None, Color::BLACK)), 1000);
    s.frame(1050);
    assert_eq!(s.alphas(), [1.]);
}

#[test]
fn null_children_fade_in_and_out_without_restarting_the_outgoing_run() {
    let mut s = Scene::new(switcher(None));
    assert!(s.alphas().is_empty());
    s.replace(Some(card(None, Color::BLACK)), 1000);
    s.frame(1050);
    assert_eq!(s.alphas(), [0.5]);
    s.frame(1100);
    s.frame(1116);
    s.replace(Some(card(None, Color::TRANSPARENT)), 1200);
    assert_eq!(s.alphas(), [1.]);
    s.replace(None, 1300);
    s.frame(1350);
    assert_eq!(s.alphas(), [0.5]);
    s.replace(None, 1350);
    s.frame(1400);
    let at_end = s.alphas();
    s.frame(1416);
    assert!(s.alphas().is_empty());
    assert_eq!(at_end, [0.]);
}

#[test]
fn a_custom_layout_builder_mounts_its_returned_subtree() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let observed = seen.clone();
    let switcher = AnimatedSwitcher::new(DURATION, Some(card(None, Color::TRANSPARENT)))
        .layout_builder(move |child, mut previous| {
            observed
                .borrow_mut()
                .push((child.is_some(), previous.len()));
            previous.extend(child);
            Widget::from(Column::new(previous)).with_key(Key::Value(12))
        });
    let s = Scene::new(switcher.into());
    assert!(s.tree.element_with_key(&Key::Value(12)).is_some());
    assert!(seen.borrow().iter().all(|&call| call == (true, 0)));
    assert_eq!(s.alphas(), [1.]);
}

fn configured(key: u64, reverse_ms: u64, curve: incular_animation::Curve) -> Widget {
    AnimatedSwitcher::new(
        Duration::from_millis(200),
        Some(card(Some(key), Color::WHITE)),
    )
    .reverse_duration(Duration::from_millis(reverse_ms))
    .switch_in_curve(curve)
    .into()
}

fn replace_configured(scene: &mut Scene, widget: Widget, ms: u64) {
    scene.tree.update(scene.root, widget).unwrap();
    scene.frame(ms);
    scene.frame(ms);
}

#[test]
fn outgoing_entry_keeps_its_reverse_duration_when_next_entry_changes_it() {
    use incular_animation::Curve;
    let mut snapshots = Vec::new();
    for incoming_ms in [200, 800] {
        let mut s = Scene::new(configured(1, 200, Curve::Linear));
        replace_configured(&mut s, configured(2, incoming_ms, Curve::Linear), 1000);
        s.frame(1100);
        assert_eq!(s.mounted(), [true, true, false]);
        snapshots.push(s.alphas()[0]);
    }
    assert_eq!(
        snapshots,
        [0.5, 0.5],
        "old alpha at 100ms for incoming reverse 200ms and 800ms"
    );
}

#[test]
fn currently_mounted_entry_keeps_its_curve_after_same_key_rebuild() {
    use incular_animation::Curve;
    let mut s = Scene::new(configured(1, 200, Curve::Linear));
    replace_configured(&mut s, configured(2, 200, Curve::Linear), 1000);
    s.frame(1100);
    assert_eq!(s.alphas().last(), Some(&0.5));
    let current = s
        .tree
        .element_with_key(&Key::Value(2))
        .expect("visible child identity");
    replace_configured(&mut s, configured(2, 200, Curve::Linear), 1100);
    assert_eq!(s.alphas().last(), Some(&0.5));
    replace_configured(&mut s, configured(2, 200, Curve::EaseIn), 1100);
    assert_eq!(s.tree.element_with_key(&Key::Value(2)), Some(current));
    assert_eq!(s.alphas().last(), Some(&0.5), "no elapsed time");
}

fn render_kinds(scene: &Scene, predicate: impl Fn(&RenderKind) -> bool) -> usize {
    let mut pending = vec![scene.root];
    let mut found = 0;
    while let Some(element) = pending.pop() {
        found += usize::from(
            scene
                .tree
                .render_id(element)
                .and_then(|render| scene.tree.render_object_kind(render))
                .is_some_and(&predicate),
        );
        if let Some(children) = scene.tree.children(element) {
            pending.extend(children);
        }
    }
    found
}

#[test]
fn replacing_the_builder_changes_all_mounted_transitions() {
    let scene = |id: Option<u64>, scaled| {
        let mut switcher = AnimatedSwitcher::new(DURATION, Some(card(id, Color::WHITE)))
            .layout_builder(|child, previous| {
                Widget::from(Column::new(child.into_iter().chain(previous)))
            });
        if scaled {
            switcher = switcher.transition_builder(|child, alpha| {
                Widget::from(incular_widgets::Transform::scale(alpha, child))
            });
        }
        Widget::from(switcher)
    };
    let mut s = Scene::new(scene(Some(1), false));
    s.frame(10);
    replace_configured(&mut s, scene(Some(2), false), 10);
    s.frame(20);
    replace_configured(&mut s, scene(Some(3), false), 20);
    s.frame(30);
    let before = s.alphas().len();
    replace_configured(&mut s, scene(None, true), 30);
    s.frame(40);
    let scales = render_kinds(&s, |kind| matches!(kind, RenderKind::Transform { .. }));
    assert_eq!((before, s.alphas().len(), scales), (3, 0, 3));
}

#[test]
fn reentering_the_same_key_does_not_leave_a_second_copy() {
    let duration = Duration::from_secs(1);
    let branch = |id| {
        let color = if id == 1 { Color::WHITE } else { Color::BLACK };
        Widget::from(AnimatedSwitcher::new(duration, Some(card(Some(id), color))))
    };
    let mut s = Scene::new(branch(1));
    replace_configured(&mut s, branch(2), 10);
    replace_configured(&mut s, branch(1), 10);
    s.frame(1010);
    let copies = render_kinds(
        &s,
        |kind| matches!(kind,RenderKind::Box {color,..} if *color == Color::WHITE),
    );
    assert_eq!(copies, 1, "mounted same-key visual children");
}

#[test]
fn zero_area_stays_zero_while_adding_and_replacing_text() {
    use incular_widgets::{Center, Directionality, SizedBox, Text, TextDirection};
    let build = |child: Option<(&str, u64)>| {
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            Center::new(
                SizedBox::shrink().child(
                    Widget::from(AnimatedSwitcher::new(
                        Duration::from_micros(500),
                        child.map(|(label, key)| {
                            Widget::from(Text::new(label)).with_key(Key::Value(key))
                        }),
                    ))
                    .with_key(Key::Value(40)),
                ),
            ),
        ))
    };
    let mut s = Scene::new(build(None));
    for (child, ms) in [(None, 0), (Some(("X", 1)), 10), (Some(("Y", 2)), 20)] {
        replace_configured(&mut s, build(child), ms);
        let target = s.tree.element_with_key(&Key::Value(40)).unwrap();
        assert_eq!(s.tree.element_bounds(target).unwrap().size, Size::ZERO);
    }
}
