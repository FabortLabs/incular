use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, GestureDetector, HitTestBehavior, RawPointerEvent, SizedBox, Transform,
};

#[test]
fn transformed_scale_reports_world_focal_point_and_local_delta() {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let target = GestureDetector::new(SizedBox::new().width(100.).height(100.))
        .behavior(HitTestBehavior::Opaque)
        .on_scale_start({
            let starts = starts.clone();
            move |d| starts.borrow_mut().push(d)
        })
        .on_scale_update({
            let updates = updates.clone();
            move |d| updates.borrow_mut().push(d)
        });
    let scene = Center::new(Transform::scale(2., target));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();
    let mut deliver = |phase, x, y, ms| {
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            1,
            2,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
    };
    deliver(PointerPhase::Down, 40., 40., 1);
    assert!(updates.borrow().is_empty());
    let start = starts.borrow()[0];
    assert_eq!(start.focal_point, Offset::new(40., 40.));
    assert_eq!(start.local_focal_point, Offset::new(20., 20.));

    deliver(PointerPhase::Move, 80., 60., 2);
    let update = updates.borrow()[0];
    assert_eq!(update.focal_point, Offset::new(80., 60.));
    assert_eq!(update.local_focal_point, Offset::new(40., 30.));
    assert_eq!(update.focal_point_delta, Offset::new(20., 10.));
    deliver(PointerPhase::Up, 80., 60., 3);
}
