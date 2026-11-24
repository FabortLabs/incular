//! Flutter 3.50.0 CustomSingleChildLayout delegate input and relayout counterparts.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    Center, CustomSingleChildLayout, Directionality, SingleChildLayoutDelegate, SizedBox,
    TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const LAYOUT: u64 = 1;
const CHILD: u64 = 2;
const VIEWPORT: Size = Size {
    width: 800.,
    height: 600.,
};

#[derive(Default)]
struct Calls {
    size: Cell<Option<Constraints>>,
    constraints: Cell<Option<Constraints>>,
    position: Cell<Option<(Size, Size)>>,
    old_delegates: RefCell<Vec<u8>>,
}

struct RecordedDelegate {
    id: u8,
    calls: Rc<Calls>,
    relayout: bool,
}

impl SingleChildLayoutDelegate for RecordedDelegate {
    fn get_size(&self, constraints: Constraints) -> Size {
        self.calls.size.set(Some(constraints));
        Size::new(200., 300.)
    }

    fn get_constraints_for_child(&self, constraints: Constraints) -> Constraints {
        self.calls.constraints.set(Some(constraints));
        Constraints::new(100., 150., 200., 400.)
    }

    fn get_position_for_child(&self, size: Size, child: Size) -> Offset {
        self.calls.position.set(Some((size, child)));
        Offset::ZERO
    }

    fn should_relayout(&self, old: &Self) -> bool {
        self.calls.old_delegates.borrow_mut().push(old.id);
        self.relayout
    }
}

struct FixedSize(Size);

impl SingleChildLayoutDelegate for FixedSize {
    fn get_size(&self, _: Constraints) -> Size {
        self.0
    }

    fn get_constraints_for_child(&self, _: Constraints) -> Constraints {
        Constraints::tight(self.0)
    }

    fn should_relayout(&self, old: &Self) -> bool {
        self.0 != old.0
    }
}

fn centered(delegate: impl SingleChildLayoutDelegate) -> Widget {
    // For the finite child constraints in this fixture, a childless Flutter
    // Container occupies the largest size allowed by the delegate.
    let child = Widget::from(SizedBox::expand()).with_key(CHILD);
    Center::new(Widget::from(CustomSingleChildLayout::new(delegate, child)).with_key(LAYOUT)).into()
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(element).expect("fixture bounds")
}

// widgets/custom_single_child_layout_test.dart:100, "Control test for CustomSingleChildLayout".
#[test]
fn delegate_measures_against_outer_constraints_and_positions_with_actual_sizes() {
    let calls = Rc::new(Calls::default());
    let mut tree = WidgetTree::new();
    tree.mount(centered(RecordedDelegate {
        id: 1,
        calls: Rc::clone(&calls),
        relayout: false,
    }))
    .expect("mount");
    tree.layout(Constraints::tight(VIEWPORT)).expect("layout");
    let loose = Some(Constraints::loose(VIEWPORT));
    assert_eq!(calls.size.get(), loose);
    assert_eq!(calls.constraints.get(), loose);
    assert_eq!(
        calls.position.get(),
        Some((Size::new(200., 300.), Size::new(150., 400.)))
    );
    assert_eq!(bounds(&tree, CHILD).origin, bounds(&tree, LAYOUT).origin);
}

// widgets/custom_single_child_layout_test.dart:121, "Test SingleChildDelegate shouldRelayout method".
#[test]
fn replacing_delegate_only_requests_layout_when_new_delegate_agrees() {
    let frame = Constraints::tight(VIEWPORT);
    let (first, skip, relayout) = (
        Rc::new(Calls::default()),
        Rc::new(Calls::default()),
        Rc::new(Calls::default()),
    );
    let scene = |id, calls: &Rc<Calls>, relayout| {
        centered(RecordedDelegate {
            id,
            calls: Rc::clone(calls),
            relayout,
        })
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(1, &first, false)).expect("mount");
    tree.layout(frame).expect("initial");
    assert!(first.constraints.get().is_some());
    assert!(first.old_delegates.borrow().is_empty());

    tree.update(root, scene(2, &skip, false))
        .expect("replace without layout");
    tree.layout(frame).expect("second frame");
    let second = (
        skip.old_delegates.borrow().contains(&1),
        skip.constraints.get().is_some(),
    );

    tree.update(root, scene(3, &relayout, true))
        .expect("replace with relayout");
    tree.layout(frame).expect("third frame");
    let third = (
        !relayout.old_delegates.borrow().is_empty(),
        relayout.constraints.get().is_some(),
    );
    assert_eq!(
        (second, third),
        ((true, false), (true, true)),
        "(new delegate called, layout performed); initial delegate received comparison against {:?}",
        first.old_delegates.borrow()
    );
}

// widgets/custom_single_child_layout_test.dart:144, "Delegate can change size".
#[test]
fn changed_fixed_size_delegate_updates_parent_and_child_geometry() {
    let frame = Constraints::tight(VIEWPORT);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(centered(FixedSize(Size::new(100., 200.))))
        .expect("mount");
    tree.layout(frame).expect("first layout");
    for key in [LAYOUT, CHILD] {
        assert_eq!(bounds(&tree, key).size, Size::new(100., 200.));
    }
    tree.update(root, centered(FixedSize(Size::new(150., 240.))))
        .expect("replace delegate");
    tree.layout(frame).expect("second layout");
    for key in [LAYOUT, CHILD] {
        assert_eq!(bounds(&tree, key).size, Size::new(150., 240.));
    }
}

// widgets/custom_single_child_layout_test.dart:172,
// "CustomSingleChildLayout does not crash at zero area": this Rust variant
// supplies the mandatory inert child and a fixed delegate; no notification fires.
#[test]
fn custom_layout_constrains_delegate_size_to_zero_area() {
    let layout = CustomSingleChildLayout::new(FixedSize(Size::new(100., 200.)), SizedBox::shrink());
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(Widget::from(layout).with_key(LAYOUT)),
    );
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).expect("mount");
    tree.layout(Constraints::tight(Size::ZERO))
        .expect("zero-area layout");
    assert_eq!(bounds(&tree, LAYOUT).size, Size::ZERO);
}
