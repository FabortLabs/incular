use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::internal::{PointerData, PointerDataChange as Data};
use incular_widgets::{GestureDetector, HitTestBehavior, Listener, RawPointerEvent, SizedBox};

const VIEW: u64 = 32;
const RATIO: f64 = 2.5;
type Trace = Rc<RefCell<Vec<RawPointerEvent>>>;

struct Fixture {
    runtime: Runtime,
    now: Instant,
    hits: Trace,
    routed: Trace,
    taps: Rc<RefCell<Vec<&'static str>>>,
}

impl Fixture {
    fn new() -> Self {
        let hits = Trace::default();
        let routed = Trace::default();
        let taps = Rc::new(RefCell::new(Vec::new()));
        let record = || {
            let hits = hits.clone();
            move |event| hits.borrow_mut().push(event)
        };
        let tap = {
            let taps = taps.clone();
            move || taps.borrow_mut().push("tap")
        };
        let content = GestureDetector::new(SizedBox::expand())
            .behavior(HitTestBehavior::Opaque)
            .on_tap(tap);
        let listener = Listener::new(content)
            .behavior(HitTestBehavior::Opaque)
            .on_pointer_down(record())
            .on_pointer_up(record())
            .on_pointer_move(record())
            .on_pointer_hover(record())
            .on_pointer_cancel(record());
        let mut runtime = Runtime::new(listener.into()).unwrap();
        let now = Instant::now();
        runtime
            .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
            .unwrap();
        runtime.pointer_router().add_global_route(None, {
            let routed = routed.clone();
            move |sample| routed.borrow_mut().push(sample.event)
        });
        Self {
            runtime,
            now,
            hits,
            routed,
            taps,
        }
    }

    fn packet(&mut self, changes: &[Data]) {
        let now = self.now;
        self.runtime.handle_pointer_data_packet(
            VIEW,
            RATIO,
            changes
                .iter()
                .copied()
                .enumerate()
                .map(move |(i, change)| PointerData {
                    view_id: VIEW,
                    pointer: 7,
                    device: 24,
                    physical_position: Offset::new(25., 50.),
                    ..PointerData::new(change, now + Duration::from_millis(i as u64))
                }),
        );
    }

    fn hit_phases(&self) -> Vec<PointerPhase> {
        self.hits.borrow().iter().map(|event| event.phase).collect()
    }

    fn routed_phases(&self) -> Vec<PointerPhase> {
        self.routed
            .borrow()
            .iter()
            .map(|event| event.phase)
            .collect()
    }
}

#[test]
fn down_and_up_reach_the_mounted_listener_and_tap_recognizer_in_order() {
    let mut f = Fixture::new();
    f.packet(&[Data::Down, Data::Up]);
    assert_eq!(f.hit_phases(), [PointerPhase::Down, PointerPhase::Up]);
    assert_eq!(f.routed_phases(), f.hit_phases());
    assert_eq!(*f.taps.borrow(), ["tap"]);
    for event in &*f.hits.borrow() {
        assert_eq!(event.button, Some(PRIMARY_POINTER_BUTTON));
        assert_eq!(event.position, Offset::new(10., 20.));
    }
}

#[test]
fn packet_motion_uses_the_hit_path_from_press_and_the_measured_logical_delta() {
    let mut f = Fixture::new();
    f.packet(&[Data::Down]);
    let moved = PointerData {
        view_id: VIEW,
        pointer: 7,
        device: 24,
        physical_position: Offset::new(550., 600.),
        physical_delta: Offset::new(30., -10.),
        synthesized: true,
        ..PointerData::new(Data::Move, f.now + Duration::from_millis(10))
    };
    let release = PointerData {
        change: Data::Up,
        physical_delta: Offset::new(12., 14.),
        time: f.now + Duration::from_millis(20),
        ..moved
    };
    f.runtime
        .handle_pointer_data_packet(VIEW, RATIO, [moved, release]);
    assert_eq!(
        f.hit_phases(),
        [PointerPhase::Down, PointerPhase::Move, PointerPhase::Up]
    );
    let hits = f.hits.borrow();
    assert_eq!(hits[1].position, Offset::new(220., 240.));
    assert_eq!(hits[1].delta, Offset::new(12., -4.));
    assert_eq!(hits[1].local_delta, hits[1].delta);
    assert!(hits[1].synthesized);
    assert_eq!(hits[2].delta, Offset::ZERO);
    assert!(f.taps.borrow().is_empty());
}

