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
