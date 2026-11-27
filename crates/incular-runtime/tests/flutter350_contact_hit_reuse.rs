use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_rendering::DisplayList;
use incular_runtime::Runtime;
use incular_widgets::{CustomPaint, CustomPainter};

#[derive(Clone)]
struct HitCounter(Rc<Cell<usize>>);

impl CustomPainter for HitCounter {
    fn paint(&self, _: Size) -> DisplayList {
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }

    fn hit_test(&self, _: Offset) -> Option<bool> {
        self.0.set(self.0.get() + 1);
        Some(false)
    }
}

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const ORIGIN: Offset = Offset::new(400., 300.);

fn host(calls: &Rc<Cell<usize>>) -> (Runtime, Instant) {
    let widget = CustomPaint::from_painter(VIEW, HitCounter(calls.clone()));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let start = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(VIEW), start)
        .unwrap();
    assert_eq!(calls.get(), 0);
    (runtime, start)
}

fn send(
    runtime: &mut Runtime,
    time: Instant,
    kind: PointerDeviceKind,
    phase: PointerPhase,
    position: Offset,
    buttons: u32,
) {
    runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind,
            position,
            phase,
            buttons,
            button: (phase == PointerPhase::Down || phase == PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
        },
        time,
    );
}

fn count(kind: PointerDeviceKind, motion: bool) -> usize {
    let calls = Rc::new(Cell::new(0));
    let (mut runtime, start) = host(&calls);
    send(
        &mut runtime,
        start + Duration::from_millis(1),
        kind,
        PointerPhase::Down,
        ORIGIN,
        PRIMARY_POINTER_BUTTON,
    );
    let current = if motion {
        let point = ORIGIN + Offset::new(1., 1.);
        send(
            &mut runtime,
            start + Duration::from_millis(2),
            kind,
            PointerPhase::Move,
            point,
            PRIMARY_POINTER_BUTTON,
        );
        point
    } else {
        ORIGIN
    };
    send(
        &mut runtime,
        start + Duration::from_millis(3),
        kind,
        PointerPhase::Up,
        current,
        0,
    );
    calls.get()
}

#[test]
fn a_mouse_click_runs_only_the_press_hit_test() {
    assert_eq!(count(PointerDeviceKind::Mouse, false), 1);
}

#[test]
fn touch_contact_motion_and_release_reuse_the_press_hit_test() {
    assert_eq!(count(PointerDeviceKind::Touch, true), 1);
}

#[test]
fn a_frame_invalidates_the_cached_mouse_press_hit() {
    let calls = Rc::new(Cell::new(0));
    let (mut runtime, start) = host(&calls);
    let at = |millis| start + Duration::from_millis(millis);
    send(
        &mut runtime,
        at(1),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        ORIGIN,
        PRIMARY_POINTER_BUTTON,
    );
    assert_eq!(calls.get(), 1);
    runtime
        .run_frame_at(Constraints::tight(VIEW), at(2))
        .unwrap();
    let before_release = calls.get();
    send(
        &mut runtime,
        at(3),
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        ORIGIN,
        0,
    );
    assert_eq!(calls.get(), before_release + 1);
}

#[test]
fn buttonless_touch_contact_motion_still_reuses_the_press_route() {
    let calls = Rc::new(Cell::new(0));
    let (mut runtime, start) = host(&calls);
    let at = |millis| start + Duration::from_millis(millis);
    let point = ORIGIN + Offset::new(1., 1.);
    send(
        &mut runtime,
        at(1),
        PointerDeviceKind::Touch,
        PointerPhase::Down,
        ORIGIN,
        PRIMARY_POINTER_BUTTON,
    );
    send(
        &mut runtime,
        at(2),
        PointerDeviceKind::Touch,
        PointerPhase::Move,
        point,
        0,
    );
    assert_eq!(calls.get(), 1);
    send(
        &mut runtime,
        at(3),
        PointerDeviceKind::Touch,
        PointerPhase::Up,
        point,
        0,
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn pressed_mouse_motion_still_hit_tests_at_the_new_position() {
    let calls = Rc::new(Cell::new(0));
    let (mut runtime, start) = host(&calls);
    send(
        &mut runtime,
        start,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        ORIGIN,
        PRIMARY_POINTER_BUTTON,
    );
    assert_eq!(calls.get(), 1);
    send(
        &mut runtime,
        start + Duration::from_millis(1),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        ORIGIN + Offset::new(1., 1.),
        PRIMARY_POINTER_BUTTON,
    );
    assert_eq!(calls.get(), 2);
}
