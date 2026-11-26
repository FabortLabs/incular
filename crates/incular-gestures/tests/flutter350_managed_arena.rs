use std::{cell::RefCell, rc::Rc};

use incular_gestures::{
    GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureArenaRegistration,
    GestureDisposition::{Accepted, Rejected},
};

const KEY: GestureArenaKey = GestureArenaKey::pointer(1, 4);

type Calls = Rc<RefCell<Vec<(&'static str, bool)>>>;

struct Probe {
    name: &'static str,
    calls: Calls,
}

impl GestureArenaClient for Probe {
    fn accept_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key.pointer_id(), Some(4));
        self.calls.borrow_mut().push((self.name, true));
    }
    fn reject_gesture(&self, key: GestureArenaKey) {
        assert_eq!(key.pointer_id(), Some(4));
        self.calls.borrow_mut().push((self.name, false));
    }
}

fn add(arena: &GestureArenaManager, calls: &Calls, name: &'static str) -> GestureArenaRegistration {
    arena.add(
        KEY,
        Rc::new(Probe {
            name,
            calls: calls.clone(),
        }),
    )
}

fn pair() -> (GestureArenaManager, Calls, [GestureArenaRegistration; 2]) {
    let arena = GestureArenaManager::new();
    let calls = Calls::default();
    let members = [add(&arena, &calls, "first"), add(&arena, &calls, "second")];
    (arena, calls, members)
}

#[test]
fn sweep_accepts_the_first_remaining_contender_before_rejecting_the_rest() {
    let (arena, calls, _) = pair();
    arena.close(KEY);
    assert!(calls.borrow().is_empty());
    arena.sweep(KEY);
    assert_eq!(*calls.borrow(), [("first", true), ("second", false)]);
}

#[test]
fn a_held_sweep_is_deferred_until_release() {
    let (arena, calls, _) = pair();
    arena.close(KEY);
    assert!(calls.borrow().is_empty());
    arena.hold(KEY);
    assert!(calls.borrow().is_empty());
    arena.sweep(KEY);
    assert!(calls.borrow().is_empty());
    arena.release(KEY);
    assert_eq!(*calls.borrow(), [("first", true), ("second", false)]);
}

#[test]
fn releasing_without_a_pending_sweep_does_not_choose_a_winner() {
    let (arena, calls, _) = pair();
    arena.close(KEY);
    assert!(calls.borrow().is_empty());
    arena.hold(KEY);
    assert!(calls.borrow().is_empty());
    arena.release(KEY);
    assert!(calls.borrow().is_empty());
    arena.sweep(KEY);
    assert_eq!(*calls.borrow(), [("first", true), ("second", false)]);
}

#[test]
fn accepting_while_open_publishes_nothing_until_close() {
    let (arena, calls, [first, _]) = pair();
    assert!(calls.borrow().is_empty());
    first.resolve(Accepted);
    assert!(calls.borrow().is_empty());
    arena.close(KEY);
    assert_eq!(*calls.borrow(), [("second", false), ("first", true)]);
}

#[test]
fn rejecting_an_eager_winner_while_open_does_not_stale_win_on_close() {
    let (arena, calls, [first, _]) = pair();
    let _third = add(&arena, &calls, "third");
    first.resolve(Accepted);
    assert!(calls.borrow().is_empty());
    first.resolve(Rejected);
    assert_eq!(*calls.borrow(), [("first", false)]);
    arena.close(KEY);
    assert_eq!(*calls.borrow(), [("first", false)]);
    arena.sweep(KEY);
    assert_eq!(
        *calls.borrow(),
        [("first", false), ("second", true), ("third", false)]
    );
}

#[test]
fn deferred_default_cannot_deliver_twice_or_act_on_a_reused_key() {
    let arena = GestureArenaManager::new();
    let calls = Calls::default();
    let first = add(&arena, &calls, "first");
    arena.close(KEY);
    assert!(calls.borrow().is_empty());
    arena.sweep(KEY);
    assert_eq!(calls.take(), [("first", true)]);
    let _second = add(&arena, &calls, "second");
    first.resolve(Rejected);
    arena.flush_deferred_resolutions();
    assert!(calls.borrow().is_empty());
    arena.close(KEY);
    assert!(calls.borrow().is_empty());
    arena.flush_deferred_resolutions();
    arena.flush_deferred_resolutions();
    assert_eq!(calls.take(), [("second", true)]);
}
