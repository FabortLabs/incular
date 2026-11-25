//! Flutter Scrollable.ensureVisible with pinned and merely floating headers.
use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, EnsureVisible, ScrollController, Scrollable, SizedBox,
    Sliver, SliverHeaderScrollBehavior, SliverPersistentHeader, SliverToBoxAdapter, TextDirection,
    Widget, internal::Key,
};

fn reveal(axis: Axis, reverse: bool) {
    for (behavior, expected) in [
        (SliverHeaderScrollBehavior::Floating, [500., 250., 0.]),
        (SliverHeaderScrollBehavior::FloatingPinned, [400., 200., 0.]),
    ] {
        let controller = ScrollController::new();
        let probe = Key::from("target");
        let main_box = |extent| match axis {
            Axis::Vertical => SizedBox::new().height(extent),
            Axis::Horizontal => SizedBox::new().width(extent),
        };
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(
                SliverPersistentHeader::with_delegate(100., 200., |_, _| SizedBox::expand())
                    .scroll_behavior(behavior),
            ),
            Box::new(SliverToBoxAdapter::new(main_box(300.))),
            Box::new(SliverToBoxAdapter::new(
                Widget::from(main_box(100.)).with_key(probe.clone()),
            )),
            Box::new(SliverToBoxAdapter::new(main_box(1200.))),
        ];
        let view = Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers)
                .scroll_direction(axis)
                .reverse(reverse)
                .controller(controller.clone()),
        );
        let mut runtime = Runtime::new(view.into()).unwrap();
        let bounds = Constraints::tight(Size::new(600., 600.));
        runtime.run_frame(bounds).unwrap();
        for (alignment, expected) in [0., 0.5, 1.].into_iter().zip(expected) {
            let target = runtime
                .tree()
                .element_with_key(&probe)
                .expect("the target is visible or cached");
            Scrollable::ensure_visible(
                runtime.tree(),
                target,
                EnsureVisible::new().alignment(alignment),
            );
            runtime.run_frame(bounds).unwrap();
            assert_eq!(
                controller.offset(),
                expected,
                "{axis:?} reversed={reverse}, {behavior:?}, alignment {alignment}"
            );
        }
    }
}

#[test]
fn reveal_uses_available_extent_in_forward_viewports() {
    reveal(Axis::Vertical, false);
    reveal(Axis::Horizontal, false);
}

#[test]
fn reveal_uses_available_extent_from_the_leading_side_in_reversed_viewports() {
    reveal(Axis::Vertical, true);
    reveal(Axis::Horizontal, true);
}
