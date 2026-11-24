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
