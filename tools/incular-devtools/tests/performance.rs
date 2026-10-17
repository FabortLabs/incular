use incular_devtools_protocol::{DevWindowId, FrameRecordEvent, FrameTimingsWire};
use incular_devtools_ui::performance::{
    FramePhase, FrameStatistics, format_micros, is_janky, percent_of_frame, phase_segments,
    stacked_total,
};

fn timings(build: u32, layout: u32, paint: u32, cpu_total: u32) -> FrameTimingsWire {
    FrameTimingsWire {
        build,
        layout,
        paint,
        semantics: 5,
        composite: 5,
        cpu_total,
        ..FrameTimingsWire::default()
    }
}

fn frame(cpu_total: u32, budget_us: Option<u32>, over_budget: bool) -> FrameRecordEvent {
    FrameRecordEvent {
        window: DevWindowId::new(1, 0),
        frame: 1,
        timings: timings(10, 20, 30, cpu_total),
        budget_us,
        over_budget,
        draw_calls: 0,
        instances: 0,
        upload_bytes: 0,
        pipelines_created: 0,
    }
}

#[test]
fn segments_follow_stacking_order_and_other_absorbs_the_remainder() {
    let segments = phase_segments(&timings(10, 20, 30, 100));
    let phases = segments.iter().map(|s| s.phase).collect::<Vec<_>>();
    assert_eq!(phases, FramePhase::ALL);
    let micros = segments.iter().map(|s| s.micros).collect::<Vec<_>>();
    assert_eq!(micros, [10, 20, 30, 5, 5, 30]);
    assert_eq!(stacked_total(&timings(10, 20, 30, 100)), 100);
}

#[test]
fn other_saturates_when_phases_overshoot_the_total() {
    let overshoot = timings(10, 20, 30, 40);
    let segments = phase_segments(&overshoot);
    assert_eq!(segments[5].micros, 0);
    // The stack is taller than cpu_total so no phase is clipped.
    assert_eq!(stacked_total(&overshoot), 70);
}

#[test]
fn empty_frame_has_no_percentages() {
    let empty = FrameTimingsWire::default();
    assert_eq!(stacked_total(&empty), 0);
    assert_eq!(percent_of_frame(0, &empty), None);
    assert_eq!(percent_of_frame(25, &timings(0, 0, 0, 100)), Some(25.));
}

#[test]
fn jank_requires_a_known_positive_budget() {
    assert!(is_janky(&frame(20_000, Some(16_666), true)));
    assert!(!is_janky(&frame(10_000, Some(16_666), false)));
    assert!(!is_janky(&frame(20_000, None, true)));
    assert!(!is_janky(&frame(20_000, Some(0), true)));
}

#[test]
fn statistics_agree_with_jank_flags() {
    let frames = [
        frame(1_000, Some(16_666), false),
        frame(30_000, Some(16_666), true),
        frame(50_000, None, true),
    ];
    let statistics = FrameStatistics::from_frames(&frames);
    assert_eq!(statistics.budgeted_samples, 2);
    assert_eq!(statistics.over_budget, 1);
    assert_eq!(statistics.jank_percent(), Some(50.));
    assert_eq!(statistics.p95_us, 50_000);
    assert_eq!(
        frames.iter().filter(|frame| is_janky(frame)).count(),
        statistics.over_budget
    );
}

#[test]
fn micros_format_switches_units_at_one_millisecond() {
    assert_eq!(format_micros(999), "999 µs");
    assert_eq!(format_micros(1_000), "1.00 ms");
    assert_eq!(format_micros(16_667), "16.67 ms");
}
