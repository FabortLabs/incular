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

#[test]
fn toggling_snapping_does_not_reset_active_page_or_duplicate_callbacks() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let build = |snapping| {
        let log = log.clone();
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(
                (0..15).map(|i| Widget::from(Text::new(i.to_string())).with_key(Key::Value(i))),
            )
            .page_snapping(snapping)
            .on_page_changed(move |i| log.borrow_mut().push(i)),
        )
        .into()
    };
    // Keep the same root fixture key that the hit/viewport helpers query.
    let build = |snapping| {
        let w: Widget = build(snapping);
        w.with_key(Key::Value(VIEW))
    };
    let mut h = Host::new(build(true));
    assert!(log.borrow().is_empty());
    let swipe = |h: &mut Host| {
        h.send(PointerPhase::Down, 100., 100., 16);
        h.send(PointerPhase::Move, -320., 100., 16);
        let now = std::mem::take(&mut *log.borrow_mut());
        h.now += Duration::from_millis(180);
        h.send(PointerPhase::Up, -320., 100., 0);
        h.settle();
        (now, h.visible(), std::mem::take(&mut *log.borrow_mut()))
    };
    let snapped = swipe(&mut h);
    h.update(build(false));
    let without_snap = swipe(&mut h);
    h.update(build(true));
    h.settle();
    assert_eq!(
        (snapped, without_snap, h.visible(), log.borrow().clone()),
        (
            (vec![1], vec![1], vec![]),
            (vec![2], vec![1, 2], vec![]),
            vec![2],
            vec![]
        )
    );
}

#[test]
fn bouncing_fling_past_first_page_settles_without_an_extra_callback_at_both_sizes() {
    use incular_widgets::{OverflowBox, PageController};
    let c = PageController::new().with_viewport_fraction(0.9);
    let log = Rc::new(RefCell::new(Vec::new()));
    let view = |overflow: bool| {
        let l = log.clone();
        let v = PageView::new(
            (0..15).map(|i| Widget::from(Text::new(i.to_string())).with_key(Key::Value(i))),
        )
        .controller(c.clone())
        .on_page_changed(move |page| l.borrow_mut().push(page))
        .physics(ScrollPhysics::BOUNCING);
        let v = Widget::from(v).with_key(Key::Value(VIEW));
        let body = if overflow {
            Widget::from(
                OverflowBox::new(v)
                    .min_width(414.)
                    .max_width(414.)
                    .min_height(736.)
                    .max_height(736.),
            )
        } else {
            v
        };
        Directionality::new(TextDirection::Ltr, body).into()
    };
    let mut h = Host::new(view(false));
    let fling = |h: &mut Host| {
        h.send(PointerPhase::Down, 400., 300., 25);
        for x in [420., 440., 460., 480., 500.] {
            h.send(PointerPhase::Move, x, 300., 25);
        }
        h.send(PointerPhase::Up, 500., 300., 0);
        h.settle();
        (
            h.bounds(VIEW).expect("viewport").size,
            h.visible(),
            c.page(),
            log.borrow().clone(),
        )
    };
    let a = fling(&mut h);
    h.update(view(true));
    let b = fling(&mut h);
    assert_eq!(
        [a, b],
        [
            (SURFACE, vec![0, 1], Some(0.), vec![]),
            (Size::new(414., 736.), vec![0, 1], Some(0.), vec![])
        ]
    );
}

#[test]
fn reusing_one_page_descriptor_does_not_link_mounted_controllers() {
    let widget = view(SURFACE, None, None, false, false);
    let mut first = Host::new(widget.clone());
    let mut second = Host::new(widget.clone());
    first.drag(-420., false);
    first.settle();
    second.frame();
    assert_eq!(first.visible(), [1]);
    assert_eq!(second.visible(), [0]);

    drop(first);
    let remounted = Host::new(widget);
    assert_eq!(remounted.visible(), [0]);
}

#[test]
fn reenabling_snapping_aligns_fractional_page_without_a_duplicate_callback() {
    use incular_widgets::PageController;

    let controller = PageController::new().with_viewport_fraction(0.75);
    let log = Rc::new(RefCell::new(Vec::new()));
    let build = |snapping| {
        let log = log.clone();
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(
                (0..5).map(|i| Widget::from(Text::new(i.to_string())).with_key(Key::Value(i))),
            )
            .controller(controller.clone())
            .page_snapping(snapping)
            .on_page_changed(move |page| log.borrow_mut().push(page)),
        )
        .into()
    };

    let mut host = Host::new(build(false));
    host.send(PointerPhase::Down, 400., 300., 16);
    for x in [370., 130., -110., -350., -590.] {
        host.send(PointerPhase::Move, x, 300., 16);
    }
    host.now += Duration::from_millis(180);
    host.send(PointerPhase::Up, -590., 300., 0);
    host.settle();
    let before = controller.page().unwrap();
    assert!(
        (1.5..2.).contains(&before),
        "fractional page before snapping: {before}"
    );
    assert_eq!(&*log.borrow(), &[1, 2]);

    host.update(build(true));
    assert!(
        host.r.frame_requested(),
        "the newly enabled snap must wake an idle window"
    );
    host.settle();
    assert_eq!(controller.page(), Some(2.));
    assert_eq!(controller.offset(), 1200.);
    assert_eq!(&*log.borrow(), &[1, 2]);
    assert_eq!(host.bounds(2).unwrap().origin.x, 100.);
}

#[test]
fn new_controller_continues_unsnapped_fractional_position_instead_of_its_initial_page() {
    use incular_widgets::PageController;

    let first = PageController::new().with_viewport_fraction(0.875);
    let second = PageController::new()
        .with_initial_page(4)
        .with_viewport_fraction(0.75);
    let build = |controller: &PageController| {
        Directionality::new(
            TextDirection::Ltr,
            PageView::new((0..6).map(|i| Text::new(i.to_string())))
                .controller(controller.clone())
                .page_snapping(false),
        )
        .into()
    };

    let mut host = Host::new(build(&first));
    assert!(first.jump_to(1260.));
    host.frame();
    assert_eq!(first.page(), Some(1.8));

    host.update(build(&second));
    assert!(first.metric_owner().is_none());
    assert!(second.metric_owner().is_some());
    assert_eq!(second.page(), Some(1.8));
    assert_eq!(second.offset(), 1080.);
    host.frame();
    assert_eq!(second.offset(), 1080.);
}
