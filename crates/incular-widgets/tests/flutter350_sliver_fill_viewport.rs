//! Behavior references: Flutter 3.50.0 widgets/sliver_fill_viewport_test.dart
//! and the adjacent authored boundary/cache probe.

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::Size;
use incular_scroll::ScrollController;
use incular_widgets::{
    CustomScrollView, Directionality, GlobalKey, Sliver, SliverFillViewport, Text, Widget,
    internal::WidgetTree,
};
use std::time::Instant;

fn scene(
    controller: &ScrollController,
    pages: &[GlobalKey],
    implicit: Option<bool>,
    axis: Axis,
    reverse: bool,
) -> Widget {
    let children = pages.iter().enumerate().map(|(index, key)| {
        Widget::from(Text::new(format!("Page {}", index + 1))).with_key(key.clone())
    });
    let mut sliver = SliverFillViewport::new(children);
    if let Some(implicit) = implicit {
        sliver = sliver.allow_implicit_scrolling(implicit);
    }
    let view = CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
        .scroll_direction(axis)
        .reverse(reverse)
        .controller(controller.clone());
    Directionality::new(TextDirection::Ltr, view).into()
}

fn frame(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::tight(size)).expect("layout");
    tree.update_compositor(Instant::now()).expect("compositor");
    tree.update_semantics();
}

fn labels(tree: &WidgetTree) -> Vec<String> {
    let mut labels = tree
        .semantics()
        .iter()
        .filter_map(|(_, node)| {
            node.label
                .as_ref()
                .filter(|label| label.starts_with("Page "))
                .cloned()
        })
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

#[test]
fn allow_implicit_scrolling_toggles_cached_semantics_without_losing_the_page() {
    let controller = ScrollController::new();
    let keys = [GlobalKey::new(), GlobalKey::new()];
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(
            &controller,
            &keys,
            Some(false),
            Axis::Vertical,
            false,
        ))
        .expect("mount");
    let size = Size::new(800., 600.);
    frame(&mut tree, size);
    let retained = tree
        .element_with_key(&keys[1].clone().into())
        .expect("second page is cached");
    assert_eq!(labels(&tree), ["Page 1"]);

    tree.update(root, scene(&controller, &keys, None, Axis::Vertical, false))
        .expect("default implicit scrolling");
    frame(&mut tree, size);
    assert_eq!(labels(&tree), ["Page 1", "Page 2"]);
    assert_eq!(
        tree.element_with_key(&keys[1].clone().into()),
        Some(retained)
    );

    tree.update(
        root,
        scene(&controller, &keys, Some(false), Axis::Vertical, false),
    )
    .expect("disable again");
    frame(&mut tree, size);
    assert_eq!(labels(&tree), ["Page 1"]);
    assert_eq!(
        tree.element_with_key(&keys[1].clone().into()),
        Some(retained)
    );
    tree.verify_invariants().expect("retained viewport");
}

#[test]
fn disabled_implicit_scrolling_tracks_half_open_page_overlap_in_both_axes() {
    for (axis, reverse) in [(Axis::Vertical, false), (Axis::Horizontal, true)] {
        let controller = ScrollController::new();
        let keys = [GlobalKey::new(), GlobalKey::new(), GlobalKey::new()];
        let mut tree = WidgetTree::new();
        tree.mount(scene(&controller, &keys, Some(false), axis, reverse))
            .expect("mount");
        let size = axis.size(600., 800.);
        frame(&mut tree, size);
        let first = tree
            .element_with_key(&keys[0].clone().into())
            .expect("first page");
        let second = tree
            .element_with_key(&keys[1].clone().into())
            .expect("second page");
        for (offset, expected) in [
            (0., &["Page 1"][..]),
            (599., &["Page 1", "Page 2"][..]),
            (600., &["Page 2"][..]),
            (601., &["Page 2", "Page 3"][..]),
            (600., &["Page 2"][..]),
        ] {
            if controller.offset() != offset {
                assert!(controller.jump_to(offset));
            }
            frame(&mut tree, size);
            assert_eq!(
                labels(&tree),
                expected,
                "axis={axis:?} reverse={reverse} scroll={offset}"
            );
            assert_eq!(tree.element_with_key(&keys[0].clone().into()), Some(first));
            assert_eq!(tree.element_with_key(&keys[1].clone().into()), Some(second));
        }
        tree.verify_invariants().expect("retained viewport");
    }
}

#[test]
fn half_viewport_page_reports_exact_paint_extent_with_or_without_end_padding() {
    use incular_scroll::SliverConstraints;
    let controller = ScrollController::new();
    for (pad, expected_extent, expected_origin) in [(true, 600., 150.), (false, 300., 0.)] {
        let mut render = SliverFillViewport::new([Text::new("0")])
            .viewport_fraction(0.5)
            .pad_ends(pad)
            .create_render_sliver(&controller, Axis::Vertical, false);
        let layout = render.perform_layout(SliverConstraints::new(
            Axis::Vertical,
            false,
            0.,
            0.,
            0.,
            600.,
            800.,
            600.,
            850.,
            0.,
        ));
        assert_eq!(layout.geometry.paint_extent, expected_extent);
        assert_eq!(layout.children.len(), 1);
        let page = &layout.children[0];
        assert_eq!(page.offset, expected_origin);
        assert_eq!(page.extent, 300.);
        assert_eq!(page.constraints.smallest(), Size::new(800., 300.));
        assert_eq!(page.constraints.biggest(), Size::new(800., 300.));
    }
}
