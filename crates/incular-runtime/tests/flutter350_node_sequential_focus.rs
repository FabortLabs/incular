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
