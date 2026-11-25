//! Flutter 3.50 baseline suppression, retained layout, and a zero-area host.

use incular_config::{Constraints, CrossAxisAlignment, TextDirection};
use incular_core::Size;
use incular_text::TextStyle;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    Center, Directionality, IgnoreBaseline, LayoutBuilder, Placeholder, Row, SizedBox, Text, Widget,
};
use std::{cell::Cell, rc::Rc};

const LARGE: Key = Key::Value(400);
const SMALL: Key = Key::Value(401);
const IGNORED: Key = Key::Value(402);

fn keyed(widget: impl Into<Widget>, key: Key) -> Widget {
    widget.into().with_key(key)
}

fn row(ignored: bool) -> Widget {
    let large = keyed(Text::new("a").style(TextStyle::new().size(128.)), LARGE);
    let small = keyed(Text::new("b").style(TextStyle::new().size(32.)), SMALL);
    let first = if ignored {
        keyed(IgnoreBaseline::new(large), IGNORED)
    } else {
        large
    };
    Directionality::new(
        TextDirection::Ltr,
        Row::new([first, small]).cross_axis_alignment(CrossAxisAlignment::Baseline),
    )
    .into()
}

fn metrics(tree: &WidgetTree, key: Key) -> (f32, Size, Option<f32>) {
    let child = tree.element_with_key(&key).expect("keyed child mounted");
    let bounds = tree.element_bounds(child).expect("child has bounds");
    let baseline = tree.baseline(tree.render_id(child).expect("child render object"));
    (bounds.origin.y, bounds.size, baseline)
}

#[test]
fn ignored_large_text_no_longer_pushes_its_small_baseline_aligned_sibling_down() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(row(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    let (large_y, large_size, large_baseline) = metrics(&tree, LARGE);
    let (small_y, small_size, small_baseline) = metrics(&tree, SMALL);
    let large_baseline = large_baseline.unwrap();
    let small_baseline = small_baseline.unwrap();
    assert_eq!(large_y, 0.);
    assert!(large_baseline > small_baseline);
    assert_eq!(small_y, large_baseline - small_baseline);

    tree.update(root, row(true)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(metrics(&tree, IGNORED), (0., large_size, None));
    assert_eq!(
        metrics(&tree, LARGE),
        (0., large_size, Some(large_baseline))
    );
    assert_eq!(
        metrics(&tree, SMALL),
        (0., small_size, Some(small_baseline))
    );

    tree.update(root, row(false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(
        metrics(&tree, SMALL),
        (small_y, small_size, Some(small_baseline))
    );
}

#[test]
fn ignore_baseline_forwards_exact_incoming_constraints_and_child_size() {
    let actual = Rc::new(Cell::new(None));
    let child = LayoutBuilder::new({
        let actual = actual.clone();
        move |_, constraints| {
            actual.set(Some(constraints));
            SizedBox::new().width(45.).height(30.).into()
        }
    });
    let mut tree = WidgetTree::new();
    tree.mount(keyed(IgnoreBaseline::new(child), IGNORED))
        .unwrap();
    let incoming = Constraints::new(30., 80., 10., 70.);
    tree.layout(incoming).unwrap();
    assert_eq!(actual.get(), Some(incoming));
    assert_eq!(metrics(&tree, IGNORED), (0., Size::new(45., 30.), None));
}

#[test]
fn ignore_baseline_placeholder_can_layout_at_zero_area() {
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(keyed(IgnoreBaseline::new(Placeholder::new()), IGNORED)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(metrics(&tree, IGNORED), (0., Size::ZERO, None));
}