#[test]
fn add_remove_packets_only_reach_the_global_router_while_hover_hits_the_listener() {
    let mut f = Fixture::new();
    f.packet(&[
        Data::Add,
        Data::Hover,
        Data::Hover,
        Data::Remove,
        Data::Add,
        Data::Hover,
    ]);
    assert_eq!(f.hit_phases(), [PointerPhase::Move; 3]);
    assert!(
        f.hits
            .borrow()
            .iter()
            .all(|event| !event.down && event.buttons == 0)
    );
    assert_eq!(
        f.routed_phases(),
        [
            PointerPhase::Enter,
            PointerPhase::Move,
            PointerPhase::Move,
            PointerPhase::Exit,
            PointerPhase::Enter,
            PointerPhase::Move,
        ]
    );
}

#[test]
fn a_cancel_record_after_down_drops_the_contact_without_tapping() {
    let mut f = Fixture::new();
    f.packet(&[Data::Down, Data::Cancel]);
    assert_eq!(f.hit_phases(), [PointerPhase::Down, PointerPhase::Cancel]);
    assert!(f.taps.borrow().is_empty());
    f.packet(&[Data::Up, Data::Down, Data::Up]);
    assert_eq!(
        f.hit_phases(),
        [
            PointerPhase::Down,
            PointerPhase::Cancel,
            PointerPhase::Down,
            PointerPhase::Up
        ]
    );
    assert_eq!(*f.taps.borrow(), ["tap"]);
}

#[test]
fn an_untracked_contact_move_is_not_mistaken_for_hover_and_other_views_are_ignored() {
    let mut f = Fixture::new();
    let zero_button_mouse_motion = PointerData {
        kind: PointerDeviceKind::Mouse,
        view_id: VIEW,
        pointer: 2,
        device: 4,
        physical_position: Offset::new(25., 25.),
        ..PointerData::new(Data::Move, f.now)
    };
    f.runtime.handle_pointer_data_packet(
        VIEW,
        RATIO,
        [
            PointerData {
                change: Data::Down,
                view_id: VIEW + 1,
                ..zero_button_mouse_motion
            },
            zero_button_mouse_motion,
        ],
    );
    assert!(f.hit_phases().is_empty());
    assert!(f.routed_phases().is_empty());
}

#[test]
fn cancelling_from_the_down_callback_is_deferred_but_preempts_the_queued_up() {
    let mut f = Fixture::new();
    let canceller = f.runtime.pointer_packet_cancellation();
    let observed = f.hits.clone();
    let hook = f
        .runtime
        .pointer_router()
        .add_global_route(None, move |sample| {
            if sample.event.phase == PointerPhase::Down {
                assert!(canceller.cancel(sample.event.device, sample.event.pointer));
                assert_eq!(observed.borrow().last().unwrap().phase, PointerPhase::Down);
            }
        });
    f.packet(&[Data::Down, Data::Up]);
    assert_eq!(f.hit_phases(), [PointerPhase::Down, PointerPhase::Cancel]);
    assert_eq!(f.routed_phases(), f.hit_phases());
    assert!(f.taps.borrow().is_empty());
    assert!(f.runtime.pointer_router().remove_global_route(hook));
    f.packet(&[Data::Down, Data::Up]);
    assert_eq!(*f.taps.borrow(), ["tap"]);
}

#[test]
fn a_host_checkpoint_drains_only_live_cancellations_and_a_dropped_view_cannot_enqueue() {
    let mut f = Fixture::new();
    let canceller = f.runtime.pointer_packet_cancellation();
    f.packet(&[Data::Down]);
    assert!(canceller.cancel(24, 900));
    assert!(canceller.cancel(24, 7));
    assert!(canceller.cancel(24, 7));
    assert_eq!(f.hit_phases(), [PointerPhase::Down]);
    f.runtime.flush_pointer_packet_cancellations();
    assert_eq!(f.hit_phases(), [PointerPhase::Down, PointerPhase::Cancel]);
    f.runtime.flush_pointer_packet_cancellations();
    assert_eq!(f.routed_phases(), f.hit_phases());
    drop(f);
    assert!(!canceller.cancel(24, 7));
}

