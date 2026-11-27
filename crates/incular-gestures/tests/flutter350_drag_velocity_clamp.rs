use std::{cell::Cell, rc::Rc, time::Duration};

use incular_core::{DragStartBehavior, Offset, PointerPhase, time::Instant};
use incular_gestures::{
    DragEndDetails, GestureCallbacks, GestureDecision, GestureRecognizer, PointerDeviceKind,
    PointerGestureRecognizer, RawPointerEvent,
};

#[derive(Clone, Copy)]
enum Axis {
    Pan,
    Horizontal,
    Vertical,
}

struct Fixture {
    recognizer: PointerGestureRecognizer,
    end: Rc<Cell<Option<DragEndDetails>>>,
}

impl Fixture {
    fn new(axis: Axis) -> Self {
        Self::with_start(axis, DragStartBehavior::Start)
    }

    fn with_start(axis: Axis, drag_start_behavior: DragStartBehavior) -> Self {
        let end = Rc::new(Cell::new(None));
        let output = Rc::clone(&end);
        let on_end =
            Some(Rc::new(move |details| output.set(Some(details))) as Rc<dyn Fn(DragEndDetails)>);
        let mut callbacks = GestureCallbacks {
            drag_start_behavior,
            ..GestureCallbacks::default()
        };
        match axis {
            Axis::Pan => callbacks.on_pan_end = on_end,
            Axis::Horizontal => callbacks.on_horizontal_drag_end = on_end,
            Axis::Vertical => callbacks.on_vertical_drag_end = on_end,
        };
        Self {
            recognizer: PointerGestureRecognizer::new(callbacks),
            end,
        }
    }

    fn sample(&mut self, pointer: u64, phase: PointerPhase, position: Offset, time: Instant) {
        let decision = self.recognizer.observe_raw(RawPointerEvent::new(
            pointer,
            1,
            PointerDeviceKind::Touch,
            u32::from(phase != PointerPhase::Up),
            position,
            phase,
            time,
        ));
        if let GestureDecision::Accept(action) = decision {
            self.recognizer.dispatch(action);
        }
    }

    fn sole_down(&mut self, pointer: u64, position: Offset, time: Instant) {
        self.sample(pointer, PointerPhase::Down, position, time);
        if let Some(action) = self.recognizer.accept_on_press() {
            self.recognizer.dispatch(action);
        }
    }

    fn drag_to_end(&mut self, base: Instant, velocity: Offset) -> DragEndDetails {
        const POINTER: u64 = 1;
        self.sole_down(POINTER, Offset::ZERO, base);
        for (factor, millis) in [(0.025, 25), (0.05, 50)] {
            self.sample(
                POINTER,
                PointerPhase::Move,
                Offset::new(velocity.x * factor, velocity.y * factor),
                base + Duration::from_millis(millis),
            );
        }
        self.sample(
            POINTER,
            PointerPhase::Up,
            Offset::new(velocity.x * 0.05, velocity.y * 0.05),
            base + Duration::from_millis(50),
        );
        self.end.take().expect("sole recognizer ends")
    }
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 0.1, "{actual} != {expected}");
}

#[test]
fn saturates_a_fast_horizontal_release_with_duplicate_first_timestamp() {
    let mut test = Fixture::with_start(Axis::Horizontal, DragStartBehavior::Down);
    let start = Instant::now();
    let time = |millis| start + Duration::from_millis(millis);
    test.sole_down(5, Offset::new(10., 25.), time(10));
    for millis in 10..=20 {
        test.sample(
            5,
            PointerPhase::Move,
            Offset::new(20. + 10. * (millis - 10) as f32, 25.),
            time(millis),
        );
    }
    test.sample(5, PointerPhase::Up, Offset::new(120., 25.), time(20));
    let end = test.end.take().unwrap();
    let velocity = end.velocity.pixels_per_second;
    assert!((0.99 * 8000. ..=8000.).contains(&velocity.x));
    assert_eq!(velocity.y, 0.);
    assert_eq!(end.primary_velocity, Some(velocity.x));
}

#[test]
fn pan_clamps_magnitude_without_rotating_the_velocity() {
    let mut test = Fixture::new(Axis::Pan);
    let start = Instant::now();
    for (index, (input, expected)) in [
        ([400., 400.], [400., 400.]),
        ([2000., -2000.], [2000., -2000.]),
        ([-8000., -8000.], [-5656.9, -5656.9]),
        ([-8000., 6000.], [-6400., 4800.]),
        ([-9000., 0.], [-8000., 0.]),
        ([-9000., -1000.], [-7951.1, -883.5]),
        ([-1000., 9000.], [-883.5, 7951.1]),
        ([0., 9000.], [0., 8000.]),
    ]
    .into_iter()
    .enumerate()
    {
        let end = test.drag_to_end(
            start + Duration::from_millis(index as u64 * 100),
            Offset::new(input[0], input[1]),
        );
        assert_eq!(end.primary_velocity, None);
        close(end.velocity.pixels_per_second.x, expected[0]);
        close(end.velocity.pixels_per_second.y, expected[1]);
    }
}

#[test]
fn axis_drags_drop_the_perpendicular_velocity_before_clamping() {
    let start = Instant::now();
    for (axis, cases) in [
        (
            Axis::Vertical,
            [
                ([500., 400.], 400.),
                ([3000., -2000.], -2000.),
                ([-9000., -9000.], -8000.),
                ([-9000., 0.], 0.),
                ([-9000., 1000.], 1000.),
                ([-1000., -9000.], -8000.),
                ([0., -9000.], -8000.),
            ],
        ),
        (
            Axis::Horizontal,
            [
                ([500., 400.], 500.),
                ([3000., -2000.], 3000.),
                ([-9000., -9000.], -8000.),
                ([-9000., 0.], -8000.),
                ([-9000., 1000.], -8000.),
                ([-1000., -9000.], -1000.),
                ([0., -9000.], 0.),
            ],
        ),
    ] {
        let mut test = Fixture::new(axis);
        for (index, (input, expected)) in cases.into_iter().enumerate() {
            let end = test.drag_to_end(
                start + Duration::from_millis(index as u64 * 100),
                Offset::new(input[0], input[1]),
            );
            let primary = end.primary_velocity.unwrap();
            close(primary, expected);
            let velocity = end.velocity.pixels_per_second;
            match axis {
                Axis::Horizontal => assert_eq!(velocity, Offset::new(primary, 0.)),
                Axis::Vertical => assert_eq!(velocity, Offset::new(0., primary)),
                Axis::Pan => unreachable!("only axis drags are used"),
            }
        }
    }
}
