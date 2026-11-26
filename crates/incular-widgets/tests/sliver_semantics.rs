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

enum Pair {
    Main(incular_widgets::SliverMainAxisGroup),
    List(SliverList),
}

impl Sliver for Pair {
    fn build(&self, controller: &incular_widgets::ScrollController) -> Widget {
        match self {
            Self::Main(group) => group.build(controller),
            Self::List(list) => list.build(controller),
        }
    }

    fn create_render_sliver(
        &self,
        controller: &incular_widgets::ScrollController,
        axis: incular_config::Axis,
        reverse: bool,
    ) -> Box<dyn incular_widgets::internal::RenderSliver> {
        match self {
            Self::Main(group) => group.create_render_sliver(controller, axis, reverse),
            Self::List(list) => list.create_render_sliver(controller, axis, reverse),
        }
    }
}

fn annotate_pair(
    box_annotations: bool,
    first: fn(Semantics) -> Semantics,
    second: fn(Semantics) -> Semantics,
) -> WidgetTree {
    let properties = [first, second];
    let children = if box_annotations {
        Pair::List(SliverList::builder(2, move |i| {
            properties[i](Semantics::new(SizedBox::new().height(10.)))
        }))
    } else {
        let slivers = properties.map(|props| -> Box<dyn Sliver> {
            Box::new(SliverSemantics::new(
                move || props(Semantics::default()),
                short_box(),
            ))
        });
        Pair::Main(incular_widgets::SliverMainAxisGroup::new(slivers))
    };
    let sliver = SliverSemantics::new(|| Semantics::default().container(true), children);
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, sliver))
        .unwrap();
    layout(&mut tree);
    tree
}

#[test]
fn distinct_current_values_remain_siblings_for_sliver_and_box_annotations() {
    for box_annotations in [false, true] {
        let tree = annotate_pair(
            box_annotations,
            |s| s.value("value one"),
            |s| s.value("value two"),
        );
        let nodes = tree
            .semantics()
            .iter()
            // The scroll ancestor's exposed position is not a child value.
            .filter(|(_, node)| node.role != SemanticRole::ScrollView)
            .filter_map(|(_, node)| node.value.as_deref().map(|value| (value, node)))
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 2);
        assert!(
            nodes
                .iter()
                .all(|(_, node)| node.state.text_direction == Some(TextDirection::Ltr))
        );
        let group = tree.semantics().iter().find_map(|(_, node)| {
            let values = node
                .children
                .iter()
                .filter_map(|&id| tree.semantics().node(id)?.value.as_deref())
                .collect::<Vec<_>>();
            (values.len() == 2).then_some(values)
        });
        assert_eq!(
            group,
            Some(vec!["value one", "value two"]),
            "box={box_annotations}"
        );
    }
}

#[test]
fn compatible_hint_and_value_merge_for_sliver_and_box_annotations() {
    for box_annotations in [false, true] {
        let tree = annotate_pair(box_annotations, |s| s.hint("hint"), |s| s.value("value"));
        let annotated = tree
            .semantics()
            .iter()
            .filter(|(_, node)| {
                node.role != SemanticRole::ScrollView
                    && (node.value.is_some() || node.description.is_some())
            })
            .map(|(_, node)| {
                (
                    node.value.as_deref(),
                    node.description.as_deref(),
                    node.state.text_direction,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            annotated,
            [(Some("value"), Some("hint"), Some(TextDirection::Ltr))],
            "box={box_annotations}"
        );
    }
}

#[test]
fn headings_on_slivers_annotate_their_text_nodes_without_turning_siblings_into_headings() {
    let mut slivers = (1u8..=6)
        .map(|level| -> Box<dyn Sliver> {
            Box::new(SliverSemantics::new(
                move || Semantics::default().heading_level(level),
                SliverToBoxAdapter::new(row(&format!("Heading level {level}"))),
            ))
        })
        .collect::<Vec<_>>();
    slivers.push(Box::new(SliverToBoxAdapter::new(row(
        "This is not a heading",
    ))));
    let scene = semantic_view(
        TextDirection::Ltr,
        incular_widgets::SliverMainAxisGroup::new(slivers),
    );
    let mut tree = WidgetTree::new();
    tree.mount(scene).unwrap();
    layout(&mut tree);
    for level in 1u8..=6 {
        let node = find(&tree, &format!("Heading level {level}"));
        assert_eq!(node.state.heading_level, Some(level));
        assert_eq!(node.role, SemanticRole::Heading);
    }
    let normal = find(&tree, "This is not a heading");
    assert_eq!(normal.state.heading_level, None);
    assert_ne!(normal.role, SemanticRole::Heading);
}

#[cfg(debug_assertions)]
#[test]
fn heading_annotation_rejects_values_outside_one_through_six() {
    for invalid in [-1, 0, 7, 8, 9] {
        assert!(std::panic::catch_unwind(|| Semantics::default().heading_level(invalid)).is_err());
    }
}

#[test]
fn a_sliver_heading_collects_plain_text_without_absorbing_an_independent_action() {
    use incular_semantics::SemanticActionKind;

    let children: [Box<dyn Sliver>; 2] = [
        Box::new(SliverToBoxAdapter::new(row("Category"))),
        Box::new(SliverToBoxAdapter::new(
            Semantics::new(SizedBox::new().height(10.))
                .label("Reveal items")
                .button(true)
                .on_tap(|| {}),
        )),
    ];
    let heading = SliverSemantics::new(
        || Semantics::default().heading_level(2),
        incular_widgets::SliverMainAxisGroup::new(children),
    );
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, heading))
        .unwrap();
    layout(&mut tree);
    let title = find(&tree, "Category");
    assert_eq!(title.state.heading_level, Some(2));
    assert!(!title.actions.contains(&SemanticActionKind::Activate));
    let action = find(&tree, "Reveal items");
    assert_eq!(action.role, SemanticRole::Button);
    assert!(action.actions.contains(&SemanticActionKind::Activate));
    assert_eq!(title.children, [action.id]);
}

