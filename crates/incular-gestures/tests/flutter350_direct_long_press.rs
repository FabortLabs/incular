use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON, time::Instant,
};
use incular_gestures::{
    DragAxis, DragCallbacks, GestureArenaClient, GestureArenaKey, GestureArenaManager,
    GestureCallbacks, GestureDisposition, LongPressGestureRecognizer, MonodragGestureRecognizer,
    PointerDeviceKind, RawPointerEvent,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Down,
    Cancel,
    Start,
    Move(Offset),
    End,
    DragStart,
}
use Event::*;

type Log = Rc<RefCell<Vec<Event>>>;

fn setup() -> (LongPressGestureRecognizer, Log) {
    setup_for(None)
}

fn setup_for(kind: Option<PointerDeviceKind>) -> (LongPressGestureRecognizer, Log) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut callbacks = GestureCallbacks {
        on_long_press_down: Some(Rc::new({
            let log = log.clone();
            move |_| log.borrow_mut().push(Down)
        })),
        on_long_press_cancel: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push(Cancel)
        })),
        on_long_press: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push(Start)
        })),
        on_long_press_move_update: Some(Rc::new({
            let log = log.clone();
            move |details| log.borrow_mut().push(Move(details.global_position))
        })),
        on_long_press_up: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push(End)
        })),
        ..Default::default()
    };
    if let Some(kind) = kind {
        callbacks.set_supported_devices([kind]);
    }
    (LongPressGestureRecognizer::new(callbacks), log)
}

fn raw(base: Instant, millis: u64, phase: PointerPhase, x: f32, y: f32) -> RawPointerEvent {
    RawPointerEvent::new(
        5,
        1,
        PointerDeviceKind::Touch,
        if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        Offset::new(x, y),
        phase,
        base + Duration::from_millis(millis),
    )
}

fn begin(recognizer: &mut LongPressGestureRecognizer, log: &Log, now: Instant) {
    let down = raw(now, 0, PointerPhase::Down, 10., 10.);
    assert!(recognizer.add_pointer(down.clone()));
    assert!(log.borrow().is_empty());
    assert!(recognizer.close_arena(5));
    assert!(log.borrow().is_empty());
    recognizer.handle_event(down);
    assert_eq!(*log.borrow(), [Down]);
}

#[test]
fn registering_and_closing_a_long_press_is_silent_until_down_and_deadline() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    for t in [300, 499] {
        press.poll(at + Duration::from_millis(t));
        assert_eq!(*log.borrow(), [Down]);
    }
    press.poll(at + Duration::from_millis(500));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.poll(at + Duration::from_secs(1));
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Start]);
}

#[test]
fn custom_duration_starts_at_100ms_not_at_the_ordinary_deadline() {
    let (press, log) = setup();
    let mut press = press.duration(Duration::from_millis(100));
    let at = Instant::now();
    begin(&mut press, &log, at);
    for t in [50, 99] {
        press.poll(at + Duration::from_millis(t));
        assert_eq!(*log.borrow(), [Down]);
    }
    press.poll(at + Duration::from_millis(100));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Start]);
}

#[test]
fn release_before_timeout_cancels_without_a_late_start() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_millis(300));
    assert_eq!(*log.borrow(), [Down]);
    press.handle_event(raw(at, 300, PointerPhase::Up, 11., 9.));
    assert_eq!(*log.borrow(), [Down, Cancel]);
    press.poll(at + Duration::from_millis(1300));
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Cancel]);
}

#[test]
fn movement_before_acceptance_cancels_the_registered_contact_once() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_millis(300));
    assert_eq!(*log.borrow(), [Down]);
    press.handle_event(raw(at, 300, PointerPhase::Move, 100., 200.));
    assert_eq!(*log.borrow(), [Down, Cancel]);
    press.poll(at + Duration::from_millis(1300));
    press.handle_event(raw(at, 1300, PointerPhase::Up, 11., 9.));
    press.poll(at + Duration::from_millis(1600));
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Cancel]);
}

