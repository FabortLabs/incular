use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, DirectionalFocusIntent, Focus, FocusNode, FocusScope, FocusScopeNode,
    FocusTraversalGroup, FocusTraversalPolicyKind, FocusTraversalRequest,
    ScrollPositionAlignmentPolicy, SizedBox, TraversalDirection, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn sequential(kind: FocusTraversalPolicyKind) {
    let node = FocusNode::new();
    let scope = FocusScopeNode::new();
    let requests = Rc::new(RefCell::new(Vec::<FocusTraversalRequest>::new()));
    let scene = FocusTraversalGroup::new(
        FocusScope::new(Focus::new(SizedBox::expand()).node(node.clone())).node(scope.clone()),
    )
    .policy(kind)
    .request_focus_callback({
        let requests = requests.clone();
        move |request| requests.borrow_mut().push(request)
    });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    for (reverse, alignment) in [
        (false, ScrollPositionAlignmentPolicy::KeepVisibleAtEnd),
        (true, ScrollPositionAlignmentPolicy::KeepVisibleAtStart),
    ] {
        assert!(if reverse {
            runtime.previous_focus_from(&scope.scope_focus_node())
        } else {
            runtime.next_focus_from(&scope.scope_focus_node())
        });
        frame(&mut runtime);
        assert_eq!(
            requests.take(),
            [FocusTraversalRequest {
                node: Some(node.clone()),
                alignment_policy: alignment
            }]
        );
        assert!(
            !node.has_focus(),
            "the custom callback replaces the default request"
        );
    }
}

#[test]
fn widget_order_focus_directions_call_the_custom_request() {
    sequential(FocusTraversalPolicyKind::WidgetOrder);
}

#[test]
fn reading_order_focus_directions_call_the_custom_request() {
    sequential(FocusTraversalPolicyKind::ReadingOrder);
}

#[test]
fn explicit_order_focus_directions_call_the_custom_request() {
    sequential(FocusTraversalPolicyKind::Ordered);
}

#[test]
fn spatial_traversal_calls_the_override_without_automatically_focusing() {
    let node = FocusNode::new();
    let requests = Rc::new(RefCell::new(Vec::<FocusTraversalRequest>::new()));
    let scene = FocusTraversalGroup::new(FocusScope::new(
        Focus::new(SizedBox::expand()).node(node.clone()),
    ))
    .request_focus_callback({
        let requests = requests.clone();
        move |request| requests.borrow_mut().push(request)
    });
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    for (direction, alignment) in [
        (
            TraversalDirection::Up,
            ScrollPositionAlignmentPolicy::KeepVisibleAtStart,
        ),
        (
            TraversalDirection::Down,
            ScrollPositionAlignmentPolicy::KeepVisibleAtEnd,
        ),
        (
            TraversalDirection::Left,
            ScrollPositionAlignmentPolicy::KeepVisibleAtStart,
        ),
        (
            TraversalDirection::Right,
            ScrollPositionAlignmentPolicy::KeepVisibleAtEnd,
        ),
    ] {
        assert!(runtime.invoke_directional_focus(DirectionalFocusIntent::new(direction)));
        frame(&mut runtime);
        assert_eq!(
            requests.take(),
            [FocusTraversalRequest {
                node: Some(node.clone()),
                alignment_policy: alignment
            }]
        );
        assert!(!node.has_focus());
    }
}

#[test]
fn a_cross_group_traversal_uses_the_initiating_policy_and_can_request_focus_itself() {
    let [first, second] = std::array::from_fn(|_| FocusNode::new());
    let log = Rc::new(RefCell::new(Vec::<FocusNode>::new()));
    let from = FocusTraversalGroup::new(Focus::new(SizedBox::square(20.)).node(first.clone()))
        .request_focus_callback({
            let log = log.clone();
            move |request| {
                let target = request.node.unwrap();
                log.borrow_mut().push(target.clone());
                target.request_focus();
            }
        });
    let to = FocusTraversalGroup::new(Focus::new(SizedBox::square(20.)).node(second.clone()))
        .request_focus_callback(|_| panic!("the destination policy must not replace the caller"));
    let mut runtime =
        Runtime::new(Column::new([Widget::from(from), Widget::from(to)]).into()).unwrap();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(runtime.next_focus_from(&first));
    frame(&mut runtime);
    assert_eq!(log.take(), [second.clone()]);
    assert!(second.has_primary_focus());
}
