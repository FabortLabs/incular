//! Exercises the actual mounted raw arena, not a manually preselected winner.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, DragStartBehavior, Offset, PointerPhase, Size, time::Instant};
use incular_gestures::{
    ArenaGestureRecognizer, GestureCallbacks, GestureRecognizerFactoryWithHandlers,
    PointerDeviceKind, PointerGestureRecognizer, RawPointerEvent, TapAndDragGestureRecognizer,
};
use incular_widgets::{RawGestureDetector, Widget, internal::WidgetTree};

type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Clone, Copy)]
enum Other {
    Tap,
    Pan,
    Vertical,
}

fn competitor(trace: &Trace, other: Other) -> PointerGestureRecognizer {
    let start = trace.clone();
    let update = trace.clone();
    let end = trace.clone();
    let cancel = trace.clone();
    let mut callbacks = GestureCallbacks::default();
    match other {
        Other::Tap => {
            callbacks.on_tap_down =
                Some(Rc::new(move |_| start.borrow_mut().push("tapdown".into())));
            callbacks.on_tap_up = Some(Rc::new(move |_| end.borrow_mut().push("tapup".into())));
            callbacks.on_tap_cancel = Some(Rc::new(move || {
                cancel.borrow_mut().push("tapscancel".into())
            }));
        }
        Other::Pan => {
            callbacks.on_pan_start =
                Some(Rc::new(move |_| start.borrow_mut().push("panstart".into())));
            callbacks.on_pan_update = Some(Rc::new(move |_| {
                update.borrow_mut().push("panupdate".into())
            }));
            callbacks.on_pan_end = Some(Rc::new(move |_| end.borrow_mut().push("panend".into())));
            callbacks.on_pan_cancel = Some(Rc::new(move || {
                cancel.borrow_mut().push("pancancel".into())
            }));
        }
        Other::Vertical => {
            callbacks.on_vertical_drag_start = Some(Rc::new(move |_| {
                start.borrow_mut().push("verticalstart".into())
            }));
            callbacks.on_vertical_drag_update = Some(Rc::new(move |_| {
                update.borrow_mut().push("verticalupdate".into())
            }));
            callbacks.on_vertical_drag_end = Some(Rc::new(move |_| {
                end.borrow_mut().push("verticalend".into())
            }));
            callbacks.on_vertical_drag_cancel = Some(Rc::new(move || {
                cancel.borrow_mut().push("verticalcancel".into())
            }));
        }
    }
    PointerGestureRecognizer::new(callbacks)
}

fn raced(
    horizontal: bool,
    eager: bool,
    other: Other,
    pointer: u64,
    device: PointerDeviceKind,
    moves: &[(f32, f32)],
) -> Vec<String> {
    let trace = Trace::default();
    let combined = trace.clone();
    let other_trace = trace.clone();
    let widget = RawGestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
        .gesture(GestureRecognizerFactoryWithHandlers::new(
            move || {
                let recognizer = if horizontal {
                    TapAndDragGestureRecognizer::horizontal_drag()
                } else {
                    TapAndDragGestureRecognizer::pan()
                };
                let record = |label: &'static str| {
                    let trace = combined.clone();
                    move |count| trace.borrow_mut().push(format!("{label}#{count}"))
                };
                let down = record("down");
                let up = record("up");
                let start = record(if horizontal {
                    "horizontaldragstart"
                } else {
                    "panstart"
                });
                let update = record(if horizontal {
                    "horizontaldragupdate"
                } else {
                    "panupdate"
                });
                let end = record(if horizontal {
                    "horizontaldragend"
                } else {
                    "panend"
                });
                let cancel = combined.clone();
                ArenaGestureRecognizer::new(
                    recognizer
                        .drag_start_behavior(DragStartBehavior::Down)
                        .eager_victory_on_drag(eager)
                        .max_consecutive_tap(3)
                        .on_tap_down(move |details| down(details.consecutive_tap_count))
                        .on_tap_up(move |details| up(details.consecutive_tap_count))
                        .on_drag_start(move |details| start(details.consecutive_tap_count))
                        .on_drag_update(move |details| update(details.consecutive_tap_count))
                        .on_drag_end(move |details| end(details.consecutive_tap_count))
                        .on_cancel(move || cancel.borrow_mut().push("cancel".into())),
                )
            },
            |_| {},
        ))
        .gesture(GestureRecognizerFactoryWithHandlers::new(
            move || competitor(&other_trace, other),
            |_| {},
        ));
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    let base = Instant::now();
    let mut send = |ms, phase, (x, y)| {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            pointer,
            0,
            device,
            u32::from(phase != PointerPhase::Up),
            Offset::new(x, y),
            phase,
            base + Duration::from_millis(ms),
        ));
    };
    send(0, PointerPhase::Down, (10., 10.));
    for (index, &position) in moves.iter().enumerate() {
        send(10 + index as u64, PointerPhase::Move, position);
    }
    send(
        40,
        PointerPhase::Up,
        moves.last().copied().unwrap_or((11., 9.)),
    );
    trace.take()
}

#[test]
fn mouse_pan_beats_a_tap_after_five_pixels_on_both_axes() {
    assert_eq!(
        raced(
            false,
            true,
            Other::Tap,
            6,
            PointerDeviceKind::Mouse,
            &[(15., 15.)]
        ),
        ["down#1", "panstart#1", "panupdate#1", "panend#1"]
    );
}

