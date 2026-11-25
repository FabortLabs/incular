use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

#[derive(Debug, PartialEq)]
enum Callback {
    Start,
    Update(Offset),
    End,
    Tap,
}

#[test]
fn pan_wins_when_its_tap_rejects_but_not_before() {
    let seen = Rc::new(RefCell::new(Vec::<Callback>::new()));
    let view = Size::new(100., 100.);
    let gesture = GestureDetector::new(Widget::box_(view, Color::WHITE))
        .on_pan_start({
            let seen = seen.clone();
            move |_| seen.borrow_mut().push(Callback::Start)
        })
        .on_pan_update({
            let seen = seen.clone();
            move |details| seen.borrow_mut().push(Callback::Update(details.delta))
        })
        .on_pan_end({
            let seen = seen.clone();
            move |_| seen.borrow_mut().push(Callback::End)
        })
        .on_tap({
            let seen = seen.clone();
            move || seen.borrow_mut().push(Callback::Tap)
        });
    let mut tree = WidgetTree::new();
    tree.mount(gesture.into()).unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    let base = Instant::now();
    let mut send = |phase, x, y, millis| {
        let _ = tree.dispatch_gesture(PointerEvent {
            pointer: 5,
            position: Offset::new(x, y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    };
    assert!(seen.borrow().is_empty());
    send(PointerPhase::Down, 10., 10., 0);
    assert!(seen.borrow().is_empty());
    send(PointerPhase::Move, 20., 20., 10);
    assert!(seen.borrow().is_empty());
    send(PointerPhase::Move, 20., 30., 20);
    assert_eq!(*seen.borrow(), [Callback::Start]);
    send(PointerPhase::Move, 20., 25., 30);
    assert_eq!(
        *seen.borrow(),
        [Callback::Start, Callback::Update(Offset::new(0., -5.))]
    );
    send(PointerPhase::Up, 20., 25., 40);
    assert_eq!(
        *seen.borrow(),
        [
            Callback::Start,
            Callback::Update(Offset::new(0., -5.)),
            Callback::End
        ]
    );
}

#[test]
fn explicit_pan_slop_remains_in_force_after_the_tap_rejects() {
    use incular_gestures::GestureCallbacks;
    let count = Rc::new(std::cell::Cell::new(0));
    let view = Size::new(100., 100.);
    let gesture =
        GestureDetector::new(Widget::box_(view, Color::WHITE)).callbacks(GestureCallbacks {
            on_tap: Some(Rc::new(|| {})),
            on_pan_start: Some(Rc::new({
                let count = count.clone();
                move |_| count.set(count.get() + 1)
            })),
            only_accept_drag_on_threshold: true,
            ..GestureCallbacks::default()
        });
    let mut tree = WidgetTree::new();
    tree.mount(gesture.into()).unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    let base = Instant::now();
    let mut send = |phase, x, y, millis| {
        let _ = tree.dispatch_gesture(PointerEvent {
            pointer: 5,
            position: Offset::new(x, y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    };
    send(PointerPhase::Down, 10., 10., 0);
    send(PointerPhase::Move, 20., 20., 10);
    send(PointerPhase::Move, 20., 30., 20);
    assert_eq!(count.get(), 0);
    send(PointerPhase::Move, 20., 50., 30);
    assert_eq!(count.get(), 1);
    send(PointerPhase::Up, 20., 50., 40);
}
