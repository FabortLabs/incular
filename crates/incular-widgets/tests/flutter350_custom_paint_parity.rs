//! Public CustomPaint phases and box sizing compared with Flutter 3.50.0.
use incular_config::Constraints;
use incular_core::Size;
use incular_rendering::DisplayList;
use incular_widgets::{
    Center, Container, CustomPaint, CustomPainter, IntrinsicHeight, IntrinsicWidth, SizedBox,
    Widget,
    internal::{Key, WidgetTree},
};
use std::{cell::RefCell, rc::Rc};

const KEY: Key = Key::Value(500);
fn layout(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::tight(size)).unwrap();
}
fn size(tree: &WidgetTree) -> Size {
    tree.element_bounds(tree.element_with_key(&KEY).unwrap())
        .unwrap()
        .size
}
fn paint(preferred: Size) -> CustomPaint {
    CustomPaint::new(preferred, DisplayList::default())
}
struct Trace(&'static str, Rc<RefCell<Vec<&'static str>>>);
impl CustomPainter for Trace {
    fn paint(&self, _: Size) -> DisplayList {
        self.1.borrow_mut().push(self.0);
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
}

#[test]
fn paint_calls_background_then_child_then_foreground() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let child = CustomPaint::from_painter(Size::ZERO, Trace("child", trace.clone()));
    let root = CustomPaint::from_painter(Size::ZERO, Trace("background", trace.clone()))
        .foreground_painter(Trace("foreground", trace.clone()))
        .child(child);
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    layout(&mut tree, Size::new(800., 600.));
    let _ = tree.paint();
    assert_eq!(*trace.borrow(), ["background", "child", "foreground"]);
}

#[test]
fn preferred_size_is_constrained_and_child_replaces_preferred_size() {
    let variants = [
        paint(Size::ZERO),
        paint(Size::ZERO).child(Container::new()),
        paint(Size::new(20., 20.)),
        paint(Size::new(2000., 100.)),
        paint(Size::ZERO).child(Container::new()),
        paint(Size::ZERO).child(SizedBox::shrink()),
    ];
    let expected = [
        Size::ZERO,
        Size::new(800., 600.),
        Size::new(20., 20.),
        Size::new(800., 100.),
        Size::new(800., 600.),
        Size::ZERO,
    ];
    let mut tree = WidgetTree::new();
    let mut actual = Vec::new();
    let mut root = None;
    for p in variants {
        let w = Center::new(Widget::from(p).with_key(KEY)).into();
        if let Some(root) = root {
            tree.update(root, w).unwrap();
        } else {
            root = Some(tree.mount(w).unwrap());
        }
        layout(&mut tree, Size::new(800., 600.));
        actual.push(size(&tree));
    }
    assert_eq!(actual, expected);
}

fn intrinsic(dimensions: CustomPaint) -> [f32; 2] {
    [false, true].map(|height| {
        let leaf = Widget::from(dimensions.clone()).with_key(KEY);
        let child = if height {
            Widget::from(IntrinsicHeight::new(leaf))
        } else {
            Widget::from(IntrinsicWidth::new(leaf))
        };
        let mut tree = WidgetTree::new();
        tree.mount(Center::new(child).into()).unwrap();
        layout(&mut tree, Size::new(800., 600.));
        let size = size(&tree);
        if height { size.height } else { size.width }
    })
}
#[test]
fn finite_preferred_size_is_reported_to_intrinsic_wrappers() {
    assert_eq!(intrinsic(paint(Size::new(20., 30.))), [20., 30.]);
}
#[test]
fn fill_preferred_size_does_not_report_infinite_intrinsics() {
    assert_eq!(intrinsic(paint(Size::ZERO).fill_to_constraints()), [0., 0.]);
}
#[test]
fn default_custom_paint_lays_out_and_paints_at_zero_area() {
    let mut tree = WidgetTree::new();
    let w = Center::new(Widget::from(paint(Size::ZERO)).with_key(KEY));
    tree.mount(w.into()).unwrap();
    layout(&mut tree, Size::ZERO);
    let _ = tree.paint();
    assert_eq!(size(&tree), Size::ZERO);
}