#[test]
fn an_eager_combined_pan_rejects_the_other_pan_before_delivering_start() {
    assert_eq!(
        raced(
            false,
            true,
            Other::Pan,
            5,
            PointerDeviceKind::Touch,
            &[(40., 45.)]
        ),
        [
            "pancancel",
            "down#1",
            "panstart#1",
            "panupdate#1",
            "panend#1"
        ]
    );
}

#[test]
fn vertical_motion_beats_the_horizontal_combined_recognizer() {
    assert_eq!(
        raced(
            true,
            true,
            Other::Vertical,
            5,
            PointerDeviceKind::Touch,
            &[(10., 45.), (10., 100.)]
        ),
        ["verticalstart", "verticalupdate", "verticalend"]
    );
}

#[test]
fn the_vertical_contender_claims_its_smaller_threshold_before_combined_pan() {
    assert_eq!(
        raced(
            false,
            true,
            Other::Vertical,
            5,
            PointerDeviceKind::Touch,
            &[(10., 45.), (10., 100.)]
        ),
        ["verticalstart", "verticalupdate", "verticalend"]
    );
}

#[test]
fn a_combined_pan_without_eager_victory_loses_to_a_pure_pan() {
    assert_eq!(
        raced(
            false,
            false,
            Other::Pan,
            5,
            PointerDeviceKind::Touch,
            &[(40., 45.)]
        ),
        ["panstart", "panend"]
    );
}

#[test]
fn erased_factories_own_distinct_concrete_recognizer_keys() {
    use incular_gestures::ErasedGestureRecognizerFactory;
    use std::any::TypeId;

    let tap: ErasedGestureRecognizerFactory = GestureRecognizerFactoryWithHandlers::new(
        || PointerGestureRecognizer::new(GestureCallbacks::default()),
        |_| {},
    )
    .into();
    let combined: ErasedGestureRecognizerFactory = GestureRecognizerFactoryWithHandlers::new(
        || ArenaGestureRecognizer::new(TapAndDragGestureRecognizer::pan()),
        |_| {},
    )
    .into();
    assert_eq!(tap.type_id(), TypeId::of::<PointerGestureRecognizer>());
    assert_eq!(
        combined.type_id(),
        TypeId::of::<ArenaGestureRecognizer<TapAndDragGestureRecognizer>>()
    );
    assert_ne!(tap.type_id(), combined.type_id());
}

#[test]
fn horizontal_drag_cancels_vertical_then_delivers_its_own_sequence() {
    assert_eq!(
        raced(
            true,
            true,
            Other::Vertical,
            5,
            PointerDeviceKind::Touch,
            &[(45., 10.)]
        ),
        [
            "verticalcancel",
            "down#1",
            "horizontaldragstart#1",
            "horizontaldragupdate#1",
            "horizontaldragend#1"
        ]
    );
}

#[test]
fn diagonal_motion_over_pan_but_under_vertical_slop_is_a_combined_pan() {
    assert_eq!(
        raced(
            false,
            true,
            Other::Vertical,
            5,
            PointerDeviceKind::Touch,
            &[(45., 25.)]
        ),
        [
            "verticalcancel",
            "down#1",
            "panstart#1",
            "panupdate#1",
            "panend#1"
        ]
    );
}

#[test]
fn a_nearly_stationary_tap_sweeps_to_the_first_registered_combined_member() {
    assert_eq!(
        raced(false, true, Other::Tap, 1, PointerDeviceKind::Touch, &[]),
        ["down#1", "up#1"]
    );
}

#[test]
fn motion_between_tap_and_pan_slop_cancels_the_pure_pan_without_starting_a_drag() {
    assert_eq!(
        raced(
            false,
            true,
            Other::Pan,
            5,
            PointerDeviceKind::Touch,
            &[(25., 25.)]
        ),
        ["pancancel"]
    );
}

#[test]
fn replacing_a_factory_of_the_same_concrete_type_mounts_only_the_replacement() {
    use incular_gestures::ErasedGestureRecognizerFactory;

    for bulk in [false, true] {
        let trace = Trace::default();
        let factory = |tag: &'static str| -> ErasedGestureRecognizerFactory {
            let trace = trace.clone();
            GestureRecognizerFactoryWithHandlers::new(
                move || {
                    let trace = trace.clone();
                    PointerGestureRecognizer::new(GestureCallbacks {
                        on_tap: Some(Rc::new(move || trace.borrow_mut().push(tag.to_owned()))),
                        ..Default::default()
                    })
                },
                |_| {},
            )
            .into()
        };
        let detector = RawGestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE));
        let detector = if bulk {
            detector.gestures([factory("old"), factory("replacement")])
        } else {
            detector
                .gesture(factory("old"))
                .gesture(factory("replacement"))
        };
        let mut tree = WidgetTree::new();
        tree.mount(detector.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(200., 200.)))
            .unwrap();
        let base = Instant::now();
        for (ms, phase, buttons) in [(0, PointerPhase::Down, 1), (20, PointerPhase::Up, 0)] {
            tree.dispatch_raw_pointer(RawPointerEvent::new(
                1,
                0,
                PointerDeviceKind::Touch,
                buttons,
                Offset::new(10., 10.),
                phase,
                base + Duration::from_millis(ms),
            ));
        }
        assert_eq!(&*trace.borrow(), &["replacement"], "bulk={bulk}");
    }
}
