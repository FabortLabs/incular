//! Button-class routing and the secondary, tertiary, tap-move and long-press
//! down/cancel callbacks of the pointer recognizer (gestures-007 and
//! gestures-008). Flutter references: `gestures/tap.dart` `isPointerAllowed`,
//! `handleTapUp` and `handleTapMove`; `gestures/long_press.dart`
//! `isPointerAllowed`, `_checkLongPressDown` and `_checkLongPressCancel`;
//! `widgets/gesture_detector.dart` for the callback families.

use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON};
use incular_gestures::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn event(position: Offset, phase: PointerPhase, base: Instant, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position,
        phase,
        time: base + Duration::from_millis(millis),
    }
}

fn counter() -> (Rc<Cell<u32>>, Rc<dyn Fn()>) {
    let count = Rc::new(Cell::new(0));
    let callback: Rc<dyn Fn()> = {
        let count = count.clone();
        Rc::new(move || count.set(count.get() + 1))
    };
    (count, callback)
}

#[test]
fn button_classes_follow_flutter_button_masks() {
    assert_eq!(
        PointerButtonClass::of_press(0),
        Some(PointerButtonClass::Primary)
    );
    assert_eq!(
        PointerButtonClass::of_press(PRIMARY_POINTER_BUTTON),
        Some(PointerButtonClass::Primary)
    );
    assert_eq!(
        PointerButtonClass::of_press(SECONDARY_POINTER_BUTTON),
        Some(PointerButtonClass::Secondary)
    );
    assert_eq!(
        PointerButtonClass::of_press(1 << 2),
        Some(PointerButtonClass::Tertiary)
    );
    // Back and forward buttons take no part in tap or long-press.
    assert_eq!(PointerButtonClass::of_press(1 << 3), None);
}

