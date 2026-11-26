//! Absolute trackpad events, compared to the pinned Flutter scale tests.
use std::{cell::RefCell, f32::consts, rc::Rc, time::Instant};

use incular_core::{DragStartBehavior, Offset, PointerPhase};
use incular_gestures::*;

#[derive(Default)]
struct Log {
    start: Vec<ScaleStartDetails>,
    end: Vec<ScaleEndDetails>,
}

struct Harness {
    scale: ScaleGestureDetector,
    log: Rc<RefCell<Log>>,
    now: Instant,
}

impl Harness {
    fn new(behavior: DragStartBehavior) -> Self {
        let log = Rc::new(RefCell::new(Log::default()));
        let starts = log.clone();
        let ends = log.clone();
        let scale = ScaleGestureDetector::with_callbacks(
            None,
            Some(Rc::new(move |details| {
                starts.borrow_mut().start.push(details)
            })),
            Some(Rc::new(move |details| ends.borrow_mut().end.push(details))),
        )
        .with_drag_start_behavior(behavior);
        Self {
            scale,
            log,
            now: Instant::now(),
        }
    }

    fn trackpad(
        &mut self,
        position: Offset,
        phase: PointerPanZoomPhase,
    ) -> Option<ScaleUpdateDetails> {
        self.scale.observe_pan_zoom(PointerPanZoomEvent {
            device: 0,
            pointer: 4,
            position,
            phase,
            time: self.now,
            synthesized: false,
        })
    }

    fn start(&mut self) {
        assert_eq!(
            self.trackpad(Offset::ZERO, PointerPanZoomPhase::Start),
            None
        );
        assert!(!self.scale.wants_acceptance());
    }

    fn end(&mut self) {
        assert_eq!(self.trackpad(Offset::ZERO, PointerPanZoomPhase::End), None);
    }

    fn update(
        &mut self,
        origin: Offset,
        pan: Offset,
        scale: f32,
        rotation: f32,
    ) -> Option<ScaleUpdateDetails> {
        self.trackpad(
            origin,
            PointerPanZoomPhase::Update {
                pan,
                scale,
                rotation,
            },
        )
    }

    fn touch(
        &mut self,
        pointer: u64,
        position: Offset,
        phase: PointerPhase,
    ) -> Option<ScaleUpdateDetails> {
        self.scale.observe_raw(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            0,
            position,
            phase,
            self.now,
        ))
    }
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn pan_zoom_absolute_updates_do_not_accumulate_and_end_is_silent() {
    let mut h = TrackpadArena::new(DragStartBehavior::Down);
    h.start(4, Offset::ZERO);
    assert_eq!(h.counts(), (0, 0));
    let pan = Offset::new(20., 30.);
    let first = h.pan(4, Offset::ZERO, pan).unwrap();
    assert_eq!(first.focal_point, pan);
    assert_eq!(first.focal_point_delta, pan);
    assert_eq!(first.pointer_count, 2);
    assert_eq!(
        h.scale.borrow().log.borrow().start[0].kind,
        Some(PointerDeviceKind::Trackpad)
    );
    for scale in [2., 1.] {
        let d = h
            .route(
                4,
                Offset::ZERO,
                PointerPanZoomPhase::Update {
                    pan,
                    scale,
                    rotation: 0.,
                },
            )
            .unwrap();
        assert_eq!(
            (d.scale, d.horizontal_scale, d.vertical_scale),
            (scale, scale, scale)
        );
        assert_eq!(d.focal_point, pan);
        assert_eq!(d.focal_point_delta, Offset::ZERO);
        assert_eq!(d.pointer_count, 2);
    }
    h.end(4);
    assert_eq!(h.counts(), (1, 1));
    let end = h.scale.borrow().log.borrow().end[0];
    assert_eq!(end.pointer_count, 0);
    assert_eq!(h.scale.borrow().scale.pointer_count(), 0);
    assert!(h.drag_log.borrow().is_empty());
}

