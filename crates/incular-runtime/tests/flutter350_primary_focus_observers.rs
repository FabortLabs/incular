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
    let a = FocusNode::new();
    let b = FocusNode::new();
    let mut runtime = Runtime::new(tree(&[(&a, "a"), (&b, "b")])).unwrap();
    frame(&mut runtime);
    let log = Rc::new(RefCell::new(Vec::new()));
    let subscription = runtime.observe_primary_focus({
        let (a, b, log) = (a.clone(), b.clone(), log.clone());
        move || {
            log.borrow_mut()
                .push((a.has_primary_focus(), b.has_primary_focus()))
        }
    });
    b.request_focus();
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
    drop(subscription);
    a.request_focus();
    frame(&mut runtime);
    assert!(a.has_primary_focus());
    assert!(log.borrow().is_empty());
}
