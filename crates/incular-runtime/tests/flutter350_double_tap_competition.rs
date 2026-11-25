//! G03: a Flutter tap does not win while the double-tap recognizer holds it.
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Center, GestureDetector, Widget};
use std::{cell::RefCell, rc::Rc, time::Duration};

type Trace = Rc<RefCell<Vec<&'static str>>>;

fn frame_bounds() -> Constraints {
    Constraints::tight(Size::new(200., 200.))
}

fn mount() -> (Runtime, Trace, Instant) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let log = |value| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(value)
    };
    let detector = GestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
        .on_tap(log("single"))
        .on_double_tap(log("double"));
    let mut runtime = Runtime::new(Center::new(detector).into()).unwrap();
    let base = Instant::now();
    runtime.run_frame_at(frame_bounds(), base).unwrap();
    (runtime, trace, base)
}

fn at(base: Instant, millis: u64) -> Instant {
    base + Duration::from_millis(millis)
}

fn tap(runtime: &mut Runtime, base: Instant, pointer: u64, down: u64, up: u64) {
    contact(
        runtime,
        at(base, down),
        pointer,
        PointerPhase::Down,
        center(),
    );
    contact(runtime, at(base, up), pointer, PointerPhase::Up, center());
}

fn center() -> Offset {
    Offset::new(100., 100.)
}

fn contact(
    runtime: &mut Runtime,
    now: Instant,
    pointer: u64,
    phase: PointerPhase,
    position: Offset,
) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer,
            device: 1,
            kind: PointerDeviceKind::Touch,
            button: Some(PRIMARY_POINTER_BUTTON),
            phase,
            buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
                PRIMARY_POINTER_BUTTON
            } else {
                0
            },
            position,
            sample: PointerSampleMetadata::EMPTY,
        },
        now,
    );
}

#[test]
fn a_valid_double_tap_never_emits_a_single() {
    let (mut runtime, trace, base) = mount();
    tap(&mut runtime, base, 1, 0, 10);
    let after_first = trace.borrow().clone();
    tap(&mut runtime, base, 2, 50, 60);
    runtime.run_frame_at(frame_bounds(), at(base, 400)).unwrap();
    assert_eq!(
        (after_first, trace.borrow().clone()),
        (vec![], vec!["double"])
    );
}

#[test]
fn an_unpaired_first_tap_wins_only_at_the_double_tap_deadline() {
    let (mut runtime, trace, base) = mount();
    tap(&mut runtime, base, 1, 0, 10);
    runtime.run_frame_at(frame_bounds(), at(base, 309)).unwrap();
    let before = trace.borrow().clone();
    runtime.run_frame_at(frame_bounds(), at(base, 310)).unwrap();
    assert_eq!((before, trace.borrow().clone()), (vec![], vec!["single"]));
}

#[test]
fn a_qualifying_second_down_pauses_the_timeout_even_when_held() {
    let (mut runtime, trace, base) = mount();
    tap(&mut runtime, base, 1, 0, 10);
    contact(&mut runtime, at(base, 50), 2, PointerPhase::Down, center());
    runtime.run_frame_at(frame_bounds(), at(base, 400)).unwrap();
    assert!(trace.borrow().is_empty());
    contact(&mut runtime, at(base, 450), 2, PointerPhase::Up, center());
    assert_eq!(*trace.borrow(), ["double"]);
}

#[test]
fn a_cancelled_or_moved_second_contact_does_not_restore_the_first_tap() {
    for phase in [PointerPhase::Cancel, PointerPhase::Move] {
        let (mut runtime, trace, base) = mount();
        tap(&mut runtime, base, 1, 0, 10);
        contact(&mut runtime, at(base, 50), 2, PointerPhase::Down, center());
        let point = if phase == PointerPhase::Move {
            Offset::new(120., 100.)
        } else {
            center()
        };
        contact(&mut runtime, at(base, 60), 2, phase, point);
        assert!(trace.borrow().is_empty(), "{phase:?}");
        if phase == PointerPhase::Move {
            contact(&mut runtime, at(base, 70), 2, PointerPhase::Up, point);
        }
        runtime.run_frame_at(frame_bounds(), at(base, 400)).unwrap();
        assert!(trace.borrow().is_empty(), "{phase:?}");
    }
}

#[test]
fn a_far_second_contact_is_the_only_ordinary_tap_delivered() {
    let (mut runtime, trace, base) = mount();
    tap(&mut runtime, base, 1, 0, 10);
    let far = Offset::new(5., 5.);
    contact(&mut runtime, at(base, 50), 2, PointerPhase::Down, far);
    contact(&mut runtime, at(base, 60), 2, PointerPhase::Up, far);
    assert_eq!(*trace.borrow(), ["single"]);
    runtime.run_frame_at(frame_bounds(), at(base, 310)).unwrap();
    assert_eq!(*trace.borrow(), ["single"]);
}

#[test]
fn a_too_early_second_press_replaces_the_first_wait() {
    let (mut runtime, trace, base) = mount();
    tap(&mut runtime, base, 1, 0, 1);
    contact(&mut runtime, at(base, 5), 2, PointerPhase::Down, center());
    assert!(trace.borrow().is_empty());
    contact(&mut runtime, at(base, 10), 2, PointerPhase::Up, center());
    runtime.run_frame_at(frame_bounds(), at(base, 309)).unwrap();
    assert!(trace.borrow().is_empty());
    runtime.run_frame_at(frame_bounds(), at(base, 310)).unwrap();
    assert_eq!(*trace.borrow(), ["single"]);
}
