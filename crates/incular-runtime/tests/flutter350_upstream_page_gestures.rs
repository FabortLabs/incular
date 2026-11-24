//! Time-controlled touch and fling traces for the pinned PageView originals.
use incular_config::Constraints;
use incular_core::DragStartBehavior;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Container, Directionality, GestureDetector, PageView, ScrollPhysics, SizedBox, Text,
    TextDirection, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};
const VIEW: u64 = 98;
type Log = Rc<RefCell<Vec<usize>>>;
fn view(size: Size, taps: Option<&Log>, changes: Option<&Log>, bounce: bool, down: bool) -> Widget {
    let children = (0..15).map(|i| {
        let box_ = Container::new()
            .height(200.)
            .color(Color::rgba(0, 0, 255, 255))
            .child(Text::new(i.to_string()));
        let w = if let Some(log) = taps {
            let log = log.clone();
            Widget::from(GestureDetector::new(box_).on_tap(move || log.borrow_mut().push(i)))
        } else {
            box_.into()
        };
        w.with_key(Key::Value(i as u64))
    });
    let mut v = PageView::new(children);
    if bounce {
        v = v.physics(ScrollPhysics::BOUNCING);
    }
    if down {
        v = v.drag_start_behavior(DragStartBehavior::Down);
    }
    if let Some(log) = changes {
        let log = log.clone();
        v = v.on_page_changed(move |i| log.borrow_mut().push(i));
    }
    let w = Widget::from(v).with_key(Key::Value(VIEW));
    Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::from_size(size).child(w)),
    )
    .into()
}
struct Host {
    r: Runtime,
    now: Instant,
    pointer: u64,
}
impl Host {
    fn new(w: Widget) -> Self {
        let mut h = Self {
            r: Runtime::new(w).unwrap(),
            now: Instant::now(),
            pointer: 0,
        };
        h.frame();
        h
    }
    fn frame(&mut self) {
        self.r
            .run_frame_at(Constraints::tight(SURFACE), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn settle(&mut self) {
        for _ in 0..120 {
            self.frame();
        }
    }
    fn update(&mut self, w: Widget) {
        let root = self.r.tree().root().unwrap();
        self.r.schedule_update(root, w).unwrap();
        self.frame();
    }
    fn bounds(&self, i: u64) -> Option<Rect> {
        let t = self.r.tree();
        t.element_with_key(&Key::Value(i))
            .and_then(|e| t.element_bounds(e))
    }
    fn visible(&self) -> Vec<u64> {
        let v = self.bounds(VIEW).unwrap();
        (0..15)
            .filter(|i| {
                self.bounds(*i).is_some_and(|r| {
                    r.origin.x + r.size.width > v.origin.x && r.origin.x < v.origin.x + v.size.width
                })
            })
            .collect()
    }
    fn send(&mut self, phase: PointerPhase, x: f32, y: f32, delay: u64) {
        if phase == PointerPhase::Down {
            self.pointer += 1;
        }
        self.r.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: self.pointer,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: Offset::new(x, y),
            },
            self.now,
        );
        self.now += Duration::from_millis(delay);
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.send(PointerPhase::Down, x, y, 16);
        self.send(PointerPhase::Up, x, y, 16);
        self.frame();
    }
    fn drag(&mut self, delta: f32, fling: bool) {
        let start = 400.;
        self.send(PointerPhase::Down, start, 300., if fling { 20 } else { 16 });
        let steps = if fling { 10 } else { 4 };
        for i in 1..=steps {
            self.send(
                PointerPhase::Move,
                start + delta * (i as f32 / steps as f32),
                300.,
                if fling { 20 } else { 16 },
            );
        }
        if !fling {
            self.now += Duration::from_millis(180);
        }
        self.send(PointerPhase::Up, start + delta, 300., 0);
        self.frame();
    }
}
#[test]
fn page_view_taps_drags_and_flings_select_expected_pages() {
    let taps = Rc::new(RefCell::new(Vec::new()));
    let mut h = Host::new(view(SURFACE, Some(&taps), None, false, true));
    h.tap(400., 300.);
    let mut first = vec![h.visible()];
    h.drag(-20., false);
    first.push(h.visible());
    h.settle();
    first.push(h.visible());
    h.drag(-401., false);
    h.settle();
    first.push(h.visible());
    h.tap(400., 300.);
    h.drag(-200., true);
    h.settle();
    first.push(h.visible());
    h.drag(200., true);
    h.settle();
    first.push(h.visible());
    assert_eq!(
        (first, taps.borrow().clone()),
        (
            vec![vec![0], vec![0, 1], vec![0], vec![1], vec![2], vec![1]],
            vec![0, 1]
        )
    );
}
#[test]
fn controllerless_page_stays_on_selected_page_across_two_resizes() {
    let build = |w, h| view(Size::new(w, h), None, None, false, false);
    let mut host = Host::new(build(600., 400.));
    let mut actual = vec![host.visible()];
    host.drag(-1250., false);
    host.settle();
    actual.push(host.visible());
    host.update(build(250., 100.));
    actual.push(host.visible());
    host.update(build(450., 400.));
    actual.push(host.visible());
    assert_eq!(actual, [vec![0], vec![2], vec![2], vec![2]]);
}
#[test]
fn bouncing_page_size_never_squishes_when_direction_reverses() {
    let actual = [-200., -500.].map(|return_delta| {
        let mut h = Host::new(view(SURFACE, None, None, true, false));
        let base = h.bounds(0);
        h.drag(100., false);
        let stretched = h.bounds(0);
        h.drag(return_delta, false);
        (base, stretched, h.bounds(0))
    });
    let mut mismatches = Vec::new();
    for (return_delta, (initial, stretched, reversed)) in [-200., -500.].into_iter().zip(actual) {
        let good = initial == Some(Rect::from_origin_size(Offset::ZERO, SURFACE))
            && stretched.is_some_and(|b| b.size == SURFACE && b.origin.x > 0.)
            && reversed.is_some_and(|b| b.size == SURFACE && b.origin.x < 0.);
        if !good {
            mismatches.push((return_delta, initial, stretched, reversed));
        }
    }
    assert!(
        mismatches.is_empty(),
        "initial, over, return at each distance: {mismatches:?}"
    );
}
#[test]
fn on_page_changed_fires_once_at_halfway_during_a_live_drag() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut h = Host::new(view(SURFACE, None, Some(&log), false, false));
    assert!(log.borrow().is_empty());
    h.send(PointerPhase::Down, 100., 100., 16);
    h.send(PointerPhase::Move, -280., 100., 16);
    let short = log.borrow().clone();
    h.send(PointerPhase::Move, -320., 100., 16);
    let crossed = log.borrow().clone();
    h.frame();
    let after_frame = log.borrow().clone();
    for x in [-360., -400., -440., -480.] {
        h.send(PointerPhase::Move, x, 100., 16);
        h.frame();
    }
    h.now += Duration::from_millis(180);
    h.send(PointerPhase::Up, -480., 100., 0);
    h.settle();
    assert_eq!(
        (
            short,
            crossed,
            after_frame,
            log.borrow().clone(),
            h.visible()
        ),
        (vec![], vec![1], vec![1], vec![1], vec![1])
    );
}
