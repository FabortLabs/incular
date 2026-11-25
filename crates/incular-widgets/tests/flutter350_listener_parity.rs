//! Original pinned Flutter Listener registrations, excluding the Dart debug inspector.
use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size, Transform as Affine};
use incular_widgets::{
    Align, BoxDecoration, Center, DecoratedBox, Listener, RawPointerEvent, SizedBox, Text,
    TextDirection, Transform, Widget, internal::WidgetTree,
};
use std::{cell::RefCell, rc::Rc, time::Instant};

fn mount(root: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree
}

fn touch(tree: &mut WidgetTree, phase: PointerPhase, point: Offset, buttons: u32) {
    tree.dispatch_raw_pointer(RawPointerEvent::new(
        1,
        1,
        PointerDeviceKind::Touch,
        buttons,
        point,
        phase,
        Instant::now(),
    ));
}

fn x() -> Text {
    Text::new("X").text_direction(TextDirection::Ltr)
}

// widgets/listener_test.dart:15.
#[test]
fn touch_down_bubbles_from_bottom_to_middle_to_top() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = |label| {
        let seen = seen.clone();
        move |_| seen.borrow_mut().push(label)
    };
    let bottom = Listener::new(x()).on_pointer_down(log("bottom"));
    let middle = Listener::new(DecoratedBox::new(bottom).decoration(BoxDecoration::new()))
        .on_pointer_down(log("middle"));
    let top = Listener::new(middle).on_pointer_down(log("top"));
    let mut tree = mount(top);
    touch(&mut tree, PointerPhase::Down, Offset::new(400., 300.), 1);
    touch(&mut tree, PointerPhase::Up, Offset::new(400., 300.), 0);
    assert_eq!(*seen.borrow(), ["bottom", "middle", "top"]);
}

// widgets/listener_test.dart:45.
#[test]
fn a_buttonless_touch_move_delivers_one_hover() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let listener = Listener::new(x()).on_pointer_hover({
        let seen = seen.clone();
        move |_| seen.borrow_mut().push("bottom")
    });
    let mut tree = mount(Center::new(
        SizedBox::new().width(300.).height(300.).child(listener),
    ));
    touch(&mut tree, PointerPhase::Enter, Offset::ZERO, 0);
    touch(&mut tree, PointerPhase::Move, Offset::new(400., 300.), 0);
    assert_eq!(*seen.borrow(), ["bottom"]);
}

// widgets/listener_test.dart:70,136,207,280. The four positions and two
// delta fields are observable. Rust RawPointerEvent does not expose a matrix.
#[test]
fn original_transformed_touch_and_wheel_event_fields() {
    for (mode, down, local, local_delta) in [
        (
            "offset",
            Offset::new(400., 300.),
            Offset::new(50., 50.),
            Offset::new(20., 30.),
        ),
        (
            "scale",
            Offset::new(100., 100.),
            Offset::new(50., 50.),
            Offset::new(10., 15.),
        ),
        (
            "scale and offset",
            Offset::new(450., 350.),
            Offset::new(50., 50.),
            Offset::new(10., 15.),
        ),
        (
            "rotation",
            Offset::new(310., 305.),
            Offset::new(55., 40.),
            Offset::new(30., -20.),
        ),
    ] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let record = || {
            let events = events.clone();
            move |event| events.borrow_mut().push(event)
        };
        let listener: Widget = Listener::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .on_pointer_down(record())
            .on_pointer_move(record())
            .on_pointer_up(record())
            .on_pointer_signal(record())
            .into();
        let scene: Widget = match mode {
            "offset" => Center::new(listener).into(),
            "scale" => Align::new(
                Alignment::TOP_LEFT,
                Transform::new(Affine::scale(2.), listener),
            )
            .into(),
            "scale and offset" => Center::new(Transform::new(Affine::scale(2.), listener)).into(),
            "rotation" => Center::new(Transform::new(
                Affine::rotation(std::f32::consts::FRAC_PI_2),
                listener,
            ))
            .into(),
            _ => unreachable!(),
        };
        let mut tree = mount(scene);
        let moved = Offset::new(20., 30.);
        touch(&mut tree, PointerPhase::Down, down, 1);
        touch(&mut tree, PointerPhase::Move, down + moved, 1);
        touch(&mut tree, PointerPhase::Up, down + moved, 0);
        assert_eq!(events.borrow().len(), 3, "{mode}");
        let _ = tree.scroll_at(down, Offset::new(0., 20.));
        let seen = events.borrow();
        assert_eq!(seen.len(), 4, "{mode}: wheel");
        for (event, want) in seen.iter().zip([
            (down, local, Offset::ZERO, Offset::ZERO),
            (down + moved, local + local_delta, moved, local_delta),
            (
                down + moved,
                local + local_delta,
                Offset::ZERO,
                Offset::ZERO,
            ),
            (down, local, Offset::ZERO, Offset::ZERO),
        ]) {
            assert_eq!((event.position, event.delta), (want.0, want.2), "{mode}");
            let position_error = event.local_position - want.1;
            let delta_error = event.local_delta - want.3;
            assert!(
                position_error.x.abs().max(position_error.y.abs()) < 0.001,
                "{mode}: {event:?}"
            );
            assert!(
                delta_error.x.abs().max(delta_error.y.abs()) < 0.001,
                "{mode}: {event:?}"
            );
        }
    }
}
