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
