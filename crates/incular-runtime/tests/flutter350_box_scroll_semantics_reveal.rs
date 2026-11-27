use incular_accessibility::AccessKitProjection;
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::SemanticNode;
use incular_widgets::{
    Column, Directionality, ScrollController, Semantics, SingleChildScrollView, SizedBox, Text,
    Widget,
};

#[test]
fn an_offscreen_label_stays_retained_and_clears_hidden_when_scrolled_into_view() {
    const LABEL: &str = "Label text 10";
    let controller = ScrollController::new();
    let children = (0..20).flat_map(|index| {
        [
            Widget::from(Text::new(format!("Label text {index}"))),
            Widget::from(SizedBox::new().height(50.)),
        ]
    });
    let scene = Directionality::new(
        TextDirection::Ltr,
        SingleChildScrollView::new(
            Semantics::new(Column::new(children))
                .container(true)
                .explicit_child_nodes(true),
        )
        .controller(controller.clone()),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let limits = Constraints::tight(Size::new(800., 600.));
    let node = |runtime: &Runtime| -> SemanticNode {
        runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(_, node)| (node.label.as_deref() == Some(LABEL)).then(|| node.clone()))
            .expect("offscreen content keeps a semantic identity")
    };
    let mut projection = AccessKitProjection::new();
    projection.activate();
    let mut original = None;
    for (offset, hidden) in [(0., true), (500., false), (0., true)] {
        if offset != controller.offset() {
            assert!(controller.jump_to(offset));
        }
        runtime.run_frame(limits).unwrap();
        let current = node(&runtime);
        let initial = original.get_or_insert((current.id, current.bounds));
        assert_eq!(current.id, initial.0);
        assert_eq!(current.state.hidden, hidden);
        assert!((current.bounds.origin.y - (initial.1.origin.y - offset)).abs() < 0.001);
        assert_eq!(current.bounds.size, initial.1.size);

        let native = projection
            .sync(runtime.tree().semantics(), 1.)
            .expect("the accessibility geometry must update")
            .into_accesskit();
        let id = projection.native_node_id(current.id).unwrap();
        let (_, node) = native
            .nodes
            .iter()
            .find(|(candidate, _)| candidate.0 == id)
            .unwrap();
        assert_eq!(node.is_hidden(), hidden);
    }
}