#[test]
fn packet_pan_zoom_retains_its_raw_listener_path_and_normalizes_pan_and_delta() {
    use incular_core::PointerPanZoomPhase;
    use incular_widgets::{PointerPanZoomDetails, Transform};

    let trace = Rc::new(RefCell::new(Vec::<PointerPanZoomDetails>::new()));
    let record = || {
        let trace = trace.clone();
        move |details| trace.borrow_mut().push(details)
    };
    let listener = Listener::new(SizedBox::expand())
        .behavior(HitTestBehavior::Opaque)
        .on_pointer_pan_zoom_start(record())
        .on_pointer_pan_zoom_update(record())
        .on_pointer_pan_zoom_end(record());
    let mut runtime = Runtime::new(Transform::scale(2., listener).into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(200., 200.)), now)
        .unwrap();
    let start = PointerData {
        view_id: VIEW,
        device: 25,
        pointer: 11,
        kind: PointerDeviceKind::Trackpad,
        physical_position: Offset::new(250., 250.),
        ..PointerData::new(Data::PanZoomStart, now)
    };
    let update = PointerData {
        change: Data::PanZoomUpdate {
            physical_pan: Offset::new(40., 20.),
            physical_pan_delta: Offset::new(30., -10.),
            scale: 1.2,
            rotation: 0.23,
        },
        physical_position: Offset::new(1400., 1500.),
        time: now + Duration::from_millis(10),
        synthesized: true,
        ..start
    };
    let end = PointerData {
        change: Data::PanZoomEnd,
        time: now + Duration::from_millis(20),
        ..update
    };
    runtime.handle_pointer_data_packet(VIEW, RATIO, [update]);
    assert!(trace.borrow().is_empty());
    runtime.handle_pointer_data_packet(VIEW, RATIO, [start, update, end, update, end]);
    let trace = trace.borrow();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].event.phase, PointerPanZoomPhase::Start);
    assert_eq!(
        trace[1].event.phase,
        PointerPanZoomPhase::Update {
            pan: Offset::new(16., 8.),
            scale: 1.2,
            rotation: 0.23
        }
    );
    assert_eq!(trace[2].event.phase, PointerPanZoomPhase::End);
    assert_eq!(trace[0].local_position, Offset::new(100., 100.));
    assert_eq!(trace[1].event.position, Offset::new(560., 600.));
    assert_eq!(trace[1].local_position, Offset::new(330., 350.));
    assert_eq!(trace[1].local_pan, Offset::new(58., 54.));
    assert_eq!(trace[1].pan_delta, Offset::new(12., -4.));
    assert_eq!(trace[1].local_pan_delta, Offset::new(6., -2.));
    assert!(trace[1].event.synthesized);
    assert!(
        trace
            .iter()
            .all(|sample| sample.event.device == 25 && sample.event.pointer == 11)
    );
}

#[test]
fn a_failed_packet_handler_reports_only_the_failed_down_and_stops_immediately() {
    use incular_widgets::internal::ConvertedPointerKind;

    let mut f = Fixture::new();
    let start = PointerData {
        view_id: VIEW,
        pointer: 7,
        device: 24,
        physical_position: Offset::new(25., 50.),
        ..PointerData::new(Data::Down, f.now)
    };
    let release = PointerData {
        change: Data::Up,
        ..start
    };
    let mut calls = 0;
    let failure = f
        .runtime
        .handle_pointer_data_packet_checked(VIEW, RATIO, [start, release], |_| {
            calls += 1;
            Err("zipzapzooey")
        })
        .unwrap_err();
    assert_eq!(calls, 1);
    assert!(
        matches!(failure.event.kind, ConvertedPointerKind::Contact { event, .. }
        if event.phase == PointerPhase::Down && event.pointer == 7 && event.device == 24)
    );
    let detail = failure.to_string();
    assert!(detail.contains("while handling a pointer data packet"));
    assert!(detail.contains("zipzapzooey"));
    assert!(detail.contains("Down"));
    assert!(!detail.contains("phase: Up"));
    assert!(f.hit_phases().is_empty());
    assert!(f.routed_phases().is_empty());
}

