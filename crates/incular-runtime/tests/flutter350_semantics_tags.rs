//! Flutter 3.50 semantics node tagging and merge behavior.
use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticNode, SemanticNodeId, SemanticState, SemanticsTag};
use incular_widgets::{Column, MergeSemantics, Semantics, SizedBox, Widget};

#[test]
fn a_node_can_add_framework_tags_without_confusing_distinct_instances() {
    let one = SemanticsTag::new("Tag One");
    let two = SemanticsTag::new("Tag Two");
    let coincident_name = SemanticsTag::new("Tag One");
    let mut node = SemanticNode {
        id: SemanticNodeId(incular_core::ArenaId::from_parts(0, 0)),
        role: Role::GenericContainer,
        label: None,
        value: None,
        description: None,
        bounds: Rect::default(),
        state: SemanticState::default(),
        actions: vec![],
        children: vec![],
    };
    assert!(!node.is_tagged(&one));
    assert!(!node.is_tagged(&two));
    node.state.add_tag(one.clone());
    assert!(node.is_tagged(&one));
    assert!(!node.is_tagged(&coincident_name));
    assert!(!node.is_tagged(&two));
    node.state.add_tag(two.clone());
    node.state.add_tag(one.clone());
    assert_eq!(node.tags(), &[one, two]);
}

#[test]
fn merged_tags_are_the_union_of_live_descendant_tags() {
    let one = SemanticsTag::new("Tag One");
    let two = SemanticsTag::new("Tag Two");
    let three = SemanticsTag::new("Tag Three");
    let compose = |include| {
        let parent = Semantics::new(SizedBox::square(10.))
            .label("parent")
            .tag(one.clone())
            .tag(two.clone());
        let child = Semantics::new(SizedBox::square(10.))
            .label("child")
            .tag(two.clone())
            .tag(three.clone());
        let items = if include {
            vec![Widget::from(parent), Widget::from(child)]
        } else {
            vec![Widget::from(parent)]
        };
        Widget::from(MergeSemantics::new(Column::new(items)))
    };
    let mut runtime = Runtime::new(compose(true)).unwrap();
    let bounds = Constraints::tight(Size::new(120., 120.));
    runtime.run_frame(bounds).unwrap();
    let merged = |runtime: &Runtime| {
        runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| {
                node.label
                    .as_deref()
                    .is_some_and(|label| label.starts_with("parent"))
            })
            .map(|(id, node)| (id, node.tags().to_vec()))
            .expect("merge node")
    };
    let (id, tags) = merged(&runtime);
    assert_eq!(tags, vec![one.clone(), two.clone(), three.clone()]);

    runtime
        .schedule_update(runtime.tree().root().unwrap(), compose(false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(merged(&runtime), (id, vec![one, two]));
}

#[test]
fn framework_viewport_tag_crosses_layout_wrappers_but_stops_at_semantic_boundaries() {
    use incular_widgets::{
        CustomScrollView, Directionality, Padding, Sliver, SliverSemantics, SliverToBoxAdapter,
        Text, TextDirection, Viewport,
    };

    let custom = SemanticsTag::new("custom");
    let custom_for_annotation = custom.clone();
    let coincident_name = SemanticsTag::new("RenderViewport.twoPane");
    let viewport_tag = Viewport::use_two_pane_semantics_tag();
    let direct = Box::new(SliverToBoxAdapter::new(Padding::all(
        10.,
        Text::new("direct"),
    )));
    let boundary = Box::new(SliverSemantics::new(
        move || {
            Semantics::default()
                .container(true)
                .label("boundary")
                .explicit_child_nodes(true)
                .tag_for_children(custom_for_annotation.clone())
        },
        SliverToBoxAdapter::new(
            Semantics::new(SizedBox::new().height(20.))
                .container(true)
                .label("inside"),
        ),
    ));
    let slivers: [Box<dyn Sliver>; 2] = [direct, boundary];
    let mut runtime = Runtime::new(
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into(),
    )
    .unwrap();
    let size = Constraints::tight(Size::new(800., 600.));

    let shape = |runtime: &Runtime| {
        let tree = runtime.tree().semantics();
        let node = |label| {
            tree.iter()
                .map(|(_, node)| node)
                .find(|node| node.label.as_deref() == Some(label))
                .unwrap()
        };
        let direct = node("direct");
        let boundary = node("boundary");
        let inside = node("inside");
        let scroll = tree
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.state.has_implicit_scrolling)
            .unwrap();
        assert!(scroll.children.contains(&direct.id));
        assert!(scroll.children.contains(&boundary.id));
        assert_eq!(&boundary.children, &[inside.id]);
        assert!(!scroll.is_tagged(viewport_tag));
        assert!(direct.is_tagged(viewport_tag));
        assert!(boundary.is_tagged(viewport_tag));
        assert!(!boundary.is_tagged(&custom));
        assert!(!inside.is_tagged(viewport_tag));
        assert!(inside.is_tagged(&custom));
        assert!(!direct.is_tagged(&coincident_name));
        (
            scroll.id,
            direct.id,
            boundary.id,
            inside.id,
            tree.revision(),
        )
    };

    runtime.run_frame(size).unwrap();
    let before = shape(&runtime);
    runtime.run_frame(size).unwrap();
    assert_eq!(shape(&runtime), before);
}

