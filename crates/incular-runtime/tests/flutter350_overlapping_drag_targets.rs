use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, DragDropContext, DragTarget, Draggable, Expanded, HitTestBehavior,
    Positioned, SizedBox, Stack, Text, TextDirection, Widget, internal::Key,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Offered(&'static str, i32),
    Dropped(&'static str, i32),
}

fn exercise(upper_accepts: bool, upper_behavior: Option<HitTestBehavior>) {
    let context = DragDropContext::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let target = |name, accepts| {
        DragTarget::builder(context.clone(), |_, _| SizedBox::expand().into())
            .on_will_accept({
                let events = events.clone();
                move |payload| {
                    events.borrow_mut().push(Event::Offered(name, payload));
                    accepts
                }
            })
            .on_accept_with_details({
                let events = events.clone();
                move |payload, _| events.borrow_mut().push(Event::Dropped(name, payload))
            })
    };
    let source = Draggable::new(
        context.clone(),
        1,
        Widget::from(Text::new("Source")).with_key("source"),
    )
    .feedback(|_| Text::new("Dragging").into());
    let lower = Positioned::new(Widget::from(target("lower", true)).with_key("lower"))
        .left(200.)
        .top(0.)
        .width(200.)
        .height(400.);
    let upper_target = target("upper", upper_accepts);
    let upper_target = if let Some(behavior) = upper_behavior {
        upper_target.hit_test_behavior(behavior)
    } else {
        upper_target
    };
    let upper = Positioned::new(Widget::from(upper_target).with_key("upper"))
        .left(0.)
        .top(0.)
        .width(300.)
        .height(400.);
    let root = Directionality::new(
        TextDirection::Ltr,
        Column::new([
            Widget::from(source),
            Widget::from(Expanded::new(Stack::new([lower, upper]))),
        ]),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    let base = Instant::now();
    runtime.run_frame_at(viewport, base).unwrap();
    let bounds = |name| {
        let tree = runtime.tree();
        tree.element_bounds(tree.element_with_key(&Key::from(name)).unwrap())
            .unwrap()
    };
    let source = bounds("source");
    let upper = bounds("upper");
    let lower = bounds("lower");
    let only_upper = upper.origin + Offset::new(20., upper.size.height / 2.);
    let overlap = lower.origin + Offset::new(20., lower.size.height / 2.);
    let event = |runtime: &mut Runtime, phase, position, millis| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 7,
                device: 0,
                kind: PointerDeviceKind::Touch,
                phase,
                position,
                buttons: if phase == PointerPhase::Up { 0 } else { 1 },
                button: None,
                sample: Default::default(),
            },
            base + Duration::from_millis(millis),
        );
        runtime
            .run_frame_at(viewport, base + Duration::from_millis(millis + 1))
            .unwrap();
    };
    event(
        &mut runtime,
        PointerPhase::Down,
        source.origin + Offset::new(source.size.width / 2., source.size.height / 2.),
        16,
    );
    event(&mut runtime, PointerPhase::Move, only_upper, 32);
    let upper_participates = upper_behavior != Some(HitTestBehavior::DeferToChild);
    let upper_blocks = upper_behavior == Some(HitTestBehavior::Opaque);
    let mut expected = if upper_participates {
        vec![Event::Offered("upper", 1)]
    } else {
        vec![]
    };
    assert_eq!(*events.borrow(), expected);
    event(&mut runtime, PointerPhase::Move, overlap, 48);
    let drop_target = if upper_participates && upper_accepts {
        Some("upper")
    } else if upper_blocks {
        None
    } else {
        if upper_participates {
            expected.push(Event::Offered("upper", 1));
        }
        expected.push(Event::Offered("lower", 1));
        Some("lower")
    };
    assert_eq!(*events.borrow(), expected);
    event(&mut runtime, PointerPhase::Up, overlap, 64);
    if let Some(name) = drop_target {
        expected.push(Event::Dropped(name, 1));
    }
    assert_eq!(*events.borrow(), expected);
}

#[test]
fn accepting_upper_keeps_precedence_when_a_lower_target_enters_the_path() {
    exercise(true, None);
}

#[test]
fn rejecting_upper_does_not_block_a_new_lower_target_in_the_hit_path() {
    exercise(false, None);
}

#[test]
fn target_hit_test_behavior_distinguishes_deferred_and_opaque_empty_children() {
    exercise(true, Some(HitTestBehavior::DeferToChild));
    exercise(false, Some(HitTestBehavior::Opaque));
}
