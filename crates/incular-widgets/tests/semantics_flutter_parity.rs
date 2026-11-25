//! Regression tests for the Flutter semantics parity findings (semantics-001,
//! 002, 004, 005, 006, 007, 008, 009, 014). Flutter references are given per
//! test.

use std::cell::Cell;
use std::rc::Rc;

use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_semantics::{OrdinalSortKey, SemanticActionKind, SemanticRole, SemanticState};
use incular_widgets::{
    Column, Directionality, GestureDetector, MergeSemantics, Opacity, Row, Semantics,
    SingleChildScrollView, SizedBox, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn mounted(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).expect("mount semantics tree");
    tree.layout(Constraints::tight(Size::new(240.0, 160.0)))
        .expect("layout semantics tree");
    tree.update_semantics();
    tree
}

fn labelled<'a>(tree: &'a WidgetTree, label: &str) -> Option<&'a incular_semantics::SemanticNode> {
    tree.semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some(label))
}

/// semantics-001: `widgets/basic.dart:8113-8126` and
/// `semantics/semantics.dart:3748-3884`. One merged node whose label joins the
/// descendant labels with a newline; the children are not exposed separately.
#[test]
fn merge_semantics_emits_one_node_with_newline_joined_label() {
    let tree = mounted(MergeSemantics::new(Column::new([
        Widget::from(
            Semantics::new(SizedBox::new().width(60.0).height(40.0))
                .label("North")
                .button(true),
        ),
        Widget::from(
            Semantics::new(SizedBox::new().width(60.0).height(40.0))
                .label("South")
                .button(true),
        ),
    ])));

    let merged = labelled(&tree, "North\nSouth").expect("merged node");
    assert!(merged.children.is_empty());
    assert!(labelled(&tree, "North").is_none());
    assert!(labelled(&tree, "South").is_none());
    assert_eq!(tree.semantics().iter().count(), 1);
}

/// semantics-002: `semantics/semantics.dart:1732-1738` leaves `enabled`
/// unspecified. Only an explicit `false` may disable a control.
#[test]
fn unspecified_enabled_is_not_disabled_and_explicit_false_is() {
    let unspecified = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Unspecified")
            .button(true)
            .on_tap(|| {}),
    );
    let node = labelled(&unspecified, "Unspecified").expect("unspecified node");
    assert!(node.state.enabled);

    let disabled = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Disabled")
            .button(true)
            .enabled(false),
    );
    assert!(
        !labelled(&disabled, "Disabled")
            .expect("disabled node")
            .state
            .enabled
    );
}

/// semantics-004: `widgets/basic.dart:7942` and `rendering/proxy_box.dart`.
/// A labelled Semantics wrapper without a role is a generic container node.
#[test]
fn label_only_semantics_creates_a_generic_container_node() {
    let tree =
        mounted(Semantics::new(SizedBox::new().width(40.0).height(20.0)).label("Label only"));
    let node = labelled(&tree, "Label only").expect("label-only node");
    assert_eq!(node.role, SemanticRole::GenericContainer);
}

/// A wrapper with no semantic information creates no node.
#[test]
fn semantics_wrapper_without_metadata_creates_no_node() {
    let tree = mounted(Semantics::new(SizedBox::new().width(40.0).height(20.0)));
    assert!(tree.semantics().is_empty());
}

/// semantics-005: `semantics/semantics.dart:2309`. `onTap` attaches the
/// action without implying the Button role.
#[test]
fn on_tap_does_not_imply_button_role() {
    let tree = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Plain")
            .on_tap(|| {}),
    );
    let node = labelled(&tree, "Plain").expect("tap node");
    assert_eq!(node.role, SemanticRole::GenericContainer);
    assert!(node.actions.contains(&SemanticActionKind::Activate));
}

/// semantics-006: hint and tooltip are separate. The hint stays in the
/// description, and the tooltip goes to the native tooltip property.
#[test]
fn tooltip_does_not_overwrite_hint() {
    let tree = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Volume")
            .hint("Double tap to adjust")
            .tooltip("Volume control"),
    );
    let node = labelled(&tree, "Volume").expect("volume node");
    assert_eq!(node.description.as_deref(), Some("Double tap to adjust"));
    assert_eq!(node.state.tooltip.as_deref(), Some("Volume control"));
}

