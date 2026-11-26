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

#[test]
fn a_scroll_calculator_receives_the_live_metrics_for_line_and_page_actions() {
    use std::cell::RefCell;

    use incular_widgets::{ScrollIncrementDetails, Scrollable, SingleChildScrollView};

    let controller = ScrollController::new();
    let key = Key::from("custom increment caller");
    let all = Rc::new(RefCell::new(Vec::<ScrollIncrementDetails>::new()));
    let increment = Rc::new(Cell::new(None));
    let scrollable = Scrollable::new({
        let key = key.clone();
        move |controller| {
            SingleChildScrollView::new(
                Widget::from(SizedBox::new().width(800.).height(1800.)).with_key(key.clone()),
            )
            .controller(controller.clone())
        }
    })
    .controller(controller.clone())
    .increment_calculator({
        let all = all.clone();
        let increment = increment.clone();
        move |details| {
            all.borrow_mut().push(details);
            increment
                .get()
                .unwrap_or_else(|| match details.increment_type {
                    ScrollIncrementType::Line => 37.,
                    ScrollIncrementType::Page => details.metrics.viewport_dimension / 3.,
                })
        }
    });
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, scrollable).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let origin = runtime.tree().element_with_key(&key).unwrap();

    let run = |runtime: &mut Runtime, time, direction, kind| {
        let intent = Intent::from(ScrollIntent::new(direction).increment_type(kind));
        assert_eq!(
            runtime.invoke_action_at(origin, &intent),
            Some(ActionResult::Handled)
        );
        frame(runtime, now, time);
        frame(runtime, now, time + 150);
    };
    run(
        &mut runtime,
        1,
        AxisDirection::Down,
        ScrollIncrementType::Line,
    );
    assert_eq!(controller.offset(), 37.);
    run(
        &mut runtime,
        200,
        AxisDirection::Down,
        ScrollIncrementType::Page,
    );
    assert_eq!(controller.offset(), 237.);
    run(
        &mut runtime,
        400,
        AxisDirection::Left,
        ScrollIncrementType::Line,
    );
    assert_eq!(controller.offset(), 237.);

    let seen = all.borrow();
    assert_eq!(
        seen.len(),
        2,
        "an off-axis request does not call the calculator"
    );
    assert_eq!(
        [seen[0].increment_type, seen[1].increment_type],
        [ScrollIncrementType::Line, ScrollIncrementType::Page]
    );
    assert_eq!([seen[0].metrics.pixels, seen[1].metrics.pixels], [0., 37.]);
    for entry in seen.iter() {
        assert_eq!(entry.metrics.axis_direction, AxisDirection::Down);
        assert_eq!(entry.metrics.viewport_dimension, 600.);
        assert_eq!(entry.metrics.min_scroll_extent, 0.);
        assert_eq!(entry.metrics.max_scroll_extent, 1200.);
    }
    drop(seen);
    increment.set(Some(f32::NAN));
    run(
        &mut runtime,
        600,
        AxisDirection::Down,
        ScrollIncrementType::Line,
    );
    assert_eq!(controller.offset(), 237.);
    assert!(!controller.has_pending_animation());
}

#[test]
fn a_parent_calculator_does_not_leak_into_a_nearer_viewport() {
    use incular_widgets::SingleChildScrollView;

    let parent = ScrollController::new();
    let child = ScrollController::new();
    let leaf_key = Key::from("nested intent caller");
    let outer = Key::from("outer intent caller");
    let calls = Rc::new(Cell::new(0));
    let nested = SingleChildScrollView::new(
        Widget::from(SizedBox::new().height(900.)).with_key(leaf_key.clone()),
    )
    .controller(child.clone());
    let column = Column::new([
        Widget::from(SizedBox::new().height(200.).child(nested)),
        Widget::from(SizedBox::new().height(900.)).with_key(outer.clone()),
    ]);
    let widget = SingleChildScrollView::builder()
        .child(column)
        .controller(parent.clone())
        .increment_calculator({
            let calls = calls.clone();
            move |_| {
                calls.set(calls.get() + 1);
                31.
            }
        })
        .build();
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let inner = runtime.tree().element_with_key(&leaf_key).unwrap();
    let outer = runtime.tree().element_with_key(&outer).unwrap();
    let intent = Intent::from(ScrollIntent::new(AxisDirection::Down));
    assert_eq!(
        runtime.invoke_action_at(inner, &intent),
        Some(ActionResult::Handled)
    );
    frame(&mut runtime, now, 1);
    frame(&mut runtime, now, 170);
    assert_eq!((parent.offset(), child.offset(), calls.get()), (0., 50., 0));

    assert_eq!(
        runtime.invoke_action_at(outer, &intent),
        Some(ActionResult::Handled)
    );
    frame(&mut runtime, now, 200);
    frame(&mut runtime, now, 370);
    assert_eq!(
        (parent.offset(), child.offset(), calls.get()),
        (31., 50., 1)
    );
}