#[test]
fn movement_after_acceptance_reports_position_and_release_without_cancelling() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_secs(1));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.handle_event(raw(at, 1000, PointerPhase::Move, 100., 200.));
    assert_eq!(*log.borrow(), [Down, Start, Move(Offset::new(100., 200.))]);
    press.handle_event(raw(at, 1000, PointerPhase::Up, 11., 9.));
    press.poll(at + Duration::from_millis(1300));
    press.dispose();
    assert_eq!(
        *log.borrow(),
        [Down, Start, Move(Offset::new(100., 200.)), End]
    );
}

#[test]
fn a_combined_button_mask_does_not_register_or_emit_even_when_down_is_routed() {
    let (mut press, log) = setup();
    let at = Instant::now();
    let physical = RawPointerEvent::new(
        5,
        1,
        PointerDeviceKind::Mouse,
        SECONDARY_POINTER_BUTTON | TERTIARY_POINTER_BUTTON,
        Offset::new(10., 10.),
        PointerPhase::Down,
        at,
    );
    assert!(!press.add_pointer(physical));
    assert!(!press.close_arena(5));
    press.handle_event(raw(at, 0, PointerPhase::Down, 10., 10.));
    assert!(log.borrow().is_empty());
    press.poll(at + Duration::from_secs(1));
    press.handle_event(raw(at, 1000, PointerPhase::Up, 11., 9.));
    press.dispose();
    assert!(log.borrow().is_empty());
    begin(&mut press, &log, at + Duration::from_secs(2));
    press.reject_gesture(5);
    assert_eq!(*log.borrow(), [Down, Cancel]);
}

#[test]
fn a_recognized_hold_notifies_up_exactly_once_without_movement() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_millis(300));
    assert_eq!(*log.borrow(), [Down]);
    press.poll(at + Duration::from_secs(1));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.handle_event(raw(at, 1000, PointerPhase::Up, 11., 9.));
    assert_eq!(*log.borrow(), [Down, Start, End]);
    // A duplicate release and later deadline must not synthesize an end.
    press.handle_event(raw(at, 1001, PointerPhase::Up, 11., 9.));
    press.poll(at + Duration::from_millis(1300));
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Start, End]);
}

#[test]
fn disposal_cancels_a_pending_routed_hold_but_not_an_unrouted_registration() {
    let (mut press, log) = setup();
    let at = Instant::now();
    assert!(press.add_pointer(raw(at, 0, PointerPhase::Down, 10., 10.)));
    press.dispose();
    assert!(log.borrow().is_empty());

    begin(&mut press, &log, at + Duration::from_secs(1));
    press.poll(at + Duration::from_millis(1300));
    assert_eq!(*log.borrow(), [Down]);
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Cancel]);
    press.poll(at + Duration::from_secs(2));
    press.handle_event(raw(at, 2000, PointerPhase::Up, 11., 9.));
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Cancel]);
}

#[test]
fn changing_buttons_before_the_deadline_cancels_once() {
    for (buttons, kind) in [
        (TERTIARY_POINTER_BUTTON, PointerDeviceKind::Mouse),
        (SECONDARY_POINTER_BUTTON, PointerDeviceKind::Touch),
    ] {
        let (mut press, log) = setup();
        let at = Instant::now();
        begin(&mut press, &log, at);
        press.poll(at + Duration::from_millis(300));
        assert_eq!(*log.borrow(), [Down]);
        let mut moved = raw(at, 300, PointerPhase::Move, 10., 10.);
        moved.kind = kind;
        moved.buttons = buttons;
        press.handle_event(moved);
        assert_eq!(*log.borrow(), [Down, Cancel]);
        press.poll(at + Duration::from_secs(1));
        press.handle_event(raw(at, 1000, PointerPhase::Up, 11., 9.));
        press.dispose();
        assert_eq!(*log.borrow(), [Down, Cancel]);
    }
}

