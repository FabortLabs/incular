use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    GestureDetector, HitTestBehavior, Listener, PointerDeviceKind, RawPointerEvent, SizedBox,
};

#[test]
fn mixed_motion_keeps_the_visual_focal_point_but_samples_only_real_velocity() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let focal_points = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let detector = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .on_scale_start(|_| {})
        .on_scale_update({
            let focal_points = focal_points.clone();
            move |details| focal_points.borrow_mut().push(details.focal_point)
        })
        .on_scale_end({
            let ends = ends.clone();
            move |details| ends.borrow_mut().push(details.velocity.pixels_per_second)
        });
    let root = Listener::new(detector).on_pointer_move({
        let events = events.clone();
        move |event| events.borrow_mut().push(event.synthesized)
    });
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(300.0, 150.0)))
        .unwrap();
    let origin = Offset::new(100.0, 100.0);
    let start = Instant::now();
    let event = |phase, at_ms, relative_x, synthesized| RawPointerEvent {
        synthesized,
        ..RawPointerEvent::new(
            1,
            7,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            origin + Offset::new(relative_x, 0.0),
            phase,
            start + Duration::from_millis(at_ms),
        )
        .with_button(match phase {
            PointerPhase::Down | PointerPhase::Up => Some(PRIMARY_POINTER_BUTTON),
            _ => None,
        })
    };
    let _ = runtime.handle_raw_pointer_event(event(PointerPhase::Down, 0, 0.0, false));
    for (index, (x, synthesized)) in [
        (20.0, false),
        (145.0, true),
        (60.0, false),
        (-20.0, true),
        (100.0, false),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = runtime.handle_raw_pointer_event(event(
            PointerPhase::Move,
            (index as u64 + 1) * 10,
            x,
            synthesized,
        ));
    }
    let _ = runtime.handle_raw_pointer_event(event(PointerPhase::Up, 55, 100.0, false));

    assert_eq!(*events.borrow(), [false, true, false, true, false]);
    let focal_points = focal_points.borrow();
    assert!(focal_points.contains(&Offset::new(245.0, 100.0)));
    assert!(focal_points.contains(&Offset::new(80.0, 100.0)));
    assert_eq!(focal_points.last(), Some(&Offset::new(200.0, 100.0)));
    let release = ends.borrow();
    let [velocity] = release.as_slice() else {
        panic!("one scale release: {release:?}");
    };
    assert!((velocity.x - 2000.0).abs() < 0.01, "{velocity:?}");
    assert!(velocity.y.abs() < 0.01, "{velocity:?}");
}
