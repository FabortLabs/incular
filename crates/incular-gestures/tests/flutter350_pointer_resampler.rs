use std::time::Duration;

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, time::Instant,
};
use incular_gestures::{PointerEventResampler, RawPointerEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Added,
    Hover,
    Down,
    Move,
    Up,
    Removed,
}

use Event::{Added, Down, Hover, Move, Removed, Up};

fn stamp(start: Instant, us: u64) -> Instant {
    start + Duration::from_micros(us)
}

fn input(start: Instant, us: u64, event: Event, x: f32, y: f32) -> RawPointerEvent {
    let phase = match event {
        Added => PointerPhase::Enter,
        Hover | Move => PointerPhase::Move,
        Down => PointerPhase::Down,
        Up => PointerPhase::Up,
        Removed => PointerPhase::Exit,
    };
    let buttons = if matches!(event, Down | Move) {
        PRIMARY_POINTER_BUTTON
    } else {
        0
    };
    RawPointerEvent::new(
        0,
        1,
        PointerDeviceKind::Mouse,
        buttons,
        Offset::new(x, y),
        phase,
        stamp(start, us),
    )
}

fn sample(
    resampler: &mut PointerEventResampler,
    start: Instant,
    us: u64,
    output: &mut Vec<RawPointerEvent>,
) {
    resampler.sample(stamp(start, us), None, |event| output.push(event));
}

type Snapshot = (u64, Event, (f32, f32), (f32, f32));

fn snapshots(events: &[RawPointerEvent], start: Instant) -> Vec<Snapshot> {
    events
        .iter()
        .map(|event| {
            assert_eq!(event.local_position, event.position);
            assert_eq!(event.local_delta, event.delta);
            let kind = match event.phase {
                PointerPhase::Enter => Added,
                PointerPhase::Move if event.down => {
                    assert_eq!(event.buttons, PRIMARY_POINTER_BUTTON);
                    Move
                }
                PointerPhase::Move => Hover,
                PointerPhase::Down => Down,
                PointerPhase::Up => Up,
                PointerPhase::Exit => Removed,
                PointerPhase::Cancel => panic!("unexpected cancel"),
            };
            (
                u64::try_from(event.time.duration_since(start).as_micros()).unwrap(),
                kind,
                (event.position.x, event.position.y),
                (event.delta.x, event.delta.y),
            )
        })
        .collect()
}

fn preload(start: Instant, rows: &[(u64, Event, f32, f32)]) -> PointerEventResampler {
    let mut resampler = PointerEventResampler::new();
    for &(us, event, x, y) in rows {
        resampler.add_event(input(start, us, event, x, y));
    }
    resampler
}

#[test]
fn basic_trace_preserves_all_lifecycle_events_and_interpolates_motion() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 50.),
            (2000, Hover, 10., 40.),
            (2000, Down, 10., 40.),
            (3000, Move, 20., 30.),
            (4000, Move, 30., 20.),
            (4000, Up, 30., 20.),
            (5000, Hover, 40., 10.),
            (6000, Hover, 50., 0.),
            (6000, Removed, 50., 0.),
        ],
    );
    let mut out = Vec::new();
    for (us, count) in [
        (500, 0),
        (1500, 1),
        (2500, 3),
        (3500, 4),
        (4500, 6),
        (5500, 7),
        (6500, 9),
        (7500, 9),
    ] {
        sample(&mut sampling, start, us, &mut out);
        assert_eq!(out.len(), count, "sample {us}");
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (1500, Added, (5., 45.), (0., 0.)),
            (2500, Hover, (15., 35.), (10., -10.)),
            (2500, Down, (15., 35.), (0., 0.)),
            (3500, Move, (25., 25.), (10., -10.)),
            (4500, Move, (35., 15.), (10., -10.)),
            (4500, Up, (35., 15.), (0., 0.)),
            (5500, Hover, (45., 5.), (10., -10.)),
            (6500, Hover, (50., 0.), (5., -5.)),
            (6500, Removed, (50., 0.), (0., 0.)),
        ]
    );
}

#[test]
fn a_quick_tap_does_not_merge_same_timestamp_transitions() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (1000, Up, 0., 0.),
            (1000, Removed, 0., 0.),
        ],
    );
    let mut out = Vec::new();
    sample(&mut sampling, start, 1500, &mut out);
    assert_eq!(
        snapshots(&out, start),
        [Added, Down, Up, Removed].map(|phase| (1500, phase, (0., 0.), (0., 0.)))
    );
}

#[test]
fn slowly_advancing_the_clock_does_not_repeat_a_sample_or_lose_a_fractional_move() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (2000, Move, 10., 0.),
            (3000, Move, 20., 0.),
            (3000, Up, 20., 0.),
            (3000, Removed, 20., 0.),
        ],
    );
    let mut out = Vec::new();
    for (us, count) in [(1500, 2), (1500, 2), (1750, 3), (2000, 4), (3000, 7)] {
        sample(&mut sampling, start, us, &mut out);
        assert_eq!(out.len(), count);
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (1500, Added, (5., 0.), (0., 0.)),
            (1500, Down, (5., 0.), (0., 0.)),
            (1750, Move, (7.5, 0.), (2.5, 0.)),
            (2000, Move, (10., 0.), (2.5, 0.)),
            (3000, Move, (20., 0.), (10., 0.)),
            (3000, Up, (20., 0.), (0., 0.)),
            (3000, Removed, (20., 0.), (0., 0.)),
        ]
    );
}

#[test]
fn quickly_advancing_the_clock_coalesces_motion_but_not_release_and_remove() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (2000, Move, 5., 0.),
            (3000, Move, 20., 0.),
            (4000, Up, 30., 0.),
            (4000, Removed, 30., 0.),
        ],
    );
    let mut out = Vec::new();
    for (us, count) in [(2500, 2), (5500, 5), (6500, 5)] {
        sample(&mut sampling, start, us, &mut out);
        assert_eq!(out.len(), count);
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (2500, Added, (12.5, 0.), (0., 0.)),
            (2500, Down, (12.5, 0.), (0., 0.)),
            (5500, Move, (30., 0.), (17.5, 0.)),
            (5500, Up, (30., 0.), (0., 0.)),
            (5500, Removed, (30., 0.), (0., 0.)),
        ]
    );
}

#[test]
fn a_final_movement_is_emitted_before_up_and_remove() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (2000, Down, 0., 0.),
            (3000, Move, 10., 0.),
            (4000, Up, 10., 0.),
            (5000, Removed, 10., 0.),
        ],
    );
    let mut out = Vec::new();
    for (us, count) in [(500, 0), (2000, 2), (5000, 5), (10000, 5)] {
        sample(&mut sampling, start, us, &mut out);
        assert_eq!(out.len(), count);
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (2000, Added, (0., 0.), (0., 0.)),
            (2000, Down, (0., 0.), (0., 0.)),
            (5000, Move, (10., 0.), (10., 0.)),
            (5000, Up, (10., 0.), (0., 0.)),
            (5000, Removed, (10., 0.), (0., 0.)),
        ]
    );
}
