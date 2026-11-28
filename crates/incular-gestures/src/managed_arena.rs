//! Flutter's explicitly closed gesture arena. The runtime's simultaneous
//! recognizer arbitration remains available as `GestureArena`.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

use crate::{GestureArenaKey, GestureDisposition};

/// The operation that determined the winner of an explicitly managed arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureArenaWinCause {
    Sweep,
    OnlyRemainingMember,
    Accepted,
}

/// A committed arena transition. The ID is the registration ID within the
/// manager, not a pointer address or a recognizer debug string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureArenaTracePhase {
    Opened,
    Added {
        member: u64,
    },
    Closed {
        members: usize,
    },
    Swept {
        members: usize,
    },
    Rejected {
        member: u64,
    },
    Winner {
        member: u64,
        cause: GestureArenaWinCause,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureArenaTrace {
    pub key: GestureArenaKey,
    pub phase: GestureArenaTracePhase,
}

pub type GestureArenaTraceHandler = Rc<dyn Fn(GestureArenaTrace)>;

/// Callbacks of a contender registered in an explicitly managed arena.
pub trait GestureArenaClient {
    fn accept_gesture(&self, key: GestureArenaKey);
    fn reject_gesture(&self, key: GestureArenaKey);
}

/// Handle to one registration. Resolutions from a finished or replaced
/// stream are ignored; a handle alone does not keep its manager alive.
#[derive(Clone)]
pub struct GestureArenaRegistration {
    manager: Weak<RefCell<State>>,
    key: GestureArenaKey,
    member: u64,
    epoch: u64,
}

impl GestureArenaRegistration {
    pub(crate) fn hold(&self) {
        if let Some(inner) = self.live_round() {
            GestureArenaManager { inner }.hold(self.key);
        }
    }

    pub(crate) fn release(&self) {
        if let Some(inner) = self.live_round() {
            GestureArenaManager { inner }.release(self.key);
        }
    }

    fn live_round(&self) -> Option<Rc<RefCell<State>>> {
        self.manager.upgrade().filter(|inner| {
            inner
                .borrow()
                .rounds
                .get(&self.key)
                .is_some_and(|round| round.epoch == self.epoch)
        })
    }

    /// Resolves this contender. Only Accepted and Rejected are decisions.
    pub fn resolve(&self, disposition: GestureDisposition) {
        let Some(inner) = self.manager.upgrade() else {
            return;
        };
        let manager = GestureArenaManager { inner };
        match disposition {
            GestureDisposition::Accepted => manager.accept(self.key, self.member),
            GestureDisposition::Rejected => manager.reject(self.key, self.member),
            GestureDisposition::Pending | GestureDisposition::Cancelled => {}
        }
    }
}

struct Member {
    id: u64,
    client: Rc<dyn GestureArenaClient>,
}

struct Round {
    epoch: u64,
    open: bool,
    held: bool,
    pending_sweep: bool,
    pending_default: bool,
    eager: Option<u64>,
    members: Vec<Member>,
}

impl Round {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            open: true,
            held: false,
            pending_sweep: false,
            pending_default: false,
            eager: None,
            members: Vec::new(),
        }
    }
}

#[derive(Default)]
struct State {
    next_member: u64,
    rounds: HashMap<GestureArenaKey, Round>,
    defaults: VecDeque<(GestureArenaKey, u64)>,
    trace: Option<GestureArenaTraceHandler>,
}

/// Flutter-style ordered arena lifecycle: collect, close, resolve or sweep.
/// An early acceptance is eager but cannot win while registration is open.
/// Hosts drain default sole-member resolutions at a microtask boundary.
#[derive(Clone, Default)]
pub struct GestureArenaManager {
    inner: Rc<RefCell<State>>,
}

