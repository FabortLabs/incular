use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GestureDetector, PointerDeviceKind, RawPointerEvent, Row, SizedBox, Widget,
    internal::HitTestBehavior,
};

fn touch(runtime: &mut Runtime, now: Instant, pointer: u64, phase: PointerPhase, position: Offset) {
    let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
        pointer,
        0,
        PointerDeviceKind::Touch,
        if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        position,
        phase,
        now,
    ));
}

#[test]
fn pending_contact_releases_after_the_same_vertical_recognizer_wins_another_arena() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let horizontal = GestureDetector::new(SizedBox::new().width(60.).height(60.))
        .behavior(HitTestBehavior::Opaque)
        .on_horizontal_drag_start({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("horizontal start")
        });
    let child = Row::new([
        Widget::from(SizedBox::new().width(15.).height(60.)),
        horizontal.into(),
    ]);
    let vertical = GestureDetector::new(Directionality::new(TextDirection::Ltr, child))
        .behavior(HitTestBehavior::Opaque)
        .on_vertical_drag_start({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("vertical start")
        })
        .on_vertical_drag_end({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("vertical end")
        });
    let mut runtime = Runtime::new(vertical.into()).unwrap();
    let limits = Constraints::tight(Size::new(75., 60.));
    let now = Instant::now();
    runtime.run_frame_at(limits, now).unwrap();
    let first = Offset::new(10., 10.);
    let second = Offset::new(20., 20.);
    touch(&mut runtime, now, 90, PointerPhase::Down, first);
    touch(&mut runtime, now, 91, PointerPhase::Down, second);
    runtime.run_frame_at(limits, now).unwrap();
    assert_eq!(*trace.borrow(), ["vertical start"]);
    touch(&mut runtime, now, 90, PointerPhase::Up, first);
    touch(&mut runtime, now, 91, PointerPhase::Up, second);
    runtime.run_frame_at(limits, now).unwrap();

    // A new stream on the same detector can win and finish independently.
    // This cannot be satisfied by wedging either of the first two contacts.
    let prior = trace.borrow().len();
    touch(&mut runtime, now, 92, PointerPhase::Down, first);
    touch(&mut runtime, now, 92, PointerPhase::Up, first);
    assert_eq!(&trace.borrow()[prior..], ["vertical start", "vertical end"]);
}
