//! Mounted counterparts of three Flutter 3.50.0 ListWheelScrollView tests.
//! Incular's fixed-extent controller takes an explicit item extent where
//! Flutter learns that extent from the attached scroll position.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_scroll::ScrollPhysics;
use incular_widgets::{
    Align, Alignment, ChangeReportingBehavior, ColoredBox, FixedExtentScrollController,
    ListWheelScrollView, SizedBox, WheelChildDelegate, Widget,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const ITEM_EXTENT: f32 = 100.;

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), at)
        .expect("lay out wheel");
}

fn rows(count: usize) -> impl Iterator<Item = Widget> {
    (0..count).map(|_| SizedBox::expand().into())
}

fn wheel(
    controller: &FixedExtentScrollController,
    children: impl IntoIterator<Item = Widget>,
    selected: Option<&Rc<RefCell<Vec<usize>>>>,
) -> Widget {
    let mut wheel = ListWheelScrollView::new(
        controller.controller(),
        ITEM_EXTENT,
        WheelChildDelegate::children(children.into_iter().collect()),
    );
    wheel.viewport_mut().set_physics(ScrollPhysics::clamping());
    if let Some(selected) = selected {
        let selected = Rc::clone(selected);
        wheel.viewport_mut().set_selection_callback(
            ChangeReportingBehavior::OnScrollUpdate,
            Some(move |index| selected.borrow_mut().push(index)),
        );
    }
    wheel.into()
}

#[test]
fn wheel_does_not_report_selection_on_first_build() {
    let fixed = FixedExtentScrollController::default();
    let selected = Rc::new(RefCell::new(Vec::new()));
    let child = SizedBox::new()
        .width(200.)
        .child(ColoredBox::new(Color::WHITE, SizedBox::shrink()));
    let mut runtime = Runtime::new(wheel(&fixed, [child.into()], Some(&selected)))
        .expect("mount one-child wheel");
    frame(&mut runtime, Instant::now());

    assert!(fixed.controller().attachment_id().is_some());
    assert_eq!(fixed.selected_item(ITEM_EXTENT), 0);
    assert!(selected.borrow().is_empty());
}

#[test]
fn wheel_selection_callback_tracks_controller_across_drag_midpoint_and_reversal() {
    let fixed = FixedExtentScrollController::with_item_extent(10, ITEM_EXTENT);
    let selected = Rc::new(RefCell::new(Vec::new()));
    let mut runtime =
        Runtime::new(wheel(&fixed, rows(100), Some(&selected))).expect("mount selected wheel");
    let start = Instant::now();
    frame(&mut runtime, start);
    assert!(fixed.controller().attachment_id().is_some());
    assert_eq!(fixed.controller().offset(), 1000.);
    assert_eq!(fixed.selected_item(ITEM_EXTENT), 10);

    let at = |ms| start + Duration::from_millis(ms);
    let finger = |runtime: &mut Runtime, phase, y, ms| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: PRIMARY_POINTER_BUTTON,
                button: (phase == PointerPhase::Down).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(10., y),
            },
            at(ms),
        );
    };
    finger(&mut runtime, PointerPhase::Down, 10., 16);
    // Match the upstream 49-px, 1-px, then reverse 70-px touch moves.
    let samples = [(32, -39.), (48, -40.), (64, 30.)].map(|(ms, y)| {
        finger(&mut runtime, PointerPhase::Move, y, ms);
        frame(&mut runtime, at(ms));
        (fixed.selected_item(ITEM_EXTENT), selected.borrow().clone())
    });
    assert_eq!(samples, [(10, vec![]), (11, vec![11]), (10, vec![11, 10])]);
}

#[test]
fn reused_wheel_controller_restarts_from_initial_item_after_detach() {
    let fixed = FixedExtentScrollController::with_item_extent(3, ITEM_EXTENT);
    let controller = fixed.controller();
    let mounted = || {
        Align::new(
            Alignment::CENTER,
            SizedBox::expand().child(wheel(&fixed, rows(100), None)),
        )
        .into()
    };
    let mut runtime = Runtime::new(mounted()).expect("mount wheel");
    let start = Instant::now();
    frame(&mut runtime, start);
    let root = runtime.tree().root().expect("stable alignment root");
    assert!(controller.attachment_id().is_some());
    assert_eq!(
        (fixed.selected_item(ITEM_EXTENT), controller.offset()),
        (3, 300.)
    );
    assert!(fixed.jump_to_item(10, ITEM_EXTENT));
    assert_eq!(
        (fixed.selected_item(ITEM_EXTENT), controller.offset()),
        (10, 1000.)
    );

    runtime
        .tree_mut()
        .update(
            root,
            Align::new(Alignment::CENTER, SizedBox::expand()).into(),
        )
        .expect("remove wheel");
    frame(&mut runtime, start + Duration::from_millis(16));
    assert!(controller.attachment_id().is_none());

    runtime
        .tree_mut()
        .update(root, mounted())
        .expect("remount wheel");
    frame(&mut runtime, start + Duration::from_millis(32));
    assert!(controller.attachment_id().is_some());
    assert_eq!(
        (fixed.selected_item(ITEM_EXTENT), controller.offset()),
        (3, 300.),
        "a new wheel position starts at the controller's configured initial item"
    );
}

