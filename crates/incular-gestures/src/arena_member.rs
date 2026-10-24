//! How the recognizers ported from Flutter's `GestureRecognizer` protocol
//! (`addPointer`, `handleEvent`, `acceptGesture`, `rejectGesture`) reach a
//! gesture arena.
//!
//! By default a recognizer is the only member of each pointer's arena, and
//! [`ArenaLink`] reproduces Flutter's `GestureArenaManager` for that case:
//! a rejection takes effect at once, and the arena resolves in the member's
//! favor when it closes after the press. With an external arena, each
//! resolution goes to the caller's resolver, which answers synchronously as
//! Flutter's arena does, or decides later through `accept_gesture` and
//! `reject_gesture`.

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use incular_core::{Offset, PointerPhase, time::Instant};

use crate::{GestureDisposition, PointerDeviceKind, RawPointerEvent};

/// Flutter's `computeHitSlop`.
pub(crate) fn compute_hit_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => crate::constants::PRECISE_POINTER_HIT_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => crate::constants::TOUCH_SLOP,
    }
}

/// Flutter's `computePanSlop`.
pub(crate) fn compute_pan_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => crate::constants::PRECISE_POINTER_PAN_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => crate::constants::PAN_SLOP,
    }
}

pub(crate) fn distance(offset: Offset) -> f32 {
    offset.x.hypot(offset.y)
}

/// Decides a recognizer's resolution in an external arena: the disposition
/// the arena gives the recognizer now, or `None` when it decides later.
pub type ArenaResolver = Box<dyn FnMut(u64, GestureDisposition) -> Option<GestureDisposition>>;

/// One recognizer's view of its gesture arenas.
#[derive(Default)]
pub(crate) struct ArenaLink {
    external: Option<ArenaResolver>,
    /// The pointers whose sole-member arena is still open for its press.
    /// It closes after the press and the sole member wins by default, so no
    /// arena outlives its press.
    open: HashSet<u64>,
    /// The last (global, local) position of each pointer, from which the
    /// events' deltas are derived.
    positions: HashMap<u64, (Offset, Offset)>,
}

impl fmt::Debug for ArenaLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArenaLink")
            .field("external", &self.external.is_some())
            .field("open", &self.open)
            .finish_non_exhaustive()
    }
}

impl ArenaLink {
    pub(crate) fn set_external(&mut self, resolver: ArenaResolver) {
        self.external = Some(resolver);
        self.open.clear();
    }

    /// `gestureArena.add`.
    pub(crate) fn add(&mut self, pointer: u64) {
        if self.external.is_none() {
            self.open.insert(pointer);
        }
    }

    /// `GestureArenaEntry.resolve`: the disposition to apply to the member
    /// now, if any. A rejection applies at once; an acceptance while the
    /// arena is open makes the member its eager winner, which the close
    /// decides.
    #[must_use]
    pub(crate) fn resolve(
        &mut self,
        pointer: u64,
        disposition: GestureDisposition,
    ) -> Option<GestureDisposition> {
        if let Some(resolver) = self.external.as_mut() {
            return resolver(pointer, disposition);
        }
        match disposition {
            GestureDisposition::Rejected => self
                .open
                .remove(&pointer)
                .then_some(GestureDisposition::Rejected),
            GestureDisposition::Accepted
            | GestureDisposition::Pending
            | GestureDisposition::Cancelled => None,
        }
    }

    /// `gestureArena.close` after the press: the sole member wins.
    #[must_use]
    pub(crate) fn close(&mut self, pointer: u64) -> Option<GestureDisposition> {
        self.open
            .remove(&pointer)
            .then_some(GestureDisposition::Accepted)
    }

    /// Fills in the event's `delta` and `localDelta` from the pointer's
    /// previous event.
    pub(crate) fn with_deltas(&mut self, mut event: RawPointerEvent) -> RawPointerEvent {
        let current = (event.position, event.local_position);
        let previous = match event.phase {
            PointerPhase::Down => None,
            _ => self.positions.get(&event.pointer).copied(),
        };
        if let Some((global, local)) = previous {
            event.delta = current.0 - global;
            event.local_delta = current.1 - local;
        } else {
            event.delta = Offset::ZERO;
            event.local_delta = Offset::ZERO;
        }
        match event.phase {
            PointerPhase::Up | PointerPhase::Cancel => {
                self.positions.remove(&event.pointer);
            }
            PointerPhase::Down | PointerPhase::Move => {
                self.positions.insert(event.pointer, current);
            }
            PointerPhase::Enter | PointerPhase::Exit => {}
        }
        event
    }
}

