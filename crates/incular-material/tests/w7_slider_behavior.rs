use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_material::{RangeSlider, RangeValues};
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticAction, SemanticActionKind};
use std::{cell::Cell, rc::Rc};

fn pointer(runtime: &mut Runtime, phase: PointerPhase, x: f32, y: f32) {
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase,
        position: Offset::new(x, y),
    });
}

fn assert_range(actual: RangeValues, start: f32, end: f32) {
    assert!((actual.start - start).abs() < 0.001, "actual={actual:?}");
    assert!((actual.end - end).abs() < 0.001, "actual={actual:?}");
}

#[test]
fn range_drag_uses_press_origin_not_accumulated_total_delta() {
    let last = Rc::new(Cell::new(RangeValues::new(0.2, 0.8)));
    let observed = last.clone();
    let mut runtime = Runtime::new(
        RangeSlider::new(RangeValues::new(0.2, 0.8))
            .divisions(Some(10))
            .on_changed(move |next| observed.set(next))
            .into(),
    )
    .expect("range slider runtime");
    let constraints = Constraints::tight(Size::new(200.0, 32.0));
    runtime.run_frame(constraints).expect("initial frame");

    // The press lands on the start thumb (0.2 of the 200px track, x = 40).
    // Flutter 3.41.7 `_RenderRangeSlider` puts its drag and tap recognizers in
    // one `GestureArenaTeam` (material/range_slider.dart:931-943), so the team
    // is the arena's sole member and wins right after the press
    // (gestures/arena.dart:254-255), handing the win to the drag, its first
    // member (gestures/team.dart `_CombiningGestureArenaMember.acceptGesture`).
    // The drag therefore starts at the press and every move is an update: the
    // thumb stays under the pointer, 60px of travel (0.2 -> 0.5), never the
    // accumulated sum of the running totals (20 + 40 + 60px).
    pointer(&mut runtime, PointerPhase::Down, 40.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 60.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 80.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 100.0, 15.0);
    pointer(&mut runtime, PointerPhase::Up, 100.0, 15.0);

    let value = last.get();
    assert!((value.start - 0.5).abs() < 0.001, "value={value:?}");
    assert!((value.end - 0.8).abs() < 0.001, "value={value:?}");
}

#[test]
fn second_range_drag_uses_the_new_start_value() {
    let last = Rc::new(Cell::new(RangeValues::new(0.2, 0.8)));
    let observed = last.clone();
    let mut runtime = Runtime::new(
        RangeSlider::new(RangeValues::new(0.2, 0.8))
            .divisions(Some(10))
            .on_changed(move |next| observed.set(next))
            .into(),
    )
    .expect("range slider runtime");
    let constraints = Constraints::tight(Size::new(200.0, 32.0));
    runtime.run_frame(constraints).expect("initial frame");

    // Each drag starts at its press: the slider's gesture team is the sole
    // arena member, so Flutter 3.41.7 accepts the drag right after the press
    // (material/range_slider.dart:931-943, gestures/arena.dart:254-255) and the
    // thumb follows the full 60px (0.2 -> 0.5).
    pointer(&mut runtime, PointerPhase::Down, 40.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 60.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 80.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 100.0, 15.0);
    pointer(&mut runtime, PointerPhase::Up, 100.0, 15.0);
    runtime.run_frame(constraints).expect("after first drag");
    assert!(
        (last.get().start - 0.5).abs() < 0.001,
        "first={:?}",
        last.get()
    );

    // The second drag presses the moved thumb (x = 100) and travels 40px from
    // the new start value: 0.5 -> 0.7.
    pointer(&mut runtime, PointerPhase::Down, 100.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 120.0, 15.0);
    pointer(&mut runtime, PointerPhase::Move, 140.0, 15.0);
    pointer(&mut runtime, PointerPhase::Up, 140.0, 15.0);
    assert!(
        (last.get().start - 0.7).abs() < 0.001,
        "second={:?}",
        last.get()
    );
}

#[test]
fn both_range_thumbs_are_keyboard_and_semantically_reachable() {
    let last = Rc::new(Cell::new(RangeValues::new(0.2, 0.8)));
    let observed = last.clone();
    let mut runtime = Runtime::new(
        RangeSlider::new(RangeValues::new(0.2, 0.8))
            .divisions(Some(10))
            .on_changed(move |next| observed.set(next))
            .into(),
    )
    .expect("range slider runtime");
    let constraints = Constraints::tight(Size::new(200.0, 32.0));
    runtime.run_frame(constraints).expect("initial frame");

    let mut sliders = runtime
        .tree()
        .semantics()
        .iter()
        .filter(|(_, node)| node.role == Role::Slider)
        .map(|(id, node)| {
            (
                id,
                node.state.numeric_value.unwrap_or_default(),
                node.actions.clone(),
            )
        })
        .collect::<Vec<_>>();
    sliders.sort_by(|left, right| left.1.total_cmp(&right.1));
    assert_eq!(
        sliders.len(),
        2,
        "each range thumb owns one slider semantic node"
    );
    for (_, _, actions) in &sliders {
        assert!(actions.contains(&SemanticActionKind::Focus));
        assert!(actions.contains(&SemanticActionKind::Increment));
        assert!(actions.contains(&SemanticActionKind::Decrement));
    }
    let start = sliders[0].0;
    let end = sliders[1].0;

    assert!(runtime.dispatch_semantic_action(start, SemanticAction::Increment));
    assert_range(last.get(), 0.3, 0.8);
    assert!(runtime.dispatch_semantic_action(end, SemanticAction::Increment));
    assert_range(last.get(), 0.3, 0.9);

    assert!(runtime.dispatch_semantic_action(start, SemanticAction::Focus));
    let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    )));
    assert_range(last.get(), 0.4, 0.9);

    assert!(runtime.dispatch_semantic_action(end, SemanticAction::Focus));
    let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
        KeyboardKey::Named(NamedKey::ArrowLeft),
        Code::ArrowLeft,
    )));
    assert_range(last.get(), 0.4, 0.8);
}