#[test]
fn a_wheel_drag_releases_to_the_nearest_item_with_fixed_extent_physics() {
    let fixed = FixedExtentScrollController::default();
    let viewport = ListWheelScrollView::new(
        fixed.controller(),
        ITEM_EXTENT,
        WheelChildDelegate::children(rows(30).collect()),
    );
    let mut runtime = Runtime::new(viewport.into()).unwrap();
    let start = Instant::now();
    frame(&mut runtime, start);
    let event = |phase, y, ms| {
        let up = phase == PointerPhase::Up;
        (
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if up { 0 } else { PRIMARY_POINTER_BUTTON },
                button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                    .then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(10., y),
            },
            start + Duration::from_millis(ms),
        )
    };
    for (phase, y, ms) in [
        (PointerPhase::Down, 150., 16),
        (PointerPhase::Move, 90., 32),
        (PointerPhase::Up, 90., 250),
    ] {
        let (event, time) = event(phase, y, ms);
        let _ = runtime.handle_input_at(event, time);
        frame(&mut runtime, time);
        if phase == PointerPhase::Move {
            assert_eq!(fixed.controller().offset(), 60.);
            assert_eq!(fixed.selected_item(ITEM_EXTENT), 1);
        }
    }
    for i in 1..=180 {
        if !runtime.frame_requested() {
            break;
        }
        frame(&mut runtime, start + Duration::from_millis(250 + i * 16));
    }
    assert!((fixed.controller().offset() - 100.).abs() <= 1.);
    assert_eq!(fixed.selected_item(ITEM_EXTENT), 1);
    assert!(!runtime.frame_requested());
}

#[test]
fn a_fixed_extent_position_learns_its_item_size_fresh_on_each_attachment() {
    for fixed in [
        FixedExtentScrollController::new(3),
        FixedExtentScrollController::with_item_extent(3, 100.),
    ] {
        let controller = fixed.controller();
        let root = |item_extent: Option<f32>| {
            let mut content = SizedBox::expand();
            if let Some(item_extent) = item_extent {
                content = content.child(ListWheelScrollView::new(
                    fixed.controller(),
                    item_extent,
                    WheelChildDelegate::children(rows(100).collect()),
                ));
            }
            Widget::from(Align::new(Alignment::CENTER, content))
        };
        let mut runtime = Runtime::new(root(Some(80.))).unwrap();
        let now = Instant::now();
        frame(&mut runtime, now);
        assert_eq!((fixed.selected_item(80.), controller.offset()), (3, 240.));
        let first = controller.attachment_id();
        assert!(fixed.jump_to_item(10, 80.));
        frame(&mut runtime, now + Duration::from_millis(16));
        assert_eq!((fixed.selected_item(80.), controller.offset()), (10, 800.));
        assert_eq!(controller.attachment_id(), first);

        let id = runtime.tree().root().unwrap();
        runtime.schedule_update(id, root(None)).unwrap();
        frame(&mut runtime, now + Duration::from_millis(32));
        assert!(controller.attachment_id().is_none());
        runtime.schedule_update(id, root(Some(120.))).unwrap();
        frame(&mut runtime, now + Duration::from_millis(48));
        assert_eq!((fixed.selected_item(120.), controller.offset()), (3, 360.));
        assert!(controller.attachment_id().is_some());
        assert_ne!(controller.attachment_id(), first);
    }
}

#[test]
fn an_explicit_deferred_jump_wins_over_the_fixed_extent_initial_item_once() {
    let fixed = FixedExtentScrollController::new(3);
    let controller = fixed.controller();
    assert!(controller.deferred_jump_to(700.));
    let mut runtime = Runtime::new(wheel(&fixed, rows(100), None)).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert_eq!(
        (fixed.selected_item(ITEM_EXTENT), controller.offset()),
        (7, 700.)
    );
    frame(&mut runtime, now + Duration::from_millis(16));
    assert_eq!(
        (fixed.selected_item(ITEM_EXTENT), controller.offset()),
        (7, 700.)
    );
}

#[test]
fn a_hot_swapped_controller_keeps_the_existing_wheel_position() {
    let internal = FixedExtentScrollController::default();
    let first = FixedExtentScrollController::new(30);
    let second = FixedExtentScrollController::new(33);
    let mut runtime = Runtime::new(wheel(&internal, rows(100), None)).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    let root = runtime.tree().root().unwrap();
    for (phase, y, ms) in [
        (PointerPhase::Down, 300., 16),
        (PointerPhase::Move, -200., 32),
        (PointerPhase::Up, -200., 250),
    ] {
        let at = now + Duration::from_millis(ms);
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                position: Offset::new(400., y),
                phase,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                    .then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
            },
            at,
        );
        frame(&mut runtime, at);
    }
    assert_eq!(internal.selected_item(ITEM_EXTENT), 5);

    runtime
        .schedule_update(root, wheel(&first, rows(100), None))
        .unwrap();
    frame(&mut runtime, now + Duration::from_millis(266));
    assert_eq!(first.selected_item(ITEM_EXTENT), 5);
    assert!(internal.controller().attachment_id().is_none());
    assert!(first.jump_to_item(50, ITEM_EXTENT));
    assert_eq!(
        (
            first.selected_item(ITEM_EXTENT),
            first.controller().offset()
        ),
        (50, 5000.)
    );

    runtime
        .schedule_update(root, wheel(&second, rows(100), None))
        .unwrap();
    frame(&mut runtime, now + Duration::from_millis(282));
    assert!(first.controller().attachment_id().is_none());
    assert_eq!(second.selected_item(ITEM_EXTENT), 50);
    assert!(second.jump_to_item(40, ITEM_EXTENT));
    assert_eq!(
        (
            second.selected_item(ITEM_EXTENT),
            second.controller().offset()
        ),
        (40, 4000.)
    );

    let returned_internal = FixedExtentScrollController::default();
    runtime
        .schedule_update(root, wheel(&returned_internal, rows(100), None))
        .unwrap();
    frame(&mut runtime, now + Duration::from_millis(298));
    assert!(first.controller().attachment_id().is_none());
    assert!(second.controller().attachment_id().is_none());
    assert_eq!(returned_internal.selected_item(ITEM_EXTENT), 40);
}
