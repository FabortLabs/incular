use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use accesskit::{Action, ActionRequest, NodeId, TreeId};
use incular_accessibility::AccessKitProjection;
use incular_config::Constraints;
use incular_core::{Color, Offset, PointerDeviceKind, Size, Transform as Affine};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNodeId};
use incular_widgets::{DragUpdateDetails, GestureDetector, Transform, Widget};

const VIEW: Size = Size {
    width: 200.,
    height: 100.,
};

fn frame(runtime: &mut Runtime) {
    runtime.run_frame(Constraints::loose(VIEW)).unwrap();
}

fn tap_node(runtime: &Runtime) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| {
            node.actions
                .contains(&SemanticActionKind::Activate)
                .then_some(id)
        })
        .expect("live gesture semantics node")
}

#[test]
fn a_horizontal_gesture_advertises_both_physical_scroll_actions_and_tap() {
    let child = Widget::box_(VIEW, Color::WHITE);
    let mut runtime = Runtime::new(
        GestureDetector::new(child)
            .on_tap(|| {})
            .on_horizontal_drag_update(|_| {})
            .into(),
    )
    .unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = tap_node(&runtime);
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().actions,
        [
            SemanticActionKind::Activate,
            SemanticActionKind::ScrollLeft,
            SemanticActionKind::ScrollRight,
        ]
    );

    runtime
        .schedule_update(
            root,
            GestureDetector::new(Widget::box_(VIEW, Color::WHITE))
                .on_tap(|| {})
                .on_horizontal_drag_update(|_| {})
                .valid_semantic_actions([
                    SemanticActionKind::Activate,
                    SemanticActionKind::ScrollLeft,
                ])
                .into(),
        )
        .unwrap();
    frame(&mut runtime);
    assert_eq!(tap_node(&runtime), id);
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().actions,
        [SemanticActionKind::Activate, SemanticActionKind::ScrollLeft]
    );
}

#[test]
fn projected_directions_dispatch_exact_deltas_at_the_transformed_center() {
    let updates = Rc::new(RefCell::new(Vec::<DragUpdateDetails>::new()));
    let taps = Rc::new(Cell::new(0));
    let build = |allowed, factor| {
        let mut gesture = GestureDetector::new(Widget::box_(VIEW, Color::WHITE))
            .on_tap({
                let taps = taps.clone();
                move || taps.set(taps.get() + 1)
            })
            .on_long_press(|| {})
            .on_horizontal_drag_update({
                let updates = updates.clone();
                move |details| updates.borrow_mut().push(details)
            })
            .on_vertical_drag_update({
                let updates = updates.clone();
                move |details| updates.borrow_mut().push(details)
            });
        if allowed {
            gesture = gesture.valid_semantic_actions([
                SemanticActionKind::Activate,
                SemanticActionKind::ScrollLeft,
                SemanticActionKind::ScrollDown,
            ]);
        }
        if let Some(factor) = factor {
            gesture = gesture.semantic_scroll_factor(factor);
        }
        Transform::new(Affine::translation(Offset::new(20., 30.)), gesture).into()
    };

    let mut runtime = Runtime::new(build(false, None)).unwrap();
    frame(&mut runtime);
    let id = tap_node(&runtime);
    let mut projection = AccessKitProjection::new();
    projection.sync(runtime.tree().semantics(), 1.0).unwrap();
    let native_id = NodeId(projection.native_node_id(id).unwrap());
    let request = |action| ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: native_id,
        data: None,
    };
    let mut stale_right = None;
    for (native, portable) in [
        (Action::ScrollLeft, SemanticAction::ScrollLeft),
        (Action::ScrollRight, SemanticAction::ScrollRight),
        (Action::ScrollUp, SemanticAction::ScrollUp),
        (Action::ScrollDown, SemanticAction::ScrollDown),
    ] {
        let mapped = projection.translate_action(&request(native)).unwrap();
        assert_eq!(mapped.node, id);
        assert_eq!(mapped.action, portable);
        if native == Action::ScrollRight {
            stale_right = Some(mapped.action.clone());
        }
        assert!(runtime.dispatch_semantic_action(id, mapped.action));
    }

    assert_eq!(
        updates
            .borrow()
            .iter()
            .map(|details| details.delta)
            .collect::<Vec<_>>(),
        [
            Offset::new(-160., 0.),
            Offset::new(160., 0.),
            Offset::new(0., -80.),
            Offset::new(0., 80.),
        ]
    );
    for (index, details) in updates.borrow().iter().enumerate() {
        assert_eq!(details.source_time, None);
        assert_eq!(details.kind, PointerDeviceKind::Unknown);
        assert_eq!(details.global_position, Offset::new(120., 80.));
        assert_eq!(details.local_position, details.global_position);
        assert_eq!(
            details.primary_delta,
            Some(if index < 2 {
                details.delta.x
            } else {
                details.delta.y
            })
        );
    }

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(true, Some(0.5)))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(tap_node(&runtime), id);
    assert!(!runtime.dispatch_semantic_action(id, stale_right.unwrap()));
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::ScrollUp));
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
    projection.sync(runtime.tree().semantics(), 1.0).unwrap();
    assert!(
        projection
            .translate_action(&request(Action::ScrollRight))
            .is_none()
    );
    assert!(
        projection
            .translate_action(&request(Action::ScrollUp))
            .is_none()
    );
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::ScrollLeft));
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::ScrollDown));
    assert_eq!(taps.get(), 1);
    assert_eq!(updates.borrow()[4].delta, Offset::new(-100., 0.));
    assert_eq!(updates.borrow()[5].delta, Offset::new(0., 50.));
}

#[test]
fn excluding_gesture_semantics_does_not_disable_a_real_pointer_tap() {
    use incular_core::{InputEvent, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata};

    let taps = Rc::new(Cell::new(0));
    let gesture = GestureDetector::new(Widget::box_(VIEW, Color::WHITE))
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        })
        .on_horizontal_drag_update(|_| {})
        .exclude_from_semantics(true);
    let mut runtime = Runtime::new(gesture.into()).unwrap();
    frame(&mut runtime);
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .all(|(_, node)| !node.actions.iter().any(|action| {
                *action == SemanticActionKind::Activate || action.is_directional_scroll()
            }))
    );
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 0,
            kind: PointerDeviceKind::Touch,
            buttons: if phase == PointerPhase::Down {
                PRIMARY_POINTER_BUTTON
            } else {
                0
            },
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::EMPTY,
            phase,
            position: Offset::new(100., 50.),
        });
    }
    assert_eq!(taps.get(), 1);
}

#[test]
fn drag_only_nodes_are_accessible_until_a_pointer_blocker_takes_effect() {
    let build = |ignoring| {
        incular_widgets::IgnorePointer::new(
            GestureDetector::new(Widget::box_(VIEW, Color::WHITE))
                .on_horizontal_drag_update(|_| {}),
        )
        .ignoring(ignoring)
        .into()
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    frame(&mut runtime);
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| {
            node.actions
                .contains(&SemanticActionKind::ScrollLeft)
                .then_some(id)
        })
        .unwrap();
    assert_eq!(
        runtime.tree().semantics().node(id).unwrap().actions,
        [
            SemanticActionKind::ScrollLeft,
            SemanticActionKind::ScrollRight
        ]
    );
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::ScrollLeft));
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(true))
        .unwrap();
    frame(&mut runtime);
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::ScrollLeft));
    assert!(runtime.tree().semantics().iter().all(|(_, node)| {
        !node
            .actions
            .iter()
            .any(|action| action.is_directional_scroll())
    }));
}
