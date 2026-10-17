#![cfg(feature = "devtools")]

//! Bounded read-only DevTools inspection of semantics, layers and counters.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_semantics::SemanticRole;
use incular_widgets::{
    Column, Opacity, Text, Widget,
    internal::{ExplicitSemantics, WidgetTree},
};

fn content(text: &str) -> Widget {
    Column::new([
        Widget::from(Opacity::new(
            0.5,
            Widget::box_(Size::new(20., 20.), Color::WHITE),
        ))
        .semantics(ExplicitSemantics::new(SemanticRole::Group).label("fader")),
        Widget::from(Text::new(text).text_direction(incular_widgets::TextDirection::Ltr)),
    ])
    .into()
}

fn mounted_tree() -> WidgetTree {
    let mut tree = WidgetTree::new();
    let root = tree.mount(content("hello")).expect("mount");
    // Rebuilding the root with changed content bumps per-element build counts.
    tree.update(root, content("changed")).expect("update");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    let _ = tree.paint();
    tree.update_semantics();
    tree
}

#[test]
fn semantics_snapshot_lists_labelled_nodes_with_parents() {
    let tree = mounted_tree();
    let (nodes, truncated) = tree.devtools_semantics_nodes();
    assert!(!truncated);
    let fader = nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("fader"))
        .expect("labelled node is listed");
    assert_eq!(fader.role, "Group");
    // Every parent and child reference names a listed node.
    for node in &nodes {
        if let Some(parent) = node.parent {
            assert!(nodes.iter().any(|other| other.id == parent));
        }
    }
    assert!(nodes.iter().any(|node| node.parent.is_none()));
}

#[test]
fn layer_snapshot_is_a_connected_tree_with_opacity() {
    let tree = mounted_tree();
    let (layers, truncated) = tree.devtools_layer_nodes();
    assert!(!truncated);
    assert!(!layers.is_empty());
    assert!(layers.iter().any(|layer| layer.kind == "Opacity"));
    assert!(layers[0].parent.is_none());
    for layer in layers.iter().skip(1) {
        let parent = layer.parent.expect("non-root layers have a parent");
        let parent = layers
            .iter()
            .find(|candidate| candidate.id == parent)
            .expect("parent is listed");
        assert!(parent.children.contains(&layer.id));
    }
}

#[test]
fn work_counts_are_ranked_and_limited() {
    let tree = mounted_tree();
    let all = tree.devtools_work_counts(1000);
    assert!(all.len() > 3);
    assert!(all.len() <= 200);
    assert!(all.windows(2).all(|pair| pair[0].builds >= pair[1].builds));
    assert!(all.iter().any(|entry| entry.builds > 0));
    assert_eq!(tree.devtools_work_counts(2).len(), 2);
    assert!(tree.devtools_work_counts(0).is_empty());
}

#[test]
fn runtime_counters_report_tree_totals() {
    let tree = mounted_tree();
    let counters = tree.devtools_runtime_counters();
    assert_eq!(counters.widgets.elements_total, tree.element_count() as u64);
    assert!(counters.widgets.layouts > 0);
    assert!(counters.widgets.paints > 0);
    assert!(counters.semantics.nodes > 0);
    assert!(counters.compositor.layers > 0);
    assert!(counters.text.layouts_requested > 0);
}
