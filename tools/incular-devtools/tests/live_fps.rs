use incular_devtools_protocol::DevWindowId;
use incular_devtools_ui::{
    inspector::InspectorModel,
    performance::{LIVE_FPS_WINDOW, live_fps},
};
use std::time::{Duration, Instant};

fn arrivals(start: Instant, count: u32, every: Duration) -> Vec<Instant> {
    (0..count).map(|index| start + every * index).collect()
}

#[test]
fn counts_only_frames_from_the_trailing_second() {
    let start = Instant::now();
    // 90 frames at 60 Hz span 1.5 s; only the last second's worth count.
    let frames = arrivals(start, 90, Duration::from_micros(16_667));
    let now = *frames.last().unwrap();
    let live = live_fps(&frames, now);
    assert!((59..=61).contains(&live.fps), "got {}", live.fps);
    assert!(!live.idle);
}

#[test]
fn decays_to_zero_and_reports_idle_once_frames_stop() {
    let start = Instant::now();
    let frames = arrivals(start, 60, Duration::from_millis(16));
    let last = *frames.last().unwrap();
    assert!(live_fps(&frames, last).fps > 0);
    let half = live_fps(&frames, last + Duration::from_millis(500));
    assert!(half.fps > 0 && half.fps < 60, "decaying, got {}", half.fps);
    let later = live_fps(&frames, last + LIVE_FPS_WINDOW + Duration::from_millis(1));
    assert_eq!(later.fps, 0);
    assert!(later.idle);
}

#[test]
fn no_frames_is_idle() {
    let live = live_fps(&[], Instant::now());
    assert_eq!((live.fps, live.idle), (0, true));
}

#[test]
fn arrivals_in_the_future_are_ignored() {
    let now = Instant::now();
    let live = live_fps(&[now + Duration::from_secs(5)], now);
    assert!(live.idle);
}

#[test]
fn the_model_reports_live_fps_per_window_and_flags_changes() {
    let window = DevWindowId::new(1, 0);
    let mut model = InspectorModel::default();
    model.activate_window(window);
    let start = Instant::now();
    for index in 0..30 {
        model.record_frame_arrival(window, start + Duration::from_millis(index * 10));
    }
    let now = start + Duration::from_millis(300);
    assert_eq!(model.live_fps(Some(window), now).fps, 30);
    // No window selected: the busiest window is reported.
    assert_eq!(model.live_fps(None, now).fps, 30);
    // First look is a change, an identical second look is not.
    assert!(model.live_fps_changed(now));
    assert!(!model.live_fps_changed(now));
    // Frames age out with the clock alone, which flags a redraw.
    assert!(model.live_fps_changed(now + Duration::from_secs(2)));
    assert!(
        model
            .live_fps(Some(window), now + Duration::from_secs(2))
            .idle
    );
}

#[test]
fn frames_stream_into_the_live_ring_without_recording() {
    let model = InspectorModel::default();
    assert!(model.recent_cpu_micros(None, 60).is_empty());
}
