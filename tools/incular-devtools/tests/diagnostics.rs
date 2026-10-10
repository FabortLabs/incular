use incular_devtools_protocol::{DevWindowId, FrameRecordEvent, FrameTimingsWire};
use incular_devtools_ui::{
    activity::{ActivityLog, RequestStatus},
    inspector::{ConsoleLevel, InspectorModel},
    performance::FrameStatistics,
};

fn frame(cpu: u32, budget: Option<u32>, over_budget: bool) -> FrameRecordEvent {
    FrameRecordEvent {
        window: DevWindowId::new(1, 0),
        frame: u64::from(cpu),
        timings: FrameTimingsWire {
            cpu_total: cpu,
            ..Default::default()
        },
        budget_us: budget,
        over_budget,
        draw_calls: 0,
        instances: 0,
        upload_bytes: 0,
        pipelines_created: 0,
    }
}

#[test]
fn console_filters_compose_and_pausing_preserves_live_capture() {
    let mut model = InspectorModel::default();
    model.push_console("INFO", "render", "frame ready");
    model.push_console("warn", "layout", "frame over budget");
    model.push_console("fatal", "render", "device lost");
    model.set_console_level(ConsoleLevel::Errors);
    assert_eq!(model.filtered_console()[0].message, "device lost");
    model.set_console_query("LAYOUT");
    assert!(model.filtered_console().is_empty());
    model.set_console_level(ConsoleLevel::Warnings);
    assert_eq!(model.filtered_console().len(), 1);
    model.set_console_level(ConsoleLevel::All);
    model.set_console_query("");
    model.set_console_paused(true);
    model.push_console("error", "render", "late event");
    assert_eq!(model.filtered_console().len(), 3);
    model.set_console_paused(true);
    assert_eq!(
        model.filtered_console().len(),
        3,
        "pausing again must preserve the same display"
    );
    model.set_console_paused(false);
    assert_eq!(model.filtered_console().len(), 4);
}

#[test]
fn console_clear_resets_byte_accounting_and_the_paused_display() {
    let mut model = InspectorModel::default();
    for _ in 0..600 {
        model.push_console("info", "target", "retained event");
    }
    assert_eq!(model.filtered_console().len(), 500);
    assert!(model.console_bytes() > 0);
    model.set_console_paused(true);
    model.clear_console();
    assert_eq!(model.console_bytes(), 0);
    assert!(model.filtered_console().is_empty());
    model.push_console("info", "target", "new event");
    assert!(model.filtered_console().is_empty());
    model.set_console_paused(false);
    assert_eq!(model.filtered_console().len(), 1);
}

#[test]
fn frame_statistics_use_nearest_rank_and_known_budgets_only() {
    let frames = (1..=20)
        .map(|cpu| frame(cpu * 1000, Some(16_667), cpu > 16))
        .collect::<Vec<_>>();
    let stats = FrameStatistics::from_frames(&frames);
    assert_eq!(stats.samples, 20);
    assert_eq!(stats.average_us, 10_500.);
    assert_eq!(stats.p95_us, 19_000);
    assert_eq!(stats.worst_us, 20_000);
    assert_eq!(stats.jank_percent(), Some(20.));
    let unknown = [frame(9000, None, true), frame(1000, Some(0), true)];
    assert_eq!(FrameStatistics::from_frames(&unknown).jank_percent(), None);
    assert_eq!(FrameStatistics::from_frames(std::iter::empty()).samples, 0);
}

#[test]
fn transport_history_survives_wire_id_reuse_and_preserves_pending_rows_on_clear() {
    let mut log = ActivityLog::default();
    let first = log.begin(1, "GetWidgetTree", 42);
    log.finish(first, RequestStatus::Completed, 1500, 200);
    let second = log.begin(1, "GetTargetInfo", 18);
    assert_ne!(first, second);
    log.finish(first, RequestStatus::Failed, 2000, 0);
    assert_eq!(
        log.entries().next().unwrap().status,
        RequestStatus::Completed
    );
    log.clear_completed();
    assert_eq!(log.entries().count(), 1);
    assert_eq!(log.entries().next().unwrap().sequence, second);
    log.finish(second, RequestStatus::TimedOut, 10_000_000, 0);
    assert_eq!(
        log.entries().next().unwrap().status,
        RequestStatus::TimedOut
    );
    log.clear_completed();
    assert_eq!(log.entries().count(), 0);
}

#[test]
fn transport_history_is_bounded_and_evicted_completions_are_safe() {
    let mut log = ActivityLog::default();
    let first = log.begin(1, "GetTargetInfo", 10);
    for request in 2..=300 {
        log.begin(request, &"x".repeat(200), 10);
    }
    assert_eq!(log.entries().count(), ActivityLog::CAPACITY);
    assert!(log.entries().all(|entry| entry.method.len() <= 96));
    log.finish(first, RequestStatus::Completed, 10, 10);
    assert_eq!(log.entries().count(), ActivityLog::CAPACITY);
}

#[test]
fn report_preserves_live_logs_even_when_the_console_display_is_paused() {
    let mut model = InspectorModel::default();
    model.set_console_paused(true);
    model.push_console("warn", "layout", "new event");
    let report = model.diagnostic_report();
    assert_eq!(report["format"], "incular-devtools-report");
    assert_eq!(report["console"][0]["message"], "new event");
    assert!(report.get("auth_token").is_none());
    assert!(report.get("discovery").is_none());
}