#[test]
fn pan_zoom_drag_start_behavior_rebases_scale_and_rotation() {
    let mut h = TrackpadArena::new(DragStartBehavior::Start);
    h.start(4, Offset::ZERO);
    let mut update = |scale, rotation| {
        h.route(
            4,
            Offset::ZERO,
            PointerPanZoomPhase::Update {
                pan: Offset::ZERO,
                scale,
                rotation,
            },
        )
    };
    assert_eq!(update(1.04, 0.5), None);
    let d = update(1.1, 1.).unwrap();
    assert_eq!((d.scale, d.rotation), (1., 0.));
    for (absolute, rotation, relative, angle) in [(1.21, 1.5, 1.1, 0.5), (0.99, 1., 0.9, 0.)] {
        let d = update(absolute, rotation).unwrap();
        close(d.scale, relative);
        close(d.horizontal_scale, relative);
        close(d.vertical_scale, relative);
        close(d.rotation, angle);
        assert_eq!(d.focal_point_delta, Offset::ZERO);
        assert_eq!(d.focal_point, Offset::ZERO);
    }
    h.end(4);
    assert_eq!(h.counts(), (1, 1));
    assert!(h.drag_log.borrow().is_empty());
}

struct SoleScale(Rc<RefCell<Harness>>);

impl GestureArenaClient for SoleScale {
    fn accept_gesture(&self, _: GestureArenaKey) {
        assert!(self.0.borrow_mut().scale.accept_without_update());
    }

    fn reject_gesture(&self, _: GestureArenaKey) {
        panic!("sole scale cannot lose its arena");
    }
}

#[test]
fn scale_trackpad_scroll_causes_scale() {
    let h = Rc::new(RefCell::new(Harness::new(DragStartBehavior::Start)));
    let arena = GestureArenaManager::new();
    let key = GestureArenaKey::pointer(0, 4);
    for (gain, pans) in [
        (
            DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR,
            [Offset::new(0., -200.), Offset::new(200., -200.)],
        ),
        (
            Offset::new(1. / 125., 0.),
            [Offset::new(125., 0.), Offset::new(125., 125.)],
        ),
    ] {
        h.borrow_mut()
            .scale
            .set_trackpad_scroll_to_scale(Some(gain));
        let previous = h.borrow().log.borrow().start.len();
        let _registration = arena.add(key, Rc::new(SoleScale(h.clone())));
        arena.close(key);
        assert_eq!(h.borrow().log.borrow().start.len(), previous);
        h.borrow_mut().start();
        arena.flush_deferred_resolutions();
        let mut h = h.borrow_mut();
        assert_eq!(h.log.borrow().start.len(), previous + 1);
        let start = h.log.borrow().start[previous];
        assert_eq!(start.focal_point, Offset::ZERO);
        assert_eq!(start.pointer_count, 2);
        for pan in pans {
            let d = h.update(Offset::ZERO, pan, 1., 0.).unwrap();
            close(d.scale, consts::E);
            assert_eq!(d.focal_point, Offset::ZERO);
            assert_eq!(d.focal_point_delta, Offset::ZERO);
            assert_eq!(d.pointer_count, 2);
        }
        h.end();
        assert_eq!(h.log.borrow().start.len(), previous + 1);
        assert_eq!(h.log.borrow().end.len(), previous + 1);
    }
}

