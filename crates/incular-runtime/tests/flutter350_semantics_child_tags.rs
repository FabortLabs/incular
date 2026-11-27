//! Flutter tagForChildren inheritance and incremental removal.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticNodeId, SemanticsTag};
use incular_widgets::{Semantics, SizedBox, Widget};

fn semantics(runtime: &Runtime, label: &str) -> (SemanticNodeId, Vec<SemanticsTag>) {
    runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some(label))
        .map(|(id, node)| (id, node.tags().to_vec()))
        .expect("retained semantic boundary")
}

#[test]
fn child_tags_accumulate_through_layout_but_stop_at_semantic_boundaries() {
    let old_tag = SemanticsTag::new("old");
    let new_tag = SemanticsTag::new("new");
    let through = SemanticsTag::new("pass-through");
    let build = |tag: Option<SemanticsTag>| {
        let leaf = Semantics::new(SizedBox::square(20.))
            .container(true)
            .label("leaf");
        let child = Semantics::new(leaf)
            .container(true)
            .explicit_child_nodes(true)
            .label("child");
        let intermediate = Semantics::new(child).tag_for_children(through.clone());
        Widget::from(
            Semantics::new(intermediate)
                .container(true)
                .explicit_child_nodes(true)
                .label("parent")
                .tag_for_children(tag),
        )
    };
    let bounds = Constraints::tight(Size::new(200., 200.));
    let mut runtime = Runtime::new(build(Some(old_tag.clone()))).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(semantics(&runtime, "parent").1.is_empty());
    assert!(semantics(&runtime, "leaf").1.is_empty());
    let (id, tags) = semantics(&runtime, "child");
    assert_eq!(tags, vec![old_tag, through.clone()]);
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(Some(new_tag.clone())))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(semantics(&runtime, "child"), (id, vec![new_tag, through.clone()]));
    assert!(semantics(&runtime, "leaf").1.is_empty());

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(None))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    let (without, tags) = semantics(&runtime, "child");
    assert_eq!(without, id);
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name(), "pass-through");
}

#[test]
fn an_ordinary_parent_tags_both_immediate_semantic_text_children_not_itself() {
    use incular_config::TextDirection;
    use incular_widgets::{Column, Directionality, Text};

    let tag = SemanticsTag::new("custom tag");
    let child = |text| Widget::from(Semantics::new(Text::new(text)).container(true));
    let parent = Semantics::new(Column::new([child("child 1"), child("child 2")]))
        .container(true)
        .tag_for_children(tag.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, parent).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let tree = runtime.tree().semantics();
    let parent = tree.node(tree.root().unwrap()).unwrap();
    assert!(parent.tags().is_empty());
    let labels = parent
        .children
        .iter()
        .map(|&id| {
            let node = tree.node(id).unwrap();
            assert_eq!(node.tags(), std::slice::from_ref(&tag));
            assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
            node.label.as_deref().unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["child 1", "child 2"]);
}

#[test]
fn a_list_view_two_pane_tag_does_not_mark_a_nested_implicit_scroller() {
    use incular_config::TextDirection;
    use incular_semantics::Role;
    use incular_widgets::{
        Column, Directionality, ListView, SingleChildScrollView, Text, Viewport,
    };

    let inner = SingleChildScrollView::new(Column::new([
        Widget::from(SizedBox::new().height(20.)),
        Widget::from(Text::new("label")),
    ]));
    let list = ListView::new([inner]);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    runtime.set_platform_semantics_enabled(false);
    let _handle = runtime.ensure_semantics();
    let limits = Constraints::tight(Size::new(800., 600.));
    let tag = Viewport::use_two_pane_semantics_tag();
    let mut old_revision = None;
    for _ in 0..2 {
        runtime.run_frame(limits).unwrap();
        let tree = runtime.tree().semantics();
        assert!(
            tree.iter()
                .any(|(_, node)| node.label.as_deref() == Some("label"))
        );
        assert!(tree.iter().any(|(_, node)| {
            node.role == Role::ScrollView && node.state.has_implicit_scrolling
        }));
        assert!(
            !tree
                .iter()
                .any(|(_, node)| { node.state.has_implicit_scrolling && node.is_tagged(tag) })
        );
        let tagged = tree
            .iter()
            .filter(|(_, node)| node.is_tagged(tag))
            .collect::<Vec<_>>();
        let [(_, index)] = tagged.as_slice() else {
            panic!("one tagged list item")
        };
        assert_eq!(index.state.item_index, Some(0));
        assert_eq!(index.role, Role::GenericContainer);
        assert!(!index.state.has_implicit_scrolling);
        let [child] = index.children.as_slice() else {
            panic!("one inner scroll boundary")
        };
        let scroller = tree.node(*child).unwrap();
        assert_eq!(scroller.role, Role::ScrollView);
        assert!(scroller.state.has_implicit_scrolling);
        assert_eq!(scroller.state.item_index, None);
        assert!(!scroller.is_tagged(tag));
        assert_eq!(
            tree.node(scroller.children[0]).unwrap().label.as_deref(),
            Some("label")
        );
        assert_eq!(
            tree.revision(),
            *old_revision.get_or_insert(tree.revision())
        );
    }
}

#[test]
fn point_actions_reach_children_through_viewport_and_index_panes() {
    use incular_config::TextDirection;
    use incular_core::Offset;
    use incular_semantics::SemanticAction;
    use incular_widgets::{Directionality, ListView, SingleChildScrollView, Text};
    use std::{cell::Cell, rc::Rc};

    let taps = Rc::new(Cell::new(0));
    let leaf = Semantics::new(Text::new("label")).container(true).on_tap({
        let taps = taps.clone();
        move || taps.set(taps.get() + 1)
    });
    let scene = Directionality::new(
        TextDirection::Ltr,
        ListView::new([SingleChildScrollView::new(leaf)]),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let tree = runtime.tree().semantics();
    let bounds = tree
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some("label")).then_some(node.bounds))
        .unwrap();
    let center = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    assert!(runtime.dispatch_semantic_action_at(center, SemanticAction::Activate));
    assert_eq!(taps.get(), 1);
}
