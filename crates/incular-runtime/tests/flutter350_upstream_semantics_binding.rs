//! Public Runtime counterparts to the pinned semantics binding/owner cases.

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticNodeId};
use incular_widgets::{
    Align, Alignment, Directionality, Semantics, SizedBox, Text, TextDirection, Transform, Widget,
};

fn scene(child: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, child).into()
}

fn semantic_id(runtime: &Runtime, label: &str) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some(label)).then_some(id))
        .expect("mounted semantic node")
}

#[test]
fn transformed_semantic_bounds_are_in_logical_view_coordinates() {
    let target = Semantics::new(SizedBox::new().width(100.).height(50.)).label("target");
    let translated = Transform::translation(Offset::new(40., 20.), target);
    let mut runtime = Runtime::new(scene(Align::new(Alignment::TOP_LEFT, translated))).unwrap();
    let metrics = WindowMetrics::new(PhysicalSize::new(800, 600), 2.);
    runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
    assert_eq!(runtime.environment().viewport, Size::new(400., 300.));
    runtime
        .run_frame(Constraints::tight(runtime.environment().viewport))
        .unwrap();

    let target = semantic_id(&runtime, "target");
    assert_eq!(
        runtime.tree().semantics().node(target).unwrap().bounds,
        Rect::from_origin_size(Offset::new(40., 20.), Size::new(100., 50.)),
    );
}

#[test]
fn show_on_screen_ignores_a_stale_semantic_node() {
    let mut runtime = Runtime::new(scene(
        Semantics::new(Text::new("Hello World")).explicit_child_nodes(true),
    ))
    .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    let stale = semantic_id(&runtime, "Hello World");

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(root, scene(SizedBox::shrink()))
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    assert!(runtime.tree().semantics().node(stale).is_none());
    assert!(!runtime.dispatch_semantic_action(stale, SemanticAction::ShowOnScreen));
}
