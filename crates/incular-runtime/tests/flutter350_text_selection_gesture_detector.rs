use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, TargetPlatform};
use incular_core::{
    Color, Offset, PRIMARY_POINTER_BUTTON, PointerContactMetadata, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    HitTestBehavior, RawPointerEvent, TextSelectionGestureCallbacks, TextSelectionGestureDetector,
    Widget,
};

use PointerDeviceKind::{Mouse, Touch};
use PointerPhase::{Down, Move, Up};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const PRESS: Offset = Offset { x: 200., y: 200. };

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Observed {
    downs: usize,
    singles: usize,
    doubles: usize,
    triples: usize,
    cancels: usize,
    long: usize,
    drag_start: usize,
    drag_update: usize,
    drag_end: usize,
    force_start: usize,
    force_end: usize,
}

struct Driver {
    runtime: Runtime,
    now: Instant,
    seen: Rc<RefCell<Observed>>,
}

impl Driver {
    fn new(platform: TargetPlatform) -> Self {
        let seen = Rc::new(RefCell::new(Observed::default()));
        let record = |change: fn(&mut Observed)| {
            let seen = seen.clone();
            move || change(&mut seen.borrow_mut())
        };
        macro_rules! count {
            ($field:ident) => {{
                let call = record(|seen| seen.$field += 1);
                Some(Rc::new(move |_| call()))
            }};
        }
        let cancel = record(|seen| seen.cancels += 1);
        let widget = TextSelectionGestureDetector::new(Widget::box_(VIEW, Color::TRANSPARENT))
            .platform(platform)
            .behavior(HitTestBehavior::Opaque)
            .callbacks(TextSelectionGestureCallbacks {
                on_tap_down: count!(downs),
                on_single_tap_up: count!(singles),
                on_single_tap_cancel: Some(Rc::new(cancel)),
                on_double_tap_down: count!(doubles),
                on_triple_tap_down: count!(triples),
                on_single_long_tap_start: count!(long),
                on_drag_selection_start: count!(drag_start),
                on_drag_selection_update: count!(drag_update),
                on_drag_selection_end: count!(drag_end),
                on_force_press_start: count!(force_start),
                on_force_press_end: count!(force_end),
                ..Default::default()
            });
        let mut driver = Self {
            runtime: Runtime::new(widget.into()).unwrap(),
            now: Instant::now(),
            seen,
        };
        driver.advance(0);
        driver
    }

    fn advance(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.runtime
            .run_frame_at(Constraints::tight(VIEW), self.now)
            .unwrap();
    }

    fn send(
        &mut self,
        pointer: u64,
        kind: PointerDeviceKind,
        phase: PointerPhase,
        position: Offset,
    ) {
        self.send_with_pressure(pointer, kind, phase, position, 0., 1.);
    }

