//! Pinned Flutter force-press registrations, replayed through normalized native samples.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{
    Color, NormalizedPressure, Offset, PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_gestures::{
    ForcePressDetails, ForcePressGestureRecognizer, PointerDeviceKind, RawPointerEvent,
};
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

const MAX: f64 = 6.66;
const ORIGIN: Offset = Offset::new(10., 10.);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Callback {
    Start,
    Peak,
    Update,
    End,
}

type Log = Rc<RefCell<Vec<(Callback, ForcePressDetails)>>>;

fn record(log: &Log, kind: Callback) -> impl Fn(ForcePressDetails) + 'static {
    let log = log.clone();
    move |details| log.borrow_mut().push((kind, details))
}

fn counts(log: &Log) -> [usize; 4] {
    let mut counts = [0; 4];
    for (kind, _) in log.borrow().iter() {
        counts[*kind as usize] += 1;
    }
    counts
}

fn recognizer(log: &Log) -> ForcePressGestureRecognizer {
    ForcePressGestureRecognizer::new()
        .on_start(record(log, Callback::Start))
        .on_peak(record(log, Callback::Peak))
        .on_update(record(log, Callback::Update))
        .on_end(record(log, Callback::End))
}

fn input(
    base: Instant,
    time: u64,
    phase: PointerPhase,
    pos: Offset,
    pressure: Option<f64>,
) -> RawPointerEvent {
    RawPointerEvent::new(
        1,
        0,
        PointerDeviceKind::Touch,
        u32::from(matches!(phase, PointerPhase::Down | PointerPhase::Move)),
        pos,
        phase,
        base + Duration::from_millis(time),
    )
    .with_sample(PointerSampleMetadata {
        pressure: pressure.and_then(|value| NormalizedPressure::from_range(value, MAX)),
        stylus: None,
    })
}

fn raw(
    force: &mut ForcePressGestureRecognizer,
    base: Instant,
    time: u64,
    phase: PointerPhase,
    psi: Option<f64>,
) {
    force.handle_raw(input(base, time, phase, ORIGIN, psi));
}

#[test]
fn linear_force_starts_at_the_first_threshold_peaks_once_and_reports_a_zero_pressure_end() {
    let log = Log::default();
    let mut force = recognizer(&log);
    let t = Instant::now();
    raw(&mut force, t, 0, PointerPhase::Down, Some(0.));
    let samples = [2.5, 2.8, 3.3, 4., 5., 1., 6., 3.3, 4., 5., 1.];
    let expected = [
        [0, 0, 0, 0],
        [1, 0, 1, 0],
        [1, 0, 2, 0],
        [1, 0, 3, 0],
        [1, 0, 4, 0],
        [1, 0, 5, 0],
        [1, 1, 6, 0],
        [1, 1, 7, 0],
        [1, 1, 8, 0],
        [1, 1, 9, 0],
        [1, 1, 10, 0],
    ];
    assert_eq!(counts(&log), [0; 4]);
    for (i, (pressure, expectation)) in samples.into_iter().zip(expected).enumerate() {
        raw(
            &mut force,
            t,
            i as u64 + 1,
            PointerPhase::Move,
            Some(pressure),
        );
        assert_eq!(counts(&log), expectation, "device pressure {pressure}");
    }
    raw(&mut force, t, 20, PointerPhase::Up, None);
    assert_eq!(counts(&log), [1, 1, 10, 1]);
    let events = log.borrow();
    assert_eq!(events[0].0, Callback::Start);
    assert_eq!(events[0].1.global_position, ORIGIN);
    assert_eq!(events[0].1.local_position, ORIGIN);
    assert_eq!(events.last().unwrap().1.pressure, 0.);
}

#[test]
fn an_accepted_but_light_press_never_starts_or_ends() {
    let log = Log::default();
    let mut force = recognizer(&log);
    let t = Instant::now();
    raw(&mut force, t, 0, PointerPhase::Down, Some(0.));
    raw(&mut force, t, 1, PointerPhase::Move, Some(2.5));
    raw(&mut force, t, 2, PointerPhase::Up, None);
    assert_eq!(counts(&log), [0; 4]);
}

#[test]
fn native_normalization_clamps_out_of_range_samples_before_they_reach_the_arena() {
    let log = Log::default();
    let mut force = recognizer(&log);
    let t = Instant::now();
    raw(&mut force, t, 0, PointerPhase::Down, Some(0.));
    for (i, pressure) in [-1., 2.5, 8.].into_iter().enumerate() {
        raw(
            &mut force,
            t,
            i as u64 + 1,
            PointerPhase::Move,
            Some(pressure),
        );
        if pressure < 8. {
            assert_eq!(counts(&log), [0; 4]);
        }
    }
    raw(&mut force, t, 4, PointerPhase::Up, None);
    assert_eq!(counts(&log), [1, 1, 1, 1]);
    let events = log.borrow();
    assert_eq!(
        events[..3]
            .iter()
            .map(|(kind, _)| *kind)
            .collect::<Vec<_>>(),
        [Callback::Start, Callback::Peak, Callback::Update]
    );
    assert!(
        events[..3]
            .iter()
            .all(|(_, details)| details.pressure == 1.)
    );
}

