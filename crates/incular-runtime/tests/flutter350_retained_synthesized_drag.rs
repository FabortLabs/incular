use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    DragEndDetails, GestureDetector, HitTestBehavior, PointerDeviceKind, RawPointerEvent, SizedBox,
};

fn horizontal_drag(samples: &[(u64, f32, f32, bool)], release_ms: u64) -> DragEndDetails {
    let ends = Rc::new(RefCell::new(Vec::new()));
    let root = GestureDetector::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .drag_start_behavior(DragStartBehavior::Down)
        .on_horizontal_drag_end({
            let ends = ends.clone();
            move |details| ends.borrow_mut().push(details)
        });
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(120.0, 75.0)))
        .unwrap();
    let t0 = Instant::now();
    let mut send = |phase, at, x, y, synthesized| {
        let event = RawPointerEvent {
            synthesized,
            ..RawPointerEvent::new(
                1,
                1,
                PointerDeviceKind::Touch,
                if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                Offset::new(x, y),
                phase,
                t0 + Duration::from_millis(at),
            )
            .with_button(match phase {
                PointerPhase::Down | PointerPhase::Up => Some(PRIMARY_POINTER_BUTTON),
                _ => None,
            })
        };
        let _ = runtime.handle_raw_pointer_event(event);
    };
    for (index, &(at, x, y, synthesized)) in samples.iter().enumerate() {
        let phase = if index == 0 {
            PointerPhase::Down
        } else {
            PointerPhase::Move
        };
        send(phase, at, x, y, synthesized);
    }
    // Dart's synthesized event is routed independently of TestPointer and
    // does not replace its remembered final real position.
    let (_, x, y, _) = samples
        .iter()
        .rev()
        .find(|sample| !sample.3)
        .copied()
        .unwrap();
    send(PointerPhase::Up, release_ms, x, y, false);
    let ends = ends.borrow();
    let [end] = ends.as_slice() else {
        panic!("expected one end, got {ends:?}");
    };
    *end
}

#[test]
fn horizontal_release_estimate_ignores_the_synthesized_wobble() {
    let end = horizontal_drag(
        &[
            (10, 10.0, 25.0, false),
            (20, 20.0, 25.0, false),
            (30, 30.0, 25.0, false),
            (40, 40.0, 25.0, false),
            (50, 50.0, 25.0, false),
            (60, 51.0, 26.0, true),
        ],
        70,
    );
    assert!(
        (end.velocity.pixels_per_second.x - 1000.0).abs() < 0.001,
        "{end:?}"
    );
    assert!(end.velocity.pixels_per_second.y.abs() < 0.001, "{end:?}");
}

#[test]
fn horizontal_quick_flick_uses_down_plus_two_real_moves() {
    let end = horizontal_drag(
        &[
            (10, 10.0, 25.0, false),
            (20, 20.0, 25.0, false),
            (30, 30.0, 25.0, false),
        ],
        40,
    );
    assert!(
        (end.velocity.pixels_per_second.x - 1000.0).abs() < 0.001,
        "{end:?}"
    );
}
