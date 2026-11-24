//! Neutral drag and dismissal callbacks from Flutter 3.50.0.
use incular_config::Constraints;
use incular_core::{
    Color, DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind,
    PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Directionality, DismissDirection, Dismissible, TextDirection, Widget};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Callback {
    Resize,
    Dismiss(DismissDirection),
}

struct Probe {
    runtime: Runtime,
    callbacks: Rc<RefCell<Vec<Callback>>>,
    start: Instant,
    next_ms: u64,
}

impl Probe {
    fn new(
        bidi: TextDirection,
        direction: DismissDirection,
        configure: impl FnOnce(Dismissible) -> Dismissible,
    ) -> Self {
        let callbacks = Rc::new(RefCell::new(Vec::new()));
        let dismiss_sink = callbacks.clone();
        let resize_sink = callbacks.clone();
        let child = Widget::box_(Size::new(100., 100.), Color::WHITE);
        let control = Dismissible::new(direction, child, move |d| {
            dismiss_sink.borrow_mut().push(Callback::Dismiss(d));
        })
        .drag_start_behavior(DragStartBehavior::Down)
        .on_resize(move || resize_sink.borrow_mut().push(Callback::Resize));
        let runtime = Runtime::new(Directionality::new(bidi, configure(control)).into()).unwrap();
        let mut probe = Self {
            runtime,
            callbacks,
            start: Instant::now(),
            next_ms: 100,
        };
        probe.frame(0);
        probe
    }

    fn frame(&mut self, ms: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(100., 100.)),
                self.start + Duration::from_millis(ms),
            )
            .unwrap();
    }

    fn drag_across(&mut self, right: bool) -> Vec<Callback> {
        let (from, to) = if right {
            (0.125, 100.125)
        } else {
            (99.875, -0.125)
        };
        let started = self.next_ms;
        for (phase, x, delay) in [
            (PointerPhase::Down, from, 0),
            (PointerPhase::Move, to, 50),
            (PointerPhase::Up, to, 100),
        ] {
            let now = self.start + Duration::from_millis(started + delay);
            self.runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: started,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Up {
                        0
                    } else {
                        PRIMARY_POINTER_BUTTON
                    },
                    button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                        .then_some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(x, 1.),
                },
                now,
            );
        }
        self.frame(started + 100);
        let first_frame = self.callbacks.borrow().clone();
        for delay in [101, 116, 200, 301, 416, 500, 650, 900] {
            self.frame(started + delay);
        }
        self.next_ms += 1200;
        first_frame
    }

    fn dismissals(&self) -> Vec<DismissDirection> {
        self.callbacks
            .borrow()
            .iter()
            .filter_map(|callback| match callback {
                Callback::Dismiss(direction) => Some(*direction),
                Callback::Resize => None,
            })
            .collect()
    }
}

fn check_end_to_start(bidi: TextDirection, right: bool) {
    let mut probe = Probe::new(bidi, DismissDirection::EndToStart, |value| value);
    probe.drag_across(!right);
    assert!(probe.dismissals().is_empty(), "disallowed gesture");
    probe.drag_across(right);
    assert_eq!(probe.dismissals(), [DismissDirection::EndToStart]);
}

#[test]
fn physical_left_is_end_to_start_in_ltr() {
    check_end_to_start(TextDirection::Ltr, false);
}

#[test]
fn physical_right_is_end_to_start_in_rtl() {
    check_end_to_start(TextDirection::Rtl, true);
}

#[test]
fn threshold_one_disables_only_the_specified_direction() {
    let mut probe = Probe::new(TextDirection::Ltr, DismissDirection::Horizontal, |value| {
        value.dismiss_threshold(DismissDirection::StartToEnd, 1.)
    });
    probe.drag_across(true);
    assert!(probe.dismissals().is_empty());
    probe.drag_across(false);
    assert_eq!(probe.dismissals(), [DismissDirection::EndToStart]);
}

#[test]
fn no_resize_duration_elides_resize_and_delivers_dismiss_once() {
    let mut probe = Probe::new(TextDirection::Ltr, DismissDirection::Horizontal, |value| {
        value.resize_duration(None)
    });
    let after_release_frame = probe.drag_across(true);
    assert_eq!(
        after_release_frame,
        [Callback::Dismiss(DismissDirection::StartToEnd)],
        "the first frame after a full-extent drag"
    );
    assert_eq!(
        probe.callbacks.borrow().as_slice(),
        [Callback::Dismiss(DismissDirection::StartToEnd)]
    );
}
