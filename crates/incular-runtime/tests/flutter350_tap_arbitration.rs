use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, GestureDetector, HitTestBehavior, Widget};

#[test]
fn retained_tap_up_waits_for_a_double_tap_competitor_to_leave_the_arena() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let record = |event| {
        let log = log.clone();
        move || log.borrow_mut().push(event)
    };
    let down = record("down");
    let up = record("up");
    let scene = Center::new(
        GestureDetector::new(Widget::box_(Size::new(120., 120.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(move |_| down())
            .on_tap_up(move |_| up())
            .on_tap(record("single"))
            .on_tap_cancel(record("cancel"))
            .on_double_tap(record("double")),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(200., 200.));
    let base = Instant::now();
    let at = |ms| base + Duration::from_millis(ms);
    runtime.run_frame_at(bounds, base).unwrap();
    let route = |runtime: &mut Runtime, pointer, phase, time| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                position: Offset::new(100., 100.),
                sample: PointerSampleMetadata::EMPTY,
            },
            at(time),
        );
    };
    let press = |runtime: &mut Runtime, pointer, from| {
        route(runtime, pointer, PointerPhase::Down, from);
        route(runtime, pointer, PointerPhase::Up, from + 10);
    };
    press(&mut runtime, 1, 0);
    assert!(
        log.borrow().is_empty(),
        "held tap reported {:?}",
        *log.borrow()
    );
    runtime.run_frame_at(bounds, at(110)).unwrap();
    assert!(log.borrow().is_empty());
    runtime.run_frame_at(bounds, at(310)).unwrap();
    assert_eq!(*log.borrow(), ["down", "up", "single"]);

    log.borrow_mut().clear();
    press(&mut runtime, 2, 320);
    press(&mut runtime, 3, 370);
    assert_eq!(*log.borrow(), ["double"]);
    runtime.run_frame_at(bounds, at(730)).unwrap();
    assert_eq!(*log.borrow(), ["double"]);

    log.borrow_mut().clear();
    route(&mut runtime, 4, PointerPhase::Down, 740);
    runtime.run_frame_at(bounds, at(850)).unwrap();
    assert_eq!(*log.borrow(), ["down"]);
    route(&mut runtime, 4, PointerPhase::Up, 850);
    assert_eq!(*log.borrow(), ["down"]);
    runtime.run_frame_at(bounds, at(1150)).unwrap();
    assert_eq!(*log.borrow(), ["down", "up", "single"]);

    log.borrow_mut().clear();
    route(&mut runtime, 5, PointerPhase::Down, 1160);
    runtime.run_frame_at(bounds, at(1270)).unwrap();
    route(&mut runtime, 5, PointerPhase::Up, 1270);
    assert_eq!(*log.borrow(), ["down"]);
    press(&mut runtime, 6, 1310);
    assert_eq!(*log.borrow(), ["down", "double"]);
}

struct TouchTapFixture {
    runtime: Runtime,
    trace: Rc<RefCell<Vec<&'static str>>>,
    drag_started: Rc<Cell<bool>>,
    base: Instant,
}

impl TouchTapFixture {
    fn new(compete_with_drag: bool) -> Self {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let record = |label| {
            let trace = trace.clone();
            move || trace.borrow_mut().push(label)
        };
        let down = record("down");
        let up = record("up");
        let mut target = GestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(move |_| down())
            .on_tap_up(move |_| up())
            .on_tap(record("tap"))
            .on_tap_cancel(record("cancel"));
        let drag_started = Rc::new(Cell::new(false));
        if compete_with_drag {
            let started = drag_started.clone();
            target = target.on_horizontal_drag_start(move |_| started.set(true));
        }
        let base = Instant::now();
        let mut fixture = Self {
            runtime: Runtime::new(target.into()).unwrap(),
            trace,
            drag_started,
            base,
        };
        fixture.pump(0);
        fixture
    }

    fn route(&mut self, pointer: u64, phase: PointerPhase, x: f32, y: f32, ms: u64) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                position: Offset::new(x, y),
                sample: PointerSampleMetadata::EMPTY,
            },
            self.base + Duration::from_millis(ms),
        );
    }

    fn pump(&mut self, ms: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(200., 200.)),
                self.base + Duration::from_millis(ms),
            )
            .unwrap();
    }
}

#[test]
fn slop_rejection_before_the_deadline_does_not_emit_tap_cancel() {
    use PointerPhase::{Down, Move, Up};
    let mut fixture = TouchTapFixture::new(true);
    fixture.route(3, Down, 10., 10., 0);
    fixture.route(3, Move, 25., 25., 1);
    fixture.route(3, Up, 25., 25., 2);
    fixture.pump(201);
    assert!(fixture.trace.borrow().is_empty());
}

#[test]
fn a_secondary_contact_cannot_fire_a_stale_primary_tap_timer() {
    use PointerPhase::{Down, Up};
    let mut fixture = TouchTapFixture::new(false);
    fixture.route(1, Down, 10., 10., 0);
    fixture.route(2, Down, 30., 30., 1);
    // A mounted detector resolves the sole candidate between input samples.
    assert_eq!(*fixture.trace.borrow(), ["down"]);
    fixture.route(1, Up, 11., 9., 2);
    assert_eq!(*fixture.trace.borrow(), ["down", "up", "tap"]);
    fixture.trace.borrow_mut().clear();
    fixture.pump(202);
    assert!(fixture.trace.borrow().is_empty());
    fixture.route(2, Up, 31., 29., 203);
    assert!(fixture.trace.borrow().is_empty());
}

#[test]
fn a_second_finger_does_not_restart_a_tap_rejected_by_an_active_drag() {
    use PointerPhase::{Down, Move, Up};
    let mut fixture = TouchTapFixture::new(true);
    fixture.route(1, Down, 0., 0., 0);
    fixture.route(1, Move, 50., 0., 1);
    assert!(fixture.drag_started.get());
    fixture.route(2, Down, 10., 20., 2);
    assert!(fixture.trace.borrow().is_empty());
    fixture.pump(203);
    fixture.route(2, Up, 10., 20., 204);
    fixture.route(1, Up, 50., 0., 205);
    assert!(fixture.trace.borrow().is_empty());

    fixture.route(3, Down, 10., 20., 206);
    fixture.route(3, Up, 10., 20., 207);
    assert_eq!(*fixture.trace.borrow(), ["down", "up", "tap"]);
}
