use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_semantics::{SemanticNode, SemanticNodeId, StringAttribute};
use incular_text::{RichText, TextSpan};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Directionality, RichParagraph, Text, Widget, WidgetSpan};

fn scene(content: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, content).into()
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_semantics();
}

fn parts(tree: &WidgetTree) -> (SemanticNodeId, Vec<&SemanticNode>) {
    let graph = tree.semantics();
    let [parent] = graph.roots() else {
        panic!("expected one paragraph semantics boundary");
    };
    let node = graph.node(*parent).unwrap();
    assert!(node.label.is_none());
    assert!(node.state.explicit_child_nodes);
    (
        *parent,
        node.children
            .iter()
            .map(|&id| graph.node(id).unwrap())
            .collect(),
    )
}

#[test]
fn an_identifier_creates_one_functional_child_between_combined_spoken_ranges() {
    let span = TextSpan::new("Hello, ")
        .child(TextSpan::new("1 new "))
        .child(TextSpan::new("semantics node ").semantics_identifier("new_semantics_node"))
        .child(TextSpan::new("has been "))
        .child(TextSpan::new("created."));
    assert_eq!(
        span.plain_text(),
        "Hello, 1 new semantics node has been created."
    );
    let mut tree = WidgetTree::new();
    tree.mount(scene(RichText::new(span))).unwrap();
    layout(&mut tree);
    let (_, ranges) = parts(&tree);
    assert_eq!(
        ranges
            .iter()
            .map(|node| (node.label.as_deref(), node.state.identifier.as_deref()))
            .collect::<Vec<_>>(),
        [
            (Some("Hello, 1 new "), None),
            (Some("semantics node "), Some("new_semantics_node")),
            (Some("has been created."), None),
        ]
    );
    assert!(
        ranges
            .windows(2)
            .all(|pair| pair[0].bounds.origin.x < pair[1].bounds.origin.x)
    );
}

