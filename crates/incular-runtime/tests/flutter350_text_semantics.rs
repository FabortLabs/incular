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

#[test]
fn semantic_label_overrides_spoken_text_with_explicit_or_inherited_direction() {
    let make = || Text::new("$$").semantics_label("Double dollars");
    for fixture in [
        make().text_direction(TextDirection::Ltr).into(),
        scene(make()),
    ] {
        let mut runtime = Runtime::new(fixture).unwrap();
        frame(&mut runtime);
        let tree = runtime.tree().semantics();
        let [id] = tree.roots() else {
            panic!("expected one semantic root: {:?}", tree.roots());
        };
        let annotation = tree.node(*id).unwrap();
        assert_eq!(annotation.label.as_deref(), Some("Double dollars"));
        assert_eq!(annotation.state.text_direction, Some(TextDirection::Ltr));
        assert!(annotation.children.is_empty());
        assert_eq!(
            tree.len(),
            1,
            "the visible source must not publish a second label"
        );
    }
}

#[test]
fn spoken_label_can_be_replaced_without_reusing_visual_text_locale_or_glyphs() {
    use incular_rendering::PaintCommand;

    let make = || Text::new("$$").locale(locale("de-DE"));
    let mut runtime = Runtime::new(scene(make())).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let (before, _) = runtime.run_frame(constraints).unwrap();
    let (id, source) = node_by_label(&runtime, "$$");
    assert_eq!(
        source.state.text_attributes.as_ref().unwrap().label,
        [StringAttribute::locale(0..2, "de-DE")]
    );

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(root, scene(make().semantics_label("Double dollars")))
        .unwrap();
    let (after, _) = runtime.run_frame(constraints).unwrap();
    let (updated_id, label) = node_by_label(&runtime, "Double dollars");
    assert_eq!(updated_id, id);
    assert!(label.state.text_attributes.is_none());
    let glyphs = |list: &incular_rendering::DisplayList| {
        list.commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::GlyphRun { run, .. } => Some(run.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let before = glyphs(&before);
    assert!(!before.is_empty());
    assert_eq!(glyphs(&after), before);

    // The lower-level label API likewise replaces the implicit label.
    let widget = Widget::from(make()).accessibility_label("Dollars");
    runtime.tree_mut().update(root, scene(widget)).unwrap();
    frame(&mut runtime);
    assert!(
        node_by_label(&runtime, "Dollars")
            .1
            .state
            .text_attributes
            .is_none()
    );
}

#[test]
fn semantic_hint_and_implicit_text_locale_keep_separate_attributes() {
    use incular_semantics::AttributedString;
    let mut runtime = Runtime::new(scene(
        Semantics::new(Text::new("😀A").locale(locale("ja-JP"))).attributed_hint(
            AttributedString::try_with_attributes("Info", [StringAttribute::spell_out(0..4)])
                .unwrap(),
        ),
    ))
    .unwrap();
    frame(&mut runtime);
    let (_, text) = node_by_label(&runtime, "😀A");
    let attributes = text.state.text_attributes.as_ref().unwrap();
    assert_eq!(attributes.label, [StringAttribute::locale(0..3, "ja-JP")]);
    assert_eq!(text.description.as_deref(), Some("Info"));
    assert_eq!(attributes.hint, [StringAttribute::spell_out(0..4)]);
}
