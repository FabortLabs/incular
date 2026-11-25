//! Flutter holds the ancestor's single-tap candidate while an inner double is possible.
use incular_config::{Alignment, Constraints};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{Align, GestureDetector, HitTestBehavior, Widget};
use std::{cell::RefCell, rc::Rc, time::Duration};

type Trace = Rc<RefCell<Vec<&'static str>>>;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(200., 200.))
}

fn mount() -> (Runtime, Trace, Instant) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |label| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(label)
    };
    let child = GestureDetector::new(Widget::box_(Size::new(100., 200.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_double_tap(record("child double"));
    let parent = GestureDetector::new(Align::new(Alignment::CENTER_LEFT, child))
        .behavior(HitTestBehavior::Opaque)
        .on_tap(record("parent single"));
    let now = Instant::now();
    let mut runtime = Runtime::new(parent.into()).unwrap();
    runtime.run_frame_at(bounds(), now).unwrap();
    (runtime, trace, now)
}

fn tap(runtime: &mut Runtime, at: Instant, pointer: u64, x: f32) {
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                button: Some(PRIMARY_POINTER_BUTTON),
                buttons,
                position: Offset::new(x, 100.),
                phase,
                sample: PointerSampleMetadata::EMPTY,
            },
            at,
        );
    }
}

#[test]
fn child_double_tap_excludes_its_parent_single_tap() {
    let (mut runtime, trace, now) = mount();
    tap(&mut runtime, now, 1, 90.);
    let first = trace.borrow().clone();
    tap(&mut runtime, now + Duration::from_millis(100), 2, 90.);
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(400))
        .unwrap();
    assert_eq!(
        (first, trace.borrow().clone()),
        (vec![], vec!["child double"])
    );
}

#[test]
fn a_parent_only_second_press_wins_without_a_late_first_callback() {
    let (mut runtime, trace, now) = mount();
    tap(&mut runtime, now, 1, 90.);
    let first = trace.borrow().clone();
    tap(&mut runtime, now + Duration::from_millis(100), 2, 120.);
    let second = trace.borrow().clone();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(400))
        .unwrap();
    assert_eq!(
        (first, second, trace.borrow().clone()),
        (vec![], vec!["parent single"], vec!["parent single"])
    );
}

#[test]
fn no_second_press_releases_the_parent_single_at_expiry() {
    let (mut runtime, trace, now) = mount();
    tap(&mut runtime, now, 1, 90.);
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(299))
        .unwrap();
    let first = trace.borrow().clone();
    runtime
        .run_frame_at(bounds(), now + Duration::from_millis(300))
        .unwrap();
    assert_eq!(
        (first, trace.borrow().clone()),
        (vec![], vec!["parent single"])
    );
}
