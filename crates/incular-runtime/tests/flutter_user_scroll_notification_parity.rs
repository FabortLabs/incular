//! Flutter's UserScrollNotification is driven by the user's direction, not
//! by pixel changes. Pulling down at the top emits a direction transition
//! even when clamping physics prevent movement.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollNotificationType, ScrollPhysics};
use incular_widgets::{ColoredBox, ListView, SizedBox};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct PullTrace {
    events: Vec<ScrollNotificationType>,
    before_first_move: usize,
    after_first_move: usize,
    first_offset: f32,
    final_offset: f32,
}

fn pull_down_at_top(physics: ScrollPhysics) -> PullTrace {
    let bounds = Constraints::tight(Size::new(800., 600.));
    let controller = ScrollController::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let _subscription = controller.add_notification_listener(move |event| {
        if event.kind != ScrollNotificationType::Metrics {
            observed.borrow_mut().push(event.kind);
        }
        false
    });
    let list = ListView::builder(40, |_| ColoredBox::new(Color::WHITE, SizedBox::shrink()))
        .item_extent(50.)
        .controller(controller.clone())
        .physics(physics);
    let mut runtime = Runtime::new(list.into()).expect("mount fixed-extent list");
    let start = Instant::now();
    let frame = |runtime: &mut Runtime, index: u64| {
        runtime
            .run_frame_at(bounds, start + Duration::from_millis(16 * index))
            .expect("render list frame");
    };
    let touch = |runtime: &mut Runtime, index: u64, phase, y| {
        let (buttons, button) = match phase {
            PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
            PointerPhase::Move => (PRIMARY_POINTER_BUTTON, None),
            PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
            _ => unreachable!("pull fixture only sends down, move and up"),
        };
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons,
                button,
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(400., y),
            },
            start + Duration::from_millis(16 * index),
        );
    };

    frame(&mut runtime, 0);
    frame(&mut runtime, 1);
    touch(&mut runtime, 1, PointerPhase::Down, 200.);
    frame(&mut runtime, 2);
    let before_first_move = events.borrow().len();
    touch(&mut runtime, 2, PointerPhase::Move, 250.);
    frame(&mut runtime, 3);
    let after_first_move = events.borrow().len();
    let first_offset = controller.offset();
    for (index, y) in [(3, 300.), (4, 350.), (5, 400.)] {
        touch(&mut runtime, index, PointerPhase::Move, y);
        frame(&mut runtime, index + 1);
    }
    touch(&mut runtime, 6, PointerPhase::Up, 400.);
    for index in 7..=36 {
        frame(&mut runtime, index);
    }

    PullTrace {
        events: events.take(),
        before_first_move,
        after_first_move,
        first_offset,
        final_offset: controller.offset(),
    }
}

#[test]
fn clamped_touch_overscroll_notifies_user_scroll_on_pull_and_idle() {
    use ScrollNotificationType::{End, Overscroll, Start, UserScroll};

    let trace = pull_down_at_top(ScrollPhysics::clamping());
    assert_eq!(trace.first_offset, 0., "the first move is clamped");
    assert_eq!(trace.final_offset, 0.);
    assert_eq!(
        trace
            .events
            .iter()
            .copied()
            .filter(|event| *event != UserScroll)
            .collect::<Vec<_>>(),
        [Start, Overscroll, Overscroll, Overscroll, Overscroll, End],
        "control: one drag bracket and four boundary samples"
    );
    assert_eq!(&trace.events[..trace.before_first_move], &[Start]);
    assert_eq!(
        &trace.events[..trace.after_first_move],
        &[Start, UserScroll, Overscroll],
        "the direction changes before applying the first clamped offset"
    );
    assert_eq!(
        trace.events,
        [
            Start, UserScroll, Overscroll, Overscroll, Overscroll, Overscroll, End, UserScroll
        ],
        "steady direction emits once, and going idle emits after End"
    );
}

#[test]
fn bouncing_touch_overscroll_notifies_user_scroll_before_first_update() {
    use ScrollNotificationType::{Start, Update, UserScroll};

    let trace = pull_down_at_top(ScrollPhysics::BOUNCING);
    assert!((trace.first_offset + 50.).abs() < 0.001);
    assert!(
        (-7. ..-6.).contains(&trace.final_offset),
        "still bouncing at 576 ms"
    );
    assert_eq!(
        trace
            .events
            .iter()
            .copied()
            .filter(|event| *event != UserScroll)
            .collect::<Vec<_>>(),
        std::iter::once(Start)
            .chain(std::iter::repeat_n(Update, 33))
            .collect::<Vec<_>>(),
        "control: scroll updates during the pull and ballistic return"
    );
    assert_eq!(&trace.events[..trace.before_first_move], &[Start]);
    assert_eq!(
        &trace.events[..trace.after_first_move],
        &[Start, UserScroll, Update],
        "the direction changes before the first scroll update"
    );
    assert_eq!(
        trace
            .events
            .iter()
            .filter(|event| **event == UserScroll)
            .count(),
        1,
        "the drag direction remains unchanged during the ballistic return"
    );
}
