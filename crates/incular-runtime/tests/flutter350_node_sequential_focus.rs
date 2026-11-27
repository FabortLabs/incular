use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, FocusTraversalGroup,
    Padding, Row, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn item(node: &FocusNode, dimension: f32) -> Focus {
    Focus::new(SizedBox::new().width(dimension).height(dimension)).node(node.clone())
}

fn initial(reverse: bool) {
    let first = FocusNode::new();
    let second = FocusNode::new();
    let scope = FocusScopeNode::new();
    let first_child = item(&first, 100.);
    let second_child = item(&second, 100.);
    let scope_widget =
        FocusScope::new(Column::new([first_child, second_child])).node(scope.clone());
    let group = FocusTraversalGroup::new(scope_widget).widget_order();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, group).into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.focused_element().is_none());
    assert!(!first.has_focus());
    assert!(!second.has_focus());

    if reverse {
        assert!(runtime.previous_focus_from(&second));
    } else {
        assert!(runtime.next_focus_from(&second));
    }
    frame(&mut runtime);
    assert_eq!(first.has_primary_focus(), !reverse);
    assert_eq!(second.has_primary_focus(), reverse);
    assert!(scope.scope_focus_node().has_focus());
}

#[test]
fn forward_invoked_from_a_node_in_an_unfocused_scope_starts_at_the_first() {
    initial(false);
}

#[test]
fn backward_invoked_from_a_node_in_an_unfocused_scope_starts_at_the_last() {
    initial(true);
}

fn cross_scope(padded: bool) {
    let outer_a = FocusNode::new();
    let outer_b = FocusNode::new();
    outer_a.set_skip_traversal(true);
    outer_b.set_skip_traversal(true);
    let inner_a = FocusNode::new();
    let inner_b = FocusNode::new();
    let first = FocusScope::new(Focus::new(item(&inner_a, 10.)).node(outer_a.clone()));
    let child = Widget::from(item(&inner_b, 10.));
    let child = if padded {
        Padding::all(5., child).into()
    } else {
        child
    };
    let second = FocusScope::new(Focus::new(child).node(outer_b.clone()));
    let group = FocusTraversalGroup::new(Row::new([first, second]));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, group).into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.focused_element().is_none());
    inner_a.request_focus();
    frame(&mut runtime);
    assert!(inner_a.has_primary_focus());
    assert!(outer_b.skip_traversal());
    assert!(runtime.next_focus_from(&outer_b));
    frame(&mut runtime);
    assert!(inner_b.has_primary_focus());
    assert!(!inner_a.has_focus());
    assert!(!outer_b.has_primary_focus());
}

#[test]
fn different_sized_wrappers_do_not_prevent_entering_the_invoking_scope() {
    cross_scope(true);
}

#[test]
fn equal_sized_wrappers_do_not_prevent_entering_the_invoking_scope() {
    cross_scope(false);
}

#[test]
fn a_detached_node_cannot_redirect_focus_and_a_scope_remembers_its_child() {
    let [a, b, c] = std::array::from_fn(|_| FocusNode::new());
    let inside = FocusScope::new(Column::new([item(&a, 30.), item(&b, 30.)]));
    let scene = Column::new([Widget::from(inside), item(&c, 30.).into()]);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    a.request_focus();
    frame(&mut runtime);
    c.request_focus();
    frame(&mut runtime);
    assert!(c.has_primary_focus());
    assert!(!runtime.next_focus_from(&FocusNode::new()));
    assert!(c.has_primary_focus());

    assert!(runtime.next_focus_from(&a));
    assert!(b.has_primary_focus());
}

fn nested_scope_cycle(backwards: bool) {
    use std::{cell::RefCell, rc::Rc};
    let changes = Rc::new(RefCell::new([None; 4]));
    let watcher = |index: usize| {
        let changes = Rc::clone(&changes);
        move |focused| changes.borrow_mut()[index] = Some(focused)
    };
    let outer = FocusScopeNode::new();
    let inner = FocusScopeNode::new();
    let [first, second] = std::array::from_fn(|_| FocusNode::new());
    let child = FocusScope::new(Column::new([
        item(&first, 100.).on_focus_change(watcher(2)),
        item(&second, 100.).on_focus_change(watcher(3)),
    ]))
    .node(inner.clone())
    .on_focus_change(watcher(1));
    let outer_scope = FocusScope::new(Column::new([child]))
        .node(outer.clone())
        .on_focus_change(watcher(0));
    let root = Directionality::new(
        TextDirection::Ltr,
        FocusTraversalGroup::new(outer_scope).widget_order(),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);

    let nodes = [first, second];
    let initial = usize::from(backwards);
    let assert_primary = |index: usize| {
        assert!(nodes[index].has_primary_focus());
        assert!(!nodes[1 - index].has_focus());
        assert!(inner.scope_focus_node().has_focus());
        assert!(outer.scope_focus_node().has_focus());
    };
    nodes[initial].request_focus();
    frame(&mut runtime);
    assert_primary(initial);
    let mut initial_callbacks = [Some(true), Some(true), None, None];
    initial_callbacks[initial + 2] = Some(true);
    assert_eq!(*changes.borrow(), initial_callbacks);

    let mut current = initial;
    for previous in [backwards, backwards, !backwards] {
        changes.borrow_mut().fill(None);
        // Invoke from the first node even when the second is primary.
        let moved = if previous {
            runtime.previous_focus_from(&nodes[0])
        } else {
            runtime.next_focus_from(&nodes[0])
        };
        assert!(moved);
        frame(&mut runtime);
        let next = 1 - current;
        assert_primary(next);
        let mut expected = [None; 4];
        expected[current + 2] = Some(false);
        expected[next + 2] = Some(true);
        assert_eq!(*changes.borrow(), expected);
        current = next;
    }
}

#[test]
fn next_focus_cycles_within_nested_scopes_without_republishing_ancestor_focus() {
    nested_scope_cycle(false);
}

#[test]
fn previous_focus_cycles_within_nested_scopes_from_the_current_primary_node() {
    nested_scope_cycle(true);
}

#[test]
fn a_mount_owned_scope_reports_focus_entering_and_leaving_its_descendants() {
    use std::{cell::RefCell, rc::Rc};
    let [inside, outside] = std::array::from_fn(|_| FocusNode::new());
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&changes);
    let scope = FocusScope::new(item(&inside, 100.))
        .on_focus_change(move |has_focus| observed.borrow_mut().push(has_focus));
    let child = Column::new([Widget::from(scope), Widget::from(item(&outside, 100.))]);
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, child).into()).unwrap();
    frame(&mut runtime);
    assert!(changes.borrow().is_empty());
    inside.request_focus();
    frame(&mut runtime);
    assert_eq!(*changes.borrow(), [true]);
    outside.request_focus();
    frame(&mut runtime);
    assert_eq!(*changes.borrow(), [true, false]);
    outside.request_focus();
    frame(&mut runtime);
    assert_eq!(*changes.borrow(), [true, false]);
}
