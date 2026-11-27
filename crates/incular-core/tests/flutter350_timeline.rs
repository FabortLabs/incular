use std::{
    borrow::Cow,
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    time::Duration,
};

use incular_core::{DebugTimeline, DebugTimelineError};

struct NoConversion;

impl From<NoConversion> for Cow<'static, str> {
    fn from(_: NoConversion) -> Self {
        panic!("disabled collection must not convert the label");
    }
}

#[test]
fn disabled_capture_is_silent_and_cannot_be_collected() {
    let timeline = DebugTimeline::new();
    assert!(!timeline.collection_enabled());
    timeline.start_sync(NoConversion);
    timeline.finish_sync().unwrap();
    assert_eq!(timeline.time_sync(NoConversion, || 28), 28);
    assert!(matches!(
        timeline.collect(),
        Err(DebugTimelineError::CollectionDisabled)
    ));
}

#[test]
fn enabled_collection_records_a_single_block_and_drains_on_read() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    timeline.start_sync("TEST");
    timeline.finish_sync().unwrap();
    timeline.set_collection_enabled(true);
    let report = timeline.collect().unwrap();
    assert_eq!(report.timed_blocks().len(), 1);
    assert_eq!(report.aggregated_blocks().len(), 1);
    let block = report.get_aggregated("TEST");
    assert_eq!(block.name, "TEST");
    assert_eq!(block.count, 1);
    let empty = timeline.collect().unwrap();
    assert!(empty.timed_blocks().is_empty());
    assert!(empty.aggregated_blocks().is_empty());
}

#[test]
fn reset_deletes_completed_samples() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    timeline.time_sync("TEST", || {});
    timeline.reset();
    let report = timeline.collect().unwrap();
    assert!(report.timed_blocks().is_empty());
    assert!(report.aggregated_blocks().is_empty());
}

#[test]
fn an_unknown_label_has_zero_count_and_duration() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    let report = timeline.collect().unwrap();
    let missing = report.get_aggregated("MISSING");
    assert_eq!(missing.name, "MISSING");
    assert_eq!(missing.count, 0);
    assert_eq!(missing.duration_micros, 0);
}

#[test]
fn time_sync_measures_the_callable_on_the_same_clock_and_keeps_its_result() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    let start = timeline.now_micros();
    let result = timeline.time_sync("TEST", || {
        std::thread::sleep(Duration::from_millis(1));
        "finished"
    });
    let end = timeline.now_micros();
    assert_eq!(result, "finished");
    let report = timeline.collect().unwrap();
    let [timed] = report.timed_blocks() else {
        panic!("expected one timed block");
    };
    assert_eq!(report.aggregated_blocks().len(), 1);
    assert_eq!(timed.name, "TEST");
    assert!(timed.start_micros >= start);
    assert!(timed.end_micros <= end);
    assert!(timed.duration_micros() > 0);
    let total = report.get_aggregated("TEST");
    assert_eq!(total.count, 1);
    assert_eq!(total.duration_micros, timed.duration_micros());
}

#[test]
fn instant_events_reach_their_sink_without_recording_a_duration() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let timeline = DebugTimeline::new().with_instant_sink({
        let events = events.clone();
        move |label, micros| events.borrow_mut().push((label.to_owned(), micros))
    });
    timeline.set_collection_enabled(true);
    let before = timeline.now_micros();
    timeline.instant_sync("TEST");
    let events = events.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "TEST");
    assert!((before..=timeline.now_micros()).contains(&events[0].1));
    let report = timeline.collect().unwrap();
    assert!(report.timed_blocks().is_empty());
    assert!(report.aggregated_blocks().is_empty());
}

#[test]
fn current_timestamp_is_available_even_before_capture() {
    let timeline = DebugTimeline::new();
    let before = timeline.now_micros();
    timeline.set_collection_enabled(true);
    assert!(timeline.now_micros() >= before);
}

#[test]
fn ten_slices_drain_without_missing_timings() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    for _ in 0..5000 {
        timeline.start_sync("TEST");
        timeline.finish_sync().unwrap();
    }
    let report = timeline.collect().unwrap();
    assert_eq!(report.timed_blocks().len(), 5000);
    assert_eq!(report.aggregated_blocks().len(), 1);
    let total = report.get_aggregated("TEST");
    assert_eq!(total.count, 5000);
    assert_eq!(
        total.duration_micros,
        report
            .timed_blocks()
            .iter()
            .map(|block| block.duration_micros())
            .sum::<u64>()
    );
}

#[test]
fn names_keep_their_recording_order_across_slice_boundaries() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    for index in 0..1750 {
        timeline.time_sync(format!("TEST{index}"), || {});
    }
    let report = timeline.collect().unwrap();
    assert_eq!(report.timed_blocks().len(), 1750);
    for (index, block) in report.timed_blocks().iter().enumerate() {
        assert_eq!(block.name, format!("TEST{index}"));
    }
}

#[test]
fn nested_blocks_record_completion_order_and_reset_keeps_open_scopes() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    timeline.time_sync("outside", || {
        timeline.time_sync("discarded", || {});
        timeline.reset();
        assert!(matches!(
            timeline.collect(),
            Err(DebugTimelineError::UnfinishedBlocks(1))
        ));
        timeline.time_sync("inside", || {});
    });
    let report = timeline.collect().unwrap();
    let blocks = report.timed_blocks();
    assert_eq!(
        blocks.iter().map(|b| b.name.as_ref()).collect::<Vec<_>>(),
        ["inside", "outside"]
    );
    assert!(blocks[1].start_micros <= blocks[0].start_micros);
    assert!(blocks[1].end_micros >= blocks[0].end_micros);
}

#[test]
fn unwinding_closes_a_scope_and_an_old_scope_cannot_pop_a_new_capture() {
    let timeline = DebugTimeline::new();
    timeline.set_collection_enabled(true);
    assert!(
        catch_unwind(AssertUnwindSafe(
            || timeline.time_sync("panic", || panic!("sample"))
        ))
        .is_err()
    );
    assert_eq!(timeline.collect().unwrap().get_aggregated("panic").count, 1);
    timeline.time_sync("old capture", || {
        timeline.set_collection_enabled(false);
        timeline.set_collection_enabled(true);
        timeline.start_sync("new capture");
    });
    timeline.finish_sync().unwrap();
    assert_eq!(
        timeline
            .collect()
            .unwrap()
            .get_aggregated("new capture")
            .count,
        1
    );
    assert!(matches!(
        timeline.finish_sync(),
        Err(DebugTimelineError::NoOpenBlock)
    ));
}
