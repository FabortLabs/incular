use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    DeviceGestureSettings, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, GestureDetector, GestureRecognizerFactoryWithHandlers,
    PointerDeviceKind, RawGestureDetector, RawPointerEvent, Row, ScrollController, SizedBox,
    Sliver, SliverToBoxAdapter, Widget,
    internal::{GestureCallbacks, HitTestBehavior, Key, PointerGestureRecognizer},
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

#[test]
fn custom_eager_raw_pan_beats_a_scrollable_on_the_same_move() {
    let key = Key::from("pan target");
    let limits = Constraints::tight(Size::new(200., 200.));
    let delta = Offset::new(30., 19.); // kTouchSlop + 1 vertically
    for eager in [true, false] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let callbacks = GestureCallbacks {
            on_pan_start: Some(Rc::new({
                let trace = trace.clone();
                move |_| trace.borrow_mut().push("pan start")
            })),
            on_pan_end: Some(Rc::new({
                let trace = trace.clone();
                move |_| trace.borrow_mut().push("pan end")
            })),
            ..GestureCallbacks::default()
        };
        let target = Widget::from(SizedBox::new().width(100.).height(100.)).with_key(key.clone());
        let detector = RawGestureDetector::new(target)
            .behavior(HitTestBehavior::Translucent)
            .gesture(GestureRecognizerFactoryWithHandlers::new(
                move || {
                    let mut pan = PointerGestureRecognizer::new(callbacks.clone());
                    if eager {
                        // The built-in pan threshold is 2 * touch slop.
                        // Configure just this raw child to claim at 18 px.
                        pan.set_gesture_settings(DeviceGestureSettings::new(Some(9.)));
                    }
                    pan
                },
                |_| {},
            ));
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(SliverToBoxAdapter::new(detector)),
            Box::new(SliverToBoxAdapter::new(SizedBox::new().height(500.))),
        ];
        let controller = ScrollController::new();
        let mut runtime = Runtime::new(Widget::from(Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers).controller(controller.clone()),
        )))
        .unwrap();
        let now = Instant::now();
        runtime.run_frame_at(limits, now).unwrap();
        assert!(controller.jump_to(30.));
        runtime.run_frame_at(limits, now).unwrap();
        let target = runtime.tree().element_with_key(&key).unwrap();
        let bounds = runtime.tree().element_bounds(target).unwrap();
        let start = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
        touch(&mut runtime, now, 1, PointerPhase::Down, start);
        assert!(trace.borrow().is_empty());
        touch(
            &mut runtime,
            now + Duration::from_millis(20),
            1,
            PointerPhase::Move,
            start + delta,
        );
        assert_eq!(
            trace.borrow().as_slice(),
            if eager { &["pan start"][..] } else { &[] }
        );
        let end = start + delta + Offset::new(0., 10.);
        touch(
            &mut runtime,
            now + Duration::from_millis(40),
            1,
            PointerPhase::Move,
            end,
        );
        let offset = controller.offset();
        touch(
            &mut runtime,
            now + Duration::from_millis(60),
            1,
            PointerPhase::Up,
            end,
        );
        if eager {
            assert_eq!(*trace.borrow(), ["pan start", "pan end"]);
            assert_eq!(offset, 30.);
        } else {
            assert!(trace.borrow().is_empty());
            assert_eq!(offset, 20., "default slop leaves the second move to scroll");
        }
    }
}