#[test]
fn touch_span_and_rotation_combine_with_trackpad_but_count_differs_from_focal_weight() {
    let mut h = Harness::new(DragStartBehavior::Down);
    h.start();
    let forty = Offset::new(40., 40.);
    assert_eq!(h.update(Offset::ZERO, forty, 1., 0.), None);
    assert!(h.scale.wants_acceptance());
    assert_eq!(h.scale.accept().unwrap().pointer_count, 2);
    assert_eq!(h.touch(2, forty, PointerPhase::Down), None);
    assert_eq!(h.log.borrow().end.len(), 1);
    let d = h
        .touch(2, Offset::new(10., 10.), PointerPhase::Move)
        .unwrap();
    assert_eq!(d.focal_point, Offset::new(25., 25.));
    assert_eq!(d.focal_point_delta, Offset::new(-15., -15.));
    assert_eq!((d.scale, d.pointer_count), (1., 3));
    assert!(h.scale.wants_acceptance());
    assert_eq!(h.scale.accept(), None);
    assert_eq!(h.touch(3, Offset::new(10., 40.), PointerPhase::Down), None);
    assert_eq!(h.log.borrow().end.len(), 2);
    let d = h.touch(3, forty, PointerPhase::Move).unwrap();
    assert_eq!(d.focal_point, Offset::new(30., 30.));
    assert_eq!(d.focal_point_delta, Offset::new(10., 0.));
    close(d.scale, consts::SQRT_2);
    assert_eq!((d.horizontal_scale, d.vertical_scale), (1., 1.));
    close(d.rotation, -consts::FRAC_PI_4);
    assert_eq!(d.pointer_count, 4);
    assert!(h.scale.wants_acceptance());
    assert_eq!(h.scale.accept(), None);
    for (origin, pan, focal, delta) in [
        (Offset::ZERO, forty, Offset::new(30., 30.), Offset::ZERO),
        (
            Offset::new(15., 15.),
            Offset::new(55., 55.),
            forty,
            Offset::new(10., 10.),
        ),
    ] {
        let d = h
            .update(origin, pan, consts::SQRT_2, consts::FRAC_PI_3)
            .unwrap();
        assert_eq!(d.focal_point, focal);
        assert_eq!(d.focal_point_delta, delta);
        close(d.scale, 2.);
        close(d.horizontal_scale, consts::SQRT_2);
        close(d.vertical_scale, consts::SQRT_2);
        close(d.rotation, consts::PI / 12.);
        assert_eq!(d.pointer_count, 4);
    }
    h.end();
    assert_eq!(h.touch(2, Offset::ZERO, PointerPhase::Up), None);
    assert_eq!(h.touch(3, Offset::ZERO, PointerPhase::Up), None);
    assert_eq!(h.log.borrow().start.len(), 3);
    assert_eq!(h.log.borrow().end.len(), 3);
    assert_eq!(h.scale.pointer_count(), 0);
}

#[test]
fn scrolling_reports_the_scale_velocity_not_a_pan_fling() {
    let h = Rc::new(RefCell::new(Harness::new(DragStartBehavior::Start)));
    h.borrow_mut()
        .scale
        .set_trackpad_scroll_to_scale(Some(DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR));
    let arena = GestureArenaManager::new();
    let key = GestureArenaKey::pointer(0, 4);
    let _registration = arena.add(key, Rc::new(SoleScale(h.clone())));
    arena.close(key);
    assert!(h.borrow().log.borrow().start.is_empty());
    h.borrow_mut().start();
    arena.flush_deferred_resolutions();

    let mut h = h.borrow_mut();
    assert_eq!(h.log.borrow().start.len(), 1);
    assert_eq!(h.log.borrow().start[0].focal_point, Offset::ZERO);
    assert!(h.log.borrow().end.is_empty());
    let start = h.now;
    for i in 0..100 {
        h.now = start + std::time::Duration::from_millis(i * 25);
        let d = h
            .update(Offset::ZERO, Offset::new(0., -10. * i as f32), 1., 0.)
            .unwrap();
        assert_eq!(d.focal_point, Offset::ZERO);
    }
    h.now = start + std::time::Duration::from_millis(2500);
    h.end();
    assert_eq!(h.log.borrow().start.len(), 1);
    let log = h.log.borrow();
    assert_eq!(log.end.len(), 1);
    assert_eq!(log.end[0].velocity, Velocity::ZERO);
    assert_eq!(log.end[0].pointer_count, 0);
    close(log.end[0].scale_velocity, 281.414_55);
}

struct ScaleContender {
    scale: Rc<RefCell<Harness>>,
    first_update: Rc<RefCell<Option<ScaleUpdateDetails>>>,
}

