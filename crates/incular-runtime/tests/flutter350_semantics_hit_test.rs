//! Flutter's semantic hit-test metadata (distinct from normal pointer hit-testing).

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticNodeId, SemanticState, SemanticsHitTestBehavior};
use incular_widgets::{Center, Column, MergeSemantics, Semantics, SizedBox, Widget};

#[test]
fn copying_semantics_preserves_every_hit_test_policy_and_boundary() {
    use SemanticsHitTestBehavior::{Defer, Opaque, Transparent};
    assert_eq!(SemanticState::default().hit_test_behavior, Defer);
    for behavior in [Defer, Opaque, Transparent] {
        let source = SemanticState {
            hit_test_behavior: behavior,
            container: true,
            ..SemanticState::default()
        };
        let copy = source.clone();
        assert_eq!(copy, source);
        assert_eq!(copy.hit_test_behavior, behavior);
        assert!(copy.container);
        let layout = Semantics::new(SizedBox::square(15.))
            .container(true)
            .label("test")
            .hit_test_behavior(copy.hit_test_behavior);
        let mut runtime = Runtime::new(layout.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(40., 40.)))
            .unwrap();
        let node = runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(_, node)| (node.label.as_deref() == Some("test")).then_some(node))
            .unwrap();
        assert_eq!(node.state.hit_test_behavior, behavior);
        assert!(node.state.container);
    }
}

#[test]
fn a_merge_inherits_the_first_non_default_policy_and_preserves_an_explicit_parent() {
    use SemanticsHitTestBehavior::{Defer, Opaque, Transparent};
    let build = |parent| {
        let leaves = Column::new([
            Widget::from(Semantics::new(SizedBox::square(15.)).label("default")),
            Widget::from(
                Semantics::new(SizedBox::square(15.))
                    .label("opaque")
                    .hit_test_behavior(Opaque),
            ),
            Widget::from(
                Semantics::new(SizedBox::square(15.))
                    .label("transparent")
                    .hit_test_behavior(Transparent),
            ),
        ]);
        let parent = Semantics::new(leaves)
            .label("parent")
            .hit_test_behavior(parent);
        Widget::from(Center::new(MergeSemantics::new(parent)))
    };
    let viewport = Constraints::tight(Size::new(200., 100.));
    let mut runtime = Runtime::new(build(Defer)).unwrap();
    let policy = |runtime: &Runtime| -> (SemanticNodeId, SemanticsHitTestBehavior) {
        runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| {
                node.label
                    .as_deref()
                    .is_some_and(|label| label.starts_with("parent"))
            })
            .map(|(id, node)| (id, node.state.hit_test_behavior))
            .unwrap()
    };
    runtime.run_frame(viewport).unwrap();
    let (id, first) = policy(&runtime);
    assert_eq!(first, Opaque);
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(Transparent))
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(policy(&runtime), (id, Transparent));
}
