//! `shared(id)` across navigator routes: no hero controller, the routes
//! share one tree, so a pushed page's element flies from the page below and
//! hands back when the page is popped.

use std::{cell::Cell, rc::Rc};

use incular::prelude::{
    Color, Directionality, KeyedSubtree, Navigator, Offset, Positioned, Rect, Route, Size,
    SizedBox, Stack, TextDirection, Widget,
};
use incular_config::Constraints;
use incular_core::{Transform, time::Instant};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{MotionExt, Transition, ms};

const WIDTH: f32 = 200.;
const HEIGHT: f32 = 300.;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

/// A page with a shared "photo" box at `at`.
fn page(at: Rect, color: Color) -> Widget {
    let photo = Widget::box_(at.size, color)
        .motion(|m| m.shared("photo").transition(Transition::linear(ms(300))));
    SizedBox::from_dimensions(
        Some(WIDTH),
        Some(HEIGHT),
        Some(Stack::new([Widget::from(
            Positioned::new(photo).left(at.origin.x).top(at.origin.y),
        )])),
    )
    .into()
}

struct Harness {
    tree: WidgetTree,
    navigator: Navigator,
    now: Instant,
}

impl Harness {
    fn new(navigator: Navigator) -> Self {
        let revision = Rc::new(Cell::new(0_u64));
        let observer = {
            let revision = revision.clone();
            navigator.observe(move |_| revision.set(revision.get() + 1))
        };
        let routes = navigator.clone();
        let root = Widget::stateful_builder(revision, move |_| {
            let _observer = &observer;
            let children: Vec<Widget> = routes
                .routes()
                .iter()
                .map(|route| KeyedSubtree::new(route.name.as_str(), route.presented_child()).into())
                .collect();
            Stack::new(children).into()
        });
        let mut tree = WidgetTree::new();
        tree.mount(Directionality::new(TextDirection::Ltr, root).into())
            .expect("mount");
        let mut harness = Self {
            tree,
            navigator,
            now: Instant::now(),
        };
        harness.frame();
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        let _ = self.tree.paint();
    }

    fn run(&mut self, millis: u64) {
        for _ in 0..millis / 16 {
            self.now += ms(16);
            self.frame();
        }
    }

    /// Motion transforms in tree order: [home photo, next photo].
    fn transforms(&self) -> Vec<Transform> {
        let mut found = Vec::new();
        let mut stack = vec![self.tree.root().expect("root")];
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

    fn scale_x(&self, n: usize) -> f32 {
        let [a, ..] = self.transforms()[n].coefficients();
        a as f32
    }
}

#[test]
fn a_pushed_page_takes_the_element_and_a_pop_hands_it_back() {
    let navigator = Navigator::new();
    navigator.push(Route::new(
        "home",
        page(rect(10., 20., 40., 40.), Color::WHITE),
    ));
    let mut h = Harness::new(navigator);
    h.navigator.push(Route::new(
        "next",
        page(rect(100., 150., 80., 60.), Color::BLACK),
    ));
    h.frame();
    h.frame();
    let start = h.scale_x(1);
    assert!(
        (start - 0.5).abs() < 0.02,
        "starts at the thumbnail's width: {start}"
    );
    h.run(500);
    assert!((h.scale_x(1) - 1.).abs() < 0.01, "lands on the detail page");

    let _ = h.navigator.pop();
    h.frame();
    h.frame();
    let back = h.scale_x(0);
    assert!(
        (back - 2.).abs() < 0.05,
        "the thumbnail flies back from the detail: {back}"
    );
    h.run(500);
    assert!((h.scale_x(0) - 1.).abs() < 0.01);
}
