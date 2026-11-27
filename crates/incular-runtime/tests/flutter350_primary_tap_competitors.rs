use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, Widget};

#[derive(Clone, Copy)]
enum ParentButton {
    Primary,
    Secondary,
    Tertiary,
}

fn compare(parent_button: ParentButton) {
    use PointerPhase::{Down, Up};

    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |label| {
        let trace = trace.clone();
        move |_| trace.borrow_mut().push(label)
    };
    let size = Size::new(200., 200.);
    let primary = GestureDetector::new(Widget::box_(size, Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap_down(record("inner.down"))
        .on_tap_up({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("inner.up")
        })
        .on_tap_cancel({
            let trace = trace.clone();
            move || trace.borrow_mut().push("inner.cancel")
        });
    let parent = GestureDetector::new(primary).behavior(HitTestBehavior::Opaque);
    let outer_cancel = {
        let trace = trace.clone();
        move || trace.borrow_mut().push("outer.cancel")
    };
    let parent = match parent_button {
        ParentButton::Primary => parent
            .on_tap_down(record("outer.down"))
            .on_tap_cancel(outer_cancel),
        ParentButton::Secondary => parent
            .on_secondary_tap_down(record("outer.down"))
            .on_secondary_tap_cancel(outer_cancel),
        ParentButton::Tertiary => parent
            .on_tertiary_tap_down(record("outer.down"))
            .on_tertiary_tap_cancel(outer_cancel),
    };
    let mut runtime = Runtime::new(parent.into()).unwrap();
    let base = Instant::now();
    let time = |ms| base + Duration::from_millis(ms);
    let bounds = Constraints::tight(size);
    runtime.run_frame_at(bounds, base).unwrap();
    let route = |runtime: &mut Runtime, phase, position, ms| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                position,
                sample: PointerSampleMetadata::EMPTY,
            },
            time(ms),
        );
    };
    route(&mut runtime, Down, Offset::new(10., 10.), 0);
    if matches!(parent_button, ParentButton::Primary) {
        assert!(trace.borrow().is_empty());
        runtime.run_frame_at(bounds, time(99)).unwrap();
        assert!(trace.borrow().is_empty());
        runtime.run_frame_at(bounds, time(100)).unwrap();
        assert_eq!(*trace.borrow(), ["inner.down", "outer.down"]);
        runtime.run_frame_at(bounds, time(500)).unwrap();
        assert_eq!(*trace.borrow(), ["inner.down", "outer.down"]);
    } else {
        assert_eq!(*trace.borrow(), ["inner.down"]);
        trace.borrow_mut().clear();
        route(&mut runtime, Up, Offset::new(11., 9.), 1);
        assert_eq!(*trace.borrow(), ["inner.up"]);
    }
}

#[test]
fn a_secondary_only_ancestor_does_not_delay_the_primary_tap_down() {
    compare(ParentButton::Secondary);
}

#[test]
fn a_tertiary_only_ancestor_does_not_delay_the_primary_tap_down() {
    compare(ParentButton::Tertiary);
}

#[test]
fn two_primary_contestants_report_down_at_the_deadline_in_hit_path_order() {
    compare(ParentButton::Primary);
}
