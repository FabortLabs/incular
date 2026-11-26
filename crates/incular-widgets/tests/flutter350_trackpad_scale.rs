use std::{cell::RefCell, f32::consts, rc::Rc, time::Instant};

use incular_config::Constraints;
use incular_core::{Color, DragStartBehavior, Offset, PointerPhase, Size};
use incular_gestures::{
    PointerDeviceKind, PointerEvent, PointerPanZoomEvent, PointerPanZoomPhase, ScaleEndDetails,
    ScaleStartDetails, ScaleUpdateDetails,
};
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

#[derive(Default)]
struct Trace {
    starts: Vec<ScaleStartDetails>,
    updates: Vec<ScaleUpdateDetails>,
    ends: Vec<ScaleEndDetails>,
    drags: usize,
}

fn mount(tree: &mut WidgetTree, trace: &Rc<RefCell<Trace>>) {
    macro_rules! record {
        ($field:ident) => {{
            let trace = trace.clone();
            move |details| trace.borrow_mut().$field.push(details)
        }};
    }
    let scale = GestureDetector::new(Widget::box_(Size::new(120., 120.), Color::BLACK))
        .drag_start_behavior(DragStartBehavior::Down)
        .on_scale_start(record!(starts))
        .on_scale_update(record!(updates))
        .on_scale_end(record!(ends));
    let drag = GestureDetector::new(scale).on_horizontal_drag_start({
        let trace = trace.clone();
        move |_| trace.borrow_mut().drags += 1
    });
    tree.mount(drag.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(120., 120.)))
        .unwrap();
}

#[test]
fn mounted_touch_and_trackpad_share_scale_and_contend_with_drag() {
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut tree = WidgetTree::new();
    mount(&mut tree, &trace);
    let now = Instant::now();
    let pan_zoom = |tree: &mut WidgetTree, position, phase| {
        tree.dispatch_pointer_pan_zoom_in_window(
            0,
            PointerPanZoomEvent {
                device: 1,
                pointer: 2,
                position,
                phase,
                time: now,
                synthesized: false,
            },
        )
    };
    let touch = |tree: &mut WidgetTree, pointer, position, phase| {
        tree.dispatch_pointer_gesture_in_window(
            0,
            1,
            PointerDeviceKind::Touch,
            PointerEvent {
                pointer,
                position,
                phase,
                time: now,
            },
        )
    };
    let update = |pan, scale, rotation| PointerPanZoomPhase::Update {
        pan,
        scale,
        rotation,
    };
    let forty = Offset::new(40., 40.);
    assert!(pan_zoom(&mut tree, Offset::ZERO, PointerPanZoomPhase::Start).is_some());
    assert!(trace.borrow().starts.is_empty());
    pan_zoom(&mut tree, Offset::ZERO, update(forty, 1., 0.));
    {
        let t = trace.borrow();
        assert_eq!(t.starts.len(), 1);
        assert_eq!(
            (t.updates[0].focal_point, t.updates[0].pointer_count),
            (forty, 2)
        );
        assert_eq!(t.updates[0].focal_point_delta, forty);
    }

    // Intentionally reuse the same numeric IDs across the protocols.
    touch(&mut tree, 2, forty, PointerPhase::Down);
    assert_eq!(trace.borrow().ends.len(), 1);
    touch(&mut tree, 2, Offset::new(10., 10.), PointerPhase::Move);
    {
        let t = trace.borrow();
        assert_eq!(t.starts.len(), 2);
        let d = t.updates[1];
        assert_eq!((d.scale, d.pointer_count), (1., 3));
        assert_eq!(d.focal_point, Offset::new(25., 25.));
        assert_eq!(d.focal_point_delta, Offset::new(-15., -15.));
    }
    touch(&mut tree, 3, Offset::new(10., 40.), PointerPhase::Down);
    assert_eq!(trace.borrow().ends.len(), 2);
    touch(&mut tree, 3, forty, PointerPhase::Move);
    {
        let t = trace.borrow();
        assert_eq!(t.starts.len(), 3);
        let d = t.updates[2];
        assert_eq!(d.focal_point, Offset::new(30., 30.));
        assert_eq!(d.focal_point_delta, Offset::new(10., 0.));
        assert!((d.scale - consts::SQRT_2).abs() < 0.0001);
        assert_eq!((d.horizontal_scale, d.vertical_scale), (1., 1.));
        assert!((d.rotation + consts::FRAC_PI_4).abs() < 0.0001);
        assert_eq!(d.pointer_count, 4);
    }
    for (position, pan, focal, delta) in [
        (Offset::ZERO, forty, Offset::new(30., 30.), Offset::ZERO),
        (
            Offset::new(15., 15.),
            Offset::new(55., 55.),
            forty,
            Offset::new(10., 10.),
        ),
    ] {
        pan_zoom(
            &mut tree,
            position,
            update(pan, consts::SQRT_2, consts::FRAC_PI_3),
        );
        let t = trace.borrow();
        let d = *t.updates.last().unwrap();
        assert_eq!((d.focal_point, d.focal_point_delta), (focal, delta));
        assert!((d.scale - 2.).abs() < 0.0001);
        assert!((d.horizontal_scale - consts::SQRT_2).abs() < 0.0001);
        assert!((d.vertical_scale - consts::SQRT_2).abs() < 0.0001);
        assert!((d.rotation - consts::PI / 12.).abs() < 0.0001);
        assert_eq!(d.pointer_count, 4);
    }
    pan_zoom(&mut tree, Offset::ZERO, PointerPanZoomPhase::End);
    assert_eq!(trace.borrow().ends.len(), 3);
    touch(&mut tree, 2, Offset::new(10., 10.), PointerPhase::Up);
    touch(&mut tree, 3, forty, PointerPhase::Up);
    let t = trace.borrow();
    assert_eq!(
        (t.starts.len(), t.updates.len(), t.ends.len(), t.drags),
        (3, 5, 3, 0)
    );
}

#[test]
fn teardown_clears_absolute_streams_and_does_not_capture_other_windows() {
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut tree = WidgetTree::new();
    mount(&mut tree, &trace);
    let mut event = PointerPanZoomEvent {
        device: 1,
        pointer: 2,
        position: Offset::new(10., 10.),
        phase: PointerPanZoomPhase::Start,
        time: Instant::now(),
        synthesized: false,
    };
    assert!(tree.dispatch_pointer_pan_zoom_in_window(2, event).is_some());
    event.phase = PointerPanZoomPhase::Update {
        pan: Offset::ZERO,
        scale: 2.,
        rotation: 0.,
    };
    assert!(tree.dispatch_pointer_pan_zoom_in_window(3, event).is_none());
    assert!(trace.borrow().starts.is_empty());
    tree.cancel_all_gesture_streams();
    assert!(tree.dispatch_pointer_pan_zoom_in_window(2, event).is_none());
    assert!(trace.borrow().starts.is_empty());
    event.phase = PointerPanZoomPhase::Start;
    assert!(tree.dispatch_pointer_pan_zoom_in_window(2, event).is_some());
    event.phase = PointerPanZoomPhase::Update {
        pan: Offset::ZERO,
        scale: 2.,
        rotation: 0.,
    };
    assert!(tree.dispatch_pointer_pan_zoom_in_window(2, event).is_some());
    assert_eq!(trace.borrow().starts.len(), 1);
}