#[test]
fn a_sliver_axis_group_does_not_mark_the_implicit_scroller_itself() {
    use incular_widgets::{
        CustomScrollView, Directionality, Padding, Sliver, SliverMainAxisGroup, SliverToBoxAdapter,
        Text, TextDirection, Viewport,
    };

    let group: [Box<dyn Sliver>; 2] = [
        Box::new(SliverToBoxAdapter::new(Padding::new(
            incular_config::EdgeInsets::only(0., 20., 0., 0.),
            SizedBox::shrink(),
        ))),
        Box::new(SliverToBoxAdapter::new(Text::new("label"))),
    ];
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverMainAxisGroup::new(group))];
    let mut runtime = Runtime::new(
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into(),
    )
    .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let tag = Viewport::use_two_pane_semantics_tag();
    let snapshot = |runtime: &Runtime| {
        let tree = runtime.tree().semantics();
        let scroll = tree
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.state.has_implicit_scrolling)
            .expect("implicit scroller is actually present");
        assert!(
            !tree
                .iter()
                .any(|(_, node)| node.state.has_implicit_scrolling && node.is_tagged(tag))
        );
        let text = tree
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.label.as_deref() == Some("label"))
            .unwrap();
        assert!(text.is_tagged(tag));
        assert!(scroll.children.contains(&text.id));
        (scroll.id, text.id, tree.revision())
    };
    runtime.run_frame(constraints).unwrap();
    let before = snapshot(&runtime);
    runtime.run_frame(constraints).unwrap();
    assert_eq!(snapshot(&runtime), before);
}

