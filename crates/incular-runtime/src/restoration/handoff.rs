//! Two-phase transfer of one persistent scope between live owners.
//!
//! The normal mounted-scope path continues to reject duplicate leases. A
//! deliberate replacement sees a private, empty snapshot while the old
//! owner is alive and can publish it only after the old claims are released.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use incular_core::{RestorationBackend, RestorationKey, RestorationScope};
use serde_json::Value;

use super::{
    ClaimedRestorationScope, RestorationHandle, RestorationManager, ScopeLease, count_scope,
    encode_path, refresh_counts, remove_snapshot_scope,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RestorationHandoffError {
    #[error("the root restoration scope cannot be replaced by a handoff")]
    RootScope,
    #[error("there is no live owner or pending replacement for this scope")]
    NoOwner,
    #[error("the previous restoration scope owner has not released its claim")]
    PreviousOwnerStillActive,
    #[error("multiple replacement owners claimed this restoration scope")]
    MultipleReplacementOwners,
    #[error("a descendant restoration scope is still claimed")]
    ActiveDescendant,
    #[error("the restoration scope handoff was committed or invalidated by reset")]
    Closed,
}

/// A fresh, private scope prepared while a previous owner is still mounted.
/// Changes are not persisted until commit. Dropping an uncommitted replacement
/// abandons it without changing the current owner's snapshot.
#[must_use = "commit the replacement or drop it to abandon the handoff"]
pub struct PendingRestorationScope {
    manager: RestorationManager,
    path: String,
    epoch: u64,
    pending: bool,
    scope: RestorationScope,
    storage: Rc<HandoffStorage>,
}

impl RestorationHandle {
    /// Begins an explicit same-turn replacement instead of sharing an
    /// existing scope or changing `claim_scope`'s strict unique ownership.
    /// The replacement initially has no values, even if the original does.
    pub fn prepare_scope_handoff(
        &self,
        existing: &RestorationScope,
    ) -> Result<PendingRestorationScope, RestorationHandoffError> {
        if existing.is_root() {
            return Err(RestorationHandoffError::RootScope);
        }
        let path = encode_path(existing.path());
        let epoch = {
            let mut state = self.manager.inner.state.borrow_mut();
            if !state.active_scope_paths.contains(&path)
                && !state.pending_scope_handoffs.contains_key(&path)
            {
                return Err(RestorationHandoffError::NoOwner);
            }
            *state
                .pending_scope_handoffs
                .entry(path.clone())
                .or_default() += 1;
            state.handoff_epoch
        };
        let storage = Rc::new(HandoffStorage(RefCell::new(Storage::Pending(
            BTreeMap::new(),
        ))));
        let scope = RestorationScope::new(storage.clone(), existing.path().iter().cloned());
        Ok(PendingRestorationScope {
            manager: self.manager.clone(),
            path,
            epoch,
            pending: true,
            scope,
            storage,
        })
    }
}

impl PendingRestorationScope {
    #[must_use]
    pub fn scope(&self) -> &RestorationScope {
        &self.scope
    }

    /// Replaces the persisted subtree with the staged values in one
    /// in-memory mutation and schedules one ordinary save. The same scope
    /// then addresses live storage. Retain the returned claim as long as
    /// the new owner is mounted. Failure leaves the live snapshot unchanged
    /// and may be retried after other owners release their claims.
    pub fn commit(&mut self) -> Result<ClaimedRestorationScope, RestorationHandoffError> {
        let mut state = self.manager.inner.state.borrow_mut();
        if !self.pending || state.handoff_epoch != self.epoch {
            return Err(RestorationHandoffError::Closed);
        }
        if state.pending_scope_handoffs.get(&self.path) != Some(&1) {
            return Err(RestorationHandoffError::MultipleReplacementOwners);
        }
        if state.active_scope_paths.contains(&self.path) {
            return Err(RestorationHandoffError::PreviousOwnerStillActive);
        }
        let descendants = format!("{}/", self.path);
        if state
            .active_scope_paths
            .iter()
            .any(|path| path.starts_with(&descendants))
        {
            return Err(RestorationHandoffError::ActiveDescendant);
        }

        let Storage::Pending(values) = self.storage.0.replace(Storage::Live(self.manager.clone()))
        else {
            unreachable!("only a pending handoff can commit");
        };
        remove_snapshot_scope(&mut state, &self.path);
        for (key, value) in values {
            count_scope(&mut state.scope_counts, &key);
            state.snapshot.state.insert(key, value);
        }
        state.snapshot.claimed_scopes.insert(self.path.clone());
        *state.scope_counts.entry(self.path.clone()).or_default() += 1;
        state.pending_scope_handoffs.remove(&self.path);
        state.active_scope_paths.insert(self.path.clone());
        refresh_counts(&mut state);
        self.pending = false;
        drop(state);
        let lease = ScopeLease {
            manager: Rc::downgrade(&self.manager.inner),
            path: self.path.clone(),
        };
        self.manager.mark_dirty();
        Ok(ClaimedRestorationScope { _lease: lease })
    }
}

impl Drop for PendingRestorationScope {
    fn drop(&mut self) {
        if !self.pending {
            return;
        }
        self.storage.0.replace(Storage::Abandoned);
        let mut state = self.manager.inner.state.borrow_mut();
        if state.handoff_epoch == self.epoch
            && let Some(count) = state.pending_scope_handoffs.get_mut(&self.path)
        {
            *count -= 1;
            if *count == 0 {
                state.pending_scope_handoffs.remove(&self.path);
            }
        }
    }
}

enum Storage {
    Pending(BTreeMap<String, Value>),
    Live(RestorationManager),
    Abandoned,
}

struct HandoffStorage(RefCell<Storage>);

impl RestorationBackend for HandoffStorage {
    fn read_value(&self, path: &[RestorationKey]) -> Option<Value> {
        let manager = match &*self.0.borrow() {
            Storage::Pending(values) => return values.get(&encode_path(path)).cloned(),
            Storage::Abandoned => return None,
            Storage::Live(manager) => manager.clone(),
        };
        manager.read(path)
    }

    fn write_value(&self, path: &[RestorationKey], value: Value) {
        let manager = match &mut *self.0.borrow_mut() {
            Storage::Pending(values) => {
                values.insert(encode_path(path), value);
                return;
            }
            Storage::Abandoned => return,
            Storage::Live(manager) => manager.clone(),
        };
        manager.write(path, value);
    }

    fn remove_value(&self, path: &[RestorationKey]) {
        let manager = match &mut *self.0.borrow_mut() {
            Storage::Pending(values) => {
                values.remove(&encode_path(path));
                return;
            }
            Storage::Abandoned => return,
            Storage::Live(manager) => manager.clone(),
        };
        manager.remove(path);
    }

    fn note_restoration_outcome(&self, restored: u64, invalid: u64) {
        let manager = match &*self.0.borrow() {
            Storage::Live(manager) => manager.clone(),
            _ => return,
        };
        manager.note_restoration_outcome(restored, invalid);
    }
}
