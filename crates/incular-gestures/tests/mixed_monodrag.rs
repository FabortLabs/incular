//! Pinned Flutter 3.50.0 drag tests for a single tenure across pointer protocols.
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};

use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase};
use incular_gestures::{
    DragAxis, DragCallbacks, GestureArenaClient, GestureArenaKey, GestureArenaManager,
    GestureArenaRegistration, GestureDisposition, MonodragGestureRecognizer, PointerDeviceKind,
    PointerPanZoomEvent, PointerPanZoomPhase, RawPointerEvent,
};

#[derive(Debug, PartialEq)]
enum Pan {
    Start,
    Move(Offset, PointerDeviceKind),
    End,
}

struct Contender {
    drag: RefCell<MonodragGestureRecognizer>,
    lost: RefCell<Vec<GestureArenaKey>>,
}

impl GestureArenaClient for Contender {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.drag.borrow_mut().accept_stream(key);
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        self.drag.borrow_mut().reject_stream(key);
        self.lost.borrow_mut().push(key);
    }
}

enum Event {
    Touch(RawPointerEvent),
    Trackpad(PointerPanZoomEvent),
}

struct Harness {
    arena: GestureArenaManager,
    contenders: [Rc<Contender>; 2],
    entries: HashMap<GestureArenaKey, [GestureArenaRegistration; 2]>,
    trace: Rc<RefCell<Vec<Pan>>>,
    now: Instant,
}

impl Harness {
    fn new() -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let started = trace.clone();
        let updated = trace.clone();
        let ended = trace.clone();
        let contender = |callbacks| {
            Rc::new(Contender {
                drag: RefCell::new(MonodragGestureRecognizer::new(DragAxis::Pan, callbacks)),
                lost: RefCell::new(Vec::new()),
            })
        };
        Self {
            arena: GestureArenaManager::new(),
            contenders: [
                contender(DragCallbacks {
                    on_start: Some(Rc::new(move |_| started.borrow_mut().push(Pan::Start))),
                    on_update: Some(Rc::new(move |details| {
                        assert_eq!(details.primary_delta, None);
                        updated
                            .borrow_mut()
                            .push(Pan::Move(details.delta, details.kind));
                    })),
                    on_end: Some(Rc::new(move |_| ended.borrow_mut().push(Pan::End))),
                }),
                contender(DragCallbacks {
                    on_start: Some(Rc::new(|_| panic!("the first pan must win"))),
                    ..DragCallbacks::default()
                }),
            ],
            entries: HashMap::new(),
            trace,
            now: Instant::now(),
        }
    }

    fn route(&mut self, key: GestureArenaKey, start: bool, end: bool, event: Event) {
        if start {
            let entries = self.contenders.each_ref().map(|client| {
                client.lost.borrow_mut().retain(|&lost| lost != key);
                let client: Rc<dyn GestureArenaClient> = client.clone();
                self.arena.add(key, client)
            });
            assert!(self.entries.insert(key, entries).is_none());
            self.arena.close(key);
        }
        let entries = self.entries.get(&key).unwrap();
        for (client, entry) in self.contenders.iter().zip(entries) {
            if client.lost.borrow().contains(&key) {
                continue;
            }
            let wants = {
                let mut drag = client.drag.borrow_mut();
                let handled = match &event {
                    Event::Touch(event) => drag.observe_raw(*event),
                    Event::Trackpad(event) => drag.observe(*event),
                };
                handled && drag.wants_acceptance()
            };
            if wants {
                entry.resolve(GestureDisposition::Accepted);
            } else if end {
                entry.resolve(GestureDisposition::Rejected);
            }
        }
        self.arena.flush_deferred_resolutions();
        if end {
            self.arena.sweep(key);
            self.entries.remove(&key);
        }
    }

    fn trackpad(&mut self, phase: PointerPanZoomPhase) {
        self.route(
            GestureArenaKey::pointer_pan_zoom(0, 0, 2),
            phase == PointerPanZoomPhase::Start,
            phase == PointerPanZoomPhase::End,
            Event::Trackpad(PointerPanZoomEvent {
                device: 0,
                pointer: 2,
                position: Offset::new(10., 10.),
                phase,
                time: self.now,
                synthesized: false,
            }),
        );
    }

    fn pan(&mut self, x: f32, y: f32) {
        self.trackpad(PointerPanZoomPhase::Update {
            pan: Offset::new(x, y),
            scale: 1.,
            rotation: 0.,
        });
    }

    fn touch(&mut self, x: f32, y: f32, phase: PointerPhase) {
        self.route(
            // Same device and numeric pointer ID to verify source identity.
            GestureArenaKey::pointer_device(0, 0, 2),
            phase == PointerPhase::Down,
            phase == PointerPhase::Up,
            Event::Touch(RawPointerEvent::new(
                2,
                0,
                PointerDeviceKind::Touch,
                PRIMARY_POINTER_BUTTON,
                Offset::new(x, y),
                phase,
                self.now,
            )),
        );
    }

    fn take(&self) -> Vec<Pan> {
        std::mem::take(&mut *self.trace.borrow_mut())
    }
}