#[test]
fn viewport_pane_lifetime_keeps_the_outer_id_and_rejects_stale_scroll_requests() {
    use incular_semantics::{SemanticAction, SemanticActionKind};
    use incular_widgets::{CustomScrollView, ScrollController, Sliver, SliverToBoxAdapter, Widget};

    let controller = ScrollController::new();
    let build = |content: bool| {
        let mut slivers: Vec<Box<dyn Sliver>> = Vec::new();
        if content {
            slivers.push(Box::new(SliverToBoxAdapter::new(
                Semantics::new(SizedBox::new().height(1600.)).label("body"),
            )));
        }
        Widget::from(CustomScrollView::new(slivers).controller(controller.clone()))
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    let element = runtime.tree().root().unwrap();
    let viewport = runtime.tree().semantics().roots()[0];
    assert_eq!(
        runtime.tree().semantics().node(viewport).unwrap().role,
        Role::ScrollView
    );
    let jump = || SemanticAction::ScrollToOffset { x: 0., y: 220. };

    let attach = |runtime: &mut Runtime| {
        runtime.schedule_update(element, build(true)).unwrap();
        runtime.run_frame(constraints).unwrap();
        let tree = runtime.tree().semantics();
        assert_eq!(tree.roots(), &[viewport]);
        let boundary = tree.node(viewport).unwrap();
        assert_eq!(boundary.role, Role::GenericContainer);
        assert!(boundary.actions.is_empty());
        assert_eq!(boundary.children.len(), 1);
        let pane = tree.node(boundary.children[0]).unwrap();
        assert_eq!(pane.role, Role::ScrollView);
        assert_eq!(pane.bounds, boundary.bounds);
        assert!(pane.state.has_implicit_scrolling);
        assert!(pane.actions.contains(&SemanticActionKind::ScrollToOffset));
        assert_eq!(
            tree.node(pane.children[0]).unwrap().label.as_deref(),
            Some("body")
        );
        pane.id
    };

    let pane = attach(&mut runtime);
    assert!(!runtime.dispatch_semantic_action(viewport, jump()));
    assert!(runtime.dispatch_semantic_action(pane, jump()));
    assert_eq!(controller.offset(), 220.);
    runtime.run_frame(constraints).unwrap();
    assert_eq!(
        runtime.tree().semantics().node(viewport).unwrap().children,
        &[pane]
    );

    runtime.schedule_update(element, build(false)).unwrap();
    runtime.run_frame(constraints).unwrap();
    let tree = runtime.tree().semantics();
    assert_eq!(tree.roots(), &[viewport]);
    assert_eq!(tree.node(viewport).unwrap().role, Role::ScrollView);
    assert!(tree.node(viewport).unwrap().children.is_empty());
    assert!(tree.node(pane).is_none());
    assert!(!runtime.dispatch_semantic_action(pane, jump()));
    let replacement = attach(&mut runtime);
    assert_ne!(replacement, pane);
    assert!(runtime.dispatch_semantic_action(replacement, jump()));
}

#[test]
fn nested_shrink_wrapped_viewport_panes_stay_aligned_during_semantic_reveals() {
    use incular_semantics::SemanticAction;
    use incular_widgets::{
        Directionality, ListView, ScrollController, SingleChildScrollView, TextDirection,
    };

    let controller = ScrollController::new();
    let list = ListView::builder(50, |i| {
        Semantics::new(SizedBox::new().height(16.)).label(i.to_string())
    })
    .item_extent(16.)
    .shrink_wrap(true);
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            SingleChildScrollView::new(list).controller(controller.clone()),
        )
        .into(),
    )
    .unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();

    let (boundary, pane, targets) = {
        let tree = runtime.tree().semantics();
        let scroller = tree.node(tree.root().unwrap()).unwrap();
        let pane = tree
            .iter()
            .map(|(_, node)| node)
            .find(|node| {
                node.role == Role::ScrollView
                    && node
                        .state
                        .scroll_metrics
                        .is_some_and(|metrics| metrics.maximum == 0.)
            })
            .unwrap();
        let boundary = tree.node(scroller.children[0]).unwrap();
        assert_eq!(boundary.children, [pane.id]);
        assert_eq!(boundary.bounds, pane.bounds);
        assert_eq!(pane.state.scroll_metrics.unwrap().child_count, Some(50));
        let mut hidden = pane
            .children
            .iter()
            .copied()
            .filter(|id| tree.node(*id).is_some_and(|node| node.state.hidden))
            .collect::<Vec<_>>();
        assert!(hidden.len() >= 3);
        let middle = hidden[hidden.len() / 2];
        let last = hidden.pop().unwrap();
        let first_hidden = hidden[0];
        let start = pane.children[0];
        (boundary.id, pane.id, [last, first_hidden, start, middle])
    };

    let mut former = controller.offset();
    for (index, target) in targets.into_iter().enumerate() {
        assert!(runtime.dispatch_semantic_action(target, SemanticAction::ShowOnScreen));
        runtime.run_frame(bounds).unwrap();
        let tree = runtime.tree().semantics();
        let boundary = tree.node(boundary).unwrap();
        let pane = tree.node(pane).unwrap();
        assert_eq!(boundary.children, [pane.id]);
        assert_eq!(boundary.bounds, pane.bounds);
        assert_eq!(pane.bounds.origin.y, -controller.offset());
        assert!(!tree.node(target).unwrap().state.hidden);
        match index {
            0 | 3 => assert!(controller.offset() > former),
            2 => assert!(controller.offset() < former),
            _ => assert_eq!(controller.offset(), former),
        }
        former = controller.offset();
    }
}

