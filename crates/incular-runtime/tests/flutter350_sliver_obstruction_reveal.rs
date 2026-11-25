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

#[test]
fn semantics_show_on_screen_reveals_the_part_under_a_pinned_header() {
    use incular_semantics::SemanticAction;
    use incular_widgets::Semantics;
    let controller = ScrollController::new();
    let target = Semantics::new(SizedBox::new().height(100.)).label("reveal this");
    let viewport = CustomScrollView::new(vec![
        Box::new(SliverPersistentHeader::new(200., SizedBox::expand()).pinned(true))
            as Box<dyn Sliver>,
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(300.))),
        Box::new(SliverToBoxAdapter::new(target)),
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(1200.))),
    ])
    .controller(controller.clone());
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
    let bounds = Constraints::tight(Size::new(600., 600.));
    runtime.run_frame(bounds).unwrap();
    assert!(controller.jump_to(475.));
    runtime.run_frame(bounds).unwrap();
    let target = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some("reveal this")).then_some(id))
        .unwrap();
    assert!(runtime.dispatch_semantic_action(target, SemanticAction::ShowOnScreen));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(controller.offset(), 300.);
}

#[test]
fn focusing_editable_text_reveals_its_caret_past_the_pinned_strip() {
    use incular_config::EdgeInsets;
    use incular_widgets::{EditableText, FocusNode, TextEditingController};
    let controller = ScrollController::new();
    let focus = FocusNode::new();
    let key = Key::from("retained-editor");
    let field = Widget::from(
        EditableText::new(TextEditingController::with_text("Hello"))
            .focus_node(focus.clone())
            .scroll_padding(EdgeInsets::ZERO),
    )
    .with_key(key.clone());
    let view = CustomScrollView::new(vec![
        Box::new(SliverPersistentHeader::new(200., SizedBox::expand()).pinned(true))
            as Box<dyn Sliver>,
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(300.))),
        Box::new(SliverToBoxAdapter::new(
            SizedBox::new().height(100.).child(field),
        )),
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(1200.))),
    ])
    .controller(controller.clone());
    let bounds = Constraints::tight(Size::new(600., 600.));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(controller.jump_to(475.));
    runtime.run_frame(bounds).unwrap();
    let caret_y = |runtime: &Runtime| {
        let tree = runtime.tree();
        let id = tree.element_with_key(&key).unwrap();
        tree.text_field_input_geometry(id)
            .unwrap()
            .caret_rect
            .origin
            .y
    };
    assert!(caret_y(&runtime) < 200.);
    focus.request_focus();
    runtime.run_frame(bounds).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(focus.has_primary_focus());
    assert!(controller.offset() < 475.);
    assert!(
        caret_y(&runtime) >= 199.999,
        "caret is at {}",
        caret_y(&runtime)
    );
}
