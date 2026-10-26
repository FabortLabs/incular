//! Flutter `PointerEvent` fields on raw listener events: `delta`, `down`,
//! `localPosition` and `localDelta`.

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size, Transform};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Listener, Padding, RawPointerEvent, Widget};

fn record(child: impl Fn(Widget) -> Widget) -> (WidgetTree, Rc<RefCell<Vec<RawPointerEvent>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let push = |seen: &Rc<RefCell<Vec<RawPointerEvent>>>| {
        let seen = seen.clone();
        move |event| seen.borrow_mut().push(event)
    };
    let listener = Listener::new(Widget::box_(Size::new(40., 40.), Color::WHITE))
        .on_pointer_down(push(&seen))
        .on_pointer_move(push(&seen))
        .on_pointer_up(push(&seen))
        .on_pointer_hover(push(&seen));
    let mut tree = WidgetTree::new();
    tree.mount(child(listener.into())).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    (tree, seen)
}

fn send(tree: &mut WidgetTree, phase: PointerPhase, buttons: u32, x: f32, y: f32) {
    tree.dispatch_raw_pointer(RawPointerEvent::new(
        1,
        1,
        PointerDeviceKind::Mouse,
        buttons,
        Offset::new(x, y),
        phase,
        Instant::now(),
    ));
}

#[test]
fn listener_events_carry_delta_down_and_local_position() {
    let (mut tree, seen) = record(|listener| Padding::new(EdgeInsets::all(10.), listener).into());
    send(&mut tree, PointerPhase::Move, 0, 20., 20.);
    send(&mut tree, PointerPhase::Down, 1, 25., 20.);
    send(&mut tree, PointerPhase::Move, 1, 30., 28.);
    send(&mut tree, PointerPhase::Up, 0, 30., 28.);
    let seen = seen.borrow();
    let summary: Vec<_> = seen
        .iter()
        .map(|event| (event.phase, event.down, event.delta, event.local_position))
        .collect();
    assert_eq!(
        summary,
        [
            // A first hover has no previous position.
            (
                PointerPhase::Move,
                false,
                Offset::ZERO,
                Offset::new(10., 10.)
            ),
            // A press starts from rest.
            (
                PointerPhase::Down,
                true,
                Offset::ZERO,
                Offset::new(15., 10.)
            ),
            (
                PointerPhase::Move,
                true,
                Offset::new(5., 8.),
                Offset::new(20., 18.)
            ),
            (PointerPhase::Up, false, Offset::ZERO, Offset::new(20., 18.)),
        ]
    );
    assert!(seen.iter().all(|event| !event.synthesized));
}

#[test]
fn local_delta_follows_the_listener_transform() {
    let (mut tree, seen) = record(|listener| {
        // Without an origin Flutter's `Transform` pivots about the top-left.
        incular_widgets::Transform::new(Transform::scale(2.), listener).into()
    });
    send(&mut tree, PointerPhase::Down, 1, 20., 20.);
    send(&mut tree, PointerPhase::Move, 1, 30., 26.);
    let seen = seen.borrow();
    let moved = seen[1];
    assert_eq!(moved.delta, Offset::new(10., 6.));
    // Scaled by two about the origin: local = global / 2.
    assert_eq!(moved.local_position, Offset::new(15., 13.));
    assert_eq!(moved.local_delta, Offset::new(5., 3.));
}
