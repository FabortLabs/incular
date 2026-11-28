use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Axis, Constraints};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    DragDropContext, DragTarget, Draggable, Positioned, SizedBox, Stack, Widget,
};

fn exercise(terminal: PointerPhase) {
    let context = DragDropContext::<i32>::new();
    let deltas = Rc::new(RefCell::new(Vec::new()));
    let offered = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(RefCell::new(Vec::new()));
    let ended = Rc::new(RefCell::new(Vec::new()));
    let source = Draggable::new(
        context.clone(),
        1,
        Widget::box_(Size::new(40., 40.), Color::BLACK),
    )
    .axis(Axis::Horizontal)
    .feedback(|_| SizedBox::square(20.).into())
    .on_drag_update({
        let deltas = deltas.clone();
        move |delta| deltas.borrow_mut().push(delta)
    })
    .on_drag_end({
        let ended = ended.clone();
        move |details| {
            ended
                .borrow_mut()
                .push((details.was_accepted, details.offset))
        }
    });
    let position_target = |left, name| {
        Positioned::new(
            DragTarget::new(context.clone(), SizedBox::expand())
                .on_will_accept({
                    let offered = offered.clone();
                    move |_| {
                        offered.borrow_mut().push(name);
                        true
                    }
                })
                .on_accept_with_details({
                    let dropped = dropped.clone();
                    move |_, offset| dropped.borrow_mut().push((name, offset))
                }),
        )
        .left(left)
        .top(50.)
        .width(100.)
        .height(80.)
    };
    let mut runtime = Runtime::new(
        Stack::new([
            Positioned::new(source).left(10.).top(50.),
            position_target(110., "first"),
            position_target(220., "second"),
        ])
        .into(),
    )
    .unwrap();
    let viewport = Constraints::tight(Size::new(400., 300.));
    let start = Instant::now();
    runtime.run_frame_at(viewport, start).unwrap();
    for (index, (phase, position)) in [
        (PointerPhase::Down, Offset::new(20., 60.)),
        (PointerPhase::Move, Offset::new(145., 75.)),
        (terminal, Offset::new(260., 80.)),
    ]
    .into_iter()
    .enumerate()
    {
        let now = start + Duration::from_millis(16 * (index as u64 + 1));
        let _ = runtime.handle_input_at(InputEvent::Pointer { phase, position }, now);
        runtime.run_frame_at(viewport, now).unwrap();
    }

    let last_feedback = Offset::new(135., 50.);
    let cancel = terminal == PointerPhase::Cancel;
    assert_eq!(&*deltas.borrow(), &[Offset::new(125., 15.)]);
    assert_eq!(&*offered.borrow(), &["first"]);
    assert_eq!(&*ended.borrow(), &[(!cancel, last_feedback)]);
    if cancel {
        assert!(dropped.borrow().is_empty());
    } else {
        assert_eq!(&*dropped.borrow(), &[("first", last_feedback)]);
    }
    assert!(context.feedback().is_none());
}

#[test]
fn release_does_not_move_the_feedback_or_offer_a_second_target() {
    exercise(PointerPhase::Up);
}

#[test]
fn cancel_ends_from_the_last_move_without_callbacks_on_the_second_target() {
    exercise(PointerPhase::Cancel);
}
