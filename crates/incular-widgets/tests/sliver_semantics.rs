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

fn semantic_view(direction: TextDirection, sliver: impl Sliver + 'static) -> Widget {
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(sliver)];
    Directionality::new(direction, CustomScrollView::new(slivers)).into()
}

fn short_box() -> SliverToBoxAdapter {
    SliverToBoxAdapter::new(SizedBox::new().height(10.))
}

#[test]
fn nested_sliver_annotations_reparent_after_inserting_and_removing_a_boundary() {
    let scene = |insert: bool| {
        let inner = SliverSemantics::new(
            move || {
                Semantics::default().container(true).label(if insert {
                    "inner B"
                } else {
                    "inner A"
                })
            },
            short_box(),
        );
        let nested = if insert {
            SliverSemantics::new(
                || Semantics::default().container(true).label("middle"),
                inner,
            )
        } else {
            inner
        };
        let outer = SliverSemantics::new(|| Semantics::default().label("outer"), nested);
        semantic_view(TextDirection::Ltr, outer)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    layout(&mut tree);
    let outer = find(&tree, "outer").id;
    assert_eq!(find(&tree, "outer").children, [find(&tree, "inner A").id]);
    tree.update(root, scene(true)).unwrap();
    layout(&mut tree);
    let middle = find(&tree, "middle");
    assert_eq!(find(&tree, "outer").id, outer);
    assert_eq!(find(&tree, "outer").children, [middle.id]);
    assert_eq!(middle.children, [find(&tree, "inner B").id]);
    assert!(
        !tree
            .semantics()
            .iter()
            .any(|(_, n)| n.label.as_deref() == Some("inner A"))
    );

    tree.update(root, scene(false)).unwrap();
    layout(&mut tree);
    assert_eq!(find(&tree, "outer").children, [find(&tree, "inner A").id]);
    for missing in ["middle", "inner B"] {
        assert!(
            !tree
                .semantics()
                .iter()
                .any(|(_, n)| n.label.as_deref() == Some(missing))
        );
    }
}

#[test]
fn sliver_label_inherits_direction_unless_an_explicit_direction_is_supplied() {
    for (ambient, explicit) in [
        (TextDirection::Ltr, None),
        (TextDirection::Rtl, None),
        (TextDirection::Ltr, Some(TextDirection::Rtl)),
        (TextDirection::Rtl, Some(TextDirection::Ltr)),
    ] {
        let mut tree = WidgetTree::new();
        let sliver = SliverSemantics::new(
            move || {
                let semantics = Semantics::default().label("test1");
                if let Some(direction) = explicit {
                    semantics.text_direction(direction)
                } else {
                    semantics
                }
            },
            short_box(),
        );
        tree.mount(semantic_view(ambient, sliver)).unwrap();
        layout(&mut tree);
        assert_eq!(
            find(&tree, "test1").state.text_direction,
            Some(explicit.unwrap_or(ambient)),
            "ambient={ambient:?} explicit={explicit:?}"
        );
    }
}

#[test]
fn sliver_label_hint_and_value_are_reported_on_the_same_node() {
    let sliver = SliverSemantics::new(
        || {
            Semantics::default()
                .label("label")
                .hint("hint")
                .value("value")
        },
        short_box(),
    );
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, sliver))
        .unwrap();
    layout(&mut tree);
    let node = find(&tree, "label");
    assert_eq!(node.description.as_deref(), Some("hint"));
    assert_eq!(node.value.as_deref(), Some("value"));
    assert!(node.children.is_empty());
}

#[test]
fn adjacent_hints_merge_under_a_sliver_boundary_with_or_without_box_annotation() {
    use incular_widgets::SliverMainAxisGroup;
    for box_annotation in [false, true] {
        let first: Box<dyn Sliver> = if box_annotation {
            Box::new(SliverToBoxAdapter::new(
                Semantics::new(SizedBox::new().height(10.)).hint("hint one"),
            ))
        } else {
            Box::new(SliverSemantics::new(
                || Semantics::default().hint("hint one"),
                short_box(),
            ))
        };
        let second: Box<dyn Sliver> = Box::new(SliverSemantics::new(
            || Semantics::default().hint("hint two"),
            short_box(),
        ));
        let sliver = SliverSemantics::new(
            || Semantics::default().container(true),
            SliverMainAxisGroup::new([first, second]),
        );
        let mut tree = WidgetTree::new();
        tree.mount(semantic_view(TextDirection::Ltr, sliver))
            .unwrap();
        layout(&mut tree);
        let descriptions = tree
            .semantics()
            .iter()
            .filter_map(|(_, node)| node.description.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(
            descriptions,
            ["hint one\nhint two"],
            "box annotation={box_annotation}"
        );
    }
}
