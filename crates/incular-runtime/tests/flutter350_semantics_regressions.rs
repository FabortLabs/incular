//! Headless semantics regressions against pinned Flutter 3.50.0.
//! See the parity audit's focused and block-semantics comparisons (C03, C23).

use std::{cell::Cell, collections::BTreeSet, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{
    BlockSemantics, Column, MergeSemantics, Padding, Semantics, SizedBox, Stack, Widget,
};

fn run(widget: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(240., 160.)))
        .unwrap();
    runtime
}

#[test]
fn merge_semantics_advertises_and_dispatches_first_descendant_tap() {
    let tap_target = |label, calls: Rc<Cell<u32>>| -> Widget {
        Semantics::new(SizedBox::new().width(60.).height(40.))
            .label(label)
            .on_tap(move || calls.set(calls.get() + 1))
            .into()
    };
    let north = Rc::new(Cell::new(0));
    let south = Rc::new(Cell::new(0));
    let mut runtime = run(MergeSemantics::new(Column::new([
        tap_target("North", north.clone()),
        tap_target("South", south.clone()),
    ])));
    let (merged_id, merged) = runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("North\nSouth"))
        .expect("the merged label contains both descendants in tree order");
    assert!(
        merged.actions.contains(&SemanticActionKind::Activate),
        "the merged node must advertise activation"
    );
    assert!(runtime.dispatch_semantic_action(merged_id, SemanticAction::Activate));
    assert_eq!((north.get(), south.get()), (1, 0));
}

fn stack_labels(middle: Widget) -> BTreeSet<String> {
    let runtime = run(Stack::new([tag("earlier"), middle, tag("later")]));
    runtime
        .tree()
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref())
        .filter(|label| !label.is_empty())
        .map(str::to_owned)
        .collect()
}

fn tag(label: &str) -> Widget {
    Semantics::new(SizedBox::new().width(40.).height(40.))
        .container(true)
        .label(label)
        .into()
}

fn assert_stack_labels(middle: impl Into<Widget>, expected: &[&str]) {
    let actual = stack_labels(middle.into());
    let expected: BTreeSet<_> = expected.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn block_semantics_directly_hides_the_preceding_sibling_only() {
    assert_stack_labels(BlockSemantics::new(tag("front")), &["front", "later"]);
}

#[test]
fn block_semantics_does_not_escape_an_explicit_semantic_boundary() {
    let inner = Stack::new([
        tag("inside earlier"),
        BlockSemantics::new(tag("front")).into(),
    ]);
    let boundary = Semantics::new(inner)
        .container(true)
        .explicit_child_nodes(true)
        .label("boundary");
    assert_stack_labels(boundary, &["boundary", "earlier", "front", "later"]);
}

#[test]
fn block_semantics_propagates_through_nonboundary_padding() {
    let padded_blocker = Padding::all(3., BlockSemantics::new(tag("front")));
    assert_stack_labels(padded_blocker, &["front", "later"]);
}

// Flutter packages/flutter/test/widgets/semantics_9_test.dart:43.
#[test]
fn block_semantics_keeps_nested_explicit_child_inside_boundary() {
    let tag =
        |label| Widget::from(Semantics::new(SizedBox::new().width(240.).height(160.)).label(label));
    let blocked_child = Semantics::new(SizedBox::new().width(240.).height(160.))
        .container(true)
        .label("#2.2.1");
    let wrapper = Semantics::new(BlockSemantics::new(blocked_child)).label("#2.2");
    let boundary = Semantics::new(Stack::new([tag("NOT#2.1"), wrapper.into(), tag("#2.3")]))
        .container(true)
        .explicit_child_nodes(true)
        .label("#2");
    let runtime = run(Stack::new([tag("#1"), boundary.into(), tag("#3")]));
    let labels: BTreeSet<_> = runtime
        .tree()
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref())
        .filter(|label| !label.is_empty())
        .collect();
    assert_eq!(
        labels,
        BTreeSet::from(["#1", "#2", "#2.2", "#2.2.1", "#2.3", "#3"])
    );
}
