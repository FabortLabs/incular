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
    let mut h = Harness::new(DragStartBehavior::Down);
    h.start();
    assert!(h.log.borrow().start.is_empty());
    let pan = Offset::new(20., 30.);
    assert_eq!(h.update(Offset::ZERO, pan, 1., 0.), None);
    assert!(h.scale.wants_acceptance());
    let first = h.scale.accept().unwrap();
    assert_eq!(first.focal_point, pan);
    assert_eq!(first.focal_point_delta, pan);
    assert_eq!(first.pointer_count, 2);
    assert_eq!(
        h.log.borrow().start[0].kind,
        Some(PointerDeviceKind::Trackpad)
    );
    for scale in [2., 1.] {
        let d = h.update(Offset::ZERO, pan, scale, 0.).unwrap();
        assert_eq!(
            (d.scale, d.horizontal_scale, d.vertical_scale),
            (scale, scale, scale)
        );
        assert_eq!(d.focal_point, pan);
        assert_eq!(d.focal_point_delta, Offset::ZERO);
        assert_eq!(d.pointer_count, 2);
    }
    h.end();
    assert_eq!(h.log.borrow().start.len(), 1);
    assert_eq!(h.log.borrow().end[0].pointer_count, 0);
    assert_eq!(h.scale.pointer_count(), 0);
}

#[test]
fn pan_zoom_drag_start_behavior_rebases_scale_and_rotation() {
    let mut h = Harness::new(DragStartBehavior::Start);
    h.start();
    assert_eq!(h.update(Offset::ZERO, Offset::ZERO, 1.04, 0.5), None);
    assert!(!h.scale.wants_acceptance());
    assert_eq!(h.update(Offset::ZERO, Offset::ZERO, 1.1, 1.), None);
    assert!(h.scale.wants_acceptance());
    let d = h.scale.accept().unwrap();
    assert_eq!((d.scale, d.rotation), (1., 0.));
    for (absolute, rotation, relative, angle) in [(1.21, 1.5, 1.1, 0.5), (0.99, 1., 0.9, 0.)] {
        let d = h
            .update(Offset::ZERO, Offset::ZERO, absolute, rotation)
            .unwrap();
        close(d.scale, relative);
        close(d.horizontal_scale, relative);
        close(d.vertical_scale, relative);
        close(d.rotation, angle);
        assert_eq!(d.focal_point_delta, Offset::ZERO);
        assert_eq!(d.focal_point, Offset::ZERO);
    }
    h.end();
    assert_eq!(h.log.borrow().start.len(), 1);
    assert_eq!(h.log.borrow().end.len(), 1);
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