#[test]
fn an_unsupported_pointer_does_not_reset_an_already_accepted_touch() {
    let (mut press, log) = setup_for(Some(PointerDeviceKind::Touch));
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_millis(500));
    assert_eq!(*log.borrow(), [Down, Start]);
    let mut mouse = raw(at, 510, PointerPhase::Down, 10., 10.);
    mouse.pointer = 101;
    mouse.kind = PointerDeviceKind::Mouse;
    assert!(!press.add_pointer(mouse));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.handle_event(raw(at, 520, PointerPhase::Move, 15., 15.));
    assert_eq!(*log.borrow(), [Down, Start, Move(Offset::new(15., 15.))]);
    press.dispose();
}

#[test]
fn filtered_touch_stays_silent_and_a_later_mouse_with_the_same_id_can_hold() {
    let (mut press, log) = setup_for(Some(PointerDeviceKind::Mouse));
    let at = Instant::now();
    let touch = raw(at, 0, PointerPhase::Down, 10., 10.);
    assert!(!press.add_pointer(touch.clone()));
    assert!(!press.close_arena(5));
    press.handle_event(touch);
    press.poll(at + Duration::from_secs(2));
    assert!(log.borrow().is_empty());
    let mut mouse = raw(at, 2000, PointerPhase::Down, 10., 10.);
    mouse.kind = PointerDeviceKind::Mouse;
    assert!(press.add_pointer(mouse.clone()));
    assert!(press.close_arena(5));
    assert!(log.borrow().is_empty());
    press.handle_event(mouse);
    assert_eq!(*log.borrow(), [Down]);
    press.poll(at + Duration::from_secs(4));
    assert_eq!(*log.borrow(), [Down, Start]);
    press.dispose();
}

#[test]
fn a_post_acceptance_button_change_keeps_current_and_next_hold_working() {
    let (mut press, log) = setup();
    let at = Instant::now();
    begin(&mut press, &log, at);
    press.poll(at + Duration::from_secs(1));
    let mut moved = raw(at, 1000, PointerPhase::Move, 10., 10.);
    moved.buttons = SECONDARY_POINTER_BUTTON;
    press.handle_event(moved);
    press.handle_event(raw(at, 1000, PointerPhase::Up, 11., 9.));
    assert_eq!(
        *log.borrow(),
        [Down, Start, Move(Offset::new(10., 10.)), End]
    );
    log.borrow_mut().clear();

    let mut next = raw(at, 1001, PointerPhase::Down, 10., 10.);
    next.pointer = 6;
    assert!(press.add_pointer(next.clone()));
    assert!(press.close_arena(6));
    press.handle_event(next);
    press.poll(at + Duration::from_millis(2001));
    assert_eq!(*log.borrow(), [Down, Start]);
    log.borrow_mut().clear();
    let mut up = raw(at, 2001, PointerPhase::Up, 11., 9.);
    up.pointer = 6;
    press.handle_event(up);
    assert_eq!(*log.borrow(), [End]);
}

const KEY: GestureArenaKey = GestureArenaKey::pointer_device(1, 1, 5);

#[derive(Default)]
struct HoldAward(Cell<Option<GestureDisposition>>);

impl GestureArenaClient for HoldAward {
    fn accept_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.0.set(Some(GestureDisposition::Accepted));
    }
    fn reject_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.0.set(Some(GestureDisposition::Rejected));
    }
}

struct HorizontalDrag(RefCell<MonodragGestureRecognizer>);

impl GestureArenaClient for HorizontalDrag {
    fn accept_gesture(&self, key: GestureArenaKey) {
        self.0.borrow_mut().accept_stream(key);
    }
    fn reject_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.0.borrow_mut().reject_stream(key);
    }
}