impl GestureArenaClient for ScaleContender {
    fn accept_gesture(&self, _: GestureArenaKey) {
        let mut scale = self.scale.borrow_mut();
        *self.first_update.borrow_mut() = scale.scale.accept();
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        self.scale
            .borrow_mut()
            .scale
            .reject_device_pointer(key.pointer_device_id().unwrap(), key.pointer_id().unwrap());
    }
}

struct DragContender(Rc<RefCell<TrackpadDragRecognizer>>);

impl GestureArenaClient for DragContender {
    fn accept_gesture(&self, _: GestureArenaKey) {
        self.0.borrow_mut().accept();
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        self.0
            .borrow_mut()
            .reject(key.pointer_device_id().unwrap(), key.pointer_id().unwrap());
    }
}

struct TrackpadArena {
    scale: Rc<RefCell<Harness>>,
    drag: Rc<RefCell<TrackpadDragRecognizer>>,
    drag_log: Rc<RefCell<Vec<&'static str>>>,
    drag_starts: Rc<RefCell<Vec<DragStartDetails>>>,
    drag_updates: Rc<RefCell<Vec<DragUpdateDetails>>>,
    first_update: Rc<RefCell<Option<ScaleUpdateDetails>>>,
    arena: GestureArenaManager,
    entries: std::collections::HashMap<u64, [GestureArenaRegistration; 2]>,
}

impl TrackpadArena {
    fn new(behavior: DragStartBehavior) -> Self {
        let drag_log = Rc::new(RefCell::new(Vec::new()));
        let starts = drag_log.clone();
        let ends = drag_log.clone();
        let drag_starts = Rc::new(RefCell::new(Vec::new()));
        let drag_updates = Rc::new(RefCell::new(Vec::new()));
        let start_details = drag_starts.clone();
        let update_details = drag_updates.clone();
        let drag = TrackpadDragRecognizer::new(
            TrackpadDragAxis::Horizontal,
            DragCallbacks {
                on_start: Some(Rc::new(move |details| {
                    starts.borrow_mut().push("drag-start");
                    start_details.borrow_mut().push(details);
                })),
                on_update: Some(Rc::new(move |details| {
                    update_details.borrow_mut().push(details)
                })),
                on_end: Some(Rc::new(move |_| ends.borrow_mut().push("drag-end"))),
                ..DragCallbacks::default()
            },
        );
        Self {
            scale: Rc::new(RefCell::new(Harness::new(behavior))),
            drag: Rc::new(RefCell::new(drag)),
            drag_log,
            drag_starts,
            drag_updates,
            first_update: Rc::new(RefCell::new(None)),
            arena: GestureArenaManager::new(),
            entries: Default::default(),
        }
    }

    fn route(
        &mut self,
        pointer: u64,
        position: Offset,
        phase: PointerPanZoomPhase,
    ) -> Option<ScaleUpdateDetails> {
        let key = GestureArenaKey::pointer(0, pointer);
        if phase == PointerPanZoomPhase::Start {
            let scale = self.arena.add(
                key,
                Rc::new(ScaleContender {
                    scale: self.scale.clone(),
                    first_update: self.first_update.clone(),
                }),
            );
            let drag = self
                .arena
                .add(key, Rc::new(DragContender(self.drag.clone())));
            assert!(self.entries.insert(pointer, [scale, drag]).is_none());
            self.arena.close(key);
        }
        let event = PointerPanZoomEvent {
            device: 0,
            pointer,
            position,
            phase,
            time: self.scale.borrow().now,
            synthesized: false,
        };
        let [scale_entry, drag_entry] = self.entries.get(&pointer).unwrap();
        let (scale_update, wants_scale) = {
            let mut scale = self.scale.borrow_mut();
            let details = scale.scale.observe_pan_zoom(event);
            (details, scale.scale.wants_acceptance())
        };
        if wants_scale {
            scale_entry.resolve(GestureDisposition::Accepted);
        }
        let wants_drag = {
            let mut drag = self.drag.borrow_mut();
            drag.observe(event) && drag.wants_acceptance()
        };
        if wants_drag {
            drag_entry.resolve(GestureDisposition::Accepted);
        }
        self.arena.flush_deferred_resolutions();
        if phase == PointerPanZoomPhase::End {
            self.arena.sweep(key);
            self.entries.remove(&pointer);
        }
        self.first_update.borrow_mut().take().or(scale_update)
    }

