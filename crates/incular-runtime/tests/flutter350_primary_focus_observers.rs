use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Directionality, Focus, FocusNode, Text, Widget};
use std::{cell::RefCell, rc::Rc};

fn tree(children: &[(&FocusNode, &'static str)]) -> Widget {
    Directionality::new(
        TextDirection::Rtl,
        Column::new(
            children
                .iter()
                .map(|(node, label)| Focus::new(Text::new(*label)).node((*node).clone()).into())
                .collect::<Vec<Widget>>(),
        ),
    )
    .into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn primary_focus_observer_reports_removing_the_focused_child_once() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let mut runtime = Runtime::new(tree(&[(&a, "a"), (&b, "b")])).unwrap();
    frame(&mut runtime);
    let log = Rc::new(RefCell::new(Vec::new()));
    let _subscription = runtime.observe_primary_focus({
        let (a, b, log) = (a.clone(), b.clone(), log.clone());
        move || {
            log.borrow_mut()
                .push((a.has_primary_focus(), b.has_primary_focus()))
        }
    });
    a.request_focus();
    assert!(log.borrow().is_empty());
    frame(&mut runtime);
    assert!(a.has_primary_focus());
    assert_eq!(log.take(), [(true, false)]);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, tree(&[(&b, "b")])).unwrap();
    frame(&mut runtime);
    frame(&mut runtime);
    assert!(!a.has_primary_focus());
    assert!(!b.has_primary_focus());
    assert_eq!(log.take(), [(false, false)]);
}

#[test]
fn focus_observer_only_reports_the_settled_request_not_intermediate_requests() {
    use incular_widgets::{FocusScope, FocusScopeNode, SizedBox};

    let scope = FocusScopeNode::new();
    let a = FocusNode::new();
    let b = FocusNode::new();
    let scene = FocusScope::new(
        Focus::new(Focus::new(SizedBox::shrink()).node(b.clone())).node(a.clone()),
    )
    .node(scope.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    let log = Rc::new(RefCell::new(Vec::new()));
    let subscription = runtime.observe_primary_focus({
        let (a, b, log) = (a.clone(), b.clone(), log.clone());
        move || {
            log.borrow_mut()
                .push((a.has_primary_focus(), b.has_primary_focus()))
        }
    });
    assert!(scope.autofocus(&b));
    assert!(log.borrow().is_empty());
    frame(&mut runtime);
    assert_eq!(log.take(), [(false, true)]);
    a.request_focus();
    b.request_focus();
    a.request_focus();
    assert!(log.borrow().is_empty());
    frame(&mut runtime);
    assert_eq!(log.take(), [(true, false)]);
    b.request_focus();
    frame(&mut runtime);
    assert_eq!(log.take(), [(false, true)]);
    b.unfocus();
    frame(&mut runtime);
    assert_eq!(log.take(), [(false, false)]);
    assert!(scope.has_primary_focus());
    drop(subscription);
    a.request_focus();
    frame(&mut runtime);
    assert!(a.has_primary_focus());
    assert!(log.borrow().is_empty());
}

#[test]
fn only_changed_ancestors_and_primary_nodes_receive_focus_notifications() {
    use incular_widgets::{FocusScope, FocusScopeNode, SizedBox};
    use std::cell::Cell;

    let top = FocusScopeNode::new();
    let left = FocusScopeNode::new();
    let right = FocusScopeNode::new();
    let child1 = FocusNode::new();
    let child2 = FocusNode::new();
    let scene = FocusScope::new(Column::new([
        FocusScope::new(Focus::new(SizedBox::shrink()).node(child1.clone())).node(left.clone()),
        FocusScope::new(Focus::new(SizedBox::shrink()).node(child2.clone())).node(right.clone()),
    ]))
    .node(top.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    let nodes = [
        top.scope_focus_node(),
        left.scope_focus_node(),
        right.scope_focus_node(),
        child1.clone(),
        child2.clone(),
    ];
    let calls = Rc::new(Cell::new([0_u8; 5]));
    let states = Rc::new(Cell::new([false; 5]));
    let _listeners: Vec<_> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (calls, states, observed) = (calls.clone(), states.clone(), node.clone());
            node.observe(move || {
                let mut counts = calls.get();
                counts[i] += 1;
                calls.set(counts);
                let mut values = states.get();
                values[i] = observed.has_focus();
                states.set(values);
            })
        })
        .collect();
    let expect = |expected_calls, expected_states| {
        assert_eq!(calls.replace([0; 5]), expected_calls);
        assert_eq!(states.replace([false; 5]), expected_states);
    };

    child1.request_focus();
    frame(&mut runtime);
    expect([1, 1, 0, 1, 0], [true, true, false, true, false]);

    child1.unfocus();
    frame(&mut runtime);
    expect([0, 1, 0, 1, 0], [false, true, false, false, false]);

    child1.request_focus();
    frame(&mut runtime);
    expect([0, 1, 0, 1, 0], [false, true, false, true, false]);

    child2.request_focus();
    frame(&mut runtime);
    expect([0, 1, 1, 1, 1], [false, false, true, false, true]);

    child1.request_focus();
    child2.request_focus();
    frame(&mut runtime);
    expect([0; 5], [false; 5]);
}

#[test]
fn sibling_scope_autofocus_and_explicit_request_notify_only_the_final_focus() {
    use incular_widgets::{FocusScope, FocusScopeNode, SizedBox};

    for autofocus_first in [true, false] {
        let a = FocusNode::new();
        let b = FocusNode::new();
        let left = FocusScopeNode::new();
        let right = FocusScopeNode::new();
        let scene = Column::new([
            FocusScope::new(Focus::new(SizedBox::shrink()).node(a.clone())).node(left.clone()),
            FocusScope::new(Focus::new(SizedBox::shrink()).node(b.clone())).node(right.clone()),
        ]);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        frame(&mut runtime);
        let log = Rc::new(RefCell::new(Vec::new()));
        let a_events = Rc::new(RefCell::new(Vec::new()));
        let _manager = runtime.observe_primary_focus({
            let (a, b, log) = (a.clone(), b.clone(), log.clone());
            move || log.borrow_mut().push((a.has_primary_focus(), b.has_primary_focus()))
        });
        let _a_listener = a.observe({
            let (a, log) = (a.clone(), a_events.clone());
            move || log.borrow_mut().push(a.has_focus())
        });

        if autofocus_first {
            assert!(right.autofocus(&b));
            a.request_focus();
        } else {
            a.request_focus();
            assert!(right.autofocus(&b));
        }
        assert!(log.borrow().is_empty());
        frame(&mut runtime);
        assert_eq!(log.take(), [(false, true)]);
        assert!(a_events.borrow().is_empty());
        assert!(b.has_primary_focus());
        assert_eq!(left.focused_child(), Some(a));
        assert_eq!(right.focused_child(), Some(b));
    }
}

#[test]
fn losing_the_primary_request_does_not_erase_an_inactive_scopes_history() {
    use incular_widgets::{FocusScope, FocusScopeNode, SizedBox};

    for queue_autofocus in [false, true] {
        let a1 = FocusNode::new();
        let a2 = FocusNode::new();
        let b = FocusNode::new();
        let left = FocusScopeNode::new();
        let right = FocusScopeNode::new();
        let scene = Column::new([
            FocusScope::new(Column::new([
                Focus::new(SizedBox::shrink()).node(a1.clone()),
                Focus::new(SizedBox::shrink()).node(a2.clone()),
            ]))
            .node(left.clone()),
            FocusScope::new(Focus::new(SizedBox::shrink()).node(b.clone())).node(right.clone()),
        ]);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        frame(&mut runtime);

        let log = Rc::new(RefCell::new(Vec::new()));
        let _manager = runtime.observe_primary_focus({
            let (a1, a2, b, log) = (a1.clone(), a2.clone(), b.clone(), log.clone());
            move || {
                log.borrow_mut().push((
                    a1.has_primary_focus(),
                    a2.has_primary_focus(),
                    b.has_primary_focus(),
                ))
            }
        });
        a1.request_focus();
        b.request_focus();
        if queue_autofocus {
            assert!(left.autofocus(&a2));
        }
        assert!(log.borrow().is_empty());
        frame(&mut runtime);
        assert_eq!(log.take(), [(false, false, true)]);
        assert_eq!(left.focused_child(), Some(a1.clone()));
        assert_eq!(right.focused_child(), Some(b));

        assert!(left.request_focus_in_scope());
        frame(&mut runtime);
        assert_eq!(log.take(), [(true, false, false)]);
        assert!(a1.has_primary_focus());
    }
}
