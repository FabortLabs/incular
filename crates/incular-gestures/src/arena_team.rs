//! Several contenders competing through a single managed arena entry.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureArenaRegistration,
    GestureDisposition,
};

type Client = Rc<dyn GestureArenaClient>;
type Captain = Rc<RefCell<Option<Client>>>;

struct Team {
    arena: GestureArenaManager,
    captain: Captain,
    streams: HashMap<GestureArenaKey, Rc<Combiner>>,
    next_member: u64,
}

/// Gesture contenders that compete as one member of a managed arena.
/// Each pointer stream has a separate membership and winner.
#[derive(Clone)]
pub struct GestureArenaTeam(Rc<RefCell<Team>>);

impl GestureArenaTeam {
    #[must_use]
    pub fn new(arena: GestureArenaManager) -> Self {
        Self(Rc::new(RefCell::new(Team {
            arena,
            captain: Captain::default(),
            streams: HashMap::new(),
            next_member: 0,
        })))
    }

    /// Sets the contender that accepts on behalf of the team. Without a
    /// captain, an explicit claimant or the first remaining member wins.
    pub fn set_captain(&self, captain: Option<Client>) {
        let target = self.0.borrow().captain.clone();
        *target.borrow_mut() = captain;
    }

    /// Adds a client to the team's entry for this pointer.
    pub fn add(&self, key: GestureArenaKey, client: Client) -> GestureTeamRegistration {
        let mut team = self.0.borrow_mut();
        team.next_member = team
            .next_member
            .checked_add(1)
            .expect("team registration IDs exhausted");
        let id = team.next_member;
        let combiner = if let Some(combiner) = team.streams.get(&key) {
            combiner.clone()
        } else {
            let combiner = Rc::new(Combiner {
                owner: Rc::downgrade(&self.0),
                captain: team.captain.clone(),
                key,
                state: RefCell::new(Combination::default()),
            });
            let entry = team.arena.add(key, combiner.clone());
            combiner.state.borrow_mut().entry = Some(entry);
            team.streams.insert(key, combiner.clone());
            combiner
        };
        combiner.state.borrow_mut().members.push((id, client));
        GestureTeamRegistration {
            combiner: Rc::downgrade(&combiner),
            id,
        }
    }
}

/// A single member's decision handle inside a gesture team.
#[derive(Clone)]
pub struct GestureTeamRegistration {
    combiner: Weak<Combiner>,
    id: u64,
}

impl GestureTeamRegistration {
    pub fn resolve(&self, disposition: GestureDisposition) {
        if let Some(combiner) = self.combiner.upgrade() {
            combiner.resolve(self.id, disposition);
        }
    }
}

#[derive(Default)]
struct Combination {
    members: Vec<(u64, Client)>,
    entry: Option<GestureArenaRegistration>,
    preferred: Option<Client>,
}

struct Combiner {
    owner: Weak<RefCell<Team>>,
    captain: Captain,
    key: GestureArenaKey,
    state: RefCell<Combination>,
}

impl Combiner {
    fn resolve(&self, id: u64, disposition: GestureDisposition) {
        match disposition {
            GestureDisposition::Accepted => {
                let entry = {
                    let mut state = self.state.borrow_mut();
                    let Some((_, client)) = state.members.iter().find(|(member, _)| *member == id)
                    else {
                        return;
                    };
                    let elected = self
                        .captain
                        .borrow()
                        .clone()
                        .unwrap_or_else(|| client.clone());
                    state.preferred.get_or_insert(elected);
                    state.entry.clone()
                };
                if let Some(entry) = entry {
                    entry.resolve(disposition);
                }
            }
            GestureDisposition::Rejected => {
                let client = {
                    let mut state = self.state.borrow_mut();
                    let Some(index) = state.members.iter().position(|(member, _)| *member == id)
                    else {
                        return;
                    };
                    let (_, client) = state.members.remove(index);
                    if state
                        .preferred
                        .as_ref()
                        .is_some_and(|winner| Rc::ptr_eq(winner, &client))
                    {
                        state.preferred = None;
                    }
                    client
                };
                client.reject_gesture(self.key);
                let entry = {
                    let state = self.state.borrow();
                    if state.members.is_empty() {
                        state.entry.clone()
                    } else {
                        None
                    }
                };
                if let Some(entry) = entry {
                    entry.resolve(disposition);
                }
            }
            GestureDisposition::Cancelled | GestureDisposition::Pending => {}
        }
    }

    fn take_members(&self) -> (Vec<(u64, Client)>, Option<Client>) {
        if let Some(owner) = self.owner.upgrade() {
            owner.borrow_mut().streams.remove(&self.key);
        }
        let state = std::mem::take(&mut *self.state.borrow_mut());
        (state.members, state.preferred)
    }
}

impl GestureArenaClient for Combiner {
    fn accept_gesture(&self, key: GestureArenaKey) {
        let (members, preferred) = self.take_members();
        let Some(winner) = preferred
            .or_else(|| self.captain.borrow().clone())
            .or_else(|| members.first().map(|(_, client)| client.clone()))
        else {
            return;
        };
        for (_, client) in members {
            if !Rc::ptr_eq(&client, &winner) {
                client.reject_gesture(key);
            }
        }
        winner.accept_gesture(key);
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        let (members, _) = self.take_members();
        for (_, client) in members {
            client.reject_gesture(key);
        }
    }
}
