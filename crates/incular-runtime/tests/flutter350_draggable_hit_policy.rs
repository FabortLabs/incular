use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    DragDropContext, DragTarget, Draggable, GestureDetector, HitTestBehavior, LongPressDraggable,
    Positioned, SizedBox, Stack, Widget,
};

#[test]
fn empty_draggable_uses_the_same_hit_policies_as_other_retained_annotators() {
    for policy in [
        None,
        Some(HitTestBehavior::Opaque),
        Some(HitTestBehavior::Translucent),
    ] {
        let context = DragDropContext::new();
        let starts = Rc::new(Cell::new(0));
        let lower_taps = Rc::new(Cell::new(0));
        let empty = || SizedBox::new().width(80.).height(50.);
        let source = Draggable::new(context.clone(), 1, empty())
            .feedback(|_| SizedBox::square(20.).into())
            .on_start({
                let starts = starts.clone();
                move |_| starts.set(starts.get() + 1)
            });
        assert_eq!(
            source.configured_hit_test_behavior(),
            HitTestBehavior::DeferToChild
        );
        let source: Widget = if let Some(policy) = policy {
            source.hit_test_behavior(policy).into()
        } else {
            source.into()
        };
        let lower = GestureDetector::new(empty())
            .behavior(HitTestBehavior::Opaque)
            .on_tap({
                let lower_taps = lower_taps.clone();
                move || lower_taps.set(lower_taps.get() + 1)
            });
        let mut runtime = Runtime::new(
            Stack::new([
                Positioned::new(lower).left(20.).top(20.),
                Positioned::new(source).left(20.).top(20.),
            ])
            .into(),
        )
        .unwrap();
        let base = Instant::now();
        let view = Constraints::tight(Size::new(320., 180.));
        runtime.run_frame_at(view, base).unwrap();
        let step = |rt: &mut Runtime, phase, position, ms| {
            let now = base + Duration::from_millis(ms);
            let _ = rt.handle_input_at(InputEvent::Pointer { phase, position }, now);
            rt.run_frame_at(view, now).unwrap();
        };
        let point = Offset::new(50., 40.);
        step(&mut runtime, PointerPhase::Down, point, 16);
        let opaque = policy == Some(HitTestBehavior::Opaque);
        assert_eq!(starts.get(), usize::from(opaque), "{policy:?}");
        step(&mut runtime, PointerPhase::Up, point, 32);
        assert_eq!(lower_taps.get(), usize::from(!opaque), "{policy:?}");

        if policy == Some(HitTestBehavior::Translucent) {
            step(&mut runtime, PointerPhase::Down, point, 48);
            let moved = point + Offset::new(0., 35.);
            step(&mut runtime, PointerPhase::Move, moved, 64);
            assert_eq!(starts.get(), 1);
            assert!(context.feedback().is_some());
            step(&mut runtime, PointerPhase::Up, moved, 80);
            assert_eq!(lower_taps.get(), 1);
        }
        assert!(!context.is_dragging());
    }
}

#[test]
fn opaque_long_press_source_can_start_over_an_empty_child() {
    let context = DragDropContext::new();
    let source =
        LongPressDraggable::new(context.clone(), 1, SizedBox::new().width(80.).height(50.))
            .hit_test_behavior(HitTestBehavior::Opaque)
            .feedback(|_| SizedBox::square(20.).into());
    assert_eq!(
        source.configured_hit_test_behavior(),
        HitTestBehavior::Opaque
    );
    let mut runtime =
        Runtime::new(Stack::new([Positioned::new(source).left(20.).top(20.)]).into()).unwrap();
    let view = Constraints::tight(Size::new(320., 180.));
    let base = Instant::now();
    runtime.run_frame_at(view, base).unwrap();
    let point = Offset::new(50., 40.);
    let _ = runtime.handle_input_at(
        InputEvent::Pointer {
            phase: PointerPhase::Down,
            position: point,
        },
        base + Duration::from_millis(16),
    );
    runtime
        .run_frame_at(view, base + Duration::from_millis(515))
        .unwrap();
    assert!(context.feedback().is_none());
    runtime
        .run_frame_at(view, base + Duration::from_millis(516))
        .unwrap();
    assert!(context.feedback().is_some());
    let _ = runtime.handle_input_at(
        InputEvent::Pointer {
            phase: PointerPhase::Up,
            position: point,
        },
        base + Duration::from_millis(532),
    );
    runtime
        .run_frame_at(view, base + Duration::from_millis(548))
        .unwrap();
    assert!(!context.is_dragging());
}

#[test]
fn configured_drag_target_reports_defer_to_child_instead_of_its_translucent_default() {
    let target = DragTarget::new(DragDropContext::<i32>::new(), SizedBox::expand());
    assert_eq!(
        target.configured_hit_test_behavior(),
        HitTestBehavior::Translucent
    );
    assert_eq!(
        target
            .hit_test_behavior(HitTestBehavior::DeferToChild)
            .configured_hit_test_behavior(),
        HitTestBehavior::DeferToChild
    );
}
