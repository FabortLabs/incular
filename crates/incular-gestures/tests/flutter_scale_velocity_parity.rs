//! Flutter 3.50.0 paired gesture audit at ab2912035e.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_core::{Offset, PointerPhase};
use incular_gestures::{PointerDeviceKind, RawPointerEvent, ScaleEndDetails, ScaleGestureDetector};

fn scale_end_velocity(synthesized_moves: bool) -> f32 {
    let ends = Rc::new(RefCell::new(Vec::new()));
    let mut scale = ScaleGestureDetector::with_callbacks(
        None,
        Some(Rc::new(|_| {})),
        Some({
            let ends = Rc::clone(&ends);
            Rc::new(move |details: ScaleEndDetails| {
                ends.borrow_mut().push(details.velocity.pixels_per_second.x);
            })
        }),
    );
    let base = Instant::now();
    let event = |phase, x, millis, synthesized| RawPointerEvent {
        synthesized,
        ..RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            0,
            Offset::new(x, 0.0),
            phase,
            base + Duration::from_millis(millis),
        )
    };
    scale.handle_raw(event(PointerPhase::Down, 0.0, 0, false));
    for sample in 1..=4 {
        scale.handle_raw(event(
            PointerPhase::Move,
            20.0 * sample as f32,
            10 * sample,
            synthesized_moves,
        ));
    }
    scale.handle_raw(event(PointerPhase::Up, 80.0, 45, false));

    let ends = ends.borrow();
    assert_eq!(ends.len(), 1, "each sequence completes one scale gesture");
    ends[0]
}

// C10: Flutter scale accepts synthesized moves for geometry, but not velocity.
#[test]
fn synthesized_touch_moves_do_not_contribute_to_scale_end_velocity() {
    assert!(scale_end_velocity(false) > 1000.0, "ordinary moves fling");
    assert_eq!(scale_end_velocity(true), 0.0);
}
