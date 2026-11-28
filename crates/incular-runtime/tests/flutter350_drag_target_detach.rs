use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{Column, DragDropContext, DragTarget, Draggable, Widget, internal::Key};

#[test]
fn dropping_after_the_entered_target_detaches_never_calls_the_dead_target() {
    let context = DragDropContext::<i32>::new();
    let revision = Rc::new(Cell::new(0_u64));
    let entered = Rc::new(Cell::new(0));
    let leaves = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let ends = Rc::new(Cell::new(0));
    let accepted = Rc::new(Cell::new(false));

    let source: Widget = Draggable::new(
        context.clone(),
        1,
        Widget::box_(Size::new(80., 30.), Color::rgba(0, 0, 255, 255)),
    )
    .feedback(|_| Widget::box_(Size::new(60., 30.), Color::BLACK))
    .on_drag_end({
        let (ends, accepted) = (ends.clone(), accepted.clone());
        move |details| {
            ends.set(ends.get() + 1);
            accepted.set(details.was_accepted);
        }
    })
    .into();
    let target = Widget::from(
        DragTarget::new(
            context.clone(),
            Widget::box_(Size::new(80., 100.), Color::WHITE),
        )
        .on_enter({
            let entered = entered.clone();
            move |_| entered.set(entered.get() + 1)
        })
        .on_leave({
            let leaves = leaves.clone();
            move |_| leaves.set(leaves.get() + 1)
        })
        .on_drop({
            let drops = drops.clone();
            move |_| drops.set(drops.get() + 1)
        }),
    )
    .with_key("target");
    let root = Widget::stateful_builder(revision.clone(), {
        let revision = revision.clone();
        move |_| {
            let mut children = vec![source.clone()];
            if revision.get() == 0 {
                children.push(target.clone());
            }
            Column::new(children)
                .text_direction(TextDirection::Ltr)
                .into()
        }
    });
    let mut runtime = Runtime::new(root).unwrap();
    let view = Constraints::tight(Size::new(320., 180.));
    let base = Instant::now();
    let frame = |rt: &mut Runtime, time| {
        rt.run_frame_at(view, base + Duration::from_millis(time))
            .unwrap();
    };
    let send = |rt: &mut Runtime, phase, position, time| {
        let _ = rt.handle_input_at(
            InputEvent::Pointer { phase, position },
            base + Duration::from_millis(time),
        );
        frame(rt, time + 1);
    };
    frame(&mut runtime, 0);
    let source_point = Offset::new(160., 15.);
    let target_point = Offset::new(160., 80.);
    send(&mut runtime, PointerPhase::Down, source_point, 10);
    send(&mut runtime, PointerPhase::Up, source_point, 20);
    assert_eq!(
        ends.get(),
        1,
        "sole press starts a drag before the control tap ends"
    );
    assert_eq!(drops.get(), 0);

    send(&mut runtime, PointerPhase::Down, source_point, 30);
    frame(&mut runtime, 20_030);
    send(&mut runtime, PointerPhase::Move, target_point, 20_046);
    assert_eq!(entered.get(), 1);
    assert_eq!(ends.get(), 1);
    assert!(context.is_dragging());

    revision.set(1);
    frame(&mut runtime, 20_062);
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("target"))
            .is_none()
    );
    runtime.tree().verify_invariants().unwrap();
    assert_eq!(ends.get(), 1, "removing the target does not end the source");
    assert_eq!((drops.get(), leaves.get()), (0, 0));
    assert!(context.is_dragging());

    send(&mut runtime, PointerPhase::Up, target_point, 20_078);
    assert_eq!(ends.get(), 2);
    assert_eq!((drops.get(), leaves.get()), (0, 0));
    assert!(!accepted.get());
    assert!(!context.is_dragging());
}
