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
