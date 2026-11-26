use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, time::Instant,
};
use incular_gestures::{
    GestureAction, GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureArenaTeam,
    GestureCallbacks, GestureDecision,
    GestureDisposition::{Accepted, Rejected},
    GestureRecognizer, PointerGestureRecognizer, RawPointerEvent,
};

const KEY: GestureArenaKey = GestureArenaKey::pointer(1, 5);
type Trace = Rc<RefCell<Vec<&'static str>>>;

struct Driver {
    recognizer: RefCell<PointerGestureRecognizer>,
    current_action: RefCell<Option<GestureAction>>,
    active: Cell<bool>,
    accepted: Cell<bool>,
}

impl Driver {
    fn new(callbacks: GestureCallbacks) -> Rc<Self> {
        Rc::new(Self {
            recognizer: RefCell::new(PointerGestureRecognizer::new(callbacks)),
            current_action: RefCell::new(None),
            active: Cell::new(true),
            accepted: Cell::new(false),
        })
    }

    fn route(
        &self,
        event: RawPointerEvent,
        resolve: impl Fn(incular_gestures::GestureDisposition),
    ) {
        if !self.active.get() {
            return;
        }
        let decision = self.recognizer.borrow_mut().observe_raw(event);
        match decision {
            GestureDecision::Accept(action) => {
                if self.accepted.get() {
                    self.recognizer.borrow_mut().dispatch(action);
                } else {
                    *self.current_action.borrow_mut() = Some(action);
                    resolve(Accepted);
                }
            }
            GestureDecision::Reject => resolve(Rejected),
            GestureDecision::Pending if event.phase == PointerPhase::Up => resolve(Rejected),
            _ => {}
        }
    }
}

impl GestureArenaClient for Driver {
    fn accept_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.accepted.set(true);
        if let Some(action) = self.current_action.take() {
            self.recognizer.borrow_mut().dispatch(action);
        }
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.active.set(false);
        self.current_action.take();
    }
}

struct Passive(Trace, &'static str);

impl GestureArenaClient for Passive {
    fn accept_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
        self.0.borrow_mut().push(self.1);
    }
    fn reject_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key, KEY);
    }
}

fn run(
    team: &GestureArenaTeam,
    arena: &GestureArenaManager,
    trace: &Trace,
    delta: Offset,
    captain: Option<Rc<Passive>>,
) {
    let h = trace.clone();
    let v = trace.clone();
    let t = trace.clone();
    let horizontal = Driver::new(GestureCallbacks {
        on_horizontal_drag_start: Some(Rc::new(move |_| h.borrow_mut().push("horizontal"))),
        ..Default::default()
    });
    let vertical = Driver::new(GestureCallbacks {
        on_vertical_drag_start: Some(Rc::new(move |_| v.borrow_mut().push("vertical"))),
        ..Default::default()
    });
    let tap = Driver::new(GestureCallbacks {
        on_tap: Some(Rc::new(move || t.borrow_mut().push("tap"))),
        ..Default::default()
    });
    let captain_ticket = captain.map(|client| team.add(KEY, client));
    let h = team.add(KEY, horizontal.clone());
    let v = team.add(KEY, vertical.clone());
    let t = arena.add(KEY, tap.clone());
    assert!(trace.borrow().is_empty());
    arena.close(KEY);
    arena.flush_deferred_resolutions();
    assert!(trace.borrow().is_empty());

    let origin = Offset::new(10., 10.);
    let start = Instant::now();
    for (index, (phase, position)) in [
        (PointerPhase::Down, origin),
        (PointerPhase::Move, origin + delta),
        (PointerPhase::Up, origin + delta),
    ]
    .into_iter()
    .enumerate()
    {
        let event = RawPointerEvent::new(
            5,
            0,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            position,
            phase,
            start + Duration::from_millis(10 * index as u64),
        );
        if phase == PointerPhase::Up
            && let Some(captain) = &captain_ticket
        {
            captain.resolve(Rejected);
        }
        horizontal.route(event, |decision| h.resolve(decision));
        vertical.route(event, |decision| v.resolve(decision));
        tap.route(event, |decision| t.resolve(decision));
        arena.flush_deferred_resolutions();
        if phase == PointerPhase::Down {
            assert!(trace.borrow().is_empty());
        }
    }
    arena.sweep(KEY);
}

#[test]
fn two_team_drag_recognizers_compete_with_an_outside_tap() {
    let trace = Trace::default();
    let arena = GestureArenaManager::new();
    let team = GestureArenaTeam::new(arena.clone());
    run(&team, &arena, &trace, Offset::ZERO, None);
    assert_eq!(trace.take(), ["tap"]);
    run(&team, &arena, &trace, Offset::new(0., 30.), None);
    assert_eq!(trace.take(), ["vertical"]);
}

#[test]
fn a_passive_captain_accepts_a_team_drag_but_not_the_outside_tap() {
    let trace = Trace::default();
    let arena = GestureArenaManager::new();
    let team = GestureArenaTeam::new(arena.clone());
    let captain = Rc::new(Passive(trace.clone(), "captain"));
    team.set_captain(Some(captain.clone()));
    run(&team, &arena, &trace, Offset::ZERO, Some(captain.clone()));
    assert_eq!(trace.take(), ["tap"]);
    run(&team, &arena, &trace, Offset::new(0., 30.), Some(captain));
    assert_eq!(trace.take(), ["captain"]);
}

#[test]
fn a_single_team_entry_defaults_to_the_first_remaining_member() {
    let trace = Trace::default();
    let arena = GestureArenaManager::new();
    let team = GestureArenaTeam::new(arena.clone());
    let first = team.add(KEY, Rc::new(Passive(trace.clone(), "first")));
    let _second = team.add(KEY, Rc::new(Passive(trace.clone(), "second")));
    first.resolve(Rejected);
    arena.close(KEY);
    assert!(trace.borrow().is_empty());
    arena.flush_deferred_resolutions();
    assert_eq!(trace.take(), ["second"]);
}
