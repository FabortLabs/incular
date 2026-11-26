//! Mounted semantic ShowOnScreen forwards the target's real rectangle.

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticRole};
use incular_widgets::{
    CustomScrollView, Directionality, ListView, ScrollController, ScrollPhysics, SizedBox, Sliver,
    SliverPadding, SliverToBoxAdapter, TextDirection, Widget, internal::ExplicitSemantics,
};

fn labeled_box(width: f32, height: f32, label: impl Into<String>) -> Widget {
    Widget::from(SizedBox::new().width(width).height(height))
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label(label))
}

#[test]
fn automatic_reveal_keeps_the_visible_part_of_a_large_tile() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            check_oversized_reveal(axis, reverse);
        }
    }
}

fn check_oversized_reveal(axis: Axis, reverse: bool) {
    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    let list = ListView::new((0..20).map(|index| labeled_box(300., 300., format!("tile {index}"))))
        .controller(controller.clone())
        .scroll_direction(axis)
        .reverse(reverse);
    let scene = Directionality::new(TextDirection::Ltr, list);
    let bounds = Constraints::tight(match axis {
        Axis::Vertical => Size::new(300., 200.),
        Axis::Horizontal => Size::new(200., 300.),
    });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(controller.offset(), 300.);
    let show = |runtime: &mut Runtime, label: &str, expected| {
        let (node, semantic) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(label))
            .unwrap_or_else(|| panic!("missing semantic target {label}"));
        let geometry = semantic.bounds;
        assert_eq!(
            axis.main_extent(geometry.size),
            300.,
            "bounds for {label}: {geometry:?}"
        );
        assert!(runtime.dispatch_semantic_action(node, SemanticAction::ShowOnScreen));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            controller.offset(),
            expected,
            "{axis:?} reverse={reverse} {label}: {geometry:?}"
        );
    };
    show(&mut runtime, "tile 1", 300.);
    show(&mut runtime, "tile 0", 100.);
    show(&mut runtime, "tile 1", 300.);
    assert!(controller.jump_to(250.));
    runtime.run_frame(bounds).unwrap();
    show(&mut runtime, "tile 0", 100.);
    assert!(controller.jump_to(150.));
    runtime.run_frame(bounds).unwrap();
    show(&mut runtime, "tile 1", 300.);
}

#[test]
fn an_implicit_scroll_veto_forwards_box_and_padded_sliver_bounds() {
    for padded in [false, true] {
        let horizontal = ScrollController::new();
        let vertical = ScrollController::new();
        let page_physics = ScrollPhysics::default().page();
        let inner: Widget = if padded {
            let slivers: [Box<dyn Sliver>; 2] = ["leading", "target"].map(|name| {
                Box::new(SliverPadding::new(
                    EdgeInsets::all(25.),
                    SliverToBoxAdapter::new(labeled_box(100., 50., name)),
                )) as Box<dyn Sliver>
            });
            CustomScrollView::new(slivers)
                .controller(horizontal.clone())
                .scroll_direction(Axis::Horizontal)
                .physics(page_physics)
                .into()
        } else {
            ListView::new([
                labeled_box(150., 100., "leading"),
                labeled_box(150., 100., "target"),
            ])
            .controller(horizontal.clone())
            .scroll_direction(Axis::Horizontal)
            .physics(page_physics)
            .into()
        };
        let spacer = || Widget::from(SizedBox::new().width(200.).height(150.));
        let outer = ListView::new([
            spacer(),
            SizedBox::new().width(200.).height(100.).child(inner).into(),
            spacer(),
        ])
        .controller(vertical.clone());
        let bounds = Constraints::tight(Size::new(200., 200.));
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, outer).into()).unwrap();
        runtime.run_frame(bounds).unwrap();
        let (node, semantic) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some("target"))
            .unwrap();
        assert_eq!(semantic.bounds.origin.y, if padded { 175. } else { 150. });
        assert!(semantic.bounds.origin.x >= Offset::ZERO.x);
        assert!(runtime.dispatch_semantic_action(node, SemanticAction::ShowOnScreen));
        runtime.run_frame(bounds).unwrap();
        assert_eq!(horizontal.offset(), 0.);
        assert_eq!(vertical.offset(), if padded { 25. } else { 50. });
    }
}