#[test]
fn disposing_a_pending_hold_defers_the_surviving_horizontal_drag_to_the_boundary() {
    let (press, log) = setup();
    let arena = GestureArenaManager::new();
    let in_dispose = Rc::new(Cell::new(false));
    let drag = Rc::new(HorizontalDrag(RefCell::new(
        MonodragGestureRecognizer::new(
            DragAxis::Horizontal,
            DragCallbacks {
                on_start: Some(Rc::new({
                    let log = log.clone();
                    let in_dispose = in_dispose.clone();
                    move |details| {
                        assert!(!in_dispose.get(), "drag started on the disposal stack");
                        assert_eq!(details.global_position, Offset::new(10., 10.));
                        log.borrow_mut().push(DragStart);
                    }
                })),
                ..Default::default()
            },
        ),
    )));
    let _drag_registration = arena.add(KEY, drag.clone());
    let award = Rc::new(HoldAward::default());
    let registration = arena.add(KEY, award.clone());
    let mut press = press.with_external_arena(move |pointer, disposition| {
        assert_eq!(pointer, 5);
        registration.resolve(disposition);
        award.0.take()
    });

    let at = Instant::now();
    let down = raw(at, 0, PointerPhase::Down, 10., 10.);
    assert!(press.add_pointer(down.clone()));
    arena.close(KEY);
    assert!(log.borrow().is_empty());
    assert!(drag.0.borrow_mut().observe_raw(down.clone()));
    assert!(!drag.0.borrow().wants_acceptance());
    press.handle_event(down);
    assert_eq!(*log.borrow(), [Down]);
    press.poll(at + Duration::from_millis(300));
    assert_eq!(*log.borrow(), [Down]);

    in_dispose.set(true);
    press.dispose();
    in_dispose.set(false);
    assert_eq!(*log.borrow(), [Down, Cancel]);
    arena.flush_deferred_resolutions();
    assert_eq!(*log.borrow(), [Down, Cancel, DragStart]);
    arena.flush_deferred_resolutions();
    assert_eq!(*log.borrow(), [Down, Cancel, DragStart]);
}

#[test]
fn each_external_long_press_waits_for_both_the_deadline_and_its_award() {
    for award_before_deadline in [false, true] {
        let (press, log) = setup();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let mut press = press.with_external_arena({
            let requests = requests.clone();
            move |pointer, disposition| {
                requests.borrow_mut().push((pointer, disposition));
                None
            }
        });
        let at = Instant::now();
        let down = raw(at, 0, PointerPhase::Down, 10., 10.);
        assert!(press.add_pointer(down.clone()));
        assert!(!press.close_arena(5));
        press.handle_event(down);
        if award_before_deadline {
            press.accept_gesture(5);
        }
        press.poll(at + Duration::from_millis(499));
        assert_eq!(*log.borrow(), [Down]);
        assert!(requests.borrow().is_empty());
        press.poll(at + Duration::from_millis(500));

        if !award_before_deadline {
            press.poll(at + Duration::from_millis(550));
            press.handle_event(raw(at, 551, PointerPhase::Move, 13., 10.));
            assert_eq!(*log.borrow(), [Down]);
            assert_eq!(*requests.borrow(), [(5, GestureDisposition::Accepted)]);
            press.accept_gesture(5);
        }
        assert_eq!(*log.borrow(), [Down, Start]);
        press.handle_event(raw(at, 600, PointerPhase::Move, 100., 200.));
        press.handle_event(raw(at, 610, PointerPhase::Up, 11., 9.));
        press.dispose();
        assert_eq!(
            *log.borrow(),
            [Down, Start, Move(Offset::new(100., 200.)), End]
        );
        assert_eq!(requests.borrow().len(), usize::from(!award_before_deadline));
    }
}

#[test]
fn a_pending_arena_award_does_not_auto_accept_on_move_or_release() {
    let (press, log) = setup();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let mut press = press.with_external_arena({
        let requests = requests.clone();
        move |pointer, disposition| {
            requests.borrow_mut().push((pointer, disposition));
            None
        }
    });
    let at = Instant::now();
    let down = raw(at, 0, PointerPhase::Down, 10., 10.);
    assert!(press.add_pointer(down.clone()));
    press.handle_event(down);
    press.handle_event(raw(at, 500, PointerPhase::Move, 15., 15.));
    assert_eq!(*log.borrow(), [Down]);
    press.handle_event(raw(at, 550, PointerPhase::Up, 11., 9.));
    press.accept_gesture(5);
    press.dispose();
    assert_eq!(*log.borrow(), [Down, Cancel]);
    assert_eq!(
        *requests.borrow(),
        [
            (5, GestureDisposition::Accepted),
            (5, GestureDisposition::Rejected)
        ]
    );
}