    fn start(&mut self, pointer: u64, position: Offset) {
        assert_eq!(
            self.route(pointer, position, PointerPanZoomPhase::Start),
            None
        );
    }

    fn end(&mut self, pointer: u64) {
        assert_eq!(
            self.route(pointer, Offset::ZERO, PointerPanZoomPhase::End),
            None
        );
    }

    fn pan(&mut self, pointer: u64, position: Offset, pan: Offset) -> Option<ScaleUpdateDetails> {
        self.route(
            pointer,
            position,
            PointerPanZoomPhase::Update {
                pan,
                scale: 1.,
                rotation: 0.,
            },
        )
    }

    fn counts(&self) -> (usize, usize) {
        let scale = self.scale.borrow();
        let log = scale.log.borrow();
        (log.start.len(), log.end.len())
    }
}

#[test]
fn scale_competes_with_horizontal_drag_for_trackpad_pan() {
    let mut h = TrackpadArena::new(DragStartBehavior::Down);
    let origin1 = Offset::new(10., 10.);
    let origin2 = Offset::new(10., 20.);
    let origin3 = Offset::new(30., 30.);

    h.start(2, origin1);
    assert_eq!(h.counts(), (0, 0));
    assert!(h.drag_log.borrow().is_empty());

    let d = h.pan(2, origin1, Offset::new(10., 40.)).unwrap();
    assert_eq!(d.focal_point, Offset::new(20., 50.));
    assert_eq!(h.counts(), (1, 0));
    h.start(3, origin2);
    assert_eq!(h.counts(), (1, 1));
    let d = h.pan(3, origin2, Offset::new(20., 0.)).unwrap();
    assert_eq!(d.pointer_count, 4);
    assert_eq!(h.counts(), (2, 1));
    h.end(2);
    assert_eq!(h.counts(), (2, 2));
    h.end(3);
    assert_eq!(h.counts(), (2, 2));

    h.start(4, origin3);
    assert_eq!(h.counts(), (2, 2));
    assert!(h.pan(4, origin3, Offset::new(70., 0.)).is_some());
    assert_eq!(h.counts(), (3, 2));
    h.end(4);
    assert_eq!(h.counts(), (3, 3));
    assert!(h.drag_log.borrow().is_empty());
}

#[test]
fn early_horizontal_pan_wins_before_scale_and_zoom_cannot_steal_it() {
    let mut h = TrackpadArena::new(DragStartBehavior::Down);
    let origin = Offset::new(30., 30.);
    h.start(4, origin);
    assert_eq!(h.pan(4, origin, Offset::new(18., 0.)), None);
    assert!(h.drag_log.borrow().is_empty());
    assert_eq!(h.pan(4, origin, Offset::new(20., 0.)), None);
    assert_eq!(*h.drag_log.borrow(), ["drag-start"]);
    assert_eq!(
        h.drag_starts.borrow()[0].global_position,
        Offset::new(50., 30.)
    );
    assert_eq!(h.drag_starts.borrow()[0].kind, PointerDeviceKind::Trackpad);
    assert!(h.drag_updates.borrow().is_empty());
    assert_eq!(
        h.route(
            4,
            Offset::new(32., 40.),
            PointerPanZoomPhase::Update {
                pan: Offset::new(70., 5.),
                scale: 2.,
                rotation: 1.,
            }
        ),
        None
    );
    assert_eq!(h.counts(), (0, 0));
    let d = h.drag_updates.borrow()[0];
    assert_eq!(d.delta, Offset::new(50., 0.));
    assert_eq!(d.primary_delta, Some(50.));
    assert_eq!(d.global_position, Offset::new(102., 45.));
    h.end(4);
    assert_eq!(*h.drag_log.borrow(), ["drag-start", "drag-end"]);
}

