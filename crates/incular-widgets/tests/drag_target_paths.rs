//! Drag-target path rules (gestures-010 and gestures-011). Flutter references:
//! `widgets/drag_target.dart` `_DragAvatar.updateDrag` (every target on the
//! path up to the first accepting one is entered; `didMove` reaches rejected
//! targets too), `didLeave`, `finishDrag` (the accepting target receives the
//! drop and the others leave), and `DragTarget.builder`, which receives the
//! candidate and rejected payloads.

mod common;

use common::*;
use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

/// A 100x80 source at x 0..100 and a 100x80 target at x 100..200, dragged
/// from the source across the target and released over it.
fn drag_across(target: impl Into<Widget>, context: DragDropContext<String>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Row::new(vec![
            Widget::from(
                Draggable::new(
                    context.clone(),
                    String::from("card"),
                    Widget::box_(Size::new(100., 80.), Color::WHITE),
                )
                .feedback(|_| Widget::box_(Size::new(100., 80.), Color::WHITE)),
            ),
            target.into(),
        ])
        .text_direction(incular_widgets::TextDirection::Ltr)
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(200., 80.)))
        .expect("layout");
    tree
}

fn release_over_target(tree: &mut WidgetTree) {
    let now = Instant::now();
    for (phase, position) in [
        (incular_core::PointerPhase::Down, Offset::new(10., 20.)),
        (incular_core::PointerPhase::Move, Offset::new(35., 20.)),
        (incular_core::PointerPhase::Move, Offset::new(140., 20.)),
        (incular_core::PointerPhase::Move, Offset::new(150., 20.)),
        (incular_core::PointerPhase::Up, Offset::new(150., 20.)),
    ] {
        let _ = tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 40,
            position,
            phase,
            time: now,
        });
    }
}

/// gestures-010: a rejecting target still receives `onMove` and `onLeave`,
/// and is not dropped on.
#[test]
fn rejecting_target_receives_move_and_leave_but_no_drop() {
    let context = DragDropContext::new();
    let moves = Rc::new(Cell::new(0));
    let left = Rc::new(Cell::new(0));
    let dropped = Rc::new(Cell::new(0));
    let target = DragTarget::new(
        context.clone(),
        Widget::box_(Size::new(100., 80.), Color::BLACK),
    )
    .on_will_accept(|_| false)
    .on_update({
        let moves = moves.clone();
        move |_, _| moves.set(moves.get() + 1)
    })
    .on_leave({
        let left = left.clone();
        move |_| left.set(left.get() + 1)
    })
    .on_drop({
        let dropped = dropped.clone();
        move |_| dropped.set(dropped.get() + 1)
    });
    let mut tree = drag_across(target, context.clone());
    release_over_target(&mut tree);
    assert!(moves.get() >= 1, "a rejected target follows the pointer");
    assert_eq!(left.get(), 1, "release leaves the rejected target");
    assert_eq!(dropped.get(), 0);
    assert!(!context.is_dragging());
}

/// gestures-010: when the innermost target rejects, the drag falls through to
/// the outer target, which takes the drop. The rejecting inner target is left.
#[test]
fn rejecting_inner_target_falls_through_to_the_accepting_outer_target() {
    let context = DragDropContext::new();
    let inner_entered = Rc::new(Cell::new(0));
    let inner_left = Rc::new(Cell::new(0));
    let inner_dropped = Rc::new(Cell::new(0));
    let outer_dropped = Rc::new(Cell::new(0));
    let inner = DragTarget::new(
        context.clone(),
        Widget::box_(Size::new(100., 80.), Color::BLACK),
    )
    .on_will_accept(|_| false)
    .on_enter({
        let inner_entered = inner_entered.clone();
        move |_| inner_entered.set(inner_entered.get() + 1)
    })
    .on_leave({
        let inner_left = inner_left.clone();
        move |_| inner_left.set(inner_left.get() + 1)
    })
    .on_drop({
        let inner_dropped = inner_dropped.clone();
        move |_| inner_dropped.set(inner_dropped.get() + 1)
    });
    let outer = DragTarget::new(context.clone(), inner).on_drop({
        let outer_dropped = outer_dropped.clone();
        move |payload| {
            assert_eq!(payload, "card");
            outer_dropped.set(outer_dropped.get() + 1);
        }
    });
    let mut tree = drag_across(outer, context.clone());
    release_over_target(&mut tree);
    assert_eq!(
        inner_entered.get(),
        0,
        "onEnter reports accepted candidates only"
    );
    assert_eq!(
        inner_left.get(),
        1,
        "the rejecting inner target is left at the drop"
    );
    assert_eq!(inner_dropped.get(), 0);
    assert_eq!(outer_dropped.get(), 1, "the outer target takes the drop");
}

/// gestures-011: the builder sees the candidate payloads and the rejected ones
/// as the pointer enters and leaves the target.
#[test]
fn builder_receives_candidate_and_rejected_payloads() {
    let context = DragDropContext::new();
    let seen = Rc::new(RefCell::new(Vec::<(usize, usize)>::new()));
    let target = DragTarget::builder(context.clone(), {
        let seen = seen.clone();
        move |candidates: Vec<String>, rejected: Vec<String>| {
            seen.borrow_mut().push((candidates.len(), rejected.len()));
            Widget::box_(Size::new(100., 80.), Color::BLACK)
        }
    })
    .on_will_accept(|payload| payload != "card");
    let mut tree = drag_across(target, context.clone());
    // The initial build sees nothing entered.
    assert_eq!(seen.borrow().first().copied(), Some((0, 0)));

    let now = Instant::now();
    for (phase, position) in [
        (incular_core::PointerPhase::Down, Offset::new(10., 20.)),
        (incular_core::PointerPhase::Move, Offset::new(35., 20.)),
        (incular_core::PointerPhase::Move, Offset::new(150., 20.)),
    ] {
        let _ = tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 41,
            position,
            phase,
            time: now,
        });
    }
    tree.layout(Constraints::tight(Size::new(200., 80.)))
        .expect("layout");
    assert!(
        seen.borrow().contains(&(0, 1)),
        "the rejected payload reaches the builder: {:?}",
        seen.borrow()
    );
}
