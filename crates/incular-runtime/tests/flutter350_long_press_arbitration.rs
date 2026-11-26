use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, SizedBox};

#[test]
fn a_pending_tap_reports_down_before_the_competing_hold_is_accepted() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |value| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(value)
    };
    let down = record("down");
    let tap_down = record("tap_down");
    let long_press = GestureDetector::new(SizedBox::new().width(100.).height(100.))
        .behavior(HitTestBehavior::Opaque)
        .on_long_press_down(move |_| down())
        .on_long_press(record("start"));
    let tap = GestureDetector::new(long_press)
        .behavior(HitTestBehavior::Opaque)
        .on_tap_down(move |_| tap_down());
    let mut runtime = Runtime::new(tap.into()).unwrap();
    let size = Constraints::tight(Size::new(100., 100.));
    let at = Instant::now();
    runtime.run_frame_at(size, at).unwrap();
    assert!(trace.borrow().is_empty());

    runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 5,
            device: 1,
            kind: PointerDeviceKind::Touch,
            button: Some(PRIMARY_POINTER_BUTTON),
            phase: PointerPhase::Down,
            buttons: PRIMARY_POINTER_BUTTON,
            position: Offset::new(10., 10.),
            sample: PointerSampleMetadata::EMPTY,
        },
        at,
    );
    assert_eq!(trace.borrow().as_slice(), ["down"]);
    runtime
        .run_frame_at(size, at + Duration::from_millis(300))
        .unwrap();
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down"]);
    runtime
        .run_frame_at(size, at + Duration::from_secs(1))
        .unwrap();
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down", "start"]);
    drop(runtime);
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down", "start"]);
}
