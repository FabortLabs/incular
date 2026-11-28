use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollNotificationType};
use incular_widgets::{
    BuildContext, Builder, Directionality, EnsureVisible, Focus, FocusNode, ListView,
    ScrollPositionAlignmentPolicy as Policy, Scrollable, SizedBox, Text, Widget,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn mount(view: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    runtime.handle_platform_event(PlatformEvent::Metrics(WindowMetrics::new(
        PhysicalSize::new(800, 600),
        1.,
    )));
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    runtime
}

#[test]
fn jumping_to_a_new_offset_notifies_the_position_listener_once() {
    let scroll = ScrollController::new();
    let mut runtime = mount(
        ListView::builder(1000, |index| Text::new(index.to_string())).controller(scroll.clone()),
    );
    let count = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    let position = scroll.add_position_listener({
        let count = Rc::clone(&count);
        let events = Rc::clone(&events);
        move || {
            assert!(
                events.borrow().is_empty(),
                "position is published before Start"
            );
            count.set(count.get() + 1);
        }
    });
    let bubble = scroll.add_listener({
        let events = Rc::clone(&events);
        move |event| {
            events.borrow_mut().push(event.kind);
            true
        }
    });

    assert!(scroll.jump_to(100.));
    assert_eq!(scroll.offset(), 100.);
    assert_eq!(count.get(), 1);
    assert_eq!(
        *events.borrow(),
        [
            ScrollNotificationType::Start,
            ScrollNotificationType::Update,
            ScrollNotificationType::End,
        ]
    );
    assert!(!scroll.jump_to(100.));
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    assert_eq!(count.get(), 1);
    drop(bubble);
    drop(position);
    assert!(scroll.jump_to(120.));
    assert_eq!(count.get(), 1);
}

#[test]
fn ensure_visible_does_not_reverse_the_selected_alignment_policy() {
    let scroll = ScrollController::new();
    let list = ListView::new((0u64..11).map(|index| {
        Widget::from(Focus::new(SizedBox::new().height(110.)).node(FocusNode::new()))
            .with_key(index)
    }))
    .controller(scroll.clone());
    let mut runtime = mount(list);
    for (index, policy, expected) in [
        (0u64, Policy::KeepVisibleAtStart, 0.),
        (1, Policy::KeepVisibleAtStart, 0.),
        (1, Policy::KeepVisibleAtEnd, 0.),
        (4, Policy::KeepVisibleAtStart, 0.),
        (5, Policy::KeepVisibleAtStart, 0.),
        (5, Policy::KeepVisibleAtEnd, 60.),
        (0u64, Policy::KeepVisibleAtStart, 0.),
    ] {
        let target = runtime.tree().element_with_key(&index.into()).unwrap();
        Scrollable::ensure_visible(
            runtime.tree(),
            target,
            EnsureVisible::new().alignment_policy(policy),
        );
        assert_eq!(scroll.offset(), expected);
        runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    }
}

#[test]
fn a_fast_jump_defers_new_content_loads_without_building_skipped_rows() {
    let scroll = ScrollController::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let list = ListView::builder(1000, {
        let seen = Rc::clone(&seen);
        move |index| {
            let seen = Rc::clone(&seen);
            Builder::new(move |context: &BuildContext<'_>| {
                seen.borrow_mut().push((
                    index,
                    Scrollable::recommend_deferred_loading_for_context(context),
                ));
                SizedBox::new().height(500.).into()
            })
        }
    })
    .item_extent(500.)
    .controller(scroll.clone());
    let mut runtime = mount(list);
    assert_eq!(
        std::mem::take(&mut *seen.borrow_mut()),
        [(0, false), (1, false)]
    );

    assert!(scroll.jump_to(50_000.));
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    let first_jump = std::mem::take(&mut *seen.borrow_mut());
    assert!(
        first_jump.len() >= 2 && first_jump.len() < 8,
        "{first_jump:?}"
    );
    assert!(
        first_jump
            .iter()
            .all(|(index, defer)| *index >= 99 && *defer)
    );

    assert!(scroll.jump_to(50_500.));
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    let small_jump = seen.borrow();
    assert!(!small_jump.is_empty());
    assert!(small_jump.iter().all(|(_, defer)| !defer));
}

