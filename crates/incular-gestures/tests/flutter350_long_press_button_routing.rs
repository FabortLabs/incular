//! Flutter's per-button hold routing and macOS-style release masks.
use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON, time::Instant,
};
use incular_gestures::{
    ButtonGestureCallbacks, GestureCallbacks, LongPressGestureRecognizer, RawPointerEvent,
};

type Trace = Rc<RefCell<Vec<(&'static str, Offset)>>>;

fn callbacks(trace: &Trace, start: &'static str, end: &'static str) -> ButtonGestureCallbacks {
    ButtonGestureCallbacks {
        on_long_press_start: Some(Rc::new({
            let trace = trace.clone();
            move |details| trace.borrow_mut().push((start, details.global_position))
        })),
        on_long_press_end: Some(Rc::new({
            let trace = trace.clone();
            move |details| trace.borrow_mut().push((end, details.global_position))
        })),
        ..Default::default()
    }
}

fn input(
    at: Instant,
    ms: u64,
    pointer: u64,
    buttons: u32,
    phase: PointerPhase,
    pos: Offset,
) -> RawPointerEvent {
    RawPointerEvent::new(
        pointer,
        1,
        PointerDeviceKind::Touch,
        buttons,
        pos,
        phase,
        at + Duration::from_millis(ms),
    )
}

fn unhandled_button_is_silent(button: u32, install_secondary: bool) {
    let trace = Trace::default();
    let primary = callbacks(&trace, "primaryStart", "primaryEnd");
    let secondary = if install_secondary {
        callbacks(&trace, "secondaryStart", "secondaryEnd")
    } else {
        ButtonGestureCallbacks::default()
    };
    let mut recognizer = LongPressGestureRecognizer::new(GestureCallbacks {
        on_long_press_start: primary.on_long_press_start,
        on_long_press_end: primary.on_long_press_end,
        secondary,
        ..Default::default()
    });
    let at = Instant::now();
    let down = input(at, 0, 2, button, PointerPhase::Down, Offset::new(30., 30.));
    assert!(!recognizer.add_pointer(down.clone()));
    assert!(!recognizer.close_arena(2));
    recognizer.handle_event(down);
    recognizer.poll(at + Duration::from_millis(700));
    recognizer.handle_event(input(
        at,
        700,
        2,
        button,
        PointerPhase::Move,
        Offset::new(100., 200.),
    ));
    recognizer.handle_event(input(
        at,
        700,
        2,
        0,
        PointerPhase::Up,
        Offset::new(100., 201.),
    ));
    recognizer.dispose();
    assert!(trace.borrow().is_empty());

    // These listeners are live; an admitted button still emits its pair.
    for (pointer, allowed, begin_name, end_name) in [
        (3, PRIMARY_POINTER_BUTTON, "primaryStart", "primaryEnd"),
        (
            4,
            SECONDARY_POINTER_BUTTON,
            "secondaryStart",
            "secondaryEnd",
        ),
    ] {
        if allowed == SECONDARY_POINTER_BUTTON && !install_secondary {
            break;
        }
        let pos = Offset::new(30., 30.);
        let down = input(at, 1000, pointer, allowed, PointerPhase::Down, pos);
        assert!(recognizer.add_pointer(down.clone()));
        assert!(recognizer.close_arena(pointer));
        recognizer.handle_event(down);
        recognizer.poll(at + Duration::from_millis(1500));
        recognizer.handle_event(input(at, 1600, pointer, 0, PointerPhase::Up, pos));
        assert_eq!(trace.take(), [(begin_name, pos), (end_name, pos)]);
    }
}

#[test]
fn secondary_does_not_dispatch_to_a_primary_only_hold() {
    unhandled_button_is_silent(SECONDARY_POINTER_BUTTON, false);
}

#[test]
fn tertiary_does_not_dispatch_to_primary_or_secondary_holds() {
    unhandled_button_is_silent(TERTIARY_POINTER_BUTTON, true);
}

fn retains_the_originating_button(alternate: bool) {
    use PointerPhase::{Down, Move, Up};
    let trace = Trace::default();
    let primary = callbacks(&trace, "primaryStart", "primaryEnd");
    let mut recognizer = LongPressGestureRecognizer::new(GestureCallbacks {
        on_long_press_start: primary.on_long_press_start,
        on_long_press_end: primary.on_long_press_end,
        ..Default::default()
    });
    let at = Instant::now();
    let ptr = if alternate { 9 } else { 8 };
    let start = Offset::new(10., 10.);
    let end = Offset::new(100., 200.);
    let down = input(at, 0, ptr, PRIMARY_POINTER_BUTTON, Down, start);
    assert!(recognizer.add_pointer(down.clone()));
    // The original supplies an unrelated pointer when closing the arena.
    assert!(!recognizer.close_arena(4));
    recognizer.handle_event(down);
    recognizer.poll(at + Duration::from_millis(1000));
    assert_eq!(trace.borrow().as_slice(), [("primaryStart", start)]);
    recognizer.handle_event(input(
        at,
        1000,
        ptr,
        PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON,
        Move,
        end,
    ));
    recognizer.poll(at + Duration::from_millis(2000));
    assert_eq!(trace.borrow().as_slice(), [("primaryStart", start)]);
    let (release_time, release_buttons) = if alternate {
        recognizer.handle_event(input(at, 2000, ptr, SECONDARY_POINTER_BUTTON, Move, end));
        recognizer.poll(at + Duration::from_millis(3000));
        assert_eq!(trace.borrow().as_slice(), [("primaryStart", start)]);
        (3000, 0)
    } else {
        (2000, SECONDARY_POINTER_BUTTON)
    };
    recognizer.handle_event(input(at, release_time, ptr, release_buttons, Up, end));
    recognizer.poll(at + Duration::from_millis(release_time + 1000));
    recognizer.dispose();
    assert_eq!(
        trace.borrow().as_slice(),
        [("primaryStart", start), ("primaryEnd", end)]
    );
}

#[test]
fn a_chord_ending_with_secondary_still_finishes_the_primary_hold() {
    retains_the_originating_button(false);
}

#[test]
fn removing_the_primary_before_release_continues_the_original_hold() {
    retains_the_originating_button(true);
}