/// The recognizer side of the arena protocol, implemented by
/// [`arena_recognizer!`].
pub(crate) trait ArenaMember {
    fn link(&mut self) -> &mut ArenaLink;
    fn member_add_pointer(&mut self, event: RawPointerEvent);
    fn member_handle_event(&mut self, event: RawPointerEvent);
    fn member_accept(&mut self, pointer: u64);
    fn member_reject(&mut self, pointer: u64);
    fn member_poll(&mut self, now: Instant);

    /// Applies a disposition the arena has decided.
    fn apply(&mut self, pointer: u64, disposition: Option<GestureDisposition>) {
        match disposition {
            Some(GestureDisposition::Accepted) => self.member_accept(pointer),
            Some(GestureDisposition::Rejected) => self.member_reject(pointer),
            _ => {}
        }
    }

    /// `GestureArenaEntry.resolve` for one pointer.
    fn settle(&mut self, pointer: u64, disposition: GestureDisposition) {
        let decided = self.link().resolve(pointer, disposition);
        self.apply(pointer, decided);
    }

    /// One event through `GestureBinding` with this recognizer alone in the
    /// arena: due timers fire, and a press adds the pointer and is routed
    /// before its arena closes. The member wins at the close, so no arena
    /// is left for a release to sweep.
    fn drive(&mut self, event: RawPointerEvent) {
        self.member_poll(event.time);
        match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => {}
            PointerPhase::Down => {
                self.member_add_pointer(event);
                self.member_handle_event(event);
                let decided = self.link().close(event.pointer);
                self.apply(event.pointer, decided);
            }
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => {
                self.member_handle_event(event);
            }
        }
    }
}

/// Implements [`ArenaMember`] and the shared public arena methods for a
/// recognizer with inherent `add_pointer`, `handle_event`,
/// `accept_gesture`, `reject_gesture` and `poll` methods and a `link` field.
macro_rules! arena_recognizer {
    ($recognizer:ty) => {
        impl $crate::arena_member::ArenaMember for $recognizer {
            fn link(&mut self) -> &mut $crate::arena_member::ArenaLink {
                &mut self.link
            }
            fn member_add_pointer(&mut self, event: $crate::RawPointerEvent) {
                self.add_pointer(event);
            }
            fn member_handle_event(&mut self, event: $crate::RawPointerEvent) {
                self.handle_event(event);
            }
            fn member_accept(&mut self, pointer: u64) {
                self.accept_gesture(pointer);
            }
            fn member_reject(&mut self, pointer: u64) {
                self.reject_gesture(pointer);
            }
            fn member_poll(&mut self, now: incular_core::time::Instant) {
                self.poll(now);
            }
        }

        impl $recognizer {
            /// Leaves arbitration to the caller. Each resolution the
            /// recognizer requests (Flutter `GestureArenaEntry.resolve`) goes
            /// to `resolver`, which returns the disposition the arena gives
            /// this recognizer at once, as Flutter's arena does when it is
            /// already closed, or `None` to report it later through
            /// `accept_gesture` or `reject_gesture`. The caller adds presses
            /// with `add_pointer` and routes every event, the press
            /// included, to `handle_event`. Without this the recognizer is
            /// the only member of each pointer's arena.
            #[must_use]
            pub fn with_external_arena(
                mut self,
                resolver: impl FnMut(
                    u64,
                    $crate::GestureDisposition,
                ) -> Option<$crate::GestureDisposition>
                + 'static,
            ) -> Self {
                self.link.set_external(Box::new(resolver));
                self
            }

            /// Handles one event with this recognizer alone in its gesture
            /// arena, as Flutter's `GestureBinding` would route it: due
            /// timers fire first, and a press is added and routed before
            /// its arena closes, which the recognizer then wins.
            pub fn handle_raw(&mut self, event: $crate::RawPointerEvent) {
                $crate::arena_member::ArenaMember::drive(self, event);
            }
        }
    };
}

pub(crate) use arena_recognizer;
