//! Flutter-protocol recognizers (`multitap.dart`, `tap_and_drag.dart`)
//! inside a `RawGestureDetector` through `ArenaGestureRecognizer`.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, time::Instant};
use incular_gestures::{
    ArenaGestureRecognizer, GestureRecognizerFactoryWithHandlers, MultiTapGestureRecognizer,
    PointerDeviceKind, RawPointerEvent, TapAndDragGestureRecognizer,
};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{RawGestureDetector, Widget};

type Log = Rc<RefCell<Vec<String>>>;

fn mount(detector: RawGestureDetector) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(detector.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    tree
}

fn send(tree: &mut WidgetTree, base: Instant, ms: u64, phase: PointerPhase, x: f32, y: f32) {
    let buttons = match phase {
        PointerPhase::Down | PointerPhase::Move => 1,
        _ => 0,
    };
    tree.dispatch_raw_pointer(RawPointerEvent::new(
        1,
        0,
        PointerDeviceKind::Touch,
        buttons,
        Offset::new(x, y),
        phase,
        base + Duration::from_millis(ms),
    ));
}

fn child() -> Widget {
    Widget::box_(Size::new(200., 200.), Color::WHITE)
}

fn multi_tap(log: &Log) -> MultiTapGestureRecognizer {
    let down = log.clone();
    let up = log.clone();
    let tap = log.clone();
    let cancel = log.clone();
    MultiTapGestureRecognizer::new()
        .on_tap_down(move |pointer, details| {
            down.borrow_mut().push(format!(
                "down {pointer} ({}, {})",
                details.global_position.x, details.global_position.y
            ));
        })
        .on_tap_up(move |pointer, _| up.borrow_mut().push(format!("up {pointer}")))
        .on_tap(move |pointer| tap.borrow_mut().push(format!("tap {pointer}")))
        .on_tap_cancel(move |pointer| cancel.borrow_mut().push(format!("cancel {pointer}")))
}

fn multi_tap_detector(log: &Log) -> RawGestureDetector {
    let log = log.clone();
    RawGestureDetector::new(child()).gesture(GestureRecognizerFactoryWithHandlers::new(
        move || ArenaGestureRecognizer::new(multi_tap(&log)),
        |_| {},
    ))
}

#[test]
fn a_multi_tap_wins_the_sweep_on_release() {
    // Alone and unresolved at the release, the tap wins Flutter's sweep:
    // acceptGesture with the final position reports the tap.
    let log = Log::default();
    let mut tree = mount(multi_tap_detector(&log));
    let base = Instant::now();
    send(&mut tree, base, 0, PointerPhase::Down, 10., 10.);
    send(&mut tree, base, 50, PointerPhase::Up, 12., 10.);
    assert_eq!(
        log.borrow().as_slice(),
        ["down 1 (10, 10)", "up 1", "tap 1"]
    );
}

#[test]
fn a_multi_tap_moved_past_the_slop_rejects_itself() {
    // 19 px is past kTouchSlop (18): _TapGesture.cancel resolves rejected
    // before winning, and rejectGesture reports the cancel.
    let log = Log::default();
    let mut tree = mount(multi_tap_detector(&log));
    let base = Instant::now();
    send(&mut tree, base, 0, PointerPhase::Down, 10., 10.);
    send(&mut tree, base, 20, PointerPhase::Move, 29., 10.);
    send(&mut tree, base, 40, PointerPhase::Up, 29., 10.);
    assert_eq!(log.borrow().as_slice(), ["down 1 (10, 10)", "cancel 1"]);
}

#[test]
fn an_accepted_tap_and_pan_keeps_receiving_its_pointer() {
    // Past kPanSlop (36) the recognizer resolves accepted; the detector's
    // arena accepts it, and the later moves and the release reach it.
    let log = Log::default();
    let factory_log = log.clone();
    let mut tree = mount(RawGestureDetector::new(child()).gesture(
        GestureRecognizerFactoryWithHandlers::new(
            move || {
                let start = factory_log.clone();
                let update = factory_log.clone();
                let end = factory_log.clone();
                ArenaGestureRecognizer::new(
                    TapAndDragGestureRecognizer::pan()
                        .on_drag_start(move |details| {
                            start.borrow_mut().push(format!(
                                "start ({}, {})",
                                details.global_position.x, details.global_position.y
                            ));
                        })
                        .on_drag_update(move |details| {
                            update.borrow_mut().push(format!(
                                "update ({}, {})",
                                details.global_position.x, details.global_position.y
                            ));
                        })
                        .on_drag_end(move |_| end.borrow_mut().push("end".to_owned())),
                )
            },
            |_| {},
        ),
    ));
    let base = Instant::now();
    send(&mut tree, base, 0, PointerPhase::Down, 0., 0.);
    send(&mut tree, base, 10, PointerPhase::Move, 20., 0.);
    send(&mut tree, base, 20, PointerPhase::Move, 40., 0.);
    send(&mut tree, base, 30, PointerPhase::Move, 50., 5.);
    send(&mut tree, base, 40, PointerPhase::Up, 50., 5.);
    assert_eq!(
        log.borrow().as_slice(),
        ["start (20, 0)", "update (40, 0)", "update (50, 5)", "end"]
    );
}
