//! Exercises the actual mounted raw arena, not a manually preselected winner.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, DragStartBehavior, Offset, PointerPhase, Size, time::Instant};
use incular_gestures::{
    ArenaGestureRecognizer, GestureCallbacks, GestureRecognizerFactoryWithHandlers,
    LongPressGestureRecognizer, PointerDeviceKind, PointerGestureRecognizer, RawPointerEvent,
    TapAndDragGestureRecognizer,
};
use incular_widgets::{RawGestureDetector, Widget, internal::WidgetTree};

type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Clone, Copy)]
enum Other {
    Tap,
    Pan,
    Vertical,
    Horizontal,
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
        Other::Horizontal => {
            callbacks.on_horizontal_drag_start = Some(Rc::new(move |_| {
                start.borrow_mut().push("basichorizontalstart".into())
            }));
            callbacks.on_horizontal_drag_update = Some(Rc::new(move |_| {
                update.borrow_mut().push("basichorizontalupdate".into())
            }));
            callbacks.on_horizontal_drag_end = Some(Rc::new(move |_| {
                end.borrow_mut().push("basichorizontalend".into())
            }));
            callbacks.on_horizontal_drag_cancel = Some(Rc::new(move || {
                cancel.borrow_mut().push("basichorizontalcancel".into())
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

fn competition_detector(
    trace: &Trace,
    horizontal: bool,
    eager: bool,
    other: Option<Other>,
    combined_first: bool,
) -> RawGestureDetector {
    let combined = trace.clone();
    let other_trace = trace.clone();
    let combined_factory = GestureRecognizerFactoryWithHandlers::new(
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
    );
    let widget = RawGestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE));
    let Some(other) = other else {
        return widget.gesture(combined_factory);
    };
    let other_factory =
        GestureRecognizerFactoryWithHandlers::new(move || competitor(&other_trace, other), |_| {});
    if combined_first {
        widget.gesture(combined_factory).gesture(other_factory)
    } else {
        widget.gesture(other_factory).gesture(combined_factory)
    }
}

fn route(
    tree: &mut WidgetTree,
    base: Instant,
    pointer: u64,
    device: PointerDeviceKind,
    moves: &[(f32, f32)],
) {
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
    let widget = competition_detector(&trace, horizontal, eager, Some(other), true);
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    route(&mut tree, Instant::now(), pointer, device, moves);
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

#[test]
fn a_sole_horizontal_member_can_drag_diagonally_after_the_press_default_win() {
    // Pinned :487: across the pan distance but below horizontal drag slop.
    let trace = Trace::default();
    let widget = competition_detector(&trace, true, true, None, true);
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    route(
        &mut tree,
        Instant::now(),
        5,
        PointerDeviceKind::Touch,
        &[(25., 45.)],
    );
    assert_eq!(
        trace.take(),
        [
            "down#1",
            "horizontaldragstart#1",
            "horizontaldragupdate#1",
            "horizontaldragend#1"
        ]
    );
}

#[test]
fn tap_rejection_defaults_the_remaining_pan_and_the_next_tap_uses_its_new_order() {
    // Pinned :865; both recognizers are retained when their order reverses.
    let trace = Trace::default();
    let make = |first| competition_detector(&trace, false, true, Some(Other::Tap), first);
    let viewport = Constraints::tight(Size::new(200., 200.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(true).into()).unwrap();
    tree.layout(viewport).unwrap();
    let stamp = Instant::now();
    route(&mut tree, stamp, 5, PointerDeviceKind::Touch, &[(25., 25.)]);
    assert_eq!(trace.take(), ["down#1", "panstart#1", "panend#1"]);
    tree.update(root, make(false).into()).unwrap();
    tree.layout(viewport).unwrap();
    route(
        &mut tree,
        stamp + Duration::from_millis(1000),
        1,
        PointerDeviceKind::Touch,
        &[],
    );
    assert_eq!(trace.take(), ["tapdown", "tapup"]);
}

#[test]
fn the_raw_default_does_not_win_before_an_ordinary_vertical_drag_joins() {
    use incular_widgets::GestureDetector;

    let trace = Trace::default();
    let raw = competition_detector(&trace, true, true, None, true);
    let start = trace.clone();
    let end = trace.clone();
    let root = GestureDetector::new(raw)
        .on_vertical_drag_start(move |_| start.borrow_mut().push("ordinary start".into()))
        .on_vertical_drag_end(move |_| end.borrow_mut().push("ordinary end".into()));
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    let now = Instant::now();
    for (ms, phase, y) in [
        (0, PointerPhase::Down, 10.),
        (10, PointerPhase::Move, 45.),
        (20, PointerPhase::Move, 100.),
        (40, PointerPhase::Up, 100.),
    ] {
        let event = RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            u32::from(phase != PointerPhase::Up),
            Offset::new(10., y),
            phase,
            now + Duration::from_millis(ms),
        );
        let hit = tree.pointer_hit_test(event.position);
        tree.dispatch_raw_pointer_with_hit(0, event, &hit);
        tree.dispatch_pointer_gesture_raw_with_hit(0, event, &hit);
        tree.settle_uncontested_raw_arena(0, event);
    }
    assert_eq!(trace.take(), ["ordinary start", "ordinary end"]);
}

fn long_press_factory(
    trace: &Trace,
) -> GestureRecognizerFactoryWithHandlers<ArenaGestureRecognizer<LongPressGestureRecognizer>> {
    let trace = trace.clone();
    GestureRecognizerFactoryWithHandlers::new(
        move || {
            let start = trace.clone();
            let update = trace.clone();
            let end = trace.clone();
            let cancel = trace.clone();
            ArenaGestureRecognizer::new(LongPressGestureRecognizer::new(GestureCallbacks {
                on_long_press_start: Some(Rc::new(move |_| {
                    start.borrow_mut().push("longpressstart".into())
                })),
                on_long_press_move_update: Some(Rc::new(move |_| {
                    update.borrow_mut().push("longpressmoveupdate".into())
                })),
                on_long_press_end: Some(Rc::new(move |_| {
                    end.borrow_mut().push("longpressend".into())
                })),
                on_long_press_cancel: Some(Rc::new(move || {
                    cancel.borrow_mut().push("longpresscancel".into())
                })),
                ..Default::default()
            }))
        },
        |_| {},
    )
}

#[test]
fn losing_a_drag_releases_state_before_the_same_pointer_is_used_again() {
    // Pinned :682: the later press uses the same physical pointer and mounted factories.
    let trace = Trace::default();
    let widget = competition_detector(&trace, true, false, Some(Other::Horizontal), true)
        .gesture(long_press_factory(&trace));
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    let now = Instant::now();
    route(&mut tree, now, 5, PointerDeviceKind::Touch, &[(28.1, 10.)]);
    assert_eq!(trace.take(), ["basichorizontalstart", "basichorizontalend"]);

    route(
        &mut tree,
        now + Duration::from_millis(40),
        5,
        PointerDeviceKind::Touch,
        &[],
    );
}

#[test]
fn a_second_consecutive_tap_claims_before_the_hold_deadline_and_can_drag() {
    // Pinned :744.
    let trace = Trace::default();
    let widget =
        competition_detector(&trace, false, true, None, true).gesture(long_press_factory(&trace));
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .unwrap();
    let now = Instant::now();
    route(&mut tree, now, 5, PointerDeviceKind::Touch, &[]);
    let mut send = |ms, phase, x, y| {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            5,
            0,
            PointerDeviceKind::Touch,
            u32::from(phase != PointerPhase::Up),
            Offset::new(x, y),
            phase,
            now + Duration::from_millis(ms),
        ));
    };
    send(190, PointerPhase::Down, 10., 10.);
    send(690, PointerPhase::Move, 40., 45.);
    send(690, PointerPhase::Up, 40., 45.);
    assert_eq!(
        trace.take(),
        [
            "longpresscancel",
            "down#1",
            "up#1",
            "down#2",
            "panstart#2",
            "panupdate#2",
            "panend#2"
        ]
    );
}

#[test]
fn unmount_cancels_a_raw_hold_once_even_if_it_already_won_the_arena() {
    let trace = Trace::default();
    let widget = RawGestureDetector::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
        .gesture(long_press_factory(&trace));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(incular_widgets::Align::new(incular_config::Alignment::CENTER, widget).into())
        .unwrap();
    let viewport = Constraints::tight(Size::new(200., 200.));
    tree.layout(viewport).unwrap();
    let now = Instant::now();
    let down = RawPointerEvent::new(
        5,
        0,
        PointerDeviceKind::Touch,
        1,
        Offset::new(20., 20.),
        PointerPhase::Down,
        now,
    );
    tree.dispatch_raw_pointer(down);
    assert!(trace.borrow().is_empty());
    tree.update(
        root,
        incular_widgets::Align::new(
            incular_config::Alignment::CENTER,
            Widget::box_(Size::new(200., 200.), Color::BLACK),
        )
        .into(),
    )
    .unwrap();
    tree.layout(viewport).unwrap();
    assert_eq!(&*trace.borrow(), &["longpresscancel"]);
    let later = now + Duration::from_secs(1);
    tree.pump_gesture_timers(later);
    tree.dispatch_raw_pointer(RawPointerEvent::new(
        5,
        0,
        PointerDeviceKind::Touch,
        0,
        Offset::new(20., 20.),
        PointerPhase::Up,
        later,
    ));
    assert_eq!(trace.take(), ["longpresscancel"]);
}
