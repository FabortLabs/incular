use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as Affine};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNodeId};
use incular_widgets::{GestureDetector, Transform, Widget};

const VIEW: Size = Size {
    width: 200.,
    height: 100.,
};

fn fixture(gesture: GestureDetector) -> Runtime {
    let widget = Transform::new(Affine::translation(Offset::new(20., 30.)), gesture);
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.run_frame(Constraints::loose(VIEW)).unwrap();
    runtime
}

fn scroll_node(runtime: &Runtime) -> SemanticNodeId {
    runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| {
            node.actions
                .iter()
                .any(|action| action.is_directional_scroll())
                .then_some(id)
        })
        .expect("semantic drag node")
}

#[test]
fn start_only_detectors_advertise_and_invoke_only_their_axes() {
    for (horizontal, accepted, rejected) in [
        (
            false,
            [SemanticAction::ScrollUp, SemanticAction::ScrollDown],
            [SemanticAction::ScrollLeft, SemanticAction::ScrollRight],
        ),
        (
            true,
            [SemanticAction::ScrollLeft, SemanticAction::ScrollRight],
            [SemanticAction::ScrollUp, SemanticAction::ScrollDown],
        ),
    ] {
        let calls = Rc::new(Cell::new(0));
        let mark = {
            let calls = calls.clone();
            move |details: incular_widgets::DragStartDetails| {
                assert_eq!(details.local_position, Offset::new(100., 50.));
                assert_eq!(details.global_position, Offset::new(120., 80.));
                assert!(details.source_time.is_none());
                assert_eq!(details.kind, None);
                calls.set(calls.get() + 1);
            }
        };
        let gesture = GestureDetector::new(Widget::box_(VIEW, Color::WHITE));
        let gesture = if horizontal {
            gesture.on_horizontal_drag_start(mark)
        } else {
            gesture.on_vertical_drag_start(mark)
        };
        let mut runtime = fixture(gesture);
        let node = scroll_node(&runtime);
        let expected = accepted.each_ref().map(SemanticAction::kind);
        assert_eq!(
            runtime.tree().semantics().node(node).unwrap().actions,
            expected
        );
        for action in rejected {
            assert!(!runtime.dispatch_semantic_action(node, action));
        }
        assert_eq!(calls.get(), 0);
        for (index, action) in accepted.into_iter().enumerate() {
            assert!(runtime.dispatch_semantic_action(node, action));
            assert_eq!(calls.get(), index + 1);
        }
    }
}

#[test]
fn a_semantic_scroll_invokes_axis_and_pan_recognizers_with_all_phases() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |phase| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(phase)
    };
    let down = |phase| {
        let record = record(phase);
        move |details: incular_widgets::DragDownDetails| {
            assert_eq!(details.local_position, Offset::new(100., 50.));
            assert_eq!(details.global_position, Offset::new(120., 80.));
            record();
        }
    };
    let start = |phase| {
        let record = record(phase);
        move |details: incular_widgets::DragStartDetails| {
            assert_eq!(details.local_position, Offset::new(100., 50.));
            assert_eq!(details.global_position, Offset::new(120., 80.));
            record();
        }
    };
    let update = |phase| {
        let record = record(phase);
        move |details: incular_widgets::DragUpdateDetails| {
            assert_eq!(details.delta, Offset::new(-160., 0.));
            assert_eq!(details.primary_delta, Some(-160.));
            assert_eq!(details.local_position, Offset::new(120., 80.));
            record();
        }
    };
    let end = |phase, velocity| {
        let record = record(phase);
        move |details: incular_widgets::DragEndDetails| {
            assert_eq!(details.primary_velocity, velocity);
            assert_eq!(details.velocity.pixels_per_second, Offset::ZERO);
            assert_eq!(details.local_position, Offset::new(-60., 50.));
            assert_eq!(details.global_position, Offset::new(-40., 80.));
            record();
        }
    };
    let mut runtime = fixture(
        GestureDetector::new(Widget::box_(VIEW, Color::WHITE))
            .on_horizontal_drag_down(down("horizontal.down"))
            .on_horizontal_drag_start(start("horizontal.start"))
            .on_horizontal_drag_update(update("horizontal.update"))
            .on_horizontal_drag_end(end("horizontal.end", Some(0.)))
            .on_pan_down(down("pan.down"))
            .on_pan_start(start("pan.start"))
            .on_pan_update(update("pan.update"))
            .on_pan_end(end("pan.end", None)),
    );
    let node = scroll_node(&runtime);
    assert!(
        runtime
            .tree()
            .semantics()
            .node(node)
            .unwrap()
            .actions
            .contains(&SemanticActionKind::ScrollLeft)
    );
    assert!(runtime.dispatch_semantic_action(node, SemanticAction::ScrollLeft));
    assert_eq!(
        *trace.borrow(),
        [
            "horizontal.down",
            "horizontal.start",
            "horizontal.update",
            "horizontal.end",
            "pan.down",
            "pan.start",
            "pan.update",
            "pan.end",
        ]
    );
}
