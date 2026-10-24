//! A tap that reported `onTapDown` reports `onTapCancel` when another
//! detector wins the arena (Flutter `rejectGesture` with `_sentTapDown`).

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{GestureDetector, Widget};

#[test]
fn an_outer_long_press_cancels_the_inner_tap() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let push = |name: &'static str| {
        let log = log.clone();
        move || log.borrow_mut().push(name)
    };
    let inner = GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .on_tap(push("tap"))
        .on_tap_down({
            let log = log.clone();
            move |_| log.borrow_mut().push("down")
        })
        .on_tap_cancel(push("cancel"));
    let outer = GestureDetector::new(inner).on_long_press(push("long press"));
    let mut tree = WidgetTree::new();
    tree.mount(outer.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    let base = Instant::now();
    tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer: 1,
        position: Offset::new(50., 50.),
        phase: PointerPhase::Down,
        time: base,
    });
    tree.pump_gesture_timers(base + Duration::from_millis(100));
    assert_eq!(log.borrow().as_slice(), ["down"]);
    tree.pump_gesture_timers(base + Duration::from_millis(500));
    assert_eq!(log.borrow().as_slice(), ["down", "cancel"]);
    tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer: 1,
        position: Offset::new(50., 50.),
        phase: PointerPhase::Up,
        time: base + Duration::from_millis(600),
    });
    assert_eq!(log.borrow().as_slice(), ["down", "cancel", "long press"]);
}