fn labeled_scroll_children(tree: &WidgetTree, parent: &SemanticNode) -> Vec<Option<String>> {
    parent
        .children
        .iter()
        .map(|id| tree.semantics().node(*id).unwrap().label.clone())
        .collect()
}

fn scroll_node(tree: &WidgetTree) -> &SemanticNode {
    tree.semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.role == SemanticRole::ScrollView)
        .unwrap()
}

fn keyed_anchor(index: u8, order: f64, named: bool) -> Box<dyn Sliver> {
    use incular_semantics::OrdinalSortKey;
    Box::new(SliverSemantics::new(
        move || {
            Semantics::default().sort_key(if named {
                OrdinalSortKey::named(order, "CustomSortKey")
            } else {
                OrdinalSortKey::new(order)
            })
        },
        SliverToBoxAdapter::new(row(&format!("Label {index}"))),
    ))
}

fn assert_sorted_viewport_graph(tree: &WidgetTree, expected_nodes: usize) {
    let semantics = tree.semantics();
    assert_eq!(semantics.roots().len(), 1);
    assert_eq!(semantics.iter().count(), expected_nodes);
    let boundary = semantics.node(semantics.root().unwrap()).unwrap();
    assert_eq!(boundary.children.len(), 1);
    let pane = scroll_node(tree);
    assert_eq!(boundary.children, [pane.id]);
    assert!(!boundary.state.has_implicit_scrolling && pane.state.has_implicit_scrolling);
    assert!(boundary.tags().is_empty() && pane.tags().is_empty());
    for (_, node) in semantics.iter() {
        assert!(node.actions.is_empty());
        assert!(node.value.is_none() && node.description.is_none());
        assert!(!node.state.hidden && !node.state.focusable && !node.state.has_enabled_state);
        if node.label.is_some() {
            assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
            assert!(node.children.is_empty());
        }
    }
}

#[test]
fn sort_keys_reorder_labeled_slivers_within_a_cross_axis_group() {
    use incular_widgets::SliverCrossAxisGroup;
    let group = SliverCrossAxisGroup::new([
        keyed_anchor(1, 3., true),
        keyed_anchor(2, 1., true),
        keyed_anchor(3, 2., true),
    ]);
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, group))
        .unwrap();
    layout(&mut tree);
    assert_sorted_viewport_graph(&tree, 5);
    assert_eq!(
        labeled_scroll_children(&tree, scroll_node(&tree)),
        [
            Some("Label 2".into()),
            Some("Label 3".into()),
            Some("Label 1".into())
        ]
    );
}

