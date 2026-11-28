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
