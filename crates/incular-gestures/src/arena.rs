use std::collections::HashMap;

use incular_core::TrackpadGestureKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureArenaSource {
    Pointer {
        device: u64,
        pointer: u64,
    },
    PointerPanZoom {
        device: u64,
        pointer: u64,
    },
    Trackpad {
        device: u64,
        kind: TrackpadGestureKind,
    },
}

/// Identifies one gesture stream within a native window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GestureArenaKey {
    pub window: u64,
    pub source: GestureArenaSource,
}

impl GestureArenaKey {
    #[must_use]
    pub const fn pointer(window: u64, pointer: u64) -> Self {
        Self::pointer_device(window, 0, pointer)
    }

    #[must_use]
    pub const fn pointer_device(window: u64, device: u64, pointer: u64) -> Self {
        Self {
            window,
            source: GestureArenaSource::Pointer { device, pointer },
        }
    }

    #[must_use]
    pub const fn trackpad(window: u64, device: u64, kind: TrackpadGestureKind) -> Self {
        Self {
            window,
            source: GestureArenaSource::Trackpad { device, kind },
        }
    }

    /// A correlated absolute trackpad stream, not a touch or an aggregate OS gesture.
    #[must_use]
    pub const fn pointer_pan_zoom(window: u64, device: u64, pointer: u64) -> Self {
        Self {
            window,
            source: GestureArenaSource::PointerPanZoom { device, pointer },
        }
    }

    #[must_use]
    pub const fn pointer_id(self) -> Option<u64> {
        match self.source {
            GestureArenaSource::Pointer { pointer, .. }
            | GestureArenaSource::PointerPanZoom { pointer, .. } => Some(pointer),
            GestureArenaSource::Trackpad { .. } => None,
        }
    }

    #[must_use]
    pub const fn pointer_device_id(self) -> Option<u64> {
        match self.source {
            GestureArenaSource::Pointer { device, .. }
            | GestureArenaSource::PointerPanZoom { device, .. } => Some(device),
            GestureArenaSource::Trackpad { .. } => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureDisposition {
    Pending,
    Accepted,
    Rejected,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GestureArenaMember(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureArenaEntry {
    pub member: GestureArenaMember,
    pub disposition: GestureDisposition,
}
/// One member of a gesture stream as the arena tracks it.
struct StreamEntry {
    member: GestureArenaMember,
    simultaneous: bool,
    disposition: GestureDisposition,
}
#[derive(Default)]
pub struct GestureArena {
    next: u64,
    streams: HashMap<GestureArenaKey, Vec<StreamEntry>>,
}
impl GestureArena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, key: GestureArenaKey, simultaneous: bool) -> GestureArenaMember {
        self.next += 1;
        let member = GestureArenaMember(self.next);
        self.streams.entry(key).or_default().push(StreamEntry {
            member,
            simultaneous,
            disposition: GestureDisposition::Pending,
        });
        member
    }
    /// Notifies the sole remaining pending member when the host closes a
    /// pointer arena. A decided or still-contested arena emits no default win.
    pub fn close_sole_member(&mut self, key: GestureArenaKey) -> Vec<GestureArenaEntry> {
        let Some(member) = self.sole_pending_member(key) else {
            return Vec::new();
        };
        self.accept(key, member)
    }

    /// The default winner, if no member has won and exactly one is pending.
    #[must_use]
    pub fn sole_pending_member(&self, key: GestureArenaKey) -> Option<GestureArenaMember> {
        let entries = self.streams.get(&key)?;
        if entries
            .iter()
            .any(|entry| entry.disposition == GestureDisposition::Accepted)
        {
            return None;
        }
        let mut pending = entries
            .iter()
            .filter(|entry| entry.disposition == GestureDisposition::Pending);
        let winner = pending.next()?.member;
        pending.next().is_none().then_some(winner)
    }

    pub fn accept(
        &mut self,
        key: GestureArenaKey,
        member: GestureArenaMember,
    ) -> Vec<GestureArenaEntry> {
        let Some(entries) = self.streams.get_mut(&key) else {
            return Vec::new();
        };
        let Some(index) = entries.iter().position(|entry| entry.member == member) else {
            return Vec::new();
        };
        if entries[index].disposition != GestureDisposition::Pending {
            return entries
                .iter()
                .map(|entry| GestureArenaEntry {
                    member: entry.member,
                    disposition: entry.disposition,
                })
                .collect();
        }
        let simultaneous = entries[index].simultaneous;
        let blocked = entries.iter().any(|entry| {
            entry.disposition == GestureDisposition::Accepted
                && !(simultaneous && entry.simultaneous)
        });
        if blocked {
            entries[index].disposition = GestureDisposition::Rejected;
        } else {
            entries[index].disposition = GestureDisposition::Accepted;
            for entry in entries.iter_mut() {
                if entry.disposition == GestureDisposition::Pending
                    && !(simultaneous && entry.simultaneous)
                {
                    entry.disposition = GestureDisposition::Rejected;
                }
            }
        }
        entries
            .iter()
            .map(|e| GestureArenaEntry {
                member: e.member,
                disposition: e.disposition,
            })
            .collect()
    }
    /// Rejects a pending member. Later resolutions do not rewrite a winner;
    /// ending a live stream is a separate cancellation.
    pub fn reject(&mut self, key: GestureArenaKey, member: GestureArenaMember) {
        if let Some(entries) = self.streams.get_mut(&key)
            && let Some(entry) = entries
                .iter_mut()
                .find(|e| e.member == member && e.disposition == GestureDisposition::Pending)
        {
            entry.disposition = GestureDisposition::Rejected;
        }
    }
    pub fn cancel(&mut self, key: GestureArenaKey) -> Vec<GestureArenaEntry> {
        self.streams
            .remove(&key)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| GestureArenaEntry {
                member: entry.member,
                disposition: GestureDisposition::Cancelled,
            })
            .collect()
    }
    /// Looks up a routed member without copying the stream's other entries.
    #[must_use]
    pub fn disposition(
        &self,
        key: GestureArenaKey,
        member: GestureArenaMember,
    ) -> Option<GestureDisposition> {
        self.streams
            .get(&key)?
            .iter()
            .find(|entry| entry.member == member)
            .map(|entry| entry.disposition)
    }

    #[must_use]
    pub fn entries(&self, key: GestureArenaKey) -> Vec<GestureArenaEntry> {
        self.streams
            .get(&key)
            .map(|v| {
                v.iter()
                    .map(|e| GestureArenaEntry {
                        member: e.member,
                        disposition: e.disposition,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
