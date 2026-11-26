use std::time::Duration;

use incular_config::Constraints;
use incular_core::{
    Color, DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind,
    PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollPhysics};
use incular_widgets::{
    ListView, MultitouchDragStrategy, ScrollBehavior, ScrollConfiguration, Widget,
};

fn compare(strategy: MultitouchDragStrategy, added: f32) {
    let controller = ScrollController::new();
    let list = ListView::new(
        (0..20)
            .map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE))
            .collect::<Vec<_>>(),
    )
    .controller(controller.clone())
    .physics(ScrollPhysics::clamping())
    .drag_start_behavior(DragStartBehavior::Down);
    let mut runtime = Runtime::new(
        ScrollConfiguration::new(list)
            .behavior(ScrollBehavior::new().multitouch_drag_strategy(strategy))
            .into(),
    )
    .unwrap();
    let at = Instant::now();
    let bounds = Constraints::tight(Size::new(100., 100.));
    runtime.run_frame_at(bounds, at).unwrap();
    assert!(controller.jump_to(100.));
    assert_eq!(controller.offset(), 100.);
    let send = |runtime: &mut Runtime, pointer, phase, x, y, at| {
        runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                position: Offset::new(x, y),
                sample: PointerSampleMetadata::EMPTY,
            },
            at,
        );
    };

    send(&mut runtime, 5, PointerPhase::Down, 45., 80., at);
    send(&mut runtime, 5, PointerPhase::Move, 45., 40., at);
    assert_eq!(controller.offset(), 140.);
    let original_activity = controller
        .current_activity_id()
        .expect("drag owns the activity");
    let next = at + Duration::from_millis(16);
    runtime.run_frame_at(bounds, next).unwrap();
    send(&mut runtime, 6, PointerPhase::Down, 55., 80., next);
    send(&mut runtime, 5, PointerPhase::Move, 45., 10., next);
    send(&mut runtime, 6, PointerPhase::Move, 55., 60., next);
    assert_eq!(controller.offset(), 140. + added, "{strategy:?}");
    assert_eq!(controller.current_activity_id(), Some(original_activity));

    // Removing the original owner must not stop or restart the activity.
    send(&mut runtime, 5, PointerPhase::Cancel, 45., 10., next);
    assert_eq!(controller.current_activity_id(), Some(original_activity));
    let last_frame = next + Duration::from_millis(16);
    runtime.run_frame_at(bounds, last_frame).unwrap();
    send(&mut runtime, 6, PointerPhase::Move, 55., 50., last_frame);
    assert_eq!(controller.offset(), 150. + added);
    send(&mut runtime, 6, PointerPhase::Cancel, 55., 50., last_frame);
    assert_eq!(controller.current_activity_id(), None);
}

#[test]
fn physical_contacts_respect_the_inherited_latest_pointer_strategy() {
    compare(MultitouchDragStrategy::LatestPointer, 20.);
}

#[test]
fn physical_contacts_respect_the_inherited_sum_strategy() {
    compare(MultitouchDragStrategy::SumAllPointers, 50.);
}

#[test]
fn physical_contacts_respect_the_inherited_frame_boundary_strategy() {
    compare(MultitouchDragStrategy::AverageBoundaryPointers, 30.);
}