#[test]
fn pinned_header_switches_semantic_panes_only_after_losing_layout_extent() {
    use incular_semantics::SemanticActionKind;
    use incular_widgets::{
        CustomScrollView, Directionality, PinnedHeaderSliver, ScrollController, Sliver,
        SliverToBoxAdapter, TextDirection, Viewport,
    };

    let controller = ScrollController::new();
    let slivers: [Box<dyn Sliver>; 3] = [
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(100.))),
        Box::new(PinnedHeaderSliver::new(
            Semantics::new(SizedBox::new().height(40.))
                .label("pinned header")
                .container(true),
        )),
        Box::new(SliverToBoxAdapter::new(
            Semantics::new(SizedBox::new().height(1200.))
                .label("scrolling content")
                .container(true),
        )),
    ];
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers).controller(controller.clone()),
        )
        .into(),
    )
    .unwrap();
    let surface = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(surface).unwrap();
    let snapshot = |runtime: &Runtime, offset: f32| {
        let tree = runtime.tree().semantics();
        let node = |label| {
            tree.iter()
                .map(|(_, node)| node)
                .find(|node| node.label.as_deref() == Some(label))
                .unwrap()
        };
        let header = node("pinned header");
        let body = node("scrolling content");
        let outer = tree.node(tree.root().unwrap()).unwrap();
        let pane = tree.node(outer.children[0]).unwrap();
        let outside = offset > 100.;
        let viewport_tag = Viewport::use_two_pane_semantics_tag();
        let exclude_tag = Viewport::exclude_from_scrolling_tag();

        assert_eq!(header.bounds.origin.y, (100. - offset).max(0.));
        assert!(!header.state.hidden);
        let tagged_header = if header.is_tagged(viewport_tag) {
            header
        } else {
            tree.iter()
                .map(|(_, node)| node)
                .find(|node| node.children.contains(&header.id))
                .unwrap()
        };
        assert!(tagged_header.is_tagged(viewport_tag));
        assert_eq!(tagged_header.is_tagged(exclude_tag), outside);
        assert!(!body.is_tagged(exclude_tag));
        assert_eq!(outer.children.contains(&tagged_header.id), outside);
        assert_eq!(pane.children.contains(&tagged_header.id), !outside);
        assert!(pane.children.contains(&body.id));
        assert_eq!(pane.bounds, outer.bounds);
        assert_eq!(pane.state.scroll_metrics.unwrap().offset, f64::from(offset));
        assert!(pane.actions.contains(&SemanticActionKind::ScrollToOffset));
        assert!(!outer.actions.contains(&SemanticActionKind::ScrollToOffset));
        (header.id, body.id, outer.id, pane.id)
    };
    let initial = snapshot(&runtime, 0.);
    for offset in [99., 100., 101., 130., 100., 0.] {
        assert!(controller.jump_to(offset));
        runtime.run_frame(surface).unwrap();
        assert_eq!(snapshot(&runtime, offset), initial);
        // Resize with no application rebuild, including while excluded.
        runtime
            .run_frame(Constraints::tight(Size::new(800., 640.)))
            .unwrap();
        assert_eq!(snapshot(&runtime, offset), initial);
        runtime.run_frame(surface).unwrap();
    }
}

#[test]
fn viewport_reports_total_count_and_first_visible_index_on_the_scroll_pane_only() {
    for axis in [
        incular_config::Axis::Vertical,
        incular_config::Axis::Horizontal,
    ] {
        for reverse in [false, true] {
            check_collection_axis(axis, reverse);
        }
    }
}

