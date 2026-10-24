//! Gesture details report positions and drag deltas in the receiving
//! widget's coordinate space (Flutter's event transform), and global
//! positions unchanged.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, Transform as CoreTransform};
use incular_gestures::{DragUpdateDetails, TapUpDetails};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{GestureDetector, Transform, Widget};

/// A detector scaled by 2 about the top-left corner: local = global / 2.
fn mount(detector: GestureDetector) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Transform::new(CoreTransform::scale(2.), detector).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    tree
}

fn send(tree: &mut WidgetTree, phase: PointerPhase, x: f32, y: f32, millis: u64, base: Instant) {
    tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer: 1,
        position: Offset::new(x, y),
        phase,
        time: base + Duration::from_millis(millis),
    });
}

fn target() -> Widget {
    Widget::box_(Size::new(200., 200.), Color::WHITE)
}

#[test]
fn tap_details_carry_the_local_position() {
    let ups = Rc::new(RefCell::new(Vec::<TapUpDetails>::new()));
    let mut tree = mount(GestureDetector::new(target()).on_tap_up({
        let ups = ups.clone();
        move |details| ups.borrow_mut().push(details)
    }));
    let base = Instant::now();
    send(&mut tree, PointerPhase::Down, 60., 80., 0, base);
    send(&mut tree, PointerPhase::Up, 60., 80., 10, base);
    let up = ups.borrow()[0];
    assert_eq!(up.global_position, Offset::new(60., 80.));
    assert_eq!(up.local_position, Offset::new(30., 40.));
}

#[test]
fn drag_updates_carry_local_positions_and_deltas() {
    let updates = Rc::new(RefCell::new(Vec::<DragUpdateDetails>::new()));
    let mut tree = mount(GestureDetector::new(target()).on_pan_update({
        let updates = updates.clone();
        move |details| updates.borrow_mut().push(details)
    }));
    let base = Instant::now();
    send(&mut tree, PointerPhase::Down, 20., 20., 0, base);
    // Past the pan slop (36 for the unknown kind); `DragStartBehavior.start`
    // reports no update for the movement that crossed it.
    send(&mut tree, PointerPhase::Move, 80., 20., 10, base);
    send(&mut tree, PointerPhase::Move, 100., 20., 20, base);
    let update = updates.borrow()[0];
    assert_eq!(update.global_position, Offset::new(100., 20.));
    assert_eq!(update.local_position, Offset::new(50., 10.));
    // 20 px on screen is 10 px in the scaled widget.
    assert_eq!(update.delta, Offset::new(10., 0.));
}
