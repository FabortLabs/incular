//! A neutral fixed-extent wheel must drive its attached controller during a
//! finger drag, before any release physics or snapping can affect the result.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_scroll::{ScrollController, ScrollPhysics};
use incular_widgets::{
    ChangeReportingBehavior, ColoredBox, FixedExtentScrollController, ListWheelScrollView,
    SizedBox, WheelChildDelegate, Widget,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct Wheel {
    runtime: Runtime,
    controller: ScrollController,
    selected: Rc<RefCell<Vec<usize>>>,
    start: Instant,
}

impl Wheel {
    fn mount() -> Self {
        let controller = FixedExtentScrollController::default().controller();
        let children = (0..40)
            .map(|index| {
                let color = if index % 2 == 0 {
                    Color::WHITE
                } else {
                    Color::BLACK
                };
                Widget::from(
                    SizedBox::new()
                        .width(300.)
                        .child(ColoredBox::new(color, SizedBox::shrink())),
                )
            })
            .collect::<Vec<_>>();
        let mut view = ListWheelScrollView::new(
            controller.clone(),
            50.,
            WheelChildDelegate::children(children),
        );
        // Explicitly match the reference; the two default policies differ
        // in whether release snapping is enabled.
        view.viewport_mut().set_physics(ScrollPhysics::clamping());
        let selected = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&selected);
        view.viewport_mut().set_selection_callback(
            ChangeReportingBehavior::OnScrollUpdate,
            Some(move |index| observed.borrow_mut().push(index)),
        );

        let mut wheel = Self {
            runtime: Runtime::new(view.into()).expect("mount list wheel"),
            controller,
            selected,
            start: Instant::now(),
        };
        wheel.frame(0);
        assert!(wheel.controller.attachment_id().is_some());
        assert_eq!(wheel.controller.viewport_extent(), 600.);
        assert_eq!(wheel.controller.max_offset(), 1950.);
        assert_eq!(wheel.controller.offset(), 0.);
        assert!(wheel.selected.borrow().is_empty());
        wheel
    }

    fn frame(&mut self, millis: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(800., 600.)),
                self.start + Duration::from_millis(millis),
            )
            .expect("lay out attached wheel");
    }

    fn touch(&mut self, phase: PointerPhase, y: f32, millis: u64) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: PRIMARY_POINTER_BUTTON,
                button: (phase == PointerPhase::Down).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(400., y),
            },
            self.start + Duration::from_millis(millis),
        );
    }
}

#[test]
fn list_wheel_touch_drag_updates_offset_and_selection_before_release() {
    let mut wheel = Wheel::mount();
    wheel.frame(16);
    wheel.touch(PointerPhase::Down, 450., 16);
    wheel.frame(32);
    assert_eq!(wheel.controller.offset(), 0.);
    wheel.touch(PointerPhase::Move, 450. - 200. / 3., 32);
    wheel.frame(48);

    let offset = wheel.controller.offset();
    let selected = wheel.selected.borrow();
    assert!(
        (offset - 200. / 3.).abs() < 0.001 && selected.as_slice() == [1],
        "the active finger drag should move ~66.67px and select item 1; got offset={offset}, selection callbacks={selected:?}"
    );
}

#[test]
fn list_wheel_programmatic_jump_updates_attached_offset_and_selection() {
    let mut wheel = Wheel::mount();
    assert!(wheel.controller.jump_to(100.));
    wheel.frame(16);
    assert_eq!(wheel.controller.offset(), 100.);
    assert_eq!(*wheel.selected.borrow(), [2]);
}