#[test]
fn nested_sort_keys_reorder_both_groups_without_flattening_explicit_child_nodes() {
    use incular_semantics::OrdinalSortKey;
    use incular_widgets::{SliverCrossAxisGroup, SliverMainAxisGroup};
    let cross = SliverCrossAxisGroup::new([
        keyed_anchor(3, 3., false),
        keyed_anchor(4, 2., false),
        keyed_anchor(5, 1., false),
    ]);
    let inner: Box<dyn Sliver> = Box::new(SliverSemantics::new(
        || {
            Semantics::default()
                .sort_key(OrdinalSortKey::named(1., "CustomSortKey"))
                .explicit_child_nodes(true)
        },
        cross,
    ));
    let group =
        SliverMainAxisGroup::new([keyed_anchor(1, 3., true), keyed_anchor(2, 2., true), inner]);
    let outer = SliverSemantics::new(
        || {
            Semantics::default()
                .sort_key(OrdinalSortKey::named(0., "CustomSortKey"))
                .explicit_child_nodes(true)
        },
        group,
    );
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, outer))
        .unwrap();
    layout(&mut tree);
    assert_sorted_viewport_graph(&tree, 9);
    let root = scroll_node(&tree);
    assert_eq!(root.children.len(), 1);
    let group = tree.semantics().node(root.children[0]).unwrap();
    assert!(group.is_tagged(incular_widgets::Viewport::use_two_pane_semantics_tag()));
    assert_eq!(
        labeled_scroll_children(&tree, group),
        [None, Some("Label 2".into()), Some("Label 1".into())]
    );
    let nested = tree.semantics().node(group.children[0]).unwrap();
    assert_eq!(
        labeled_scroll_children(&tree, nested),
        [
            Some("Label 5".into()),
            Some("Label 4".into()),
            Some("Label 3".into())
        ]
    );
}

#[test]
fn nested_slivers_without_keys_keep_their_spatial_reading_order() {
    use incular_widgets::{SliverCrossAxisGroup, SliverMainAxisGroup};
    let plain = |label| -> Box<dyn Sliver> { Box::new(SliverToBoxAdapter::new(row(label))) };
    let cross: Box<dyn Sliver> = Box::new(SliverCrossAxisGroup::new([
        plain("Label 3"),
        plain("Label 4"),
        plain("Label 5"),
    ]));
    let main = SliverMainAxisGroup::new([plain("Label 1"), plain("Label 2"), cross]);
    let mut tree = WidgetTree::new();
    tree.mount(semantic_view(TextDirection::Ltr, main)).unwrap();
    layout(&mut tree);
    assert_eq!(
        labeled_scroll_children(&tree, scroll_node(&tree)),
        (1..=5)
            .map(|i| Some(format!("Label {i}")))
            .collect::<Vec<_>>()
    );
}

