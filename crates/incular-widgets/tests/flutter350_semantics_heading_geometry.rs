//! Public tree observations from Flutter 3.50.0 widgets/semantics_test.dart.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_semantics::{SemanticNode, SemanticRole};
use incular_widgets::{
    Column, ExcludeSemantics, FittedBox, MergeSemantics, Padding, Semantics, SingleChildScrollView,
    SizedBox, Widget, internal::WidgetTree,
};

fn mounted(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    tree.update_semantics();
    tree
}

fn nodes(tree: &WidgetTree) -> Vec<&SemanticNode> {
    tree.semantics().iter().map(|(_, node)| node).collect()
}

#[test]
fn tooltip_only_annotation_is_exposed_without_a_label() {
    let tree = mounted(Semantics::new(SizedBox::shrink()).tooltip("test1"));
    let matching: Vec<_> = nodes(&tree)
        .into_iter()
        .filter(|node| node.state.tooltip.as_deref() == Some("test1"))
        .collect();
    assert_eq!(matching.len(), 1);
    assert!(matching[0].label.as_deref().is_none_or(str::is_empty));
}

#[test]
fn replacing_child_semantics_exposes_the_new_label_only() {
    let inner = Semantics::new(SizedBox::new().width(40.).height(20.)).label("other label");
    let outer = |child: Widget| Semantics::new(Padding::all(1., child)).label("label");
    let active = mounted(outer(ExcludeSemantics::new(inner.clone()).into()));
    let control = mounted(outer(ExcludeSemantics::new(inner).excluding(false).into()));
    let labels = |tree: &WidgetTree| {
        let mut result: Vec<_> = nodes(tree)
            .into_iter()
            .filter_map(|node| node.label.clone())
            .filter(|s| !s.is_empty())
            .collect();
        result.sort();
        result
    };
    assert_eq!(labels(&control), ["label", "other label"]);
    assert_eq!(labels(&active), ["label"]);
}

#[test]
fn singular_fitted_box_pose_removes_scroll_subtree_from_semantics() {
    let fit = |height, mark| {
        let column = Column::new(Vec::<Widget>::new());
        let scroll = SingleChildScrollView::new(column);
        let child = match mark {
            true => Widget::from(Semantics::new(scroll).label("inside")),
            false => Widget::from(scroll),
        };
        Column::new([Widget::from(
            SizedBox::new()
                .width(500.)
                .height(height)
                .child(FittedBox::new(
                    SizedBox::new().width(266.).height(55.).child(child),
                )),
        )])
    };
    assert!(
        nodes(&mounted(fit(55., true)))
            .iter()
            .any(|node| node.label.as_deref() == Some("inside"))
    );
    let original = mounted(fit(0., false));
    let labeled = mounted(fit(0., true));
    let retained = |tree: &WidgetTree| {
        nodes(tree)
            .iter()
            .map(|node| (node.role, node.label.clone(), node.bounds))
            .collect::<Vec<_>>()
    };
    let original_nodes = retained(&original);
    let labeled_nodes = retained(&labeled);
    assert!(
        original_nodes.is_empty()
            && !labeled_nodes
                .iter()
                .any(|(_, label, _)| label.as_deref() == Some("inside")),
        "original empty scroll subtree: {original_nodes:?}; labeled scroll: {labeled_nodes:?}"
    );
}

#[test]
fn fitted_box_republishes_visible_semantics_after_a_zero_size_frame() {
    for merge in [false, true] {
        let key = incular_widgets::internal::Key::from("retained fitted box");
        let scene = |height| {
            let child =
                Semantics::new(Widget::box_(Size::new(80., 40.), Color::WHITE)).label("inside");
            let fitted = Widget::from(FittedBox::new(child)).with_key(key.clone());
            let frame = SizedBox::new().width(100.).height(height).child(fitted);
            if merge {
                MergeSemantics::new(Semantics::new(frame).label("outside")).into()
            } else {
                frame.into()
            }
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(scene(40.)).unwrap();
        let bounds = Constraints::loose(Size::new(100., 80.));
        let mut retained = None;
        for (height, visible) in [(40., true), (0., false), (40., true)] {
            tree.update(root, scene(height)).unwrap();
            tree.layout(bounds).unwrap();
            let list = tree.paint();
            assert_eq!(
                list.commands().iter().any(|op| matches!(
                    op,
                    PaintCommand::Rect {
                        color: Color::WHITE,
                        ..
                    }
                )),
                visible,
                "the empty fit must not submit its drawable child",
            );
            tree.update_semantics();
            let element = tree.element_with_key(&key).unwrap();
            let render = tree.render_id(element).unwrap();
            assert_eq!(render, *retained.get_or_insert(render));
            assert_eq!(
                tree.content_transform(render)
                    .and_then(incular_core::Transform::inverse)
                    .is_some(),
                visible,
            );
            assert_eq!(
                nodes(&tree).iter().any(|node| node
                    .label
                    .as_deref()
                    .is_some_and(|label| label.contains("inside"))),
                visible,
                "retained merge={merge}, height={height}",
            );
        }
    }
}

fn annotation(level: Option<u8>, label: &str, child: impl Into<Widget>) -> Widget {
    let annotation = Semantics::new(child).label(label);
    if let Some(level) = level {
        annotation.heading_level(level).into()
    } else {
        annotation.into()
    }
}

#[test]
fn default_and_valid_heading_levels_appear_in_the_public_tree() {
    for level in [None, Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)] {
        let tree = mounted(annotation(level, "heading", SizedBox::shrink()));
        let node = nodes(&tree)
            .into_iter()
            .find(|node| node.label.as_deref() == Some("heading"))
            .unwrap();
        assert_eq!(node.state.heading_level, level);
        assert_eq!(node.role == SemanticRole::Heading, level.is_some());
    }
}