/// semantics-007: `widgets/gesture_detector.dart:1575-1600`. A GestureDetector
/// tap is a semantic action whose dispatch runs the detector's `on_tap`.
#[test]
fn gesture_detector_tap_is_exposed_and_dispatches_its_callback() {
    let calls = Rc::new(Cell::new(0));
    let tapped = calls.clone();
    let tree = mounted(
        GestureDetector::new(SizedBox::new().width(120.0).height(40.0))
            .on_tap(move || tapped.set(tapped.get() + 1)),
    );
    let (id, node) = tree
        .semantics()
        .iter()
        .find(|(_, node)| node.actions.contains(&SemanticActionKind::Activate))
        .expect("gesture tap node");
    assert_eq!(node.role, SemanticRole::GenericContainer);
    let element = tree.element_for_semantic_node(id).expect("element");
    let callback = tree
        .semantic_action_callback(element, SemanticActionKind::Activate)
        .expect("tap callback");
    callback();
    assert_eq!(calls.get(), 1);
}

/// semantics-008: `semantics/semantics.dart:2325`. A long-press callback is a
/// semantic action and dispatches its callback.
#[test]
fn on_long_press_is_a_semantic_action() {
    let calls = Rc::new(Cell::new(0));
    let pressed = calls.clone();
    let tree = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Menu")
            .on_long_press(move || pressed.set(pressed.get() + 1)),
    );
    let (id, node) = tree
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Menu"))
        .expect("menu node");
    assert!(node.actions.contains(&SemanticActionKind::LongPress));
    let element = tree.element_for_semantic_node(id).expect("element");
    let callback = tree
        .semantic_action_callback(element, SemanticActionKind::LongPress)
        .expect("long press callback");
    callback();
    assert_eq!(calls.get(), 1);
}

/// semantics-009: `semantics/semantics.dart:2125`. The increased and
/// decreased values reach the retained state.
#[test]
fn increased_and_decreased_values_are_retained() {
    let tree = mounted(
        Semantics::new(SizedBox::new().width(40.0).height(20.0))
            .label("Stepper")
            .value("5")
            .increased_value("6")
            .decreased_value("4")
            .on_increase(|| {})
            .on_decrease(|| {}),
    );
    let node = labelled(&tree, "Stepper").expect("stepper node");
    assert_eq!(node.state.increased_value.as_deref(), Some("6"));
    assert_eq!(node.state.decreased_value.as_deref(), Some("4"));
}

/// Default semantic state: `enabled` defaults to unspecified (true) so that a
/// node built with `SemanticState::default()` is not treated as disabled.
#[test]
fn default_semantic_state_is_enabled() {
    assert!(SemanticState::default().enabled);
}

/// Labels of the children of the node labelled `label`, in traversal order.
fn child_labels(tree: &WidgetTree, label: &str) -> Vec<String> {
    let node = labelled(tree, label).expect("parent node");
    node.children
        .iter()
        .filter_map(|id| tree.semantics().node(*id))
        .filter_map(|child| child.label.clone())
        .collect()
}

fn button(label: &str) -> Semantics {
    Semantics::new(SizedBox::new().width(40.0).height(20.0))
        .label(label)
        .button(true)
}

/// semantics-010: `semantics/semantics.dart:4222-4268`. Siblings with
/// compatible ordinal keys are reordered by key; an unkeyed sibling keeps its
/// place and splits the keyed runs around it.
#[test]
fn sort_keys_reorder_keyed_siblings_and_keep_unkeyed_order() {
    let keyed =
        |label: &str, order: f64| Widget::from(button(label).sort_key(OrdinalSortKey::new(order)));
    let tree = mounted(
        Semantics::new(Column::new([
            keyed("C", 3.0),
            keyed("A", 1.0),
            keyed("B", 2.0),
        ]))
        .label("ordered")
        .group(true),
    );
    assert_eq!(child_labels(&tree, "ordered"), ["A", "B", "C"]);

    let split = mounted(
        Semantics::new(Column::new([
            Widget::from(button("X")),
            keyed("C", 3.0),
            keyed("A", 1.0),
        ]))
        .label("split")
        .group(true),
    );
    assert_eq!(child_labels(&split, "split"), ["X", "A", "C"]);
}

#[test]
fn ordinal_sort_keys_keep_widget_order_for_signed_zero_ties() {
    let tree = mounted(
        Semantics::new(Column::new([
            Widget::from(button("positive zero").sort_key(OrdinalSortKey::new(0.))),
            Widget::from(button("negative zero").sort_key(OrdinalSortKey::new(-0.))),
        ]))
        .label("zero ties")
        .group(true),
    );
    assert_eq!(
        child_labels(&tree, "zero ties"),
        ["positive zero", "negative zero"]
    );
}