#[test]
fn after_failure_valid_queued_contacts_resume_before_new_input_without_replaying_the_error() {
    use incular_widgets::internal::ConvertedPointerKind;

    let mut f = Fixture::new();
    let down = PointerData {
        view_id: VIEW,
        pointer: 7,
        device: 24,
        physical_position: Offset::new(25., 50.),
        ..PointerData::new(Data::Down, f.now)
    };
    let up = |pointer| PointerData {
        pointer,
        change: Data::Up,
        ..down
    };
    let failure = f
        .runtime
        .handle_pointer_data_packet_checked(
            VIEW,
            RATIO,
            [down, up(7), PointerData { pointer: 8, ..down }, up(8)],
            |_| Err("intentionally failed"),
        )
        .unwrap_err();
    assert_eq!(failure.cause, "intentionally failed");
    let mut seen = Vec::new();
    f.runtime
        .handle_pointer_data_packet_checked(
            VIEW,
            RATIO,
            [PointerData { pointer: 9, ..down }, up(9)],
            |converted| {
                if let ConvertedPointerKind::Contact { event, .. } = converted.kind {
                    seen.push((event.pointer, event.phase));
                }
                Ok::<_, std::convert::Infallible>(())
            },
        )
        .unwrap();
    assert_eq!(
        seen,
        [
            (7, PointerPhase::Up),
            (8, PointerPhase::Down),
            (8, PointerPhase::Up),
            (9, PointerPhase::Down),
            (9, PointerPhase::Up),
        ]
    );
    assert_eq!(*f.taps.borrow(), ["tap", "tap"]);
    assert_eq!(
        f.routed
            .borrow()
            .iter()
            .map(|sample| sample.pointer)
            .collect::<Vec<_>>(),
        [8, 8, 9, 9]
    );
}

#[test]
fn a_native_packet_waits_for_the_warm_up_build_before_contact_dispatch() {
    let mut fixture = Fixture::new();
    assert!(fixture.runtime.schedule_warm_up_frame());
    fixture.packet(&[Data::Down, Data::Up]);
    assert!(fixture.hit_phases().is_empty());
    assert!(fixture.routed_phases().is_empty());
    let frame = fixture.now + Duration::from_millis(16);
    assert!(fixture.runtime.begin_warm_up_frame_at(frame));
    assert!(fixture.routed_phases().is_empty());
    fixture.runtime.draw_warm_up_frame(Constraints::tight(Size::new(100., 100.))).unwrap().unwrap();
    assert_eq!(fixture.hit_phases(), [PointerPhase::Down, PointerPhase::Up]);
    assert_eq!(fixture.routed_phases(), [PointerPhase::Down, PointerPhase::Up]);
    let events = fixture.routed.borrow();
    assert_eq!(events[0].time, fixture.now);
    assert_eq!(events[1].time, fixture.now + Duration::from_millis(1));
}

#[test]
fn a_cancellation_during_warm_up_keeps_its_order_between_packet_contacts() {
    let mut fixture = Fixture::new();
    let cancel = fixture.runtime.pointer_packet_cancellation();
    assert!(fixture.runtime.schedule_warm_up_frame());
    fixture.packet(&[Data::Down]);
    assert!(cancel.cancel(24, 7));
    fixture.runtime.flush_pointer_packet_cancellations();
    fixture.packet(&[Data::Up]);
    assert!(fixture.routed_phases().is_empty());
    fixture.runtime.run_frame_at(
        Constraints::tight(Size::new(200., 200.)),
        fixture.now + Duration::from_millis(16),
    ).unwrap();
    assert_eq!(fixture.routed_phases(), [PointerPhase::Down, PointerPhase::Cancel]);
    assert_eq!(fixture.hit_phases(), [PointerPhase::Down, PointerPhase::Cancel]);
    assert!(fixture.taps.borrow().is_empty());
}
