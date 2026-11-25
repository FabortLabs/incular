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
