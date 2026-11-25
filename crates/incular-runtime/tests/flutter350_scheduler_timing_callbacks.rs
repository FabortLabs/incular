use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::{
    Application, FrameRecord, FrameTimingCallbacks, FrameTimingSubscription, RenderFrameMetrics,
};
use incular_widgets::Widget;

#[test]
fn timing_callback_panics_are_reported_without_interrupting_empty_batches() {
    let callbacks = FrameTimingCallbacks::default();
    let trace = Rc::new(RefCell::new(Vec::new()));
    callbacks.set_error_handler({
        let trace = trace.clone();
        move |error| trace.borrow_mut().push(error.to_string())
    });
    let failing = callbacks.add(|frames| {
        assert!(frames.is_empty());
        panic!("Exception: Test");
    });
    let succeeding = callbacks.add({
        let trace = trace.clone();
        move |frames| {
            assert!(frames.is_empty());
            trace.borrow_mut().push("next observer".into());
        }
    });

    callbacks.report(&[]);
    assert_eq!(*trace.borrow(), ["Exception: Test", "next observer"]);
    drop(failing);
    drop(succeeding);
    callbacks.report(&[]);
    assert_eq!(trace.borrow().len(), 2);
}

#[test]
fn timing_callbacks_snapshot_order_and_recheck_removals_during_dispatch() {
    let callbacks = FrameTimingCallbacks::default();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let old = Rc::new(RefCell::new(None::<FrameTimingSubscription>));
    let new = Rc::new(RefCell::new(None::<FrameTimingSubscription>));
    let first = callbacks.add({
        let callbacks = callbacks.clone();
        let (trace, old, new) = (trace.clone(), old.clone(), new.clone());
        move |_| {
            trace.borrow_mut().push("first");
            drop(old.borrow_mut().take());
            if new.borrow().is_none() {
                new.borrow_mut().replace(callbacks.add({
                    let trace = trace.clone();
                    move |_| trace.borrow_mut().push("new")
                }));
            }
        }
    });
    old.borrow_mut().replace(callbacks.add({
        let trace = trace.clone();
        move |_| trace.borrow_mut().push("removed")
    }));
    let last = callbacks.add({
        let trace = trace.clone();
        move |_| trace.borrow_mut().push("last")
    });

    callbacks.report(&[]);
    assert_eq!(*trace.borrow(), ["first", "last"]);
    trace.borrow_mut().clear();
    callbacks.report(&[]);
    assert_eq!(*trace.borrow(), ["first", "last", "new"]);
    drop((first, last));
}

#[test]
fn frame_timing_callback_receives_the_completed_renderer_merged_record() {
    let size = Size::new(100., 100.);
    let mut app = Application::new(move |_| Widget::box_(size, Color::WHITE)).unwrap();
    let window = app.primary_window();
    let callbacks = app.frame_timing_callbacks();
    let observed = Rc::new(RefCell::new(Vec::<FrameRecord>::new()));
    let _subscription = callbacks.add({
        let observed = observed.clone();
        move |frames| observed.borrow_mut().extend_from_slice(frames)
    });
    let (_, stats) = app
        .run_window_frame_at(window, Constraints::tight(size), Instant::now())
        .unwrap()
        .unwrap();
    assert!(observed.borrow().is_empty());
    app.note_render_metrics(
        window,
        RenderFrameMetrics {
            prepare_us: 3,
            encode_us: 5,
            submit_us: 7,
            ..RenderFrameMetrics::default()
        },
        None,
    );
    let records = observed.borrow();
    assert_eq!(records.len(), 1);
    assert!(records[0].frame > 0);
    assert_eq!(
        records[0].timings.cpu_total,
        stats.timings.cpu_total.saturating_add(15)
    );
    assert_eq!(records[0].timings.build, stats.timings.build);
}

#[test]
fn native_timeline_event_preserves_exact_flutter_sample_and_units() {
    use incular_runtime::{FrameTimelineEvent, NativeFrameTiming};
    let size = Size::new(100., 100.);
    let mut app = Application::new(move |_| Widget::box_(size, Color::WHITE)).unwrap();
    let callbacks = app.frame_timing_callbacks();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _listen = callbacks.add_timeline({
        let seen = seen.clone();
        move |event| seen.borrow_mut().push(event)
    });
    let window = app.primary_window();
    app.run_window_frame_at(window, Constraints::tight(size), Instant::now())
        .unwrap();
    app.note_render_metrics(window, RenderFrameMetrics::default(), None);
    assert!(
        seen.borrow().is_empty(),
        "ordinary metrics have no raw timestamps"
    );

    let timing = NativeFrameTiming {
        frame_number: 1991,
        vsync_start_us: 5000,
        build_start_us: 10000,
        build_finish_us: 15000,
        raster_start_us: 16000,
        raster_finish_us: 20000,
    };
    app.report_native_frame_timings(&[timing]);
    assert_eq!(seen.borrow().len(), 1);
    assert_eq!(FrameTimelineEvent::NAME, "Incular.Frame");
    assert_eq!(
        serde_json::to_value(seen.borrow()[0]).unwrap(),
        serde_json::json!({
            "number": 1991,
            "startTime": 10000,
            "elapsed": 15000,
            "build": 5000,
            "raster": 4000,
            "vsyncOverhead": 5000
        })
    );

    // A missing/reordered timestamp is not a synthetic fast frame.
    app.report_native_frame_timings(&[NativeFrameTiming {
        raster_start_us: 1000,
        ..timing
    }]);
    assert_eq!(seen.borrow().len(), 1);
    app.report_native_frame_timings(&[
        NativeFrameTiming {
            frame_number: 1992,
            ..timing
        },
        NativeFrameTiming {
            frame_number: 1993,
            ..timing
        },
    ]);
    assert_eq!(
        seen.borrow()
            .iter()
            .map(|frame| frame.number)
            .collect::<Vec<_>>(),
        [1991, 1992, 1993]
    );
}