#[test]
fn out_of_range_heading_cannot_claim_heading_role() {
    // Flutter's original debug assertion rejects even constructing these
    // values. Rust uses u8, so compare representable bad values and check
    // that accepting one does not produce a spurious heading.
    let advertised: Vec<_> = [0, 7, 8, 9]
        .into_iter()
        .filter(|&level| {
            std::panic::catch_unwind(|| {
                let tree = mounted(annotation(Some(level), "heading", SizedBox::shrink()));
                nodes(&tree)
                    .into_iter()
                    .any(|node| node.role == SemanticRole::Heading)
            })
            .unwrap_or(false)
        })
        .collect();
    assert!(
        advertised.is_empty(),
        "invalid heading roles advertised for {advertised:?}"
    );
}

/// widgets/semantics_test.dart:2020, including invalid signed values.
#[test]
#[cfg(debug_assertions)]
fn invalid_heading_levels_are_rejected_at_construction() {
    for bad in [-1, 0, 7, 8, 9] {
        assert!(
            std::panic::catch_unwind(|| Semantics::new(SizedBox::shrink()).heading_level(bad))
                .is_err(),
            "level {bad} was not rejected"
        );
    }
}

#[test]
fn merged_heading_uses_parent_then_first_non_null_descendant() {
    let cases = [
        (None, None, None),
        (None, Some(1), Some(1)),
        (None, Some(2), Some(2)),
        (Some(1), None, Some(1)),
        (Some(2), None, Some(2)),
        (Some(3), Some(2), Some(3)),
        (Some(4), Some(1), Some(4)),
        (Some(2), Some(3), Some(2)),
        (Some(1), Some(5), Some(1)),
    ];
    let actual: Vec<_> = cases
        .iter()
        .map(|&(parent, child, _)| {
            let child = annotation(child, "child", SizedBox::new().width(40.).height(20.));
            let tree = mounted(MergeSemantics::new(annotation(
                parent,
                "parent",
                Padding::all(1., child),
            )));
            let node = nodes(&tree)
                .into_iter()
                .find(|node| node.label.as_deref() == Some("parent\nchild"))
                .expect("combined label");
            node.state.heading_level
        })
        .collect();
    assert_eq!(actual, cases.map(|(_, _, expected)| expected));
}

#[test]
fn merged_heading_updates_without_replacing_the_retained_node() {
    let scene = |parent, first, second| {
        let children = Column::new([
            annotation(first, "first", SizedBox::new().width(40.).height(20.)),
            annotation(second, "second", SizedBox::new().width(40.).height(20.)),
        ]);
        MergeSemantics::new(annotation(parent, "parent", children)).into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(None, Some(2), Some(4))).unwrap();
    let mut retained = None;
    for (parent, first, second, expected) in [
        (None, Some(2), Some(4), Some(2)),
        (None, None, Some(4), Some(4)),
        (Some(1), Some(2), Some(4), Some(1)),
        (None, None, None, None),
    ] {
        tree.update(root, scene(parent, first, second)).unwrap();
        tree.layout(bounds).unwrap();
        tree.update_semantics();
        let (id, node) = tree
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some("parent\nfirst\nsecond"))
            .unwrap();
        assert_eq!(node.state.heading_level, expected);
        assert_eq!(node.role == SemanticRole::Heading, expected.is_some());
        assert_eq!(id, *retained.get_or_insert(id));
    }
}
