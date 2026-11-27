//! Flutter 3.50.0 rendering/independent_layout_test.dart.
use std::{cell::Cell, rc::Rc, time::Instant};

use incular_config::Constraints;
use incular_core::Size;
use incular_rendering::DisplayList;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, CustomPaint, CustomPainter, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const CHILD: Size = Size {
    width: 800.,
    height: 10.,
};

struct Painter(Rc<Cell<Option<Size>>>);

impl CustomPainter for Painter {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.set(Some(size));
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

struct Scene {
    widget: Widget,
    painted: Rc<Cell<Option<Size>>>,
}

impl Scene {
    fn new() -> Self {
        let painted = Rc::new(Cell::new(None));
        let child = Widget::from(SizedBox::square(10.)).with_key(1_u64);
        let paint = CustomPaint::from_painter(Size::ZERO, Painter(painted.clone())).child(child);
        let widget = Center::new(SizedBox::new().width(800.).child(paint)).into();
        Self { widget, painted }
    }

    fn expect(&self, tree: &WidgetTree, size: Option<Size>, painted: Option<Size>) {
        let child = tree.element_with_key(&Key::from(1_u64)).unwrap();
        assert_eq!(tree.element_layout_size(child), size);
        assert_eq!(self.painted.get(), painted);
    }

    fn drive_offscreen(&self, tree: &mut WidgetTree) {
        tree.layout(Constraints::tight(VIEW)).unwrap();
        self.expect(tree, Some(CHILD), None);
        tree.update_compositor(Instant::now()).unwrap();
        self.expect(tree, Some(CHILD), None);
        let _ = tree.paint();
        self.expect(tree, Some(CHILD), Some(CHILD));
    }
}

fn verify(onscreen_first: bool) {
    let onscreen = Scene::new();
    let offscreen = Scene::new();
    let mut live = Runtime::new(onscreen.widget.clone()).unwrap();
    let mut isolated = WidgetTree::new();
    isolated.mount(offscreen.widget.clone()).unwrap();
    onscreen.expect(live.tree(), None, None);
    offscreen.expect(&isolated, None, None);
    let live_frame = |live: &mut Runtime| {
        live.run_frame(Constraints::tight(VIEW)).unwrap();
        onscreen.expect(live.tree(), Some(CHILD), Some(CHILD));
    };
    if onscreen_first {
        live_frame(&mut live);
        offscreen.expect(&isolated, None, None);
        offscreen.drive_offscreen(&mut isolated);
    } else {
        offscreen.drive_offscreen(&mut isolated);
        onscreen.expect(live.tree(), None, None);
        live_frame(&mut live);
    }
}

#[test]
fn onscreen_layout_does_not_touch_the_independent_tree() {
    verify(true);
}

#[test]
fn independent_layout_does_not_touch_the_onscreen_tree() {
    verify(false);
}