#[test]
fn trackpad_drag_applies_axis_and_start_behavior_in_widget_coordinates() {
    let now = Instant::now();
    let downs = Rc::new(RefCell::new(Vec::new()));
    let motions = Rc::new(RefCell::new(Vec::new()));
    let data = DragCallbacks {
        on_start: Some(Rc::new({
            let downs = downs.clone();
            move |d| downs.borrow_mut().push(d)
        })),
        on_update: Some(Rc::new({
            let motions = motions.clone();
            move |d| motions.borrow_mut().push(d)
        })),
        ..DragCallbacks::default()
    };
    let event = |pointer, phase| PointerPanZoomEvent {
        device: 7,
        pointer,
        position: Offset::new(10., 20.),
        phase,
        time: now,
        synthesized: false,
    };

    let mut vertical = TrackpadDragRecognizer::new(TrackpadDragAxis::Vertical, data.clone())
        .with_drag_start_behavior(DragStartBehavior::Down);
    vertical.set_local_transform(incular_core::Transform::scale(0.5));
    assert!(vertical.observe(event(9, PointerPanZoomPhase::Start)));
    assert!(!vertical.observe(event(8, PointerPanZoomPhase::End)));
    assert!(vertical.observe(event(
        9,
        PointerPanZoomPhase::Update {
            pan: Offset::new(40., 18.),
            scale: 1.,
            rotation: 0.,
        }
    )));
    assert!(!vertical.wants_acceptance());
    assert!(vertical.observe(event(
        9,
        PointerPanZoomPhase::Update {
            pan: Offset::new(40., 20.),
            scale: 1.,
            rotation: 0.,
        }
    )));
    assert!(vertical.wants_acceptance());
    vertical.accept();
    assert_eq!(downs.borrow()[0].local_position, Offset::new(5., 10.));
    assert_eq!(downs.borrow()[0].source_time, Some(now));
    let d = motions.borrow()[0];
    assert_eq!(d.primary_delta, Some(10.));
    assert_eq!(d.delta, Offset::new(0., 10.));
    assert_eq!(d.local_position, Offset::new(5., 20.));
    assert_eq!(d.global_position, Offset::new(10., 40.));

    let mut pan = TrackpadDragRecognizer::new(TrackpadDragAxis::Pan, data);
    assert!(pan.observe(event(10, PointerPanZoomPhase::Start)));
    // 20 out then 20 back: the axis-free pan counts path length, not
    // displacement, so it wins even though the net pan is now zero.
    for (offset, wants) in [(Offset::new(20., 0.), false), (Offset::ZERO, true)] {
        assert!(pan.observe(event(
            10,
            PointerPanZoomPhase::Update {
                pan: offset,
                scale: 1.,
                rotation: 0.,
            }
        )));
        assert_eq!(pan.wants_acceptance(), wants);
    }
    pan.accept();
    assert_eq!(downs.borrow()[1].global_position, Offset::new(10., 20.));
    assert_eq!(motions.borrow().len(), 1);
}

#[test]
fn rejecting_a_touch_twice_does_not_evict_an_absolute_stream_with_the_same_identity() {
    let mut h = Harness::new(DragStartBehavior::Down);
    h.start();
    assert_eq!(h.touch(4, Offset::ZERO, PointerPhase::Down), None);
    assert_eq!(h.scale.pointer_count(), 3);
    for _ in 0..2 {
        h.scale.reject_device_pointer(0, 4);
        assert_eq!(h.scale.pointer_count(), 2);
    }
    assert_eq!(h.update(Offset::ZERO, Offset::ZERO, 2., 0.), None);
    assert!(h.scale.wants_acceptance());
    assert_eq!(h.scale.accept().unwrap().scale, 2.);
    h.end();
    assert_eq!(h.scale.pointer_count(), 0);
}
