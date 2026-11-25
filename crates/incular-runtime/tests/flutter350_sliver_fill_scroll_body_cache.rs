//! Pinned Flutter scroll-body sizing when paint and cache bounds diverge.
use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, Semantics, SizedBox, Sliver,
    SliverFillRemaining, SliverFixedExtentList, TextDirection, Widget, internal::Key,
};

#[test]
fn horizontal_cache_gives_scroll_body_a_real_width() {
    check_cache_budget(Axis::Horizontal);
}

#[test]
fn vertical_cache_gives_scroll_body_a_real_height() {
    check_cache_budget(Axis::Vertical);
}

fn check_cache_budget(axis: Axis) {
    const VIEW: Size = Size {
        width: 800.,
        height: 600.,
    };
    const FILL: Key = Key::Value(50);
    const LABEL: &str = "Filling child";
    let controller = ScrollController::new();
    let view_extent = axis.main_extent(VIEW);
    let fill = Widget::from(
        SizedBox::new().width(100.).height(100.).child(
            Semantics::new(Widget::box_(Size::ZERO, Color::WHITE))
                .label(LABEL)
                .container(true),
        ),
    )
    .with_key(FILL);
    let slivers: Vec<Box<dyn Sliver>> = vec![
        Box::new(SliverFixedExtentList::new(8, 150., |_| SizedBox::shrink())),
        Box::new(SliverFillRemaining::new(fill)),
    ];
    let view = CustomScrollView::new(slivers)
        .controller(controller.clone())
        .scroll_direction(axis)
        .cache_extent(250.);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    let mut retained = None;
    let mut actual = Vec::new();
    let expected = [
        (-250., 0., None),
        (-200., 50., Some(true)),
        (0., 100., Some(true)),
        (25., 25., Some(false)),
        (50., 50., Some(false)),
    ];
    for (delta, _, _) in expected {
        assert!(controller.jump_to(1200. - view_extent + delta));
        runtime.run_frame(Constraints::tight(VIEW)).unwrap();
        let tree = runtime.tree();
        let element = tree.element_with_key(&FILL).expect("retained fill");
        assert_eq!(element, *retained.get_or_insert(element));
        let bounds = tree.element_bounds(element).expect("fill bounds");
        let mut semantics = tree
            .semantics()
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some(LABEL));
        let hidden = semantics.next().map(|(_, node)| node.state.hidden);
        assert!(semantics.next().is_none());
        let origin = if axis == Axis::Vertical {
            bounds.origin.y
        } else {
            bounds.origin.x
        };
        assert_eq!(origin, view_extent - delta);
        assert_eq!(controller.max_offset(), 1200.);
        actual.push((delta, axis.main_extent(bounds.size), hidden));
    }
    assert_eq!(
        actual, expected,
        "cached and painted fill sizing in {axis:?}"
    );
}