impl GestureArenaManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Observes arena changes without changing their decisions. No trace
    /// events are sent by default. The callback runs outside the arena borrow.
    pub fn set_trace_handler(
        &self,
        handler: Option<GestureArenaTraceHandler>,
    ) -> Option<GestureArenaTraceHandler> {
        std::mem::replace(&mut self.inner.borrow_mut().trace, handler)
    }

    fn trace(&self, key: GestureArenaKey, phase: GestureArenaTracePhase) {
        let handler = self.inner.borrow().trace.clone();
        if let Some(handler) = handler {
            let _ = catch_unwind(AssertUnwindSafe(move || {
                handler(GestureArenaTrace { key, phase })
            }));
        }
    }

    /// Adds a contender. The first add for a stream opens its arena.
    ///
    /// # Panics
    ///
    /// Panics when the same round has already closed registration.
    pub fn add(
        &self,
        key: GestureArenaKey,
        client: Rc<dyn GestureArenaClient>,
    ) -> GestureArenaRegistration {
        let (member, epoch, opened) = {
            let mut state = self.inner.borrow_mut();
            state.next_member = state
                .next_member
                .checked_add(1)
                .expect("arena registration IDs exhausted");
            let member = state.next_member;
            let round = state
                .rounds
                .entry(key)
                .or_insert_with(|| Round::new(member));
            assert!(round.open, "cannot add to a closed gesture arena");
            round.members.push(Member { id: member, client });
            (member, round.epoch, member == round.epoch)
        };
        if opened {
            self.trace(key, GestureArenaTracePhase::Opened);
        }
        self.trace(key, GestureArenaTracePhase::Added { member });
        GestureArenaRegistration {
            manager: Rc::downgrade(&self.inner),
            key,
            member,
            epoch,
        }
    }

    /// Closes registration and settles an eager winner if there is still
    /// competition. A sole default winner is deferred, not called inline.
    pub fn close(&self, key: GestureArenaKey) {
        let (epoch, members) = {
            let mut state = self.inner.borrow_mut();
            let Some(round) = state.rounds.get_mut(&key) else {
                return;
            };
            round.open = false;
            (round.epoch, round.members.len())
        };
        self.trace(key, GestureArenaTracePhase::Closed { members });
        self.try_settle(key, epoch);
    }

    /// Prevents a sweep from resolving this round until released.
    pub fn hold(&self, key: GestureArenaKey) {
        if let Some(round) = self.inner.borrow_mut().rounds.get_mut(&key) {
            round.held = true;
        }
    }

    /// Releases a held round. A previously requested sweep runs now.
    pub fn release(&self, key: GestureArenaKey) {
        let pending = self
            .inner
            .borrow_mut()
            .rounds
            .get_mut(&key)
            .is_some_and(|round| {
                round.held = false;
                round.pending_sweep
            });
        if pending {
            self.sweep(key);
        }
    }

    /// Awards a closed round to its first remaining contender unless held.
    ///
    /// # Panics
    ///
    /// Panics if registration is still open.
    pub fn sweep(&self, key: GestureArenaKey) {
        let round = {
            let mut state = self.inner.borrow_mut();
            let Some(round) = state.rounds.get_mut(&key) else {
                return;
            };
            assert!(!round.open, "cannot sweep an open gesture arena");
            if round.held {
                round.pending_sweep = true;
                return;
            }
            state.rounds.remove(&key)
        };
        if let Some(round) = round {
            self.trace(
                key,
                GestureArenaTracePhase::Swept {
                    members: round.members.len(),
                },
            );
            let mut members = round.members.into_iter();
            if let Some(winner) = members.next() {
                self.trace(
                    key,
                    GestureArenaTracePhase::Winner {
                        member: winner.id,
                        cause: GestureArenaWinCause::Sweep,
                    },
                );
                winner.client.accept_gesture(key);
                for loser in members {
                    loser.client.reject_gesture(key);
                }
            }
        }
    }

    /// Runs deferred default wins after the current input dispatch. A stale
    /// deferred operation cannot resolve a newer round with the same key.
    pub fn flush_deferred_resolutions(&self) {
        loop {
            let work = self.inner.borrow_mut().defaults.pop_front();
            let Some((key, epoch)) = work else {
                break;
            };
            let winner = {
                let mut state = self.inner.borrow_mut();
                if state.rounds.get(&key).is_some_and(|round| {
                    round.epoch == epoch && !round.open && round.members.len() == 1
                }) {
                    state
                        .rounds
                        .remove(&key)
                        .and_then(|mut round| round.members.pop())
                } else {
                    None
                }
            };
            if let Some(winner) = winner {
                self.trace(
                    key,
                    GestureArenaTracePhase::Winner {
                        member: winner.id,
                        cause: GestureArenaWinCause::OnlyRemainingMember,
                    },
                );
                winner.client.accept_gesture(key);
            }
        }
    }

    fn accept(&self, key: GestureArenaKey, member: u64) {
        {
            let mut state = self.inner.borrow_mut();
            let Some(round) = state.rounds.get_mut(&key) else {
                return;
            };
            if !round.members.iter().any(|entry| entry.id == member) {
                return;
            }
            if round.open {
                round.eager.get_or_insert(member);
                return;
            }
        }
        self.finish(key, member);
    }

    fn reject(&self, key: GestureArenaKey, member: u64) {
        let (client, epoch) = {
            let mut state = self.inner.borrow_mut();
            let Some(round) = state.rounds.get_mut(&key) else {
                return;
            };
            let Some(index) = round.members.iter().position(|entry| entry.id == member) else {
                return;
            };
            if round.eager == Some(member) {
                round.eager = None;
            }
            (round.members.remove(index).client, round.epoch)
        };
        self.trace(key, GestureArenaTracePhase::Rejected { member });
        client.reject_gesture(key);
        self.try_settle(key, epoch);
    }

    fn try_settle(&self, key: GestureArenaKey, epoch: u64) {
        let winner = {
            let mut state = self.inner.borrow_mut();
            let Some(round) = state.rounds.get_mut(&key) else {
                return;
            };
            if round.open || round.epoch != epoch {
                return;
            }
            match round.members.len() {
                0 => {
                    state.rounds.remove(&key);
                    None
                }
                1 => {
                    if !round.pending_default {
                        round.pending_default = true;
                        state.defaults.push_back((key, epoch));
                    }
                    None
                }
                _ => round.eager,
            }
        };
        if let Some(winner) = winner {
            self.finish(key, winner);
        }
    }

    fn finish(&self, key: GestureArenaKey, winner: u64) {
        let (winner, losers) = {
            let mut state = self.inner.borrow_mut();
            let Some(index) = state
                .rounds
                .get(&key)
                .and_then(|round| round.members.iter().position(|entry| entry.id == winner))
            else {
                return;
            };
            let mut round = state.rounds.remove(&key).expect("live arena round");
            (round.members.remove(index), round.members)
        };
        // Explicit acceptance rejects the other members before calling the
        // winner. Sweep has the opposite order. No callback borrows State.
        self.trace(
            key,
            GestureArenaTracePhase::Winner {
                member: winner.id,
                cause: GestureArenaWinCause::Accepted,
            },
        );
        for loser in losers {
            loser.client.reject_gesture(key);
        }
        winner.client.accept_gesture(key);
    }
}
