//! Indexed scroll refresh preserves values, actions, retained siblings and lifecycle.
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_scroll::ScrollPhysics;
use incular_semantics::{SemanticActionKind, SemanticNode, SemanticRole};
use incular_widgets::internal::{ExplicitSemantics, WidgetTree};
use incular_widgets::{
    Column, CustomScrollView, ExcludeSemantics, Row, ScrollController, SingleChildScrollView,
    SizedBox, Sliver, SliverToBoxAdapter, Widget,
};

fn labeled_box(label: impl Into<String>, height: f32) -> Widget {
    Widget::box_(Size::new(100., height), Color::WHITE)
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label(label))
}

fn viewport(controller: &ScrollController, label: &'static str, sliver: bool) -> Widget {
    let child = labeled_box(format!("{label} content"), 800.);
    let scroll: Widget = if sliver {
        let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverToBoxAdapter::new(child))];
        CustomScrollView::new(slivers)
            .controller(controller.clone())
            .physics(ScrollPhysics::default())
            .into()
    } else {
        SingleChildScrollView::new(child)
            .controller(controller.clone())
            .physics(ScrollPhysics::default())
            .into()
    };
    Widget::from(SizedBox::from_dimensions(
        Some(100.),
        Some(100.),
        Some(scroll.accessibility_label(label)),
    ))
    .with_key(label)
}

fn find<'a>(tree: &'a WidgetTree, label: &str) -> &'a SemanticNode {
    tree.semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing semantic node {label}"))
}

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(300., 100.)))
        .expect("layout");
    scroll_frame(tree);
}

fn scroll_frame(tree: &mut WidgetTree) {
    tree.update_compositor(Instant::now()).expect("compositor");
    tree.update_semantics();
}

fn assert_scroll(tree: &WidgetTree, label: &str, controller: &ScrollController) {
    let node = find(tree, label);
    assert_eq!(node.role, SemanticRole::ScrollView);
    assert_eq!(node.value, None);
    let metrics = node.state.scroll_metrics.expect("typed scroll position");
    assert_eq!(metrics.offset, f64::from(controller.offset()));
    assert_eq!(metrics.minimum, f64::from(controller.min_offset()));
    assert_eq!(metrics.maximum, f64::from(controller.max_offset()));
    assert_eq!(
        node.actions.contains(&SemanticActionKind::ScrollBackward),
        controller.offset() > controller.min_offset()
    );
    assert_eq!(
        node.actions.contains(&SemanticActionKind::ScrollForward),
        controller.offset() < controller.max_offset()
    );
}

#[test]
fn scrolling_updates_only_the_moving_semantics_and_keeps_static_nodes() {
    let controller = ScrollController::new();
    let static_branch = Widget::from(Column::new(
        (0..128).map(|index| labeled_box(format!("static {index}"), 2.)),
    ))
    .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("static group"));
    let root = Widget::from(
        Row::new([viewport(&controller, "viewport", false), static_branch])
            .text_direction(TextDirection::Ltr),
    )
    .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("root"));
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    frame(&mut tree);
    assert_scroll(&tree, "viewport", &controller);
    let moving = find(&tree, "viewport content").clone();
    let retained: Vec<_> = tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .filter(|node| {
            node.label
                .as_ref()
                .is_some_and(|label| label.starts_with("static"))
        })
        .cloned()
        .collect();
    let before = tree.diagnostics();

    for offset in [40., controller.max_offset(), 0.] {
        controller.jump_to(offset);
        scroll_frame(&mut tree);
        assert_scroll(&tree, "viewport", &controller);
        let current = find(&tree, "viewport content");
        assert_eq!(current.id, moving.id, "scrolling retains semantic identity");
        assert_eq!(current.bounds.origin.y, moving.bounds.origin.y - offset);
        for node in &retained {
            assert_eq!(tree.semantics().node(node.id), Some(node));
        }
    }
    assert_eq!(
        tree.diagnostics().layouts,
        before.layouts,
        "compositor-only scrolling"
    );
    assert_eq!(
        tree.diagnostics().rebuilds,
        before.rebuilds,
        "no widget rebuilds"
    );
}

#[test]
fn scroll_refresh_tracks_both_viewport_kinds_exclusion_removal_and_replacement() {
    let ordinary = ScrollController::new();
    let sliver = ScrollController::new();
    let excluded = ScrollController::new();
    let hidden = || Widget::from(ExcludeSemantics::new(viewport(&excluded, "hidden", false)));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Row::new([
                viewport(&ordinary, "ordinary", false),
                viewport(&sliver, "sliver", true),
                hidden(),
            ])
            .text_direction(TextDirection::Ltr)
            .into(),
        )
        .expect("mount");
    frame(&mut tree);
    let old_id = find(&tree, "ordinary").id;
    let sliver_id = find(&tree, "sliver").id;

    for controller in [&ordinary, &sliver, &excluded] {
        controller.jump_to(40.);
    }
    scroll_frame(&mut tree);
    assert_scroll(&tree, "ordinary", &ordinary);
    assert_scroll(&tree, "sliver", &sliver);
    assert_eq!(
        tree.semantics()
            .iter()
            .filter(|(_, node)| node.role == SemanticRole::ScrollView)
            .count(),
        2
    );
    assert!(tree.semantics().iter().all(|(_, node)| {
        !node
            .label
            .as_ref()
            .is_some_and(|label| label.starts_with("hidden"))
    }));

    tree.update(
        root,
        Row::new([
            viewport(&sliver, "sliver", true),
            labeled_box("replacement", 100.),
            hidden(),
        ])
        .text_direction(TextDirection::Ltr)
        .into(),
    )
    .expect("remove ordinary viewport");
    frame(&mut tree);
    assert!(tree.semantics().node(old_id).is_none());
    ordinary.jump_to(80.);
    sliver.jump_to(sliver.max_offset());
    excluded.jump_to(80.);
    scroll_frame(&mut tree);
    assert_scroll(&tree, "sliver", &sliver);
    assert_eq!(
        find(&tree, "sliver").id,
        sliver_id,
        "reordering keeps the viewport"
    );

    let replacement = ScrollController::new();
    tree.update(
        root,
        Row::new([
            viewport(&sliver, "sliver", true),
            viewport(&replacement, "new viewport", false),
            hidden(),
        ])
        .text_direction(TextDirection::Ltr)
        .into(),
    )
    .expect("mount replacement viewport");
    frame(&mut tree);
    replacement.jump_to(60.);
    sliver.jump_to(0.);
    scroll_frame(&mut tree);
    assert_scroll(&tree, "new viewport", &replacement);
    assert_scroll(&tree, "sliver", &sliver);
    assert!(
        tree.semantics().node(old_id).is_none(),
        "stale semantic identity stays retired"
    );
}
