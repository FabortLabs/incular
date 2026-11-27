//! Flutter widgets/dismissible_test.dart: disabled drag behavior and identity.

use incular_config::{Constraints, TextDirection};
use incular_core::{
    DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, DismissDirection, Dismissible, ListView, Padding, ScrollController,
    ScrollPhysics, SizedBox, Text, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

#[test]
fn none_does_not_dismiss_or_win_an_ancestor_scroll_drag() {
    for scrollable in [false, true] {
        let dismissed = Rc::new(RefCell::new(Vec::new()));
        let rows = (0..9).map(|i| {
            let events = dismissed.clone();
            let child = SizedBox::new().width(100.).height(100.).child(
                Widget::from(Text::new(i.to_string())).with_key(Key::String(format!("item {i}"))),
            );
            Widget::from(
                Dismissible::new(DismissDirection::None, child, move |_| {
                    events.borrow_mut().push(i)
                })
                .drag_start_behavior(DragStartBehavior::Down)
                .cross_axis_end_offset(0.5),
            )
            .with_key(i as u64)
        });
        let controller = ScrollController::new();
        let list = ListView::new(rows)
            .item_extent(100.)
            .controller(controller.clone())
            .drag_start_behavior(DragStartBehavior::Down);
        let list = if scrollable {
            list
        } else {
            list.physics(ScrollPhysics::NEVER_SCROLLABLE)
        };
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, Padding::all(10., list)).into())
                .unwrap();
        let bounds = Constraints::tight(VIEW);
        let mut time = Instant::now();
        runtime.run_frame_at(bounds, time).unwrap();
        let root_item = runtime
            .tree()
            .element_with_key(&Key::from("item 0"))
            .unwrap();
        let box_ = runtime.tree().element_bounds(root_item).unwrap();
        let x = box_.origin.x;
        let y = box_.origin.y;
        let w = box_.size.width;
        let h = box_.size.height;
        let left = (Offset::new(x + w - 0.1, y), Offset::new(x - 0.1, y));
        let right = (Offset::new(x + 0.1, y), Offset::new(x + w + 0.1, y));
        let up = (Offset::new(x, y + h - 0.1), Offset::new(x, y - 0.1));
        let down = (Offset::new(x + 0.1, y), Offset::new(x + 0.1, y + h));
        let drags = if scrollable {
            [left, right, down, up]
        } else {
            [left, right, up, down]
        };
        for (step, (from, to)) in drags.into_iter().enumerate() {
            time += Duration::from_millis(64);
            for (phase, position) in [
                (PointerPhase::Down, from),
                (PointerPhase::Move, to),
                (PointerPhase::Up, to),
            ] {
                runtime.handle_input_at(
                    InputEvent::PointerWithMetadata {
                        pointer: step as u64 + 1,
                        device: 1,
                        kind: PointerDeviceKind::Touch,
                        buttons: if phase == PointerPhase::Up {
                            0
                        } else {
                            PRIMARY_POINTER_BUTTON
                        },
                        button: matches!(phase, PointerPhase::Up | PointerPhase::Down)
                            .then_some(PRIMARY_POINTER_BUTTON),
                        sample: PointerSampleMetadata::EMPTY,
                        phase,
                        position,
                    },
                    time,
                );
            }
            runtime.run_frame_at(bounds, time).unwrap();
            assert!(dismissed.borrow().is_empty());
            let expected = if scrollable && step == 3 { 100. } else { 0. };
            assert!(
                (controller.offset() - expected).abs() < 0.001,
                "scrollable={scrollable}, step={step}, offset={}",
                controller.offset()
            );
        }
        if !scrollable {
            assert_eq!(
                runtime.tree().element_with_key(&Key::from("item 0")),
                Some(root_item)
            );
        }
    }
}

#[test]
fn changing_to_none_preserves_the_childs_render_identity() {
    let scene = |direction| {
        Directionality::new(
            TextDirection::Ltr,
            Widget::from(
                Dismissible::new(
                    direction,
                    SizedBox::new()
                        .width(100.)
                        .height(100.)
                        .child(Widget::from(Text::new("I Love Flutter!")).with_key("child")),
                    |_| {},
                )
                .resize_duration(None)
                .drag_start_behavior(DragStartBehavior::Down),
            )
            .with_key("Dismissible"),
        )
    };
    let mut runtime = Runtime::new(scene(DismissDirection::Horizontal).into()).unwrap();
    let bounds = Constraints::tight(VIEW);
    runtime.run_frame(bounds).unwrap();
    let root = runtime.tree().root().unwrap();
    let child = runtime
        .tree()
        .element_with_key(&Key::from("child"))
        .unwrap();
    let render = runtime.tree().render_id(child).unwrap();
    runtime
        .schedule_update(root, scene(DismissDirection::None).into())
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(
        runtime.tree().element_with_key(&Key::from("child")),
        Some(child)
    );
    assert_eq!(runtime.tree().render_id(child), Some(render));
}
