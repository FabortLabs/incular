//! `layout()` / `layout_position()`: FLIP movement on the compositor, size
//! by relaying out only the element, and scroll-aware measurement.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as CoreTransform};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition, ms};
use incular_widgets::{
    Column, Container, ScrollController, SingleChildScrollView, SizedBox, Widget,
};

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

    /// Every motion transform in tree order.
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

    fn translations(&self) -> Vec<Offset> {
        self.transforms()
            .into_iter()
            .map(|transform| transform.translation_offset())
            .collect()
    }
}

fn item(key: &'static str, color: Color) -> Widget {
    Widget::box_(Size::new(100., 50.), color)
        .with_key(key)
        .motion(|m| m.layout_position().transition(Transition::linear(ms(160))))
}

fn list(order: &[&'static str]) -> Widget {
    Column::new(order.iter().map(|key| item(key, Color::WHITE))).into()
}

#[test]
fn reorders_glide_on_the_compositor() {
    let mut h = Harness::new(list(&["a", "b"]));
    assert!(h.translations().iter().all(|t| t.y.abs() < 0.01));
    h.update(list(&["b", "a"]));
    let start = h.translations();
    // In tree order: b (now at y 0, came from 50), a (now at 50, came from 0).
    assert!(
        (start[0].y - 50.).abs() < 0.5,
        "b starts where it was: {start:?}"
    );
    assert!(
        (start[1].y + 50.).abs() < 0.5,
        "a starts where it was: {start:?}"
    );
    let before = h.tree.diagnostics();
    h.run(ms(80));
    let mid = h.translations();
    assert!(mid[0].y > 5. && mid[0].y < 45., "gliding: {mid:?}");
    let after = h.tree.diagnostics();
    assert_eq!(after.layouts, before.layouts, "movement never relayouts");
    h.run(ms(200));
    assert!(h.translations().iter().all(|t| t.y.abs() < 0.01), "settled");
}

#[test]
fn layout_animates_size_by_relaying_out_the_element() {
    let sized = |width: f32| -> Widget {
        Column::new([Widget::from(
            Container::new()
                .width(width)
                .height(40.)
                .color(Color::WHITE),
        )
        .with_key("card")
        .motion(|m| m.layout().transition(Transition::linear(ms(160))))])
        .into()
    };
    let mut h = Harness::new(sized(100.));
    let card = |h: &Harness| {
        let column = h.tree.children(h.root).unwrap()[0];
        h.tree.element_bounds(column).unwrap().size
    };
    assert_eq!(card(&h), Size::new(100., 40.));
    h.update(sized(200.));
    h.run(ms(80));
    let mid = card(&h);
    assert!(mid.width > 120. && mid.width < 180., "growing: {mid:?}");
    h.run(ms(200));
    assert_eq!(card(&h), Size::new(200., 40.));
}

#[test]
fn layout_position_jumps_size() {
    let sized = |width: f32| -> Widget {
        Column::new([Widget::from(
            Container::new()
                .width(width)
                .height(40.)
                .color(Color::WHITE),
        )
        .with_key("card")
        .motion(|m| m.layout_position().transition(Transition::linear(ms(160))))])
        .into()
    };
    let mut h = Harness::new(sized(100.));
    h.update(sized(200.));
    h.frame();
    let column = h.tree.children(h.root).unwrap()[0];
    assert_eq!(
        h.tree.element_bounds(column).unwrap().size,
        Size::new(200., 40.)
    );
}

#[test]
fn scrolling_never_animates_layout() {
    let controller = ScrollController::new();
    let content = Column::new([
        Widget::from(SizedBox::new().width(100.).height(300.)),
        item("probe", Color::BLACK),
        Widget::from(SizedBox::new().width(100.).height(600.)),
    ]);
    let mut h = Harness::new(
        SingleChildScrollView::new(content)
            .controller(controller.clone())
            .into(),
    );
    controller.jump_to(150.);
    h.run(ms(32));
    assert!(
        h.translations().iter().all(|t| t.y.abs() < 0.01),
        "{:?}",
        h.translations()
    );
}
