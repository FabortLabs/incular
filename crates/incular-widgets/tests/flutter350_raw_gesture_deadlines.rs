use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, time::Instant};
use incular_gestures::{
    ArenaGestureRecognizer, GestureCallbacks, GestureRecognizerFactoryWithHandlers,
    LongPressGestureRecognizer, PointerDeviceKind, RawPointerEvent, TapAndDragGestureRecognizer,
};
use incular_widgets::{RawGestureDetector, Widget, internal::WidgetTree};

type Trace = Rc<RefCell<Vec<&'static str>>>;

fn hold(
    trace: &Trace,
) -> GestureRecognizerFactoryWithHandlers<ArenaGestureRecognizer<LongPressGestureRecognizer>> {
    let trace = trace.clone();
    GestureRecognizerFactoryWithHandlers::new(
        move || {
            let start = trace.clone();
            let end = trace.clone();
            let cancel = trace.clone();
            ArenaGestureRecognizer::new(LongPressGestureRecognizer::new(GestureCallbacks {
                on_long_press_start: Some(Rc::new(move |_| start.borrow_mut().push("start"))),
                on_long_press_end: Some(Rc::new(move |_| end.borrow_mut().push("end"))),
                on_long_press_cancel: Some(Rc::new(move || cancel.borrow_mut().push("cancel"))),
                ..Default::default()
            }))
        },
        |_| {},
    )
}

fn tap(
    trace: &Trace,
) -> GestureRecognizerFactoryWithHandlers<ArenaGestureRecognizer<TapAndDragGestureRecognizer>> {
    let trace = trace.clone();
    GestureRecognizerFactoryWithHandlers::new(
        move || {
            let down = trace.clone();
            let up = trace.clone();
            ArenaGestureRecognizer::new(
                TapAndDragGestureRecognizer::pan()
                    .max_consecutive_tap(3)
                    .on_tap_down(move |d| {
                        down.borrow_mut().push(if d.consecutive_tap_count == 1 {
                            "down#1"
                        } else {
                            "down#2"
                        });
                    })
                    .on_tap_up(move |d| {
                        up.borrow_mut().push(if d.consecutive_tap_count == 1 {
                            "up#1"
                        } else {
                            "up#2"
                        });
                    }),
            )
        },
        |_| {},
    )
}

fn detector() -> RawGestureDetector {
    RawGestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
}

fn mount(detector: RawGestureDetector) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(detector.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree
}

fn pointer(tree: &mut WidgetTree, at: Instant, phase: PointerPhase) {
    tree.dispatch_raw_pointer(RawPointerEvent::new(
        7,
        0,
        PointerDeviceKind::Touch,
        u32::from(phase == PointerPhase::Down),
        Offset::new(30., 30.),
        phase,
        at,
    ));
}

#[test]
fn a_raw_hold_keeps_requesting_time_until_it_starts_at_500_ms() {
    let events = Trace::default();
    let mut tree = mount(detector().gesture(hold(&events)));
    let first = Instant::now();
    pointer(&mut tree, first, PointerPhase::Down);
    assert!(tree.pump_gesture_timers(first + Duration::from_millis(499)));
    assert!(events.borrow().is_empty());

    assert!(!tree.pump_gesture_timers(first + Duration::from_millis(500)));
    assert_eq!(&*events.borrow(), &["start"]);
    pointer(
        &mut tree,
        first + Duration::from_millis(550),
        PointerPhase::Up,
    );
    assert_eq!(&*events.borrow(), &["start", "end"]);
}

#[test]
fn a_late_frame_settles_the_earliest_raw_deadline_even_if_registered_last() {
    let events = Trace::default();
    let mut tree = mount(detector().gesture(hold(&events)).gesture(tap(&events)));
    let first = Instant::now();
    pointer(&mut tree, first, PointerPhase::Down);
    pointer(&mut tree, first, PointerPhase::Up);
    assert_eq!(events.take(), ["cancel", "down#1", "up#1"]);

    let second = first + Duration::from_millis(150);
    pointer(&mut tree, second, PointerPhase::Down);
    assert!(tree.pump_gesture_timers(second + Duration::from_millis(99)));
    assert!(events.borrow().is_empty());

    // Both deadlines are due at this frame. The tap's 100 ms deadline must
    // win ahead of the hold's 500 ms, even though the hold registered first.
    assert!(!tree.pump_gesture_timers(second + Duration::from_millis(500)));
    assert_eq!(&*events.borrow(), &["down#2"]);
    pointer(
        &mut tree,
        second + Duration::from_millis(520),
        PointerPhase::Up,
    );
    assert_eq!(events.take(), ["down#2", "up#2"]);
}
