//! Flutter list-wheel selection notification timing and valid indices.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_scroll::ScrollPhysics;
use incular_widgets::{
    ChangeReportingBehavior, FixedExtentScrollController, ListWheelScrollView, SizedBox,
    WheelChildDelegate,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct Wheel {
    runtime: Runtime,
    selection: Rc<RefCell<Vec<usize>>>,
    start: Instant,
}

impl Wheel {
    fn mount(behavior: ChangeReportingBehavior) -> Self {
        Self::mount_with_physics(behavior, ScrollPhysics::clamping())
    }

    fn mount_with_physics(behavior: ChangeReportingBehavior, physics: ScrollPhysics) -> Self {
        let fixed = FixedExtentScrollController::default();
        let mut wheel = ListWheelScrollView::new(
            fixed.controller(),
            100.,
            WheelChildDelegate::children((0..10).map(|_| SizedBox::expand().into()).collect()),
        );
        wheel.viewport_mut().set_physics(physics);
        let selection = Rc::new(RefCell::new(Vec::new()));
        let log = selection.clone();
        wheel
            .viewport_mut()
            .set_selection_callback(behavior, Some(move |item| log.borrow_mut().push(item)));
        let start = Instant::now();
        let mut runtime = Runtime::new(wheel.into()).unwrap();
        runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), start)
            .unwrap();
        assert!(selection.borrow().is_empty());
        Self {
            runtime,
            selection,
            start,
        }
    }

    fn at(&mut self, phase: PointerPhase, y: f32, ms: u64) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                position: Offset::new(10., y),
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
            self.start + Duration::from_millis(ms),
        );
    }
}

#[test]
fn scroll_update_reports_new_nearest_items_before_layout() {
    let mut wheel = Wheel::mount(ChangeReportingBehavior::OnScrollUpdate);
    wheel.at(PointerPhase::Down, 10., 16);
    for (i, (y, expected)) in [
        (-39., &[][..]),
        (-40., &[1]),
        (-139., &[1]),
        (-140., &[1, 2]),
        (-90., &[1, 2, 1]),
    ]
    .into_iter()
    .enumerate()
    {
        wheel.at(PointerPhase::Move, y, 32 + i as u64 * 16);
        assert_eq!(wheel.selection.borrow().as_slice(), expected);
    }
}

#[test]
fn scroll_end_reports_once_per_completed_position() {
    let mut wheel = Wheel::mount(ChangeReportingBehavior::OnScrollEnd);
    wheel.at(PointerPhase::Down, 10., 16);
    for (i, y) in [-39., -40., -139., -140.].into_iter().enumerate() {
        wheel.at(PointerPhase::Move, y, 32 + i as u64 * 16);
        assert!(wheel.selection.borrow().is_empty());
    }
    wheel.at(PointerPhase::Up, -140., 240);
    assert_eq!(*wheel.selection.borrow(), [2]);
    wheel.at(PointerPhase::Down, 10., 256);
    wheel.at(PointerPhase::Move, 110., 272);
    assert_eq!(*wheel.selection.borrow(), [2]);
    wheel.at(PointerPhase::Up, 110., 480);
    assert_eq!(*wheel.selection.borrow(), [2, 1]);
}

#[test]
fn a_wheel_never_reports_an_item_past_its_delegate_range() {
    let mut wheel = Wheel::mount(ChangeReportingBehavior::OnScrollUpdate);
    wheel.at(PointerPhase::Down, 10., 16);
    wheel.at(PointerPhase::Move, 80., 32);
    assert!(wheel.selection.borrow().is_empty());
    for step in 0..200 {
        wheel.at(PointerPhase::Move, -(step as f32 * 10.), 48 + step * 16);
    }
    assert_eq!(*wheel.selection.borrow(), (1..=9).collect::<Vec<_>>());
}

#[test]
fn an_end_callback_waits_for_the_fixed_extent_settle() {
    let mut wheel = Wheel::mount_with_physics(
        ChangeReportingBehavior::OnScrollEnd,
        ScrollPhysics::clamping().fixed_extent_snapping(100.),
    );
    wheel.at(PointerPhase::Down, 150., 16);
    wheel.at(PointerPhase::Move, 90., 32);
    wheel.at(PointerPhase::Up, 90., 250);
    assert!(wheel.selection.borrow().is_empty());
    for step in 0..=180 {
        if !wheel.runtime.frame_requested() {
            break;
        }
        wheel
            .runtime
            .run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                wheel.start + Duration::from_millis(250 + step * 16),
            )
            .unwrap();
    }
    assert_eq!(*wheel.selection.borrow(), [1]);
    assert!(!wheel.runtime.frame_requested());
}

#[test]
fn replacing_a_callback_continues_selection_and_unmount_drops_the_binding() {
    let fixed = FixedExtentScrollController::default();
    let old = Rc::new(RefCell::new(Vec::new()));
    let replacement = Rc::new(RefCell::new(Vec::new()));
    let scene = |selection: Option<&Rc<RefCell<Vec<usize>>>>| {
        let mut content = SizedBox::expand();
        if let Some(selection) = selection {
            let selected = selection.clone();
            let mut wheel = ListWheelScrollView::new(
                fixed.controller(),
                100.,
                WheelChildDelegate::children((0..10).map(|_| SizedBox::expand().into()).collect()),
            );
            wheel.viewport_mut().set_selection_callback(
                ChangeReportingBehavior::OnScrollUpdate,
                Some(move |index| selected.borrow_mut().push(index)),
            );
            content = content.child(wheel);
        }
        incular_widgets::Align::new(incular_widgets::Alignment::CENTER, content)
    };
    let mut runtime = Runtime::new(scene(Some(&old)).into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let controller = fixed.controller();
    assert!(controller.jump_to(70.));
    assert_eq!(*old.borrow(), [1]);

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(Some(&replacement)).into())
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(controller.jump_to(71.));
    assert!(replacement.borrow().is_empty());
    assert!(controller.jump_to(170.));
    assert_eq!(*replacement.borrow(), [2]);
    assert_eq!(*old.borrow(), [1]);

    runtime.schedule_update(root, scene(None).into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(controller.attachment_id().is_none());
    assert!(controller.jump_to(270.));
    assert_eq!(*replacement.borrow(), [2]);
}