    fn send_with_pressure(
        &mut self,
        pointer: u64,
        kind: PointerDeviceKind,
        phase: PointerPhase,
        position: Offset,
        pressure: f32,
        pressure_max: f32,
    ) {
        let _ = self.runtime.handle_raw_pointer_event(
            RawPointerEvent::new(
                pointer,
                1,
                kind,
                if phase == Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                position,
                phase,
                self.now,
            )
            .with_button((phase != Move).then_some(PRIMARY_POINTER_BUTTON))
            .with_sample(PointerSampleMetadata {
                contact: PointerContactMetadata {
                    raw_pressure: Some(pressure),
                    pressure_min: 0.,
                    pressure_max,
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }

    fn tap(&mut self, pointer: u64) {
        self.send(pointer, Touch, Down, PRESS);
        self.send(pointer, Touch, Up, PRESS);
    }

    fn read(&self) -> Observed {
        *self.seen.borrow()
    }
}

#[test]
fn all_six_taps_contribute_a_down_even_when_they_form_one_series() {
    let mut driver = Driver::new(TargetPlatform::Android);
    for pointer in 1..=6 {
        driver.tap(pointer);
        if pointer < 6 {
            driver.advance(150);
        }
    }
    assert_eq!(driver.read().downs, 6);
}

#[test]
fn seven_rapid_taps_alternate_platform_counts_and_single_up() {
    for (platform, expected) in [
        (
            TargetPlatform::Android,
            [
                (1, 0, 0),
                (1, 1, 0),
                (1, 1, 1),
                (2, 1, 1),
                (2, 2, 1),
                (2, 2, 2),
                (3, 2, 2),
            ],
        ),
        (
            TargetPlatform::Ios,
            [
                (1, 0, 0),
                (1, 1, 0),
                (1, 1, 1),
                (1, 1, 2),
                (1, 1, 3),
                (1, 1, 4),
                (1, 1, 5),
            ],
        ),
        (
            TargetPlatform::Windows,
            [
                (1, 0, 0),
                (1, 1, 0),
                (1, 1, 1),
                (1, 2, 1),
                (1, 2, 2),
                (1, 3, 2),
                (1, 3, 3),
            ],
        ),
    ] {
        let mut driver = Driver::new(platform);
        for (index, (single, double, triple)) in expected.into_iter().enumerate() {
            driver.tap(index as u64 + 1);
            if index < 6 {
                driver.advance(50);
            }
            assert_eq!(
                driver.read(),
                Observed {
                    downs: index + 1,
                    singles: single,
                    doubles: double,
                    triples: triple,
                    ..Default::default()
                },
                "{platform:?}, tap {}",
                index + 1
            );
        }
    }
}

#[test]
fn a_quick_second_tap_held_down_is_a_double_down_without_a_single_up() {
    let mut driver = Driver::new(TargetPlatform::Android);
    driver.tap(1);
    driver.advance(50);
    driver.send(2, Touch, Down, PRESS);
    driver.advance(200);
    let expected = Observed {
        downs: 2,
        singles: 1,
        doubles: 1,
        ..Default::default()
    };
    assert_eq!(driver.read(), expected);
    driver.send(2, Touch, Up, PRESS);
    assert_eq!(driver.read(), expected);
}

#[test]
fn early_and_late_diagonal_swipes_have_no_single_up_or_cancel() {
    for delay in [20, 120] {
        let mut driver = Driver::new(TargetPlatform::Android);
        driver.send(1, Touch, Down, PRESS);
        driver.advance(delay);
        let next = PRESS + Offset::new(100., 100.);
        driver.send(1, Touch, Move, next);
        driver.advance(0);
        let seen = driver.read();
        assert_eq!(
            (
                seen.downs,
                seen.singles,
                seen.cancels,
                seen.doubles,
                seen.long
            ),
            (1, 0, 0, 0, 0)
        );
        if delay == 20 {
            driver.send(1, Touch, Up, next);
            let seen = driver.read();
            assert_eq!(
                (
                    seen.downs,
                    seen.singles,
                    seen.cancels,
                    seen.doubles,
                    seen.long
                ),
                (1, 0, 0, 0, 0)
            );
        }
    }
}

#[test]
fn touch_long_hold_wins_but_an_identical_mouse_hold_is_a_tap() {
    for (kind, singles, long) in [(Touch, 0, 1), (Mouse, 1, 0)] {
        let mut driver = Driver::new(TargetPlatform::Android);
        driver.send(1, kind, Down, PRESS);
        driver.advance(2000);
        driver.send(1, kind, Up, PRESS);
        driver.advance(0);
        let seen = driver.read();
        assert_eq!((seen.downs, seen.singles, seen.long), (1, singles, long));
    }
}

#[test]
fn touch_mouse_and_delayed_mouse_drags_report_the_complete_sequence() {
    for (kind, delay) in [(Touch, 0), (Mouse, 0), (Mouse, 2000)] {
        let mut driver = Driver::new(TargetPlatform::Android);
        let next = PRESS + Offset::new(210., 200.);
        driver.send(1, kind, Down, PRESS);
        driver.advance(delay);
        driver.send(1, kind, Move, next);
        driver.advance(0);
        driver.send(1, kind, Up, next);
        let seen = driver.read();
        assert_eq!(
            (
                seen.downs,
                seen.singles,
                seen.cancels,
                seen.drag_start,
                seen.drag_update,
                seen.drag_end
            ),
            (1, 0, 0, 1, 1, 1)
        );
    }
}

#[test]
fn four_force_presses_restart_recognition_even_with_the_same_pointer_id() {
    let mut driver = Driver::new(TargetPlatform::Android);
    let point = Offset::new(400., 50.);
    for index in 0..4 {
        driver.send_with_pressure(1, Touch, Down, point, 0., 6.);
        driver.send_with_pressure(1, Touch, Move, point, 0.5, 1.);
        driver.send_with_pressure(1, Touch, Up, point, 0., 1.);
        driver.advance(if index == 0 { 0 } else { 20 });
    }
    assert_eq!(driver.read().force_start, 4);
}

#[test]
fn a_force_press_after_a_tap_wins_without_a_double_tap_down() {
    let mut driver = Driver::new(TargetPlatform::Android);
    let point = Offset::new(400., 50.);
    driver.send_with_pressure(1, Touch, Down, point, 0., 6.);
    driver.send_with_pressure(1, Touch, Move, point, 0., 1.);
    driver.advance(50);
    driver.send_with_pressure(1, Touch, Up, point, 0., 1.);

    driver.send_with_pressure(1, Touch, Down, point, 0., 6.);
    driver.send_with_pressure(1, Touch, Move, point, 0.5, 1.);
    assert_eq!(driver.read().force_start, 1);
    driver.advance(50);
    driver.send_with_pressure(1, Touch, Up, point, 0., 1.);
    let seen = driver.read();
    assert_eq!((seen.force_end, seen.doubles), (1, 0));
}

#[test]
fn updated_handlers_keep_the_live_tap_series_and_report_reset_before_a_new_one() {
    type Trace = Rc<RefCell<Vec<(u32, &'static str)>>>;
    let old = Trace::default();
    let new = Trace::default();
    let make = |log: &Trace, always| {
        let record = |what: &'static str| {
            let log = log.clone();
            move || log.borrow_mut().push((0, what))
        };
        let track_started = record("track");
        let reset = record("reset");
        let user = record("user");
        let down_log = log.clone();
        TextSelectionGestureDetector::new(Widget::box_(VIEW, Color::TRANSPARENT))
            .platform(TargetPlatform::Android)
            .behavior(HitTestBehavior::Opaque)
            .callbacks(TextSelectionGestureCallbacks {
                on_tap_track_start: Some(Rc::new(track_started)),
                on_tap_track_reset: Some(Rc::new(reset)),
                on_tap_down: Some(Rc::new(move |details| {
                    down_log
                        .borrow_mut()
                        .push((details.consecutive_tap_count, "down"));
                })),
                on_user_tap: Some(Rc::new(user)),
                on_user_tap_always_called: always,
                on_drag_selection_start: Some(Rc::new(|_| {})),
                ..Default::default()
            })
            .into()
    };
    let mut driver = Driver {
        runtime: Runtime::new(make(&old, false)).unwrap(),
        now: Instant::now(),
        seen: Default::default(),
    };
    driver.advance(0);
    driver.tap(1);
    assert_eq!(*old.borrow(), [(0, "track"), (1, "down"), (0, "user")]);

    let root = driver.runtime.tree().root().unwrap();
    driver
        .runtime
        .schedule_update(root, make(&new, true))
        .unwrap();
    driver.advance(50);
    driver.tap(2);
    assert_eq!(*new.borrow(), [(0, "track"), (2, "down"), (0, "user")]);
    assert_eq!(*old.borrow(), [(0, "track"), (1, "down"), (0, "user")]);

    driver.advance(300);
    // Flutter keeps a stale series until the following contact.
    assert_eq!(new.borrow().len(), 3);
    driver.tap(3);
    assert_eq!(
        *new.borrow(),
        [
            (0, "track"),
            (2, "down"),
            (0, "user"),
            (0, "reset"),
            (0, "track"),
            (1, "down"),
            (0, "user")
        ]
    );
}

#[test]
fn force_can_claim_the_arena_on_a_supported_initial_pressure_sample() {
    for (supported, raw, max) in [(true, 3., 6.), (false, 0.8, 1.)] {
        let mut driver = Driver::new(TargetPlatform::Android);
        driver.send_with_pressure(1, Touch, Down, PRESS, raw, max);
        driver.advance(0);
        let seen = driver.read();
        assert_eq!(
            (seen.force_start, seen.singles),
            (usize::from(supported), 0)
        );
        driver.send_with_pressure(1, Touch, Up, PRESS, 0., 1.);
        driver.advance(0);
        let seen = driver.read();
        assert_eq!(
            (seen.force_start, seen.force_end, seen.singles),
            (
                usize::from(supported),
                usize::from(supported),
                usize::from(!supported)
            )
        );
    }
}
