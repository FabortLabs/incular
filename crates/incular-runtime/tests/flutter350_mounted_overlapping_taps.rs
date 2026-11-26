use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, Row, TextDirection, Widget};

fn detector(ups: Rc<RefCell<Vec<Offset>>>) -> GestureDetector {
    GestureDetector::new(Widget::box_(Size::new(400., 600.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap_up(move |details| ups.borrow_mut().push(details.global_position))
}

fn fixture(siblings: bool) -> (Runtime, Rc<RefCell<Vec<Offset>>>, Instant) {
    let ups = Rc::new(RefCell::new(Vec::new()));
    let pane: Widget = if siblings {
        Row::new([detector(ups.clone()), detector(ups.clone())])
            .text_direction(TextDirection::Ltr)
            .into()
    } else {
        detector(ups.clone()).into()
    };
    let mut runtime = Runtime::new(pane).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();
    (runtime, ups, now)
}

fn input(
    runtime: &mut Runtime,
    now: Instant,
    pointer: u64,
    phase: PointerPhase,
    x: f32,
    time: u64,
) {
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer,
            device: 1,
            kind: PointerDeviceKind::Touch,
            button: Some(PRIMARY_POINTER_BUTTON),
            phase,
            buttons: if phase == PointerPhase::Down {
                PRIMARY_POINTER_BUTTON
            } else {
                0
            },
            position: Offset::new(x, 50.),
            sample: PointerSampleMetadata::EMPTY,
        },
        now + Duration::from_millis(time),
    );
}

fn same_detector(reverse: bool) {
    use PointerPhase::{Down, Up};
    let (mut runtime, ups, now) = fixture(false);
    let mut send = |pointer, phase, x, time| input(&mut runtime, now, pointer, phase, x, time);
    send(1, Down, 50., 0);
    send(2, Down, 75., 1);
    assert!(ups.borrow().is_empty());
    if reverse {
        send(2, Up, 75., 2);
        assert!(ups.borrow().is_empty());
        send(1, Up, 50., 3);
    } else {
        send(1, Up, 50., 2);
        assert_eq!(ups.borrow().as_slice(), [Offset::new(50., 50.)]);
        send(3, Down, 90., 3);
        send(3, Up, 90., 4);
        send(2, Up, 75., 5);
    }
    assert_eq!(ups.borrow().as_slice(), [Offset::new(50., 50.)]);

    send(4, Down, 100., 10);
    send(4, Up, 100., 11);
    assert_eq!(
        ups.borrow().as_slice(),
        [Offset::new(50., 50.), Offset::new(100., 50.)]
    );
}

#[test]
fn only_the_first_contact_taps_when_released_fifo() {
    same_detector(false);
}

#[test]
fn only_the_first_contact_taps_when_released_filo() {
    same_detector(true);
}

#[test]
fn sibling_detectors_can_take_two_concurrent_taps() {
    use PointerPhase::{Down, Up};
    let (mut runtime, ups, now) = fixture(true);
    input(&mut runtime, now, 1, Down, 50., 0);
    input(&mut runtime, now, 2, Down, 450., 1);
    input(&mut runtime, now, 2, Up, 450., 2);
    input(&mut runtime, now, 1, Up, 50., 3);
    assert_eq!(
        ups.borrow().as_slice(),
        [Offset::new(450., 50.), Offset::new(50., 50.)]
    );
}

#[test]
fn canceling_the_primary_does_not_promote_a_held_secondary_contact() {
    use PointerPhase::{Cancel, Down, Up};
    let (mut runtime, ups, now) = fixture(false);
    let mut send = |pointer, phase, x, time| input(&mut runtime, now, pointer, phase, x, time);
    send(1, Down, 50., 0);
    send(2, Down, 75., 1);
    send(1, Cancel, 50., 2);
    send(3, Down, 90., 3);
    send(3, Up, 90., 4);
    send(2, Up, 75., 5);
    assert!(ups.borrow().is_empty());

    send(4, Down, 100., 6);
    send(4, Up, 100., 7);
    assert_eq!(ups.borrow().as_slice(), [Offset::new(100., 50.)]);
}
