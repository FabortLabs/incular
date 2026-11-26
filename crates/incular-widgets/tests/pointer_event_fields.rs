//! Flutter `PointerEvent` fields on raw listener events: `delta`, `down`,
//! `localPosition` and `localDelta`.

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size, Transform};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Listener, Padding, Positioned, RawPointerEvent, Stack, Widget, internal::Key,
};

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

#[test]
fn active_listener_keeps_its_down_coordinates_after_moving_and_rescaling() {
    let (mut tree, seen) = moving_listener(true);
    send(&mut tree, PointerPhase::Down, 1, 20., 20.);
    move_listener(&mut tree, &seen, true);
    send(&mut tree, PointerPhase::Move, 1, 30., 28.);
    send(&mut tree, PointerPhase::Up, 0, 30., 28.);
    send(&mut tree, PointerPhase::Move, 0, 60., 20.);
    send(&mut tree, PointerPhase::Down, 1, 60., 20.);
    assert_eq!(
        local_samples(&seen),
        [
            (Offset::new(10., 10.), Offset::ZERO),
            (Offset::new(15., 14.), Offset::new(5., 4.)),
            (Offset::new(15., 14.), Offset::ZERO),
            (Offset::new(5., 5.), Offset::new(7.5, -2.)),
            (Offset::new(5., 5.), Offset::ZERO),
        ],
    );
}

#[test]
fn disabled_hit_transform_never_enters_the_listener_coordinate_path() {
    let (mut tree, seen) = moving_listener(false);
    send(&mut tree, PointerPhase::Down, 1, 20., 20.);
    move_listener(&mut tree, &seen, false);
    send(&mut tree, PointerPhase::Move, 1, 30., 28.);
    send(&mut tree, PointerPhase::Up, 0, 30., 28.);
    send(&mut tree, PointerPhase::Move, 0, 60., 20.);
    send(&mut tree, PointerPhase::Down, 1, 60., 20.);
    assert_eq!(
        local_samples(&seen),
        [
            (Offset::new(20., 20.), Offset::ZERO),
            (Offset::new(30., 28.), Offset::new(10., 8.)),
            (Offset::new(30., 28.), Offset::ZERO),
            (Offset::new(20., 20.), Offset::new(30., -8.)),
            (Offset::new(20., 20.), Offset::ZERO),
        ],
    );
}

type Trace = Rc<RefCell<Vec<RawPointerEvent>>>;

fn moving_scene(trace: &Trace, left: f32, scale: f32, transform_hits: bool) -> Widget {
    let record = || {
        let trace = trace.clone();
        move |event| trace.borrow_mut().push(event)
    };
    let child = Widget::from(
        Listener::new(Widget::box_(Size::new(40., 40.), Color::WHITE))
            .on_pointer_down(record())
            .on_pointer_move(record())
            .on_pointer_up(record())
            .on_pointer_hover(record()),
    )
    .with_key(Key::Value(1));
    let transform = if transform_hits {
        Widget::from(incular_widgets::Transform::new(
            Transform::scale(scale),
            child,
        ))
    } else {
        Widget::from(
            incular_widgets::FractionalTranslation::new(Offset::new(scale / 2., 0.), child)
                .transform_hit_tests(false),
        )
    };
    Stack::new([Positioned::new(transform).left(left).top(0.)]).into()
}

fn moving_listener(transform_hits: bool) -> (WidgetTree, Trace) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let mut tree = WidgetTree::new();
    tree.mount(moving_scene(&trace, 0., 2., transform_hits))
        .unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    (tree, trace)
}

fn move_listener(tree: &mut WidgetTree, trace: &Trace, transform_hits: bool) {
    let key = Key::Value(1);
    let before = tree.element_with_key(&key);
    tree.update(
        tree.root().unwrap(),
        moving_scene(trace, 40., 4., transform_hits),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    assert!(before.is_some());
    assert_eq!(tree.element_with_key(&key), before);
}

fn local_samples(trace: &Trace) -> Vec<(Offset, Offset)> {
    trace
        .borrow()
        .iter()
        .map(|e| (e.local_position, e.local_delta))
        .collect()
}

#[test]
fn tree_pointer_router_observes_hit_callbacks_then_local_routes_and_native_scroll() {
    let (mut tree, listener) = record(|listener| listener);
    let observations = Rc::new(RefCell::new(Vec::new()));
    let router = tree.pointer_router();
    router.add_route(1, Some(Transform::scale(0.5)), {
        let listener = listener.clone();
        let observations = observations.clone();
        move |routed| {
            assert!(!listener.borrow().is_empty(), "the hit path goes first");
            observations.borrow_mut().push(("local", routed.event));
        }
    });
    router.add_global_route(None, {
        let observations = observations.clone();
        move |routed| observations.borrow_mut().push(("global", routed.event))
    });

    send(&mut tree, PointerPhase::Down, 1, 20., 20.);
    send(&mut tree, PointerPhase::Move, 1, 24., 26.);
    tree.scroll_at(Offset::new(24., 26.), Offset::new(0., 8.));
    let observed = observations.borrow();
    let labels: Vec<_> = observed.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, ["local", "global", "local", "global", "global"]);
    assert_eq!(observed[2].1.local_position, Offset::new(12., 13.));
    assert_eq!(observed[2].1.local_delta, Offset::new(2., 3.));
    assert_eq!(observed[3].1.local_delta, Offset::new(4., 6.));
    assert_eq!(observed[4].1.scroll_delta, Offset::new(0., 8.));
}
