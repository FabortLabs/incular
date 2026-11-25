//! The inherited visibility of an IndexedStack branch is independent of layout.
use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Center, Directionality, IndexedStack, Positioned, SizedBox, TextDirection, Visibility,
    Widget, internal::Key,
};
use std::{cell::Cell, rc::Rc};

const STACK: Key = Key::Value(20);
const POSITIONED: Key = Key::Value(21);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn scene(widget: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, widget).into()
}

fn observe(key: Key, size: Size, seen: Rc<Cell<Option<bool>>>, reads: Rc<Cell<usize>>) -> Widget {
    Widget::from(Builder::new(move |ctx| {
        reads.set(reads.get() + 1);
        seen.set(Some(Visibility::of(ctx)));
        Widget::box_(size, Color::WHITE)
    }))
    .with_key(key)
}

#[test]
fn indexed_stack_publishes_visibility_to_all_retained_branches() {
    let seen = [(); 3].map(|_| Rc::new(Cell::new(None)));
    let reads = [(); 3].map(|_| Rc::new(Cell::new(0)));
    let keys = [Key::Value(10), Key::Value(11), Key::Value(12)];
    let children = std::array::from_fn::<_, 3, _>(|i| {
        observe(
            keys[i].clone(),
            Size::new(50., 50.),
            Rc::clone(&seen[i]),
            Rc::clone(&reads[i]),
        )
    });
    let build = |selection| scene(IndexedStack::new(children.clone()).index(selection));
    let mut runtime = Runtime::new(build(None)).unwrap();
    let frame = Constraints::tight(VIEW);
    runtime.run_frame(frame).unwrap();
    let root = runtime.tree().root().unwrap();
    let before = keys
        .each_ref()
        .map(|key| runtime.tree().element_with_key(key).unwrap());
    let mut actual = vec![seen.each_ref().map(|cell| cell.get())];
    for selection in [Some(0), Some(1), Some(2), None] {
        runtime.schedule_update(root, build(selection)).unwrap();
        runtime.run_frame(frame).unwrap();
        assert_eq!(
            keys.each_ref()
                .map(|key| runtime.tree().element_with_key(key).unwrap()),
            before
        );
        actual.push(seen.each_ref().map(|cell| cell.get()));
    }
    assert_eq!(
        actual,
        [
            [Some(false), Some(false), Some(false)],
            [Some(true), Some(false), Some(false)],
            [Some(false), Some(true), Some(false)],
            [Some(false), Some(false), Some(true)],
            [Some(false), Some(false), Some(false)],
        ],
    );
    assert_eq!(reads.each_ref().map(|cell| cell.get()), [3; 3]);
}

#[test]
fn positioned_parent_data_remains_direct_while_the_sibling_is_hidden() {
    let seen = Rc::new(Cell::new(None));
    let sibling = observe(
        Key::Value(50),
        Size::new(50., 50.),
        Rc::clone(&seen),
        Rc::new(Cell::new(0)),
    );
    let positioned = Widget::from(
        Positioned::new(SizedBox::new())
            .left(10.)
            .top(20.)
            .width(30.)
            .height(40.),
    )
    .with_key(POSITIONED);
    let stack = Widget::from(IndexedStack::new([positioned, sibling])).with_key(STACK);
    let mut runtime = Runtime::new(scene(stack)).unwrap();
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    let tree = runtime.tree();
    let p = tree.element_with_key(&POSITIONED).unwrap();
    assert_eq!(tree.parent(p), tree.element_with_key(&STACK));
    assert_eq!(
        tree.element_bounds(p),
        Some(Rect::from_origin_size(
            Offset::new(10., 20.),
            Size::new(30., 40.)
        ))
    );
    assert_eq!(seen.get(), Some(false));
}

#[test]
fn visibility_remains_conjunctive_and_follows_retained_keyed_branch_reorders() {
    let keys = [Key::Value(70), Key::Value(71)];
    let reads = [(); 2].map(|_| Rc::new(Cell::new(0)));
    let seen = [(); 2].map(|_| Rc::new(Cell::new(None)));
    let readers = std::array::from_fn::<_, 2, _>(|i| {
        observe(
            keys[i].clone(),
            Size::new(30., 25.),
            Rc::clone(&seen[i]),
            Rc::clone(&reads[i]),
        )
    });
    let branch = |i: usize| {
        Widget::from(Visibility::maintain(readers[i].clone())).with_key(Key::Value(80 + i as u64))
    };
    let children = [branch(0), branch(1)];
    let scene = |order: [usize; 2], outer| {
        scene(
            Visibility::maintain(Center::new(
                IndexedStack::new(order.map(|index| children[index].clone())).index(0),
            ))
            .visible(outer),
        )
    };

    let mut runtime = Runtime::new(scene([0, 1], true)).unwrap();
    let frame = Constraints::tight(VIEW);
    runtime.run_frame(frame).unwrap();
    let root = runtime.tree().root().unwrap();
    let initial = keys
        .each_ref()
        .map(|key| runtime.tree().element_with_key(key).unwrap());
    let mut result = vec![seen.each_ref().map(|cell| cell.get())];
    for (order, outer) in [([1, 0], true), ([1, 0], false), ([0, 1], true)] {
        runtime.schedule_update(root, scene(order, outer)).unwrap();
        runtime.run_frame(frame).unwrap();
        assert_eq!(
            keys.each_ref()
                .map(|key| runtime.tree().element_with_key(key).unwrap()),
            initial
        );
        result.push(seen.each_ref().map(|cell| cell.get()));
    }
    assert_eq!(
        result,
        [
            [Some(true), Some(false)],
            [Some(false), Some(true)],
            [Some(false), Some(false)],
            [Some(true), Some(false)],
        ],
    );
}
