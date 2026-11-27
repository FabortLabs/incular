use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, Positioned, SizedBox, Stack,
    TextDirection, TraversalEdgeBehavior as Edge, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn deliver(runtime: &mut Runtime, name: NamedKey, code: Code) -> bool {
    let key = KeyboardKey::Named(name);
    let handled = runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), code));
    runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, code));
    frame(runtime);
    handled
}

#[test]
fn directional_edge_policies_change_without_a_rebuild() {
    let nodes = std::array::from_fn::<_, 6, _>(|_| FocusNode::new());
    let child_scope = FocusScopeNode::new();
    assert_eq!(
        child_scope.directional_traversal_edge_behavior(),
        Edge::Stop
    );
    let callbacks = Rc::new(RefCell::new([None; 6]));
    let stop = |index: usize, x, y| -> Widget {
        let callbacks = callbacks.clone();
        Positioned::new(
            Focus::new(SizedBox::new().width(100.).height(100.))
                .node(nodes[index].clone())
                .on_focus_change(move |focused| callbacks.borrow_mut()[index] = Some(focused)),
        )
        .left(x)
        .top(y)
        .into()
    };
    // The inner scope contains [1] over [2], with [3] staggered one row
    // lower and to the right. The outer scope contains [0], [4], and [5].
    let inner = FocusScope::new(Stack::new([
        stop(1, 0., 0.),
        stop(2, 0., 100.),
        stop(3, 100., 200.),
    ]))
    .node(child_scope.clone());
    let scene = FocusScope::new(Stack::new([
        stop(0, 350., 0.),
        Positioned::new(inner)
            .left(0.)
            .top(100.)
            .width(800.)
            .height(300.)
            .into(),
        stop(4, 0., 400.),
        stop(5, 100., 400.),
    ]));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);

    for (policy, down_target, up_target) in [
        (Edge::Stop, Some(3), Some(1)),
        (Edge::ClosedLoop, Some(1), Some(2)),
        (Edge::ParentScope, Some(5), Some(0)),
        (Edge::LeaveFlutterView, None, None),
    ] {
        child_scope.set_directional_traversal_edge_behavior(policy);
        assert_eq!(child_scope.directional_traversal_edge_behavior(), policy);
        for (start, target, key, code) in [
            (3, down_target, NamedKey::ArrowDown, Code::ArrowDown),
            (1, up_target, NamedKey::ArrowUp, Code::ArrowUp),
        ] {
            nodes[start].request_focus();
            frame(&mut runtime);
            assert!(nodes[start].has_primary_focus());
            callbacks.borrow_mut().fill(None);
            deliver(&mut runtime, key, code);
            let mut changes = [None; 6];
            if target != Some(start) {
                changes[start] = Some(false);
                if let Some(target) = target {
                    changes[target] = Some(true);
                }
            }
            assert_eq!(*callbacks.borrow(), changes, "{policy:?}, start {start}");
            for (i, node) in nodes.iter().enumerate() {
                assert_eq!(
                    node.has_primary_focus(),
                    target == Some(i),
                    "{policy:?}, start {start}, node {i}"
                );
            }
        }
    }
}

fn tab_targets(a: &FocusNode, b: &FocusNode) -> incular_widgets::Column {
    incular_widgets::Column::new(
        [a, b].map(|node| Focus::new(SizedBox::new().width(100.).height(100.)).node(node.clone())),
    )
}

fn check_tab_focus(a: &FocusNode, b: &FocusNode, expected: Option<usize>) {
    assert_eq!(a.has_primary_focus(), expected == Some(0));
    assert_eq!(b.has_primary_focus(), expected == Some(1));
}

