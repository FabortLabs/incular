use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    GestureDetector, HitTestBehavior, TapCallbackKind, Widget, set_tap_callback_error_handler,
};

#[test]
fn a_deferred_single_tap_reports_callback_failure_without_aborting_the_frame() {
    let failures = Rc::new(RefCell::new(Vec::new()));
    let previous = set_tap_callback_error_handler(Some(Rc::new({
        let failures = failures.clone();
        move |failure| {
            failures
                .borrow_mut()
                .push((failure.callback, failure.message().map(str::to_owned)));
        }
    })));
    let size = Size::new(60., 60.);
    let widget = GestureDetector::new(Widget::box_(size, Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap(|| panic!("retained tap failed"))
        .on_double_tap(|| {});
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let now = Instant::now();
    let bounds = Constraints::tight(size);
    runtime.run_frame_at(bounds, now).unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(incular_core::InputEvent::Pointer {
            phase,
            position: Offset::new(20., 20.),
        });
    }
    assert!(
        failures.borrow().is_empty(),
        "the single tap still awaits the double-tap deadline"
    );
    runtime
        .run_frame_at(bounds, now + Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        *failures.borrow(),
        [(TapCallbackKind::Tap, Some("retained tap failed".into()))]
    );
    runtime
        .run_frame_at(bounds, now + Duration::from_secs(2))
        .unwrap();
    assert_eq!(failures.borrow().len(), 1);
    set_tap_callback_error_handler(previous);
}