#[test]
fn a_position_listener_can_start_a_replacement_activity_during_a_wheel_update() {
    let scroll = ScrollController::new();
    let _runtime = mount(ListView::new([SizedBox::new().height(2000.)]).controller(scroll.clone()));
    let position = scroll.add_position_listener({
        let scroll = scroll.clone();
        move || {
            assert_eq!(scroll.offset(), 25.);
            assert!(scroll.begin_activity());
        }
    });
    scroll.apply_pointer_scroll(incular_scroll::ScrollPhysics::CLAMPING, 25.);
    assert!(
        scroll.is_scrolling(),
        "the wheel must not finish the replacement activity"
    );
    drop(position);
    scroll.end_activity();
    assert!(!scroll.is_scrolling());
}

#[test]
fn wheel_activity_is_observable_before_position_and_until_after_end() {
    let scroll = ScrollController::new();
    let mut runtime =
        mount(ListView::new([SizedBox::new().height(2000.)]).controller(scroll.clone()));
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |event| {
        let trace = Rc::clone(&trace);
        let scroll = scroll.clone();
        move || {
            trace
                .borrow_mut()
                .push((event, scroll.offset(), scroll.is_scrolling()))
        }
    };
    let _moving = scroll.add_scrolling_listener(record("scrolling"));
    let _position = scroll.add_position_listener(record("position"));
    let _bubble = scroll.add_listener({
        let start = record("start");
        let update = record("update");
        let end = record("end");
        move |notice| {
            match notice.kind {
                ScrollNotificationType::Start => start(),
                ScrollNotificationType::Update => update(),
                ScrollNotificationType::End => end(),
                _ => {}
            }
            false
        }
    });

    let _ = runtime.handle_input(incular_core::InputEvent::ScrollAt {
        position: incular_core::Offset::new(30., 30.),
        delta: incular_core::Offset::new(0., 25.),
    });
    assert_eq!(
        *trace.borrow(),
        [
            ("scrolling", 0., true),
            ("position", 25., true),
            ("start", 25., true),
            ("update", 25., true),
            ("end", 25., true),
            ("scrolling", 25., false),
        ],
    );
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
    assert_eq!(trace.borrow().len(), 6);
}

#[test]
fn scrolling_notifier_is_true_inside_the_position_listener_for_a_300_pixel_wheel_move() {
    let scroll = ScrollController::new();
    let mut runtime = mount(
        ListView::new((0..50).map(|i| {
            SizedBox::new()
                .height(100.)
                .child(Text::new(format!("Item {i}")))
        }))
        .controller(scroll.clone()),
    );
    let observed = Rc::new(Cell::new(false));
    let _position = scroll.add_position_listener({
        let observed = observed.clone();
        let scroll = scroll.clone();
        move || observed.set(scroll.is_scrolling())
    });
    let _ = runtime.handle_input(incular_core::InputEvent::ScrollAt {
        position: incular_core::Offset::new(400., 300.),
        delta: incular_core::Offset::new(0., 300.),
    });
    assert!(observed.get());
    assert_eq!(scroll.offset(), 300.);
    assert!(!scroll.is_scrolling());
}

#[test]
fn a_scrolling_observer_can_take_over_before_the_wheel_commits_pixels() {
    let scroll = ScrollController::new();
    let _runtime = mount(ListView::new([SizedBox::new().height(2000.)]).controller(scroll.clone()));
    let observer = scroll.add_scrolling_listener({
        let scroll = scroll.clone();
        move || {
            if scroll.is_scrolling() {
                assert_eq!(scroll.offset(), 0.);
                assert!(scroll.begin_activity());
            }
        }
    });
    scroll.apply_pointer_scroll(incular_scroll::ScrollPhysics::CLAMPING, 25.);
    assert!(scroll.is_scrolling());
    assert_eq!(scroll.offset(), 0.);
    drop(observer);
    scroll.end_activity();
    assert!(!scroll.is_scrolling());
}