#[test]
fn traversal_order_keeps_a_grid_with_touching_elements_in_reading_order() {
    let rows = (0..3)
        .map(|row| {
            Widget::from(Row::new(
                (0..3)
                    .map(|column| {
                        let label = format!("{row} - {column}");
                        let box_widget = Widget::from(SizedBox::from_dimensions(
                            Some(50.),
                            Some(50.),
                            Some(Text::new(label.clone())),
                        ))
                        .with_key(Key::from(label.as_str()));
                        Widget::from(Semantics::new(box_widget))
                    })
                    .collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    let grid = Semantics::new(Column::new(rows))
        .label("touching grid")
        .container(true)
        .explicit_child_nodes(true);
    let tree = mounted(Directionality::new(TextDirection::Ltr, grid));
    let expected = (0..3)
        .flat_map(|row| (0..3).map(move |column| format!("{row} - {column}")))
        .collect::<Vec<_>>();
    let mut bounds: Vec<Rect> = Vec::new();
    for label in &expected {
        let id = tree
            .element_with_key(&Key::from(label.as_str()))
            .expect("mounted cell");
        bounds.push(tree.element_bounds(id).expect("cell bounds"));
    }
    assert!(bounds.iter().all(|rect| rect.size == Size::new(50., 50.)));
    for (index, rect) in bounds.iter().enumerate() {
        if index % 3 != 0 {
            assert_eq!(bounds[index - 1].origin.x + 50., rect.origin.x);
        }
        if index >= 3 {
            assert_eq!(bounds[index - 3].origin.y + 50., rect.origin.y);
        }
    }
    assert_eq!(child_labels(&tree, "touching grid"), expected);
}

/// semantics-012: `widgets/basic.dart:7942`. `container(true)` makes a bare
/// annotation its own accessibility node.
#[test]
fn container_forces_a_node_for_a_bare_annotation() {
    let bare = mounted(Semantics::new(SizedBox::new().width(10.0).height(10.0)));
    assert!(bare.semantics().is_empty());

    let forced = mounted(Semantics::new(SizedBox::new().width(10.0).height(10.0)).container(true));
    assert_eq!(forced.semantics().iter().count(), 1);
}

/// semantics-012: `semantics/semantics.dart:5129,5186`. `explicitChildNodes`
/// keeps a merging node's children separate and out of its merged label.
#[test]
fn explicit_child_nodes_keep_children_out_of_the_merged_label() {
    let group = || {
        Semantics::new(Column::new([
            Widget::from(button("North")),
            Widget::from(button("South")),
        ]))
        .label("Group")
    };

    let merged = mounted(MergeSemantics::new(group()));
    assert!(labelled(&merged, "Group\nNorth\nSouth").is_some());

    let explicit = mounted(MergeSemantics::new(group().explicit_child_nodes(true)));
    assert!(labelled(&explicit, "Group").is_some());
    assert!(labelled(&explicit, "North").is_some());
    assert!(labelled(&explicit, "South").is_some());
    assert_eq!(child_labels(&explicit, "Group"), ["North", "South"]);
}

/// implicit-004: `widgets/proxy_box.dart:957`. A zero-alpha opacity hides its
/// child's semantics unless `alwaysIncludeSemantics` is set.
#[test]
fn zero_opacity_hides_child_semantics_unless_always_included() {
    let child = || button("Faded");
    assert!(labelled(&mounted(Opacity::new(0.0, child())), "Faded").is_none());
    assert!(labelled(&mounted(Opacity::new(0.5, child())), "Faded").is_some());
    assert!(
        labelled(
            &mounted(Opacity::new(0.0, child()).always_include_semantics(true)),
            "Faded"
        )
        .is_some()
    );
}

/// semantics-011: `semantics/semantics.dart:2758,2800`. A node inside a scroll
/// viewport advertises `ShowOnScreen`; one outside does not.
#[test]
fn nodes_inside_a_scroll_view_advertise_show_on_screen() {
    let tree = mounted(Column::new([
        Widget::from(button("Outside")),
        Widget::from(SingleChildScrollView::new(button("Inside"))),
    ]));
    let inside = labelled(&tree, "Inside").expect("inside node");
    assert!(inside.actions.contains(&SemanticActionKind::ShowOnScreen));
    let outside = labelled(&tree, "Outside").expect("outside node");
    assert!(!outside.actions.contains(&SemanticActionKind::ShowOnScreen));
}