/// gestures-007: a secondary press fires only `onSecondaryTap`, never the
/// primary `onTap`.
#[test]
fn secondary_press_fires_only_the_secondary_tap() {
    let (primary, on_tap) = counter();
    let (secondary, on_secondary_tap) = counter();
    let callbacks = GestureCallbacks {
        on_tap: Some(on_tap),
        secondary: ButtonGestureCallbacks {
            on_tap: Some(on_secondary_tap),
            ..ButtonGestureCallbacks::default()
        },
        ..GestureCallbacks::default()
    };
    let mapped = callbacks.for_button_class(PointerButtonClass::Secondary);
    let mut recognizer = PointerGestureRecognizer::new(mapped);
    let base = Instant::now();
    let origin = Offset::new(10., 10.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    assert!(recognizer.handle(event(origin, PointerPhase::Up, base, 50)));
    assert_eq!(secondary.get(), 1);
    assert_eq!(primary.get(), 0);
}

/// A primary press keeps the primary callbacks and none of the button's.
#[test]
fn primary_press_ignores_secondary_callbacks() {
    let (primary, on_tap) = counter();
    let (secondary, on_secondary_tap) = counter();
    let callbacks = GestureCallbacks {
        on_tap: Some(on_tap),
        secondary: ButtonGestureCallbacks {
            on_tap: Some(on_secondary_tap),
            ..ButtonGestureCallbacks::default()
        },
        ..GestureCallbacks::default()
    };
    let mapped = callbacks.for_button_class(PointerButtonClass::Primary);
    let mut recognizer = PointerGestureRecognizer::new(mapped);
    let base = Instant::now();
    let origin = Offset::new(10., 10.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    recognizer.handle(event(origin, PointerPhase::Up, base, 50));
    assert_eq!(primary.get(), 1);
    assert_eq!(secondary.get(), 0);
}

/// A secondary press has no drag or pan competitor: a mapped set with only
/// primary drags carries no pointer recognizer at all.
#[test]
fn secondary_press_does_not_join_primary_drags() {
    let starts = Rc::new(Cell::new(0));
    let callbacks = GestureCallbacks {
        on_pan_start: Some({
            let starts = starts.clone();
            Rc::new(move |_: DragStartDetails| starts.set(starts.get() + 1))
        }),
        ..GestureCallbacks::default()
    };
    let mapped = callbacks.for_button_class(PointerButtonClass::Secondary);
    assert!(!mapped.has_pointer_recognizer());
    assert!(mapped.on_pan_start.is_none());
    assert!(mapped.on_horizontal_drag_start.is_none());
    assert!(mapped.on_scale_update.is_none());
    assert_eq!(starts.get(), 0);
}

/// gestures-007: the tertiary button routes to `onTertiaryTapUp` and not to
/// the secondary family.
#[test]
fn tertiary_press_fires_the_tertiary_tap_up() {
    let up = Rc::new(Cell::new(0));
    let (secondary, on_secondary_tap) = counter();
    let callbacks = GestureCallbacks {
        on_tap_up: None,
        secondary: ButtonGestureCallbacks {
            on_tap: Some(on_secondary_tap),
            ..ButtonGestureCallbacks::default()
        },
        tertiary: ButtonGestureCallbacks {
            on_tap_up: Some(Rc::new({
                let up = up.clone();
                move |_: TapUpDetails| up.set(up.get() + 1)
            })),
            ..ButtonGestureCallbacks::default()
        },
        ..GestureCallbacks::default()
    };
    let mapped = callbacks.for_button_class(PointerButtonClass::Tertiary);
    let mut recognizer = PointerGestureRecognizer::new(mapped);
    let base = Instant::now();
    let origin = Offset::new(4., 4.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    recognizer.handle(event(origin, PointerPhase::Up, base, 40));
    assert_eq!(up.get(), 1);
    assert_eq!(secondary.get(), 0);
}

/// gestures-007: a secondary long press fires `onSecondaryLongPress` at its
/// deadline, as the primary long press does.
#[test]
fn secondary_long_press_fires_at_its_deadline() {
    let (long_press, on_long_press) = counter();
    let callbacks = GestureCallbacks {
        secondary: ButtonGestureCallbacks {
            on_long_press: Some(on_long_press),
            ..ButtonGestureCallbacks::default()
        },
        ..GestureCallbacks::default()
    };
    let mapped = callbacks.for_button_class(PointerButtonClass::Secondary);
    let mut recognizer = PointerGestureRecognizer::new(mapped);
    let base = Instant::now();
    let origin = Offset::new(20., 20.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    let GestureDecision::Accept(action) = recognizer.poll(base + Duration::from_millis(500)) else {
        panic!("the secondary long press starts at its deadline");
    };
    recognizer.dispatch(action);
    assert_eq!(long_press.get(), 1);
}

/// gestures-008: `onLongPressDown` fires at the press, and a release before
/// the deadline reports `onLongPressCancel`.
#[test]
fn long_press_down_reports_cancel_when_released_early() {
    let down = Rc::new(Cell::new(0));
    let (cancel, on_cancel) = counter();
    let (long_press, on_long_press) = counter();
    let callbacks = GestureCallbacks {
        on_long_press_down: Some(Rc::new({
            let down = down.clone();
            move |_: LongPressDownDetails| down.set(down.get() + 1)
        })),
        on_long_press_cancel: Some(on_cancel),
        on_long_press: Some(on_long_press),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let origin = Offset::new(8., 8.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    assert_eq!(down.get(), 1);
    recognizer.handle(event(origin, PointerPhase::Up, base, 120));
    assert_eq!(cancel.get(), 1);
    assert_eq!(long_press.get(), 0);
}

/// gestures-008: a long press that starts at its deadline does not cancel on
/// release.
#[test]
fn long_press_down_does_not_cancel_after_the_long_press_started() {
    let (cancel, on_cancel) = counter();
    let (long_press, on_long_press) = counter();
    let callbacks = GestureCallbacks {
        on_long_press_down: Some(Rc::new(|_: LongPressDownDetails| {})),
        on_long_press_cancel: Some(on_cancel),
        on_long_press: Some(on_long_press),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let origin = Offset::new(8., 8.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    let GestureDecision::Accept(action) = recognizer.poll(base + Duration::from_millis(500)) else {
        panic!("the long press starts at its deadline");
    };
    recognizer.dispatch(action);
    assert_eq!(long_press.get(), 1);
    recognizer.handle(event(origin, PointerPhase::Up, base, 700));
    assert_eq!(cancel.get(), 0);
}

/// gestures-008: movement beyond slop before the deadline rejects the long
/// press (Flutter `LongPressGestureRecognizer` resolves rejected), so it
/// reports `onLongPressCancel` and never starts at the deadline.
#[test]
fn movement_beyond_slop_cancels_the_long_press() {
    let (cancel, on_cancel) = counter();
    let (long_press, on_long_press) = counter();
    let callbacks = GestureCallbacks {
        on_long_press_down: Some(Rc::new(|_: LongPressDownDetails| {})),
        on_long_press_cancel: Some(on_cancel),
        on_long_press: Some(on_long_press),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let origin = Offset::new(8., 8.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    recognizer.handle(event(
        origin + Offset::new(0., 60.),
        PointerPhase::Move,
        base,
        20,
    ));
    assert_eq!(cancel.get(), 1);
    assert!(matches!(
        recognizer.poll(base + Duration::from_millis(500)),
        GestureDecision::Pending
    ));
    assert_eq!(long_press.get(), 0);
}

/// gestures-008: `onTapMove` reports each movement with its delta, in order.
#[test]
fn tap_move_reports_each_movement_delta() {
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let callbacks = GestureCallbacks {
        on_tap: Some(Rc::new(|| {})),
        on_tap_move: Some(Rc::new({
            let deltas = deltas.clone();
            move |details: TapMoveDetails| deltas.borrow_mut().push(details.delta)
        })),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let origin = Offset::new(0., 0.);
    recognizer.handle(event(origin, PointerPhase::Down, base, 0));
    recognizer.handle(event(Offset::new(3., 0.), PointerPhase::Move, base, 10));
    recognizer.handle(event(Offset::new(3., 2.), PointerPhase::Move, base, 20));
    assert_eq!(
        *deltas.borrow(),
        vec![Offset::new(3., 0.), Offset::new(0., 2.)]
    );
}
