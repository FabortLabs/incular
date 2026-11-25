//! Pinned Flutter render box layout, paint suppression, and retained reparenting.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_rendering::DisplayList;
use incular_widgets::{
    Center, CustomPaint, CustomPainter, CustomSingleChildLayout, FractionallySizedBox, GlobalKey,
    Offstage, SingleChildLayoutDelegate, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
}

fn size(tree: &WidgetTree, key: u64) -> Size {
    let id = tree.element_with_key(&Key::from(key)).unwrap();
    tree.element_bounds(id).unwrap().size
}

// rendering/constraints_test.dart:13
#[test]
fn fractions_size_the_child_without_expanding_the_tight_parent() {
    let leaf = Widget::from(SizedBox::expand()).with_key(1_u64);
    let test = Widget::from(
        FractionallySizedBox::new(leaf)
            .width_factor(2.)
            .height_factor(0.5),
    )
    .with_key(2_u64);
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(SizedBox::square(200.).child(test)).into())
        .unwrap();
    layout(&mut tree);
    assert_eq!(tree.root_layout_size(), Some(VIEW));
    assert_eq!(size(&tree, 2), Size::new(200., 200.));
    assert_eq!(size(&tree, 1), Size::new(400., 100.));
}

struct PaintCounter(Rc<Cell<u32>>);

impl CustomPainter for PaintCounter {
    fn paint(&self, _size: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        DisplayList::default()
    }

    fn should_repaint(&self, _old: &Self) -> bool {
        false
    }
}

// rendering/offstage_test.dart:13
#[test]
fn offstage_lays_out_a_constrained_descendant_without_painting() {
    let calls = Rc::new(Cell::new(0));
    let leaf = Widget::from(SizedBox::square(10.)).with_key(1_u64);
    let child = Widget::from(
        CustomPaint::from_painter(Size::ZERO, PaintCounter(calls.clone())).child(leaf),
    );
    let scene = |offstage| {
        Center::new(
            SizedBox::new()
                .width(800.)
                .child(Offstage::new(child.clone()).offstage(offstage)),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(true)).unwrap();
    assert_eq!(size(&tree, 1), Size::ZERO);
    assert_eq!(calls.get(), 0);
    layout(&mut tree);
    let _ = tree.paint();
    assert_eq!(size(&tree, 1), Size::new(800., 10.));
    assert_eq!(calls.get(), 0);

    tree.update(root, scene(false)).unwrap();
    layout(&mut tree);
    let _ = tree.paint();
    assert_eq!(size(&tree, 1), Size::new(800., 10.));
    assert_eq!(calls.get(), 1);
}

struct LayoutCounter(Rc<Cell<u32>>);

impl SingleChildLayoutDelegate for LayoutCounter {
    fn get_size(&self, constraints: Constraints) -> Size {
        self.0.set(self.0.get() + 1);
        constraints.biggest()
    }

    fn should_relayout(&self, _old: &Self) -> bool {
        false
    }
}

// rendering/relayout_boundary_test.dart:9
#[test]
fn moving_a_global_subtree_across_identically_sized_wrappers_does_not_relayout_it() {
    let calls = Rc::new(Cell::new(0));
    let key = GlobalKey::new();
    let counter = CustomSingleChildLayout::new(LayoutCounter(calls.clone()), SizedBox::shrink());
    let keyed = Widget::from(Center::new(counter)).with_key(key);
    let mut tree = WidgetTree::new();
    let original = Center::new(SizedBox::square(100.).child(Center::new(
        SizedBox::square(100.).child(Center::new(keyed.clone())),
    )));
    let root = tree.mount(original.into()).unwrap();
    layout(&mut tree);
    let element = tree.element_with_key(&Key::Global(key)).unwrap();
    let parent = tree.parent(element);
    let render = tree.render_id(element).unwrap();
    assert_eq!(calls.get(), 1);

    let moved = Center::new(SizedBox::square(100.).child(keyed.clone()));
    tree.update(root, moved.into()).unwrap();
    layout(&mut tree);
    assert_eq!(tree.element_with_key(&Key::Global(key)), Some(element));
    assert_eq!(tree.render_id(element), Some(render));
    assert_ne!(tree.parent(element), parent);
    assert_eq!(calls.get(), 1);

    // Changed constraints must still lay out the same retained object.
    let resized = Center::new(SizedBox::square(75.).child(keyed));
    tree.update(root, resized.into()).unwrap();
    layout(&mut tree);
    assert_eq!(tree.render_id(element), Some(render));
    assert_eq!(calls.get(), 2);
}
