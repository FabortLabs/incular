use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Alignment, Constraints};
use incular_core::{InputEvent, Offset, PointerPhase, Rect, Size};
use incular_rendering::LayerLink;
use incular_runtime::Runtime;
use incular_widgets::{
    CompositedTransformFollower, CompositedTransformTarget, Directionality, GestureDetector,
    HitTestBehavior, Positioned, SizedBox, Stack, TextDirection, Widget, internal::Key,
};

const ALIGNMENTS: [Alignment; 5] = [
    Alignment::TOP_LEFT,
    Alignment::TOP_RIGHT,
    Alignment::CENTER,
    Alignment::BOTTOM_LEFT,
    Alignment::BOTTOM_RIGHT,
];
const TARGET: Offset = Offset::new(123., 456.);
const FOLLOWER_SIZE: Size = Size {
    width: 2.,
    height: 2.,
};
const TARGET_SIZE: Size = Size {
    width: 10.,
    height: 10.,
};

fn exercise(target_anchor: Alignment, follower_anchor: Alignment, expected: Offset) {
    let link = LayerLink::new();
    let calls = Rc::new(Cell::new(0));
    let child: Widget = SizedBox::square(2.).into();
    let detector = GestureDetector::new(child.with_key("follower child"))
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let calls = calls.clone();
            move || calls.set(calls.get() + 1)
        });
    let detector = Widget::from(detector).with_key("opaque detector");
    let target = Positioned::new(CompositedTransformTarget::new(
        link.clone(),
        SizedBox::square(10.),
    ))
    .left(TARGET.x)
    .top(TARGET.y);
    let follower = CompositedTransformFollower::new(link, detector)
        .target_anchor(target_anchor)
        .follower_anchor(follower_anchor);
    let scene = Directionality::new(
        TextDirection::Ltr,
        Stack::new([Widget::from(target), Widget::from(follower)]),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();
    for key in ["opaque detector", "follower child"] {
        let tree = runtime.tree();
        let element = tree.element_with_key(&Key::from(key)).unwrap();
        assert_eq!(
            tree.element_bounds(element),
            Some(Rect::from_origin_size(expected, FOLLOWER_SIZE)),
            "target {target_anchor:?}, follower {follower_anchor:?}, key {key}"
        );
    }
    assert_eq!(calls.get(), 0);
    for (index, phase) in [PointerPhase::Down, PointerPhase::Up]
        .into_iter()
        .enumerate()
    {
        let _ = runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: expected + Offset::new(1., 1.),
            },
            now + Duration::from_millis(16 + index as u64),
        );
    }
    assert_eq!(calls.get(), 1);
}

#[test]
fn original_opaque_detector_uses_the_default_anchors() {
    exercise(Alignment::TOP_LEFT, Alignment::TOP_LEFT, TARGET);
}

#[test]
fn applying_each_anchor_pair_moves_the_actual_gesture_target() {
    for target in ALIGNMENTS {
        for follower in ALIGNMENTS {
            let expected = TARGET
                + Offset::new(
                    (target.x + 1.) * TARGET_SIZE.width / 2.,
                    (target.y + 1.) * TARGET_SIZE.height / 2.,
                )
                - Offset::new(
                    (follower.x + 1.) * FOLLOWER_SIZE.width / 2.,
                    (follower.y + 1.) * FOLLOWER_SIZE.height / 2.,
                );
            exercise(target, follower, expected);
        }
    }
}
