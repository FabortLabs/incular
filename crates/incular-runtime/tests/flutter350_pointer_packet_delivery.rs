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