fn check_collection_axis(axis: incular_config::Axis, reverse: bool) {
    use incular_accessibility::MobileAccessibilityProjection;
    use incular_widgets::{
        CustomScrollView, Directionality, PinnedHeaderSliver, ScrollController, Sliver,
        SliverFixedExtentList, TextDirection,
    };

    let controller = ScrollController::new();
    let slivers: [Box<dyn Sliver>; 2] = [
        Box::new(PinnedHeaderSliver::new(
            Semantics::new(SizedBox::square(20.))
                .container(true)
                .label("pinned"),
        )),
        Box::new(SliverFixedExtentList::new(50, 50., |i| {
            Semantics::new(SizedBox::square(50.))
                .container(true)
                .label(format!("row {i}"))
        })),
    ];
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .scroll_direction(axis)
                .reverse(reverse)
                .semantic_child_count(50),
        )
        .into(),
    )
    .unwrap();
    let surface = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(surface).unwrap();
    let tree = runtime.tree().semantics();
    let outer = tree.root().unwrap();
    let pane = tree.node(outer).unwrap().children[0];
    let first = tree
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.state.item_index == Some(0))
        .unwrap()
        .id;
    let mut projection = MobileAccessibilityProjection::new();
    let mut native_pane = None;
    for (offset, index) in [
        (0., 0),
        (49., 0),
        (50., 1),
        (55., 1),
        (70., 1),
        (120., 2),
        (70., 1),
        (0., 0),
    ] {
        let _ = controller.jump_to(offset);
        runtime.run_frame(surface).unwrap();
        let tree = runtime.tree().semantics();
        assert_eq!(tree.root(), Some(outer));
        let outside = tree.node(outer).unwrap();
        assert!(outside.state.scroll_metrics.is_none());
        assert!(outside.state.item_index.is_none());
        assert_eq!(outside.children[0], pane);
        let header = tree
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.label.as_deref() == Some("pinned"))
            .unwrap();
        assert_eq!(header.state.item_index, None);
        let contents = tree.node(pane).unwrap();
        let metrics = contents.state.scroll_metrics.unwrap();
        assert_eq!(metrics.child_count, Some(50));
        assert_eq!(
            metrics.first_visible_index,
            Some(index),
            "{axis:?} reverse={reverse} at {offset}",
        );
        let row = tree
            .node(first)
            .expect("cached indexed row stays materialized");
        assert_eq!(row.state.item_index, Some(0));
        assert_eq!(
            row.state.hidden,
            index > 0,
            "{axis:?} reverse={reverse} at {offset}"
        );
        let update = projection
            .sync(tree)
            .expect("a scroll frame publishes changed metrics");
        let record = update
            .nodes
            .iter()
            .find(|node| node.semantic.id == pane)
            .unwrap();
        assert_eq!(record.semantic.state.scroll_metrics, Some(metrics));
        if let Some(native_pane) = native_pane {
            assert_eq!(record.native_id, native_pane);
        }
        native_pane = Some(record.native_id);
    }
}

#[test]
fn grid_scroll_count_counts_tiles_not_materialized_rows() {
    use incular_widgets::{
        Directionality, GridView, ScrollController, SliverGridDelegate, TextDirection,
    };

    let controller = ScrollController::new();
    let grid = GridView::builder(31, SliverGridDelegate::fixed_cross_axis_count(3), |i| {
        Semantics::new(SizedBox::shrink()).label(format!("tile {i}"))
    })
    .controller(controller.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, grid).into()).unwrap();
    let surface = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(surface).unwrap();
    for (offset, first) in [(0., 0), (300., 3)] {
        let _ = controller.jump_to(offset);
        runtime.run_frame(surface).unwrap();
        let tree = runtime.tree().semantics();
        let metrics = tree
            .iter()
            .find_map(|(_, node)| node.state.scroll_metrics)
            .unwrap();
        assert_eq!(metrics.child_count, Some(31));
        assert_eq!(metrics.first_visible_index, Some(first));
    }
}