#[test]
fn sequential_edge_policies_change_without_rebuilding_the_scope() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let scope = FocusScopeNode::new();
    assert_eq!(scope.traversal_edge_behavior(), Edge::ClosedLoop);
    let scene = FocusScope::new(tab_targets(&a, &b)).node(scope.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);

    a.request_focus();
    frame(&mut runtime);
    check_tab_focus(&a, &b, Some(0));
    assert!(runtime.next_focus());
    check_tab_focus(&a, &b, Some(1));
    assert!(runtime.next_focus());
    check_tab_focus(&a, &b, Some(0));

    scope.set_traversal_edge_behavior(Edge::LeaveFlutterView);
    assert_eq!(scope.traversal_edge_behavior(), Edge::LeaveFlutterView);
    assert!(runtime.next_focus());
    check_tab_focus(&a, &b, Some(1));
    assert!(!deliver(&mut runtime, NamedKey::Tab, Code::Tab));
    check_tab_focus(&a, &b, None);

    scope.set_traversal_edge_behavior(Edge::Stop);
    assert_eq!(scope.traversal_edge_behavior(), Edge::Stop);
    b.request_focus();
    frame(&mut runtime);
    assert!(!runtime.next_focus());
    check_tab_focus(&a, &b, Some(1));

    a.request_focus();
    frame(&mut runtime);
    scope.set_traversal_edge_behavior(Edge::ClosedLoop);
    assert_eq!(scope.traversal_edge_behavior(), Edge::ClosedLoop);
    assert!(runtime.previous_focus());
    check_tab_focus(&a, &b, Some(1));
}

#[test]
fn tab_wraps_at_default_native_route_edges() {
    use incular_navigation::{Navigator, Page};
    use incular_runtime::RouteOutlet;

    let a = FocusNode::new();
    let b = FocusNode::new();
    let navigator = Navigator::new();
    let mut runtime = Runtime::new(
        incular_widgets::Column::new([SizedBox::new().width(800.).height(600.)]).into(),
    )
    .unwrap();
    let parent = runtime.spawner().scope();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(&navigator, &parent)));
    let root = runtime.tree().root().unwrap();
    runtime
        .register_builder(root, {
            let outlet = outlet.clone();
            move || {
                incular_widgets::Column::new([SizedBox::from_dimensions(
                    Some(800.),
                    Some(600.),
                    Some(Directionality::new(
                        TextDirection::Ltr,
                        outlet.borrow().widget(),
                    )),
                )])
                .into()
            }
        })
        .unwrap();
    frame(&mut runtime);
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    navigator.push_page(Page::new("home", tab_targets(&a, &b)));
    let layout = Constraints::tight(Size::new(800., 600.));
    RouteOutlet::present_frame(&outlet, &mut runtime, layout).unwrap();

    a.request_focus();
    frame(&mut runtime);
    check_tab_focus(&a, &b, Some(0));
    #[cfg(not(target_arch = "wasm32"))]
    for (reverse, expected) in [(false, 1), (true, 0), (false, 1), (false, 0), (true, 1)] {
        assert!(if reverse {
            runtime.previous_focus()
        } else {
            runtime.next_focus()
        });
        check_tab_focus(&a, &b, Some(expected));
    }
    #[cfg(target_arch = "wasm32")]
    {
        assert!(runtime.next_focus());
        check_tab_focus(&a, &b, Some(1));
        assert!(!runtime.next_focus());
        check_tab_focus(&a, &b, None);
        a.request_focus();
        frame(&mut runtime);
        assert!(!runtime.previous_focus());
        check_tab_focus(&a, &b, None);
    }
}

#[test]
fn sequential_parent_scope_crossings_reenter_at_the_approached_edge() {
    use incular_widgets::Column;

    let nodes = std::array::from_fn::<_, 4, _>(|_| FocusNode::new());
    let inner = FocusScopeNode::new();
    inner.set_traversal_edge_behavior(Edge::ParentScope);
    let control = |index: usize| {
        Focus::new(SizedBox::new().width(100.).height(100.)).node(nodes[index].clone())
    };
    let scene = FocusScope::new(Column::new([
        Widget::from(control(0)),
        FocusScope::new(tab_targets(&nodes[1], &nodes[2]))
            .node(inner)
            .into(),
        control(3).into(),
    ]));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    nodes[0].request_focus();
    frame(&mut runtime);
    for (reverse, expected) in [
        (false, 1),
        (false, 2),
        (false, 3),
        (true, 2),
        (true, 1),
        (true, 0),
        (false, 1),
    ] {
        assert!(if reverse {
            runtime.previous_focus()
        } else {
            runtime.next_focus()
        });
        for (index, node) in nodes.iter().enumerate() {
            assert_eq!(
                node.has_primary_focus(),
                index == expected,
                "expected {expected}, got {index}"
            );
        }
    }
}
