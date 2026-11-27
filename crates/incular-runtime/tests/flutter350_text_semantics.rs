use incular_accessibility::{AccessKitProjection, MobileAccessibilityProjection};
use incular_config::{Constraints, Locale, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAnnotationFlag, SemanticNode, SemanticNodeId, StringAttribute};
use incular_widgets::{Directionality, SelectableRegion, Semantics, Text, Widget};

fn scene(widget: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, widget).into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn node_by_label<'a>(runtime: &'a Runtime, label: &str) -> (SemanticNodeId, &'a SemanticNode) {
    let tree = runtime.tree().semantics();
    tree.iter()
        .find(|(_, node)| node.label.as_deref() == Some(label))
        .unwrap()
}

fn locale(tag: &str) -> Locale {
    tag.parse().unwrap()
}

fn assert_text_locale(widget: impl Into<Widget>, text: &str, tag: &str) {
    let mut runtime = Runtime::new(scene(widget)).unwrap();
    frame(&mut runtime);
    let (id, node) = node_by_label(&runtime, text);
    let expected = [StringAttribute::locale(0..text.encode_utf16().count(), tag)];
    assert_eq!(node.state.text_attributes.as_ref().unwrap().label, expected);
    let mut projection = MobileAccessibilityProjection::new();
    let update = projection.sync(runtime.tree().semantics()).unwrap();
    let native = projection.native_node_id(id).unwrap();
    let published = update
        .nodes
        .iter()
        .find(|node| node.native_id == native)
        .unwrap();
    assert_eq!(published.semantic.label.as_deref(), Some(text));
    assert_eq!(
        published
            .semantic
            .state
            .text_attributes
            .as_ref()
            .unwrap()
            .label,
        expected
    );
    let mut desktop = AccessKitProjection::new();
    let update = desktop
        .sync(runtime.tree().semantics(), 1.)
        .unwrap()
        .into_accesskit();
    let native = desktop.native_node_id(id).unwrap();
    let (_, published) = update.nodes.iter().find(|(id, _)| id.0 == native).unwrap();
    assert_eq!(published.value(), Some(text));
    assert_eq!(published.language(), Some(tag));
}

#[test]
fn explicit_text_locale_is_published_as_the_label_language_tag() {
    let text = "Flutter 2050";
    assert_text_locale(Text::new(text).locale(locale("de-DE")), text, "de-DE");
}

#[test]
fn selection_region_preserves_the_text_label_language_tag() {
    let text = "Flutter 2050";
    assert_text_locale(
        SelectableRegion::new(Text::new(text).locale(locale("de-DE"))),
        text,
        "de-DE",
    );
}

#[test]
fn text_semantics_identifier_is_on_the_labeled_node() {
    let mut runtime = Runtime::new(scene(
        Text::new("Hello World").semantics_identifier("my-text-identifier"),
    ))
    .unwrap();
    frame(&mut runtime);
    let (id, node) = node_by_label(&runtime, "Hello World");
    assert_eq!(node.state.identifier.as_deref(), Some("my-text-identifier"));
    let mut projection = MobileAccessibilityProjection::new();
    let update = projection.sync(runtime.tree().semantics()).unwrap();
    let native = projection.native_node_id(id).unwrap();
    let published = update
        .nodes
        .iter()
        .find(|node| node.native_id == native)
        .unwrap();
    assert_eq!(
        published.semantic.state.identifier.as_deref(),
        Some("my-text-identifier")
    );
    let mut desktop = AccessKitProjection::new();
    let update = desktop
        .sync(runtime.tree().semantics(), 1.)
        .unwrap()
        .into_accesskit();
    let native = desktop.native_node_id(id).unwrap();
    let (_, published) = update.nodes.iter().find(|(id, _)| id.0 == native).unwrap();
    assert_eq!(published.author_id(), Some("my-text-identifier"));
}

#[test]
fn route_scope_keeps_its_identified_text_child_separate() {
    let mut runtime = Runtime::new(scene(
        Semantics::new(Text::new("Dialog Text").semantics_identifier("dialog-text-identifier"))
            .scopes_route(true)
            .explicit_child_nodes(true),
    ))
    .unwrap();
    frame(&mut runtime);
    let (text_id, text) = node_by_label(&runtime, "Dialog Text");
    let tree = runtime.tree().semantics();
    let (route_id, route) = tree
        .iter()
        .find(|(_, node)| {
            node.state
                .annotation_flags
                .get(SemanticAnnotationFlag::ScopesRoute)
                == Some(true)
        })
        .unwrap();
    assert_eq!(
        text.state.identifier.as_deref(),
        Some("dialog-text-identifier")
    );
    assert_ne!(route.state.identifier, text.state.identifier);
    assert_ne!(route_id, text_id);
    assert_eq!(route.children, [text_id]);
}

#[test]
fn locale_range_uses_utf16_and_metadata_can_be_replaced_and_removed() {
    let mut runtime = Runtime::new(scene(
        Text::new("😀A")
            .locale(locale("de-DE"))
            .semantics_identifier("old"),
    ))
    .unwrap();
    frame(&mut runtime);
    let (id, initial) = node_by_label(&runtime, "😀A");
    assert_eq!(
        initial.state.text_attributes.as_ref().unwrap().label,
        [StringAttribute::locale(0..3, "de-DE")]
    );
    let mut projection = MobileAccessibilityProjection::new();
    projection.sync(runtime.tree().semantics()).unwrap();
    let native = projection.native_node_id(id).unwrap();

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(
            root,
            scene(
                Text::new("😀A")
                    .locale(locale("ja-JP"))
                    .semantics_identifier("new"),
            ),
        )
        .unwrap();
    frame(&mut runtime);
    let update = projection.sync(runtime.tree().semantics()).unwrap();
    let node = &update
        .nodes
        .iter()
        .find(|node| node.native_id == native)
        .unwrap()
        .semantic;
    assert_eq!(node.state.identifier.as_deref(), Some("new"));
    assert_eq!(
        node.state.text_attributes.as_ref().unwrap().label,
        [StringAttribute::locale(0..3, "ja-JP")]
    );

    runtime
        .tree_mut()
        .update(root, scene(Text::new("😀A")))
        .unwrap();
    frame(&mut runtime);
    let (current, plain) = node_by_label(&runtime, "😀A");
    assert_eq!(current, id);
    assert!(plain.state.identifier.is_none());
    assert!(plain.state.text_attributes.is_none());
}

#[test]
fn desktop_does_not_promote_a_partial_range_to_the_whole_node() {
    use incular_semantics::AttributedString;

    let text = Semantics::new(Text::new("😀A")).attributed_label(
        AttributedString::try_with_attributes("😀A", [StringAttribute::locale(0..2, "ja-JP")])
            .unwrap(),
    );
    let mut runtime = Runtime::new(scene(text)).unwrap();
    frame(&mut runtime);
    let (id, _) = node_by_label(&runtime, "😀A");
    let mut desktop = AccessKitProjection::new();
    let update = desktop
        .sync(runtime.tree().semantics(), 1.)
        .unwrap()
        .into_accesskit();
    let native = desktop.native_node_id(id).unwrap();
    let (_, published) = update.nodes.iter().find(|(id, _)| id.0 == native).unwrap();
    assert_eq!(published.language(), None);
}