#[test]
fn a_nonfinite_sample_after_peak_does_not_dispatch_an_update_or_prevent_release() {
    let log = Log::default();
    let mut force = recognizer(&log);
    let t = Instant::now();
    raw(&mut force, t, 0, PointerPhase::Down, Some(0.));
    raw(&mut force, t, 1, PointerPhase::Move, Some(2.5));
    assert_eq!(counts(&log), [0; 4]);
    raw(&mut force, t, 2, PointerPhase::Move, Some(6.));
    assert_eq!(counts(&log), [1, 1, 1, 0]);
    raw(&mut force, t, 3, PointerPhase::Move, Some(f64::NAN));
    assert_eq!(counts(&log), [1, 1, 1, 0]);
    raw(&mut force, t, 4, PointerPhase::Up, None);
    assert_eq!(counts(&log), [1, 1, 1, 1]);
}

#[test]
fn an_ease_in_interpolator_delays_start_and_peak_by_the_expected_amount() {
    let log = Log::default();
    let mut force = recognizer(&log).interpolation(|min, max, value| {
        assert_eq!((min, max), (0., 1.), "adapters pass normalized bounds");
        Curve::EaseIn.apply((value - min) / (max - min))
    });
    let t = Instant::now();
    raw(&mut force, t, 0, PointerPhase::Down, Some(0.));
    let samples = [2.5, 2.8, 3.3, 4., 5., 1., 6., 3.3, 4., 6.5, 1.];
    let expected = [
        [0, 0, 0, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
        [1, 0, 1, 0],
        [1, 0, 2, 0],
        [1, 0, 3, 0],
        [1, 0, 4, 0],
        [1, 0, 5, 0],
        [1, 0, 6, 0],
        [1, 1, 7, 0],
        [1, 1, 8, 0],
    ];
    for (i, (pressure, expectation)) in samples.into_iter().zip(expected).enumerate() {
        raw(
            &mut force,
            t,
            i as u64 + 1,
            PointerPhase::Move,
            Some(pressure),
        );
        assert_eq!(counts(&log), expectation, "device pressure {pressure}");
    }
    raw(&mut force, t, 20, PointerPhase::Up, None);
    assert_eq!(counts(&log), [1, 1, 8, 1]);
    assert_eq!(log.borrow()[0].1.global_position, ORIGIN);
}

#[test]
fn retained_pan_and_force_compete_until_one_wins_and_neither_ends_an_unaccepted_press() {
    for (events, expected, pan_expected) in [
        (
            vec![
                (PointerPhase::Move, Offset::new(30., 30.), Some(1.)),
                (PointerPhase::Move, ORIGIN, Some(2.5)),
                (PointerPhase::Move, ORIGIN, Some(4.)),
            ],
            [0; 4],
            1,
        ),
        (vec![], [0; 4], 0),
        (
            vec![(PointerPhase::Move, ORIGIN, Some(3.))],
            [1, 0, 1, 1],
            0,
        ),
    ] {
        let log = Log::default();
        let pan = Rc::new(std::cell::Cell::new(0));
        let mut tree = WidgetTree::new();
        let size = Size::new(100., 80.);
        let detector = GestureDetector::new(Widget::box_(size, Color::WHITE))
            .on_force_press_start(record(&log, Callback::Start))
            .on_force_press_peak(record(&log, Callback::Peak))
            .on_force_press_update(record(&log, Callback::Update))
            .on_force_press_end(record(&log, Callback::End))
            .on_pan_start({
                let pan = pan.clone();
                move |_| pan.set(pan.get() + 1)
            });
        tree.mount(detector.into()).unwrap();
        tree.layout(Constraints::tight(size)).unwrap();
        let t = Instant::now();
        let mut dispatch = |i, phase, position, pressure| {
            let raw = input(t, i, phase, position, pressure);
            let hit = tree.pointer_hit_test(position);
            let _ = tree.dispatch_pointer_gesture_with_hit_sample(
                raw.device,
                0,
                raw.kind,
                raw.buttons,
                raw.sample,
                raw.legacy(),
                &hit,
            );
        };
        dispatch(0, PointerPhase::Down, ORIGIN, Some(1.));
        assert_eq!(counts(&log), [0; 4]);
        assert_eq!(pan.get(), 0);
        for (i, (phase, pos, pressure)) in events.into_iter().enumerate() {
            dispatch(i as u64 + 1, phase, pos, pressure);
            assert_eq!(counts(&log)[3], 0);
            if pan_expected > 0 {
                assert_eq!(pan.get(), 1);
                assert_eq!(counts(&log), [0; 4]);
            }
        }
        dispatch(20, PointerPhase::Up, ORIGIN, None);
        assert_eq!(counts(&log), expected);
        assert_eq!(pan.get(), pan_expected);
    }
}