#[test]
fn trackpad_drag_allows_touch_to_join_the_same_tenure() {
    let mut h = Harness::new();
    h.trackpad(PointerPanZoomPhase::Start);
    assert!(h.take().is_empty());
    h.pan(20., 20.);
    assert!(h.take().is_empty());
    h.pan(30., 30.);
    assert_eq!(h.take(), [Pan::Start]);
    h.pan(30., 25.);
    assert_eq!(
        h.take(),
        [Pan::Move(Offset::new(0., -5.), PointerDeviceKind::Trackpad)]
    );

    h.touch(20., 20., PointerPhase::Down);
    assert!(h.take().is_empty());
    h.touch(25., 25., PointerPhase::Move);
    assert_eq!(
        h.take(),
        [Pan::Move(Offset::new(5., 5.), PointerDeviceKind::Touch)]
    );
    h.touch(25., 25., PointerPhase::Up);
    assert!(h.take().is_empty());
    h.trackpad(PointerPanZoomPhase::End);
    assert_eq!(h.take(), [Pan::End]);
}

#[test]
fn touch_drag_accepts_trackpad_pan_without_waiting_for_slop() {
    let mut h = Harness::new();
    h.touch(20., 20., PointerPhase::Down);
    assert!(h.take().is_empty());
    h.touch(60., 60., PointerPhase::Move);
    assert_eq!(h.take(), [Pan::Start]);
    h.touch(70., 70., PointerPhase::Move);
    assert_eq!(
        h.take(),
        [Pan::Move(Offset::new(10., 10.), PointerDeviceKind::Touch)]
    );

    h.trackpad(PointerPanZoomPhase::Start);
    assert!(h.take().is_empty());
    h.pan(20., 20.);
    assert_eq!(
        h.take(),
        [Pan::Move(
            Offset::new(20., 20.),
            PointerDeviceKind::Trackpad
        )]
    );
    h.pan(30., 30.);
    assert_eq!(
        h.take(),
        [Pan::Move(
            Offset::new(10., 10.),
            PointerDeviceKind::Trackpad
        )]
    );
    h.trackpad(PointerPanZoomPhase::End);
    assert!(h.take().is_empty());
    h.touch(70., 70., PointerPhase::Up);
    assert_eq!(h.take(), [Pan::End]);
}

#[test]
fn after_latest_pointer_lifts_the_first_resumes_from_its_own_last_position() {
    let mut h = Harness::new();
    h.touch(20., 20., PointerPhase::Down);
    h.touch(70., 70., PointerPhase::Move);
    assert_eq!(h.take(), [Pan::Start]);
    h.trackpad(PointerPanZoomPhase::Start);
    h.touch(90., 90., PointerPhase::Move);
    assert!(h.take().is_empty());
    h.trackpad(PointerPanZoomPhase::End);
    h.touch(96., 97., PointerPhase::Move);
    assert_eq!(
        h.take(),
        [Pan::Move(Offset::new(6., 7.), PointerDeviceKind::Touch)]
    );
    h.touch(96., 97., PointerPhase::Up);
    assert_eq!(h.take(), [Pan::End]);
}
