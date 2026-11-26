//! Flutter 3.50.0 widgets/scrollable_helpers_test.dart:546.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::{AxisDirection, Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionResult, Column, Directionality, Expanded, Intent, ListView, PrimaryScrollController,
    ScrollController, ScrollIncrementType, ScrollIntent, SizedBox, Text, Widget, internal::Key,
};

const SIZE: Size = Size {
    width: 800.,
    height: 600.,
};

fn list(controller: &ScrollController, reverse: bool) -> Widget {
    let children = [
        "The cow as white as milk",
        "The cape as red as blood",
        "The hair as yellow as corn",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, text)| {
        Widget::from(SizedBox::new().height(600.).child(Text::new(text)))
            .with_key(Key::Value((index + 1) as u64))
    });
    ListView::new(children)
        .controller(controller.clone())
        .reverse(reverse)
        .into()
}

fn frame(runtime: &mut Runtime, now: Instant, milliseconds: u64) {
    runtime
        .run_frame_at(
            Constraints::tight(SIZE),
            now + Duration::from_millis(milliseconds),
        )
        .expect("frame");
}

fn visible_pages(runtime: &Runtime) -> Vec<u64> {
    (1..=3)
        .filter(|&index| {
            runtime
                .tree()
                .element_with_key(&Key::Value(index))
                .and_then(|id| runtime.tree().element_bounds(id))
                .is_some_and(|bounds| {
                    bounds.origin.y < SIZE.height && bounds.origin.y + bounds.size.height > 0.
                })
        })
        .collect()
}

#[test]
fn page_intents_scroll_from_the_invoking_element_without_key_events() {
    let controller = ScrollController::new();
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, list(&controller, false)).into())
            .unwrap();
    let observed = Rc::new(Cell::new(0));
    let _keys = runtime.add_hardware_key_handler({
        let observed = observed.clone();
        move |_| {
            observed.set(observed.get() + 1);
            false
        }
    });
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    assert_eq!(visible_pages(&runtime), [1]);

    let page = Intent::from(
        ScrollIntent::new(AxisDirection::Down).increment_type(ScrollIncrementType::Page),
    );
    for (time, offset, expected) in [(1, 480., &[1, 2][..]), (1002, 960., &[2, 3][..])] {
        let origin = runtime
            .tree()
            .element_with_key(&Key::Value(2))
            .expect("first adjacent page remains in the cache");
        assert_eq!(
            runtime.invoke_action_at(origin, &page),
            Some(ActionResult::Handled)
        );
        frame(&mut runtime, now, time);
        frame(&mut runtime, now, time + 1000);
        assert_eq!(controller.offset(), offset);
        assert_eq!(visible_pages(&runtime), expected);
    }
    assert_eq!(observed.get(), 0);
}

#[test]
fn an_intent_outside_the_viewport_can_use_its_inherited_primary_controller() {
    let controller = ScrollController::new();
    let origin = Key::from("intent origin");
    let column = Column::new([
        Widget::box_(Size::ZERO, Color::TRANSPARENT).with_key(origin.clone()),
        Expanded::new(list(&controller, false)).into(),
    ]);
    let widget = Directionality::new(
        TextDirection::Ltr,
        PrimaryScrollController::new(controller.clone(), column),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let caller = runtime
        .tree()
        .element_with_key(&origin)
        .expect("sibling caller");
    let line = Intent::from(ScrollIntent::new(AxisDirection::Down));
    assert_eq!(
        runtime.invoke_action_at(caller, &line),
        Some(ActionResult::Handled)
    );
    frame(&mut runtime, now, 1);
    frame(&mut runtime, now, 200);
    assert_eq!(controller.offset(), 50.);
}

#[test]
fn scroll_intents_use_the_physical_direction_and_ignore_the_other_axis() {
    let controller = ScrollController::new();
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, list(&controller, true)).into())
            .unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let origin = runtime
        .tree()
        .element_with_key(&Key::Value(1))
        .expect("first row");
    for (time, direction, expected) in [
        (1, AxisDirection::Up, 50.),
        (300, AxisDirection::Left, 50.),
        (600, AxisDirection::Down, 0.),
    ] {
        assert_eq!(
            runtime.invoke_action_at(origin, &Intent::from(ScrollIntent::new(direction))),
            Some(ActionResult::Handled)
        );
        frame(&mut runtime, now, time);
        frame(&mut runtime, now, time + 150);
        assert_eq!(controller.offset(), expected);
    }
}

#[test]
fn a_nearer_authored_action_shadows_default_intent_scrolling() {
    use incular_widgets::{Actions, FocusableActionDetector};

    let controller = ScrollController::new();
    let command = ScrollIntent::new(AxisDirection::Down);
    let calls = Rc::new(Cell::new(0));
    let mut actions = Actions::<ScrollIntent>::typed();
    actions.register(command, {
        let calls = calls.clone();
        move || calls.set(calls.get() + 1)
    });
    let content = FocusableActionDetector::new(list(&controller, false)).actions(Rc::new(actions));
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, content).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let row = runtime.tree().element_with_key(&Key::Value(1)).unwrap();
    assert_eq!(
        runtime.invoke_action_at(row, &Intent::from(command)),
        Some(ActionResult::Handled),
    );
    frame(&mut runtime, now, 200);
    assert_eq!(calls.get(), 1);
    assert_eq!(controller.offset(), 0.);
}