#[test]
fn sliver_viewport_calculators_are_reconfigurable_without_replacing_the_controller() {
    use incular_widgets::{CustomScrollView, Sliver, SliverToBoxAdapter};

    for custom_view in [false, true] {
        let controller = ScrollController::new();
        let caller = Key::from("scroll calculator live");
        let content = || Widget::from(SizedBox::new().height(1800.)).with_key(caller.clone());
        let build = |custom_increment| {
            let viewport: Widget = if custom_view {
                let view = CustomScrollView::new(vec![
                    Box::new(SliverToBoxAdapter::new(content())) as Box<dyn Sliver>,
                ])
                .controller(controller.clone());
                if custom_increment {
                    view.increment_calculator(|_| 17.).into()
                } else {
                    view.into()
                }
            } else if custom_increment {
                ListView::typed_builder()
                    .children([content()])
                    .controller(controller.clone())
                    .increment_calculator(|_| 17.)
                    .build()
                    .into()
            } else {
                ListView::new([content()])
                    .controller(controller.clone())
                    .into()
            };
            Directionality::new(TextDirection::Ltr, viewport).into()
        };
        let mut runtime = Runtime::new(build(true)).unwrap();
        let now = Instant::now();
        frame(&mut runtime, now, 0);
        let row = runtime.tree().element_with_key(&caller).unwrap();
        let intent = Intent::from(ScrollIntent::new(AxisDirection::Down));
        assert_eq!(
            runtime.invoke_action_at(row, &intent),
            Some(ActionResult::Handled)
        );
        frame(&mut runtime, now, 1);
        frame(&mut runtime, now, 200);
        assert_eq!(controller.offset(), 17.);

        let root = runtime.tree().root().unwrap();
        runtime
            .schedule_update(root, build(false))
            .expect("remove increment calculator");
        frame(&mut runtime, now, 210);
        let row = runtime.tree().element_with_key(&caller).unwrap();
        assert_eq!(
            runtime.invoke_action_at(row, &intent),
            Some(ActionResult::Handled)
        );
        frame(&mut runtime, now, 220);
        frame(&mut runtime, now, 400);
        assert_eq!(controller.offset(), 67.);
    }
}

#[test]
fn a_wrapped_scrollable_calculator_follows_only_its_own_controller() {
    use incular_widgets::{Scrollable, SingleChildScrollView};

    let primary = ScrollController::new();
    let other = ScrollController::new();
    let owned_key = Key::from("owned increment origin");
    let other_key = Key::from("other increment origin");
    let scrollable = Scrollable::new({
        let owned_key = owned_key.clone();
        let other_key = other_key.clone();
        let other = other.clone();
        move |controller| {
            let scroll = |controller: &ScrollController, key: &Key| {
                SingleChildScrollView::new(
                    Widget::from(SizedBox::new().height(900.)).with_key(key.clone()),
                )
                .controller(controller.clone())
            };
            Column::new([
                Widget::from(
                    SizedBox::new()
                        .height(300.)
                        .child(scroll(controller, &owned_key)),
                ),
                Expanded::new(scroll(&other, &other_key)).into(),
            ])
        }
    })
    .controller(primary.clone())
    .increment_calculator(|_| 23.);
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, scrollable).into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now, 0);
    let intent = Intent::from(ScrollIntent::new(AxisDirection::Down));
    for (key, time) in [(&owned_key, 1), (&other_key, 200)] {
        let element = runtime.tree().element_with_key(key).unwrap();
        assert_eq!(
            runtime.invoke_action_at(element, &intent),
            Some(ActionResult::Handled)
        );
        frame(&mut runtime, now, time);
        frame(&mut runtime, now, time + 170);
    }
    assert_eq!((primary.offset(), other.offset()), (23., 50.));
}
