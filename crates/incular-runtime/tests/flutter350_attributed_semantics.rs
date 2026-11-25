use incular_accessibility::MobileAccessibilityProjection;
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{AttributedString, SemanticTextAttributes, StringAttribute};
use incular_widgets::{
    Directionality, MergeSemantics, Padding, Placeholder, Semantics, SizedBox, Widget,
};

fn annotated(value: &str, attribute: StringAttribute) -> AttributedString {
    AttributedString::try_with_attributes(value, [attribute]).unwrap()
}

#[test]
fn typed_label_value_and_hint_keep_attributes_in_each_retained_mobile_field() {
    let label = StringAttribute::spell_out(0..5);
    let value = StringAttribute::locale(0..5, "en-MX");
    let hint = StringAttribute::spell_out(1..2);
    let props = Semantics::new(Placeholder::new())
        .label("initial label")
        .value("initial value")
        .hint("initial hint")
        .attributed_label(annotated("label", label.clone()))
        .attributed_value(annotated("value", value.clone()))
        .attributed_hint(annotated("hint", hint.clone()));
    let explicit = props.explicit().unwrap();
    assert_eq!(explicit.label.as_deref(), Some("label"));
    assert_eq!(explicit.value.as_deref(), Some("value"));
    assert_eq!(explicit.description.as_deref(), Some("hint"));
    let expected = SemanticTextAttributes {
        label: vec![label],
        value: vec![value],
        hint: vec![hint],
    };
    assert_eq!(explicit.state.text_attributes.as_deref(), Some(&expected));
    let intermediate = props.clone().label("plain label").explicit().unwrap();
    let attrs = intermediate.state.text_attributes.as_ref().unwrap();
    assert!(attrs.label.is_empty());
    assert_eq!(attrs.value, expected.value);
    assert_eq!(attrs.hint, expected.hint);
    let replacement = String::from("plain from a borrowed String");
    let typed = Semantics::builder()
        .label(&replacement)
        .value(annotated("label", StringAttribute::spell_out(0..5)))
        .build();
    let typed = typed.explicit().unwrap();
    assert_eq!(typed.label.as_deref(), Some(replacement.as_str()));
    assert_eq!(
        typed.state.text_attributes.as_ref().unwrap().value,
        [StringAttribute::spell_out(0..5)]
    );

    let mut runtime = Runtime::new(props.clone().into()).unwrap();
    let constraints = Constraints::tight(Size::new(100., 100.));
    runtime.run_frame(constraints).unwrap();
    let mut mobile = MobileAccessibilityProjection::new();
    let initial = mobile.sync(runtime.tree().semantics()).unwrap();
    let native = initial
        .nodes
        .iter()
        .find(|node| node.semantic.label.as_deref() == Some("label"))
        .unwrap();
    assert_eq!(native.semantic.value.as_deref(), Some("value"));
    assert_eq!(native.semantic.description.as_deref(), Some("hint"));
    assert_eq!(
        native.semantic.state.text_attributes.as_deref(),
        Some(&expected)
    );

    let root = runtime.tree().root().unwrap();
    let plain = props
        .label("plain label")
        .value("plain value")
        .hint("plain hint");
    let explicit = plain.explicit().unwrap();
    assert!(explicit.state.text_attributes.is_none());
    runtime.tree_mut().update(root, plain.into()).unwrap();
    runtime.run_frame(constraints).unwrap();
    let update = mobile.sync(runtime.tree().semantics()).unwrap();
    assert_eq!(update.nodes.len(), 1);
    assert_eq!(update.nodes[0].native_id, native.native_id);
    assert_eq!(
        update.nodes[0].semantic.label.as_deref(),
        Some("plain label")
    );
    assert_eq!(
        update.nodes[0].semantic.value.as_deref(),
        Some("plain value")
    );
    assert_eq!(
        update.nodes[0].semantic.description.as_deref(),
        Some("plain hint")
    );
    assert!(update.nodes[0].semantic.state.text_attributes.is_none());
}

#[test]
fn merged_label_offsets_count_newlines_direction_marks_and_utf16() {
    let scene = |direction| -> Widget {
        let inner = Semantics::new(SizedBox::square(10.))
            .attributed_label(annotated("مرحبا", StringAttribute::locale(0..2, "ar")))
            .text_direction(direction)
            .container(true);
        Directionality::new(
            TextDirection::Ltr,
            MergeSemantics::new(Semantics::new(Padding::zero(inner)).label("😀")),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(TextDirection::Rtl)).unwrap();
    let constraints = Constraints::tight(Size::new(100., 100.));
    runtime.run_frame(constraints).unwrap();
    let mut mobile = MobileAccessibilityProjection::new();
    let initial = mobile.sync(runtime.tree().semantics()).unwrap();
    assert_eq!(initial.nodes.len(), 1);
    let node = &initial.nodes[0];
    assert_eq!(
        node.semantic.label.as_deref(),
        Some("😀\n\u{202b}مرحبا\u{202c}")
    );
    assert_eq!(
        node.semantic.state.text_attributes.as_ref().unwrap().label,
        [StringAttribute::locale(4..6, "ar")]
    );

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(root, scene(TextDirection::Ltr))
        .unwrap();
    runtime.run_frame(constraints).unwrap();
    let update = mobile.sync(runtime.tree().semantics()).unwrap();
    assert_eq!(update.nodes.len(), 1);
    assert_eq!(update.nodes[0].native_id, node.native_id);
    assert_eq!(update.nodes[0].semantic.label.as_deref(), Some("😀\nمرحبا"));
    assert_eq!(
        update.nodes[0]
            .semantic
            .state
            .text_attributes
            .as_ref()
            .unwrap()
            .label,
        [StringAttribute::locale(3..5, "ar")]
    );
}