#[test]
fn changing_sliver_sort_keys_reorders_the_same_labeled_nodes() {
    use incular_widgets::SliverCrossAxisGroup;

    let scene = |orders: [f64; 3]| {
        semantic_view(
            TextDirection::Ltr,
            SliverCrossAxisGroup::new(
                orders
                    .into_iter()
                    .enumerate()
                    .map(|(i, order)| keyed_anchor(i as u8 + 1, order, true)),
            ),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene([3., 1., 2.])).unwrap();
    layout(&mut tree);
    let by_label =
        |tree: &WidgetTree| ["Label 1", "Label 2", "Label 3"].map(|label| find(tree, label).id);
    let original = by_label(&tree);
    assert_eq!(
        scroll_node(&tree).children,
        [original[1], original[2], original[0]]
    );
    tree.update(root, scene([1., 3., 2.])).unwrap();
    layout(&mut tree);
    assert_eq!(by_label(&tree), original);
    assert_eq!(
        scroll_node(&tree).children,
        [original[0], original[2], original[1]]
    );
}

#[test]
fn semantic_validation_merges_all_nested_sliver_pairs_without_losing_the_label() {
    use incular_semantics::SemanticValidationResult::{Invalid, None, Valid};
    for (outer, inner, expected) in [
        (None, None, None),
        (None, Valid, Valid),
        (None, Invalid, Invalid),
        (Valid, None, Valid),
        (Valid, Valid, Valid),
        (Valid, Invalid, Invalid),
        (Invalid, None, Invalid),
        (Invalid, Valid, Invalid),
        (Invalid, Invalid, Invalid),
    ] {
        let inner = SliverSemantics::new(
            move || Semantics::default().validation_result(inner),
            SliverToBoxAdapter::new(row("validation")),
        );
        let outer =
            SliverSemantics::new(move || Semantics::default().validation_result(outer), inner);
        let mut tree = WidgetTree::new();
        tree.mount(semantic_view(TextDirection::Ltr, outer))
            .unwrap();
        layout(&mut tree);
        let node = find(&tree, "validation");
        assert_eq!(node.state.effective_validation_result(), expected);
        assert_eq!(node.state.is_invalid(), expected == Invalid);
        assert_eq!(scroll_node(&tree).children, [node.id]);
        assert!(node.children.is_empty());
    }
}

#[test]
fn updating_a_sliver_validation_result_keeps_the_same_explicit_annotation_node() {
    use incular_semantics::SemanticValidationResult::{Invalid, None, Valid};
    let scene = |result| {
        semantic_view(
            TextDirection::Ltr,
            SliverSemantics::new(
                move || {
                    Semantics::default()
                        .label("submit")
                        .validation_result(result)
                },
                short_box(),
            ),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(None)).unwrap();
    layout(&mut tree);
    let id = find(&tree, "submit").id;
    let mut revision = tree.semantics().revision();
    for state in [Valid, Invalid, None] {
        tree.update(root, scene(state)).unwrap();
        layout(&mut tree);
        let node = find(&tree, "submit");
        assert_eq!(node.id, id);
        assert_eq!(node.state.effective_validation_result(), state);
        assert!(tree.semantics().revision() > revision);
        revision = tree.semantics().revision();
    }
}

#[test]
fn hint_and_value_merges_match_the_complete_two_pane_graph() {
    use incular_widgets::Viewport;
    type Annotation = fn(Semantics) -> Semantics;

    let variants: [(Annotation, Annotation); 3] = [
        (|s| s.hint("hint one"), |s| s.hint("hint two")),
        (|s| s.value("value one"), |s| s.value("value two")),
        (|s| s.hint("hint"), |s| s.value("value")),
    ];
    let tag = Viewport::use_two_pane_semantics_tag();
    for box_annotations in [false, true] {
        for (variant, (first, second)) in variants.into_iter().enumerate() {
            let widgets = annotate_pair(box_annotations, first, second);
            let tree = widgets.semantics();
            assert_eq!(tree.roots().len(), 1);
            let outer = tree.node(tree.root().unwrap()).unwrap();
            assert!(!outer.state.has_implicit_scrolling);
            assert_eq!(outer.children.len(), 1);
            let pane = tree.node(outer.children[0]).unwrap();
            assert!(pane.state.has_implicit_scrolling);
            assert_eq!(pane.children.len(), 1);
            let annotation = tree.node(pane.children[0]).unwrap();
            assert_eq!(annotation.tags(), &[tag.clone()]);
            assert!(outer.tags().is_empty() && pane.tags().is_empty());
            assert!(outer.value.is_none() && pane.value.is_none());
            assert!(outer.description.is_none() && pane.description.is_none());
            for (_, node) in tree.iter() {
                assert!(node.label.is_none());
                assert!(node.actions.is_empty());
                assert!(node.state.tooltip.is_none());
                assert!(
                    !node.state.hidden && !node.state.focusable && !node.state.has_enabled_state
                );
            }
            match variant {
                0 => {
                    assert_eq!(
                        annotation.description.as_deref(),
                        Some("hint one\nhint two")
                    );
                    assert!(annotation.value.is_none());
                    assert_eq!(annotation.state.text_direction, Some(TextDirection::Ltr));
                    assert!(annotation.children.is_empty());
                    assert_eq!(tree.iter().count(), 3);
                }
                1 => {
                    assert!(annotation.value.is_none() && annotation.description.is_none());
                    assert_eq!(annotation.children.len(), 2);
                    for (&id, value) in annotation.children.iter().zip(["value one", "value two"]) {
                        let child = tree.node(id).unwrap();
                        assert_eq!(child.value.as_deref(), Some(value));
                        assert_eq!(child.state.text_direction, Some(TextDirection::Ltr));
                        assert!(child.children.is_empty() && child.tags().is_empty());
                        assert!(child.description.is_none());
                    }
                    assert_eq!(tree.iter().count(), 5);
                }
                _ => {
                    assert_eq!(annotation.value.as_deref(), Some("value"));
                    assert_eq!(annotation.description.as_deref(), Some("hint"));
                    assert_eq!(annotation.state.text_direction, Some(TextDirection::Ltr));
                    assert!(annotation.children.is_empty());
                    assert_eq!(tree.iter().count(), 3);
                }
            }
        }
    }
}
