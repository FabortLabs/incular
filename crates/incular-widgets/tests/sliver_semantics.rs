//! Flutter's `SliverSemantics` and `SliverEnsureSemantics`.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_semantics::{SemanticNode, SemanticRole};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    CustomScrollView, Directionality, Semantics, SizedBox, Sliver, SliverEnsureSemantics,
    SliverList, SliverSemantics, SliverToBoxAdapter, Text, Widget,
};

fn row(label: &str) -> Widget {
    SizedBox::from_dimensions(None, Some(40.), Some(Text::new(label))).into()
}

fn view(items: &'static [&'static str]) -> Widget {
    let slivers: Vec<Box<dyn Sliver>> = vec![
        Box::new(SliverToBoxAdapter::new(row("before"))),
        Box::new(SliverSemantics::new(
            || {
                Semantics::new(SizedBox::new())
                    .role(SemanticRole::List)
                    .label("group")
            },
            SliverList::builder(items.len(), move |index| row(items[index])),
        )),
        Box::new(SliverEnsureSemantics::new(SliverToBoxAdapter::new(row(
            "after",
        )))),
    ];
    Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
}

fn find<'a>(tree: &'a WidgetTree, label: &str) -> &'a SemanticNode {
    tree.semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no node labelled {label}"))
}

/// The labels at or below `node`, depth first.
fn labels(tree: &WidgetTree, node: &SemanticNode) -> Vec<String> {
    let mut out = Vec::new();
    for child in &node.children {
        let child = tree.semantics().node(*child).expect("child node");
        out.extend(child.label.clone());
        out.extend(labels(tree, child));
    }
    out
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(200., 400.)))
        .expect("layout");
    tree.update_semantics();
}

#[test]
fn the_sliver_node_holds_its_children_only() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(view(&["a", "b"])).expect("mount");
    layout(&mut tree);
    let group = find(&tree, "group");
    assert_eq!(group.role, SemanticRole::List);
    assert_eq!(labels(&tree, group), ["a", "b"]);
    // The node covers the sliver: below "before", two rows tall.
    assert_eq!(group.bounds.origin.y, 40.);
    assert_eq!(group.bounds.size.height, 80.);
    // The other slivers stay beside it, and "after" is kept by
    // SliverEnsureSemantics.
    let parent = tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| {
            node.children.iter().any(|child| {
                tree.semantics().node(*child).map(|c| c.label.as_deref()) == Some(Some("group"))
            })
        })
        .expect("group parent");
    let siblings: Vec<_> = parent
        .children
        .iter()
        .filter_map(|child| tree.semantics().node(*child)?.label.clone())
        .collect();
    assert_eq!(siblings, ["before", "group", "after"]);

    // A rebuilt list stays grouped.
    tree.update(root, view(&["a", "b", "c"])).expect("update");
    layout(&mut tree);
    assert_eq!(labels(&tree, find(&tree, "group")), ["a", "b", "c"]);
    let parent_children = tree
        .semantics()
        .iter()
        .filter(|(_, node)| node.label.as_deref() == Some("c"))
        .count();
    assert_eq!(parent_children, 1);
}
