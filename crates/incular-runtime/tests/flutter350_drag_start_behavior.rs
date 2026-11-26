use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, SizedBox, Widget};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Start(Instant, Offset),
    Update(Instant, Offset),
}

fn run(behavior: DragStartBehavior, competing: bool) {
    use DragStartBehavior::{Down as FromDown, Start as FromAward};
    use Event::{Start, Update};
    use PointerPhase::{Down, Move, Up};
    let trace = Rc::new(RefCell::new(Vec::new()));
    let horizontal = GestureDetector::new(SizedBox::new().width(100.).height(100.))
        .behavior(HitTestBehavior::Opaque)
        .drag_start_behavior(behavior)
        .on_horizontal_drag_start({
            let trace = trace.clone();
            move |data| {
                trace
                    .borrow_mut()
                    .push(Start(data.source_time.unwrap(), data.global_position))
            }
        })
        .on_horizontal_drag_update({
            let trace = trace.clone();
            move |data| {
                trace
                    .borrow_mut()
                    .push(Update(data.source_time.unwrap(), data.delta))
            }
        });
    let widget: Widget = if competing {
        GestureDetector::new(horizontal)
            .behavior(HitTestBehavior::Opaque)
            .on_vertical_drag_start(|_| panic!("vertical drag must lose"))
            .into()
    } else {
        horizontal.into()
    };
    let mut runtime = Runtime::new(widget).unwrap();
    let constraints = Constraints::tight(Size::new(100., 100.));
    let origin = Instant::now();
    runtime.run_frame_at(constraints, origin).unwrap();
    let at = |ms| origin + Duration::from_millis(ms);
    let mut send = |phase, position, ms| {
        runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 5,
                device: 1,
                kind: PointerDeviceKind::Touch,
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                buttons: if phase == Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                position,
                sample: PointerSampleMetadata::EMPTY,
            },
            at(ms),
        );
        runtime.run_frame_at(constraints, at(ms)).unwrap();
    };
    let first = Offset::new(10., 10.);
    assert!(trace.borrow().is_empty());
    send(Down, first, 100);
    if !competing {
        assert_eq!(trace.borrow().as_slice(), [Start(at(100), first)]);
        let moved = Offset::new(20., 25.);
        send(Move, moved, 200);
        send(Move, moved, 300);
        assert_eq!(
            trace.borrow().as_slice(),
            [
                Start(at(100), first),
                Update(at(200), Offset::new(10., 0.)),
                Update(at(300), Offset::ZERO),
            ]
        );
        send(Up, moved, 310);
    } else {
        assert!(trace.borrow().is_empty());
        let moved = Offset::new(30., 10.);
        send(Move, moved, 200);
        if behavior == FromAward {
            assert_eq!(trace.borrow().as_slice(), [Start(at(200), moved)]);
        } else {
            assert_eq!(behavior, FromDown);
            assert_eq!(
                trace.borrow().as_slice(),
                [Start(at(200), first), Update(at(200), Offset::new(20., 0.))]
            );
        }
        send(Up, moved, 210);
    }
}

#[test]
fn drag_start_policy_reports_the_origins_deltas_and_source_timestamps_of_all_four_cases() {
    for behavior in [DragStartBehavior::Down, DragStartBehavior::Start] {
        for competing in [false, true] {
            run(behavior, competing);
        }
    }
}