#[test]
fn resizing_and_persistent_headers_have_distinct_scroll_exclusion_rules() {
    use incular_widgets::{
        CustomScrollView, Directionality, ScrollController, Sliver, SliverHeaderScrollBehavior,
        SliverPersistentHeader, SliverResizingHeader, SliverToBoxAdapter, TextDirection, Viewport,
    };

    for variant in 0..3 {
        let persistent = variant != 0;
        let controller = ScrollController::new();
        let label = || {
            Semantics::new(SizedBox::new().height(300.))
                .container(true)
                .label("header")
        };
        let header: Box<dyn Sliver> = match variant {
            0 => Box::new(SliverResizingHeader::new(300., 300., label())),
            1 => Box::new(SliverPersistentHeader::new(300., label()).pinned(true)),
            _ => Box::new(
                SliverResizingHeader::with_delegate(300., 300., move |_, _| label())
                    .scroll_behavior(SliverHeaderScrollBehavior::Pinned),
            ),
        };
        let slivers: [Box<dyn Sliver>; 3] = [
            Box::new(SliverToBoxAdapter::new(SizedBox::new().height(100.))),
            header,
            Box::new(SliverToBoxAdapter::new(
                Semantics::new(SizedBox::new().height(1600.)).label("content"),
            )),
        ];
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                CustomScrollView::new(slivers).controller(controller.clone()),
            )
            .into(),
        )
        .unwrap();
        let surface = Constraints::tight(Size::new(800., 600.));
        runtime.run_frame(surface).unwrap();
        let mut retained_id = None;
        for offset in [0_f32, 99., 100., 101., 400., 100., 0.] {
            let _ = controller.jump_to(offset);
            runtime.run_frame(surface).unwrap();
            let tree = runtime.tree().semantics();
            let outer = tree.node(tree.root().unwrap()).unwrap();
            let pane = tree.node(outer.children[0]).unwrap();
            let header = tree
                .iter()
                .map(|(_, node)| node)
                .find(|node| node.label.as_deref() == Some("header"))
                .unwrap();
            let excluded = persistent || offset > 100.;
            assert_eq!(header.bounds.origin.y, (100. - offset).max(0.));
            assert_eq!(header.bounds.size, Size::new(800., 300.));
            assert!(header.is_tagged(Viewport::use_two_pane_semantics_tag()));
            assert_eq!(
                header.is_tagged(Viewport::exclude_from_scrolling_tag()),
                excluded
            );
            assert_eq!(outer.children.contains(&header.id), excluded);
            assert_eq!(pane.children.contains(&header.id), !excluded);
            if let Some(id) = retained_id {
                assert_eq!(header.id, id);
            }
            retained_id = Some(header.id);
        }
    }
}