#[test]
fn labeled_fragments_keep_ids_and_pronunciation_when_real_inline_children_are_interleaved() {
    let paragraph = |last: &str| {
        let span = TextSpan::new("ab")
            .locale("es-MX".parse().unwrap())
            .child(WidgetSpan::sized(7, Size::new(15., 15.)))
            .child(
                TextSpan::new(last)
                    .semantics_label("🦀")
                    .semantics_identifier("active"),
            );
        scene(RichParagraph::new(RichText::new(span)).child(7, Text::new("inline widget")))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(paragraph("cd")).unwrap();
    layout(&mut tree);
    let (parent, children) = parts(&tree);
    assert_eq!(
        children
            .iter()
            .map(|n| n.label.as_deref())
            .collect::<Vec<_>>(),
        [Some("ab"), Some("inline widget"), Some("🦀")]
    );
    let [a, _, b] = children.as_slice() else {
        unreachable!()
    };
    for child in [a, b] {
        assert_eq!(
            child.state.text_attributes.as_ref().unwrap().label,
            [StringAttribute::locale(0..2, "es-MX")]
        );
    }
    let ids: Vec<_> = children.iter().map(|node| node.id).collect();
    tree.update(root, paragraph("ef")).unwrap();
    layout(&mut tree);
    let (after, children) = parts(&tree);
    assert_eq!(after, parent);
    assert_eq!(children.iter().map(|node| node.id).collect::<Vec<_>>(), ids);
}

#[test]
fn displayed_bytes_not_spoken_label_length_locate_fragments_and_ellipsis_hides_children() {
    use incular_text::{TextOverflow, TextStyle};
    use incular_widgets::{GlobalKey, Semantics, SizedBox};

    const FAMILY: &str = "Pinned rich fragment Ahem";
    let Some(font_path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    incular_text::register_application_font_as(FAMILY, std::fs::read(font_path).unwrap());
    let style = TextStyle::new().font_family(FAMILY).font_size(10.);
    let key = GlobalKey::new();
    let paragraph = |alias: &str| {
        let span = TextSpan::new("AB")
            .style(style.clone())
            .semantics_label(alias)
            .spell_out(true)
            .child(WidgetSpan::new(1))
            .child(TextSpan::new("CD").semantics_identifier("named"));
        let rich = RichParagraph::new(
            RichText::new(span)
                .max_lines(Some(1))
                .overflow(TextOverflow::Ellipsis),
        )
        .child(
            1,
            Semantics::new(SizedBox::new().width(60.).height(10.))
                .container(true)
                .label("inline widget"),
        );
        scene(Widget::from(rich).with_key(key))
    };
    let layout_at = |tree: &mut WidgetTree, width| {
        tree.layout(Constraints::tight(Size::new(width, 40.)))
            .unwrap();
        tree.update_semantics();
    };
    let named_range = |tree: &WidgetTree| {
        let (_, children) = parts(tree);
        let [literal, inline, named] = children.as_slice() else {
            panic!("expected a visible literal, placeholder and named range");
        };
        assert_eq!(inline.label.as_deref(), Some("inline widget"));
        assert_eq!(named.state.identifier.as_deref(), Some("named"));
        let attrs = literal.state.text_attributes.as_ref().unwrap();
        let units = literal.label.as_ref().unwrap().encode_utf16().count();
        assert_eq!(attrs.label, [StringAttribute::spell_out(0..units)]);
        let element = tree.element_with_key(&key.into()).unwrap();
        let laid_out = tree.text_layout(tree.render_id(element).unwrap()).unwrap();
        let [(left, right)] = laid_out.lines[0].selection_spans(5, 7)[..] else {
            panic!("named literal bytes must have one visual selection range");
        };
        assert_eq!(named.bounds.origin.x, left.floor() - 4.);
        assert_eq!(
            named.bounds.origin.x + named.bounds.size.width,
            right.ceil() + 4.
        );
        named.bounds
    };

    let mut tree = WidgetTree::new();
    let root = tree.mount(paragraph("alpha 🦀 beta")).unwrap();
    layout_at(&mut tree, 120.);
    let original_bounds = named_range(&tree);
    tree.update(root, paragraph("an even longer spoken label"))
        .unwrap();
    layout_at(&mut tree, 120.);
    assert_eq!(named_range(&tree), original_bounds);
    // This width retains the placeholder but cuts the named range exactly
    // where the synthetic ellipsis begins.
    layout_at(&mut tree, 94.);
    let (_, children) = parts(&tree);
    assert_eq!(
        children
            .iter()
            .filter_map(|n| n.label.as_deref())
            .collect::<Vec<_>>(),
        ["an even longer spoken label", "inline widget"]
    );
    layout_at(&mut tree, 36.);
    let (_, children) = parts(&tree);
    assert!(children.iter().all(|node| {
        node.state.identifier.is_none() && node.label.as_deref() != Some("inline widget")
    }));
    assert!(!tree.semantics().iter().any(|(_, node)| {
        node.state.identifier.as_deref() == Some("named")
            || node.label.as_deref() == Some("inline widget")
    }));
    layout_at(&mut tree, 120.);
    assert_eq!(named_range(&tree), original_bounds);
}

#[test]
fn switching_between_split_and_plain_semantics_releases_stale_fragment_ids() {
    let split = || {
        scene(RichText::new(
            TextSpan::new("plain ").child(TextSpan::new("marked").semantics_identifier("named")),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(split()).unwrap();
    layout(&mut tree);
    let (parent, children) = parts(&tree);
    let old_ids: Vec<_> = children.iter().map(|node| node.id).collect();

    tree.update(root, scene(RichText::new(TextSpan::new("plain marked"))))
        .unwrap();
    layout(&mut tree);
    let graph = tree.semantics();
    assert_eq!(graph.roots(), [parent]);
    assert_eq!(
        graph.node(parent).unwrap().label.as_deref(),
        Some("plain marked")
    );
    assert!(old_ids.iter().all(|&id| graph.node(id).is_none()));

    tree.update(root, split()).unwrap();
    layout(&mut tree);
    let (same_parent, children) = parts(&tree);
    assert_eq!(same_parent, parent);
    let new_ids: Vec<_> = children.iter().map(|node| node.id).collect();
    assert!(new_ids.iter().all(|id| !old_ids.contains(id)));

    tree.set_semantics_enabled(false);
    assert!(tree.semantics().is_empty());
    tree.set_semantics_enabled(true);
    layout(&mut tree);
    let (rebuilt, children) = parts(&tree);
    assert_ne!(rebuilt, parent);
    assert!(children.iter().all(|node| !new_ids.contains(&node.id)));
}
