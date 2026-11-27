use accesskit::{Invalid, Role as AccessKitRole};
use incular_accessibility::AccessKitProjection;
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticNode, SemanticValidationResult};
use incular_widgets::{Directionality, ListView, Semantics, Text, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn labelled<'a>(runtime: &'a Runtime, label: &str) -> &'a SemanticNode {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some(label)).then_some(node))
        .expect("label is exposed")
}

#[test]
fn all_six_headings_are_navigable_native_headings_and_plain_text_is_not() {
    let children = (1u8..=6)
        .map(|level| {
            Widget::from(
                Semantics::new(Text::new(format!("Heading level {level}"))).heading_level(level),
            )
            .with_key(u64::from(level))
        })
        .chain([Widget::from(Text::new("This is not a heading"))])
        .collect::<Vec<_>>();
    let view = Directionality::new(TextDirection::Ltr, ListView::new(children));
    let mut runtime = Runtime::new(view.into()).unwrap();
    frame(&mut runtime);
    let mut projection = AccessKitProjection::new();
    projection.activate();
    let native = projection
        .sync(runtime.tree().semantics(), 1.)
        .unwrap()
        .into_accesskit();
    let projected = |node: &SemanticNode| {
        let id = projection.native_node_id(node.id).unwrap();
        &native
            .nodes
            .iter()
            .find(|(candidate, _)| candidate.0 == id)
            .unwrap()
            .1
    };
    for level in 1..=6 {
        let label = format!("Heading level {level}");
        let node = labelled(&runtime, &label);
        assert_eq!(node.state.heading_level, Some(level));
        assert_eq!(projected(node).role(), AccessKitRole::Heading);
        assert_eq!(projected(node).level(), Some(usize::from(level)));
        assert_eq!(projected(node).label(), Some(label.as_str()));
        assert_eq!(node.role, Role::Heading);
    }
    let plain = labelled(&runtime, "This is not a heading");
    assert_eq!(plain.state.heading_level, None);
    assert_eq!(projected(plain).role(), AccessKitRole::Label);
    assert_eq!(projected(plain).value(), Some("This is not a heading"));
}

#[test]
fn explicit_annotations_describe_none_valid_and_invalid_before_collection() {
    use SemanticValidationResult::{Invalid, None, Valid};
    for value in [None, Valid, Invalid] {
        let name = format!("Validation result {value:?}");
        let descriptor = Semantics::new(Text::new(&name).text_direction(TextDirection::Ltr))
            .text_direction(TextDirection::Ltr)
            .validation_result(value);
        let explicit = descriptor.explicit().expect("the annotation is retained");
        assert_eq!(explicit.state.effective_validation_result(), value);
        let mut runtime = Runtime::new(descriptor.into()).unwrap();
        frame(&mut runtime);
        let node = labelled(&runtime, &name);
        assert_eq!(node.state.effective_validation_result(), value);
    }
}

#[test]
fn nested_validation_results_follow_precedence_and_clear_native_invalid_state() {
    use SemanticValidationResult::{Invalid as Bad, None as Unspecified, Valid};
    let cases = [
        (Unspecified, Unspecified, Unspecified),
        (Unspecified, Valid, Valid),
        (Unspecified, Bad, Bad),
        (Valid, Unspecified, Valid),
        (Valid, Valid, Valid),
        (Valid, Bad, Bad),
        (Bad, Unspecified, Bad),
        (Bad, Valid, Bad),
        (Bad, Bad, Bad),
        (Unspecified, Valid, Valid),
    ];
    let build = |outer, inner| {
        Semantics::new(
            Semantics::new(
                Text::new(format!("Outer = {outer:?}; inner = {inner:?}"))
                    .text_direction(TextDirection::Ltr),
            )
            .validation_result(inner),
        )
        .validation_result(outer)
        .text_direction(TextDirection::Ltr)
    };
    let scene = |outer, inner| Directionality::new(TextDirection::Ltr, build(outer, inner));
    let mut runtime = Runtime::new(scene(Unspecified, Unspecified).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let mut projection = AccessKitProjection::new();
    projection.activate();
    for (outer, inner, expected) in cases {
        runtime
            .schedule_update(root, scene(outer, inner).into())
            .unwrap();
        frame(&mut runtime);
        let node = labelled(&runtime, &format!("Outer = {outer:?}; inner = {inner:?}"));
        assert_eq!(node.state.effective_validation_result(), expected);
        let native = projection
            .sync(runtime.tree().semantics(), 1.)
            .unwrap()
            .into_accesskit();
        let id = projection.native_node_id(node.id).unwrap();
        let (_, projected) = native
            .nodes
            .iter()
            .find(|(candidate, _)| candidate.0 == id)
            .unwrap();
        assert_eq!(
            projected.invalid(),
            (expected == Bad).then_some(Invalid::True)
        );
    }
}

#[test]
fn a_child_container_stops_outer_validation_from_overriding_its_label() {
    use SemanticValidationResult::{Invalid, Valid};
    let scene = |boundary| {
        let inner = Semantics::new(Text::new("status"))
            .container(boundary)
            .validation_result(Valid);
        Directionality::new(
            TextDirection::Ltr,
            Semantics::new(inner).validation_result(Invalid),
        )
    };
    let mut runtime = Runtime::new(scene(false).into()).unwrap();
    frame(&mut runtime);
    assert_eq!(
        labelled(&runtime, "status")
            .state
            .effective_validation_result(),
        Invalid
    );
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(true).into()).unwrap();
    frame(&mut runtime);
    assert_eq!(
        labelled(&runtime, "status")
            .state
            .effective_validation_result(),
        Valid
    );
}
