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

#[test]
fn incrementally_delivered_hardware_samples_match_preloaded_input() {
    let start = Instant::now();
    let rows = [
        (1000, Added, 0., 50.),
        (2000, Hover, 10., 40.),
        (2000, Down, 10., 40.),
        (3000, Move, 20., 30.),
        (4000, Move, 30., 20.),
        (4000, Up, 30., 20.),
        (5000, Hover, 40., 10.),
        (6000, Hover, 50., 0.),
        (6000, Removed, 50., 0.),
    ];
    let mut streaming = PointerEventResampler::new();
    let mut out = Vec::new();
    let mut delivered = 0;
    for (add_count, at, total) in [
        (1, 500, 0),
        (3, 500, 0),
        (3, 1500, 1),
        (4, 1500, 1),
        (4, 2500, 3),
        (6, 2500, 3),
        (6, 3500, 4),
        (7, 3500, 4),
        (7, 4500, 6),
        (9, 4500, 6),
        (9, 5500, 7),
        (9, 6500, 9),
        (9, 7500, 9),
    ] {
        for &(us, event, x, y) in &rows[delivered..add_count] {
            streaming.add_event(input(start, us, event, x, y));
        }
        delivered = add_count;
        sample(&mut streaming, start, at, &mut out);
        assert_eq!(
            out.len(),
            total,
            "sample {at} with {add_count} input events"
        );
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
fn jumping_across_multiple_presses_keeps_every_transition() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (2000, Move, 10., 0.),
            (3000, Up, 10., 0.),
            (4000, Hover, 20., 0.),
            (4000, Down, 20., 0.),
            (5000, Move, 30., 0.),
            (5000, Up, 30., 0.),
            (5000, Removed, 30., 0.),
        ],
    );
    let mut out = Vec::new();
    for (at, total) in [(1500, 2), (5500, 7), (6500, 7)] {
        sample(&mut sampling, start, at, &mut out);
        assert_eq!(out.len(), total);
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (1500, Added, (5., 0.), (0., 0.)),
            (1500, Down, (5., 0.), (0., 0.)),
            (5500, Move, (30., 0.), (25., 0.)),
            (5500, Up, (30., 0.), (0., 0.)),
            (5500, Down, (30., 0.), (0., 0.)),
            (5500, Up, (30., 0.), (0., 0.)),
            (5500, Removed, (30., 0.), (0., 0.)),
        ]
    );
}

#[test]
fn sampling_after_an_entire_unobserved_contact_keeps_only_its_transitions() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (4000, Move, 30., 0.),
            (4000, Up, 30., 0.),
            (4000, Removed, 30., 0.),
        ],
    );
    let mut out = Vec::new();
    for (at, total) in [(500, 0), (5500, 4), (6500, 4)] {
        sample(&mut sampling, start, at, &mut out);
        assert_eq!(out.len(), total);
    }
    assert_eq!(
        snapshots(&out, start),
        [Added, Down, Up, Removed].map(|phase| (5500, phase, (30., 0.), (0., 0.)))
    );
}

#[test]
fn stop_flushes_original_timestamps_positions_and_deltas_then_resets() {
    let start = Instant::now();
    let original = [
        (1000, Added, 0., 0.),
        (2000, Down, 0., 0.),
        (3000, Move, 10., 0.),
        (4000, Move, 20., 0.),
        (4000, Up, 20., 0.),
        (5000, Removed, 20., 0.),
    ]
    .map(|(us, phase, x, y)| {
        let mut event = input(start, us, phase, x, y);
        if phase == Move {
            event.delta = Offset::new(10., 0.);
            event.local_delta = event.delta;
        }
        event
    });
    let mut sampling = PointerEventResampler::new();
    for event in original {
        sampling.add_event(event);
    }
    let mut out = Vec::new();
    sample(&mut sampling, start, 500, &mut out);
    assert!(out.is_empty());
    sampling.stop(|event| out.push(event));
    assert_eq!(out, original);
    assert!(!sampling.has_pending_events());
    assert!(!sampling.is_tracked());
    assert!(!sampling.is_down());
    sample(&mut sampling, start, 10000, &mut out);
    assert_eq!(out.len(), 6);
}

#[test]
fn the_next_frame_can_pull_release_forward_without_pulling_future_motion() {
    let start = Instant::now();
    let mut sampling = preload(
        start,
        &[
            (1000, Added, 0., 0.),
            (1000, Down, 0., 0.),
            (2000, Move, 10., 0.),
            (3000, Move, 20., 0.),
            (3000, Up, 20., 0.),
            (4000, Hover, 30., 0.),
            (4000, Removed, 30., 0.),
        ],
    );
    let mut out = Vec::new();
    for (at, next, total) in [
        (500, 1500, 0),
        (1500, 2500, 2),
        (2500, 3500, 4),
        (3500, 4500, 6),
    ] {
        sampling.sample(stamp(start, at), Some(stamp(start, next)), |event| {
            out.push(event)
        });
        assert_eq!(out.len(), total);
    }
    assert_eq!(
        snapshots(&out, start),
        [
            (1500, Added, (5., 0.), (0., 0.)),
            (1500, Down, (5., 0.), (0., 0.)),
            (2500, Move, (15., 0.), (10., 0.)),
            (2500, Up, (15., 0.), (0., 0.)),
            (3500, Hover, (25., 0.), (10., 0.)),
            (3500, Removed, (25., 0.), (0., 0.)),
        ]
    );
    sample(&mut sampling, start, 10000, &mut out);
    assert_eq!(out.len(), 6);
}
