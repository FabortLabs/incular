use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{AspectRatio, LayoutBuilder, Padding, SizedBox, Widget};

fn assert_padding_queries(child: Widget, expected: [f32; 4]) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Padding::all(15., child).into()).unwrap();
    let box_id = tree.render_id(root).unwrap();
    let layout_before = tree.root_layout_size();
    for (cross, expected) in [0., 10., 80., f32::INFINITY].into_iter().zip(expected) {
        assert_eq!(tree.min_intrinsic_width(box_id, cross), Some(expected));
        assert_eq!(tree.max_intrinsic_width(box_id, cross), Some(expected));
        assert_eq!(tree.min_intrinsic_height(box_id, cross), Some(expected));
        assert_eq!(tree.max_intrinsic_height(box_id, cross), Some(expected));
    }
    assert_eq!(tree.root_layout_size(), layout_before);
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::new(10., 10.)));
}

#[test]
fn padding_propagates_a_boring_fixed_intrinsic_size() {
    assert_padding_queries(
        SizedBox::builder().width(20.).height(20.).build().into(),
        [50.; 4],
    );
}

#[test]
fn padding_subtracts_cross_insets_before_measuring_an_aspect_ratio() {
    // A zero-sized child has the same intrinsic result as Flutter's childless
    // aspect box when the perpendicular dimension is unbounded.
    assert_padding_queries(
        AspectRatio::new(1., SizedBox::builder().width(0.).height(0.).build()).into(),
        [30., 30., 80., 30.],
    );
}

#[test]
fn unsupported_and_invalid_queries_do_not_execute_layout_callbacks() {
    let calls = Rc::new(Cell::new(0));
    let shared = Rc::clone(&calls);
    let builder = LayoutBuilder::new(move |_, _| {
        shared.set(shared.get() + 1);
        SizedBox::builder().width(20.).height(20.).build().into()
    });
    let mut tree = WidgetTree::new();
    let root = tree.mount(Padding::all(15., builder).into()).unwrap();
    let box_id = tree.render_id(root).unwrap();
    let before = calls.get();
    assert_eq!(tree.min_intrinsic_width(box_id, 80.), None);
    assert_eq!(tree.max_intrinsic_width(box_id, -1.), None);
    assert_eq!(tree.min_intrinsic_height(box_id, f32::NAN), None);
    assert_eq!(tree.max_intrinsic_height(box_id, f32::NEG_INFINITY), None);
    assert_eq!(calls.get(), before);
}