#[test]
fn immediate_viewport_tag_stops_at_nested_sliver_and_lazy_box_containers() {
    use incular_widgets::{
        CustomScrollView, Directionality, Sliver, SliverList, SliverMainAxisGroup, SliverSemantics,
        SliverToBoxAdapter, Text, TextDirection, Viewport,
    };

    for lazy_box in [false, true] {
        let group = if lazy_box {
            SliverSemantics::new(
                || Semantics::default().container(true),
                SliverList::builder(2, |i| {
                    Semantics::new(Text::new(format!("child {}", i + 1))).container(true)
                }),
            )
        } else {
            let slivers: [Box<dyn Sliver>; 2] = ["child 1", "child 2"].map(|label| {
                Box::new(SliverSemantics::new(
                    || Semantics::default().container(true),
                    SliverToBoxAdapter::new(Text::new(label)),
                )) as Box<dyn Sliver>
            });
            SliverSemantics::new(
                || Semantics::default().container(true),
                SliverMainAxisGroup::new(slivers),
            )
        };
        let slivers: [Box<dyn Sliver>; 1] = [Box::new(group)];
        let mut runtime = Runtime::new(
            Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into(),
        )
        .unwrap();
        let bounds = Constraints::tight(Size::new(800., 600.));
        runtime.run_frame(bounds).unwrap();
        let shape = |runtime: &Runtime| {
            let tree = runtime.tree().semantics();
            let outer = tree.node(tree.root().unwrap()).unwrap();
            assert_eq!(outer.children.len(), 1);
            assert!(outer.label.is_none());
            assert!(outer.tags().is_empty());
            assert!(!outer.state.has_implicit_scrolling);
            let pane = tree.node(outer.children[0]).unwrap();
            assert!(pane.state.has_implicit_scrolling);
            assert!(pane.label.is_none());
            assert!(pane.tags().is_empty());
            assert_eq!(pane.children.len(), 1);
            let group = tree.node(pane.children[0]).unwrap();
            assert!(group.label.is_none());
            let tag = Viewport::use_two_pane_semantics_tag();
            assert_eq!(group.tags(), &[tag.clone()]);
            assert_eq!(group.children.len(), 2);
            for (&id, label) in group.children.iter().zip(["child 1", "child 2"]) {
                let child = tree.node(id).unwrap();
                assert_eq!(
                    child.label.as_deref(),
                    Some(label),
                    "lazy_box={lazy_box}\n{}",
                    runtime.tree().semantics_debug_dump(),
                );
                assert_eq!(child.state.text_direction, Some(TextDirection::Ltr));
                assert!(child.tags().is_empty());
                assert!(!child.state.has_implicit_scrolling);
                assert!(child.children.is_empty());
            }
            for (_, node) in tree.iter() {
                assert!(node.actions.is_empty());
                assert!(node.value.is_none());
                assert!(node.description.is_none());
                assert!(node.state.tooltip.is_none());
                assert!(node.state.selection.is_none());
                assert!(node.state.increased_value.is_none());
                assert!(node.state.decreased_value.is_none());
                assert!(node.state.checked.is_none());
                assert!(node.state.toggled.is_none());
                assert!(node.state.expanded.is_none());
                assert!(node.state.identifier.is_none());
                assert!(node.state.link_url.is_none());
                assert!(node.state.heading_level.is_none());
                assert!(!node.state.hidden);
                assert!(!node.state.has_enabled_state);
                assert!(!node.state.focusable);
                assert!(!node.state.focused);
            }
            (
                outer.id,
                pane.id,
                group.id,
                group.children.clone(),
                tree.revision(),
            )
        };
        let before = shape(&runtime);
        runtime.run_frame(bounds).unwrap();
        assert_eq!(shape(&runtime), before);
    }
}

#[test]
fn sliver_container_rebuilds_its_absorbed_label_and_can_release_it_as_an_explicit_child() {
    use incular_widgets::{
        CustomScrollView, Directionality, Sliver, SliverSemantics, SliverToBoxAdapter, Text,
        TextDirection,
    };

    let build = |text: &'static str, explicit_children| {
        let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
            move || {
                Semantics::default()
                    .container(true)
                    .explicit_child_nodes(explicit_children)
            },
            SliverToBoxAdapter::new(Text::new(text)),
        ))];
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
    };
    let mut runtime = Runtime::new(build("initial", false)).unwrap();
    let surface = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(surface).unwrap();
    let root = runtime.tree().root().unwrap();
    let anchor_id = |runtime: &Runtime| {
        let tree = runtime.tree().semantics();
        let outer = tree.node(tree.root().unwrap()).unwrap();
        let inner = tree.node(outer.children[0]).unwrap();
        inner.children[0]
    };
    let id = anchor_id(&runtime);
    let mut external_child = None;
    for (text, explicit) in [
        ("initial", false),
        ("changed", false),
        ("standalone", true),
        ("absorbed", false),
    ] {
        runtime
            .schedule_update(root, build(text, explicit))
            .unwrap();
        runtime.run_frame(surface).unwrap();
        assert_eq!(anchor_id(&runtime), id);
        let tree = runtime.tree().semantics();
        let anchor = tree.node(id).unwrap();
        if explicit {
            assert_eq!(anchor.label, None);
            assert_eq!(anchor.children.len(), 1);
            let child = tree.node(anchor.children[0]).unwrap();
            assert_eq!(child.label.as_deref(), Some(text));
            external_child = Some(child.id);
        } else {
            assert_eq!(anchor.label.as_deref(), Some(text));
            assert!(anchor.children.is_empty());
            if let Some(previous_child) = external_child {
                assert!(tree.node(previous_child).is_none());
            }
        }
    }
}
