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
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};

use incular_core::{Offset, PointerPhase, time::Instant};

use crate::{
    GestureAction, GestureDecision, GestureDisposition, GestureRecognizer, PointerDeviceKind,
    PointerEvent, RawPointerEvent,
};

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

        impl $crate::arena_member::ArenaRecognizer for $recognizer {
            fn set_external_arena(&mut self, resolver: $crate::ArenaResolver) {
                self.link.set_external(resolver);
            }
            fn add_pointer(&mut self, event: $crate::RawPointerEvent) {
                <$recognizer>::add_pointer(self, event);
            }
            fn handle_event(&mut self, event: $crate::RawPointerEvent) {
                <$recognizer>::handle_event(self, event);
            }
            fn accept_gesture(&mut self, pointer: u64) {
                <$recognizer>::accept_gesture(self, pointer);
            }
            fn reject_gesture(&mut self, pointer: u64) {
                <$recognizer>::reject_gesture(self, pointer);
            }
            fn poll(&mut self, now: incular_core::time::Instant) {
                <$recognizer>::poll(self, now);
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

/// The arena protocol of the recognizers ported from Flutter's
/// `GestureRecognizer` (`addPointer`, `handleEvent`, `acceptGesture`,
/// `rejectGesture`), through which [`ArenaGestureRecognizer`] joins them to
/// a `RawGestureDetector`.
pub trait ArenaRecognizer: 'static {
    /// Hands every resolution the recognizer requests to `resolver`.
    fn set_external_arena(&mut self, resolver: ArenaResolver);
    fn add_pointer(&mut self, event: RawPointerEvent);
    fn handle_event(&mut self, event: RawPointerEvent);
    fn accept_gesture(&mut self, pointer: u64);
    fn reject_gesture(&mut self, pointer: u64);
    /// Fires the recognizer's timers that are due at `now`.
    fn poll(&mut self, now: Instant);
}

type Requests = Rc<RefCell<Vec<(u64, GestureDisposition)>>>;

/// An [`ArenaRecognizer`] as a [`GestureRecognizer`], the type a
/// `RawGestureDetector` factory builds.
///
/// The detector's arena stands in for Flutter's: the recognizer's
/// resolutions become its decisions, and a press neither side has resolved
/// by its release goes to the first member still pending, as Flutter's
/// arena sweep does. Once accepted, a pointer's later events reach the
/// recognizer directly.
pub struct ArenaGestureRecognizer<R: ArenaRecognizer> {
    recognizer: R,
    requests: Requests,
    /// Each tracked pointer and whether the detector's arena accepted it.
    pointers: HashMap<u64, bool>,
    /// The pointer and phase of the event observed last; a cancellation
    /// from the detector concerns that pointer.
    last: Option<(u64, PointerPhase)>,
}

impl<R: ArenaRecognizer> ArenaGestureRecognizer<R> {
    #[must_use]
    pub fn new(mut recognizer: R) -> Self {
        let requests = Requests::default();
        let sink = requests.clone();
        recognizer.set_external_arena(Box::new(move |pointer, disposition| {
            sink.borrow_mut().push((pointer, disposition));
            None
        }));
        Self {
            recognizer,
            requests,
            pointers: HashMap::new(),
            last: None,
        }
    }

    #[must_use]
    pub fn recognizer(&self) -> &R {
        &self.recognizer
    }

    pub fn recognizer_mut(&mut self) -> &mut R {
        &mut self.recognizer
    }

    /// The decision for `pointer` after the recognizer saw `phase`.
    fn decide(&mut self, pointer: u64, phase: PointerPhase) -> GestureDecision {
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        let mut decision = GestureDecision::Pending;
        for (requested, disposition) in requests {
            let Some(accepted) = self.pointers.get(&requested).copied() else {
                continue;
            };
            match disposition {
                // This stream's undecided pointer: the arena decides.
                GestureDisposition::Accepted if !accepted && requested == pointer => {
                    decision = GestureDecision::Accept(GestureAction::ArenaAccepted { pointer });
                }
                GestureDisposition::Rejected if !accepted && requested == pointer => {
                    self.pointers.remove(&requested);
                    self.recognizer.reject_gesture(requested);
                    return GestureDecision::Reject;
                }
                // A rejection of an accepted pointer, or of another
                // stream's, needs no arena and applies at once.
                GestureDisposition::Rejected => {
                    self.pointers.remove(&requested);
                    self.recognizer.reject_gesture(requested);
                }
                GestureDisposition::Accepted
                | GestureDisposition::Pending
                | GestureDisposition::Cancelled => {}
            }
        }
        if decision == GestureDecision::Pending
            && phase == PointerPhase::Up
            && self.pointers.get(&pointer) == Some(&false)
        {
            // Flutter's sweep: the first member still pending wins.
            decision = GestureDecision::Accept(GestureAction::ArenaAccepted { pointer });
        }
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel)
            && self.pointers.get(&pointer) == Some(&true)
        {
            self.pointers.remove(&pointer);
        }
        decision
    }
}

impl<R: ArenaRecognizer> GestureRecognizer for ArenaGestureRecognizer<R> {
    fn observe(&mut self, event: PointerEvent) -> GestureDecision {
        self.observe_raw(RawPointerEvent::from(event))
    }

    fn observe_raw(&mut self, event: RawPointerEvent) -> GestureDecision {
        self.recognizer.poll(event.time);
        match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => return GestureDecision::Pending,
            PointerPhase::Down => {
                self.pointers.insert(event.pointer, false);
                self.recognizer.add_pointer(event);
            }
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => {
                if !self.pointers.contains_key(&event.pointer) {
                    return GestureDecision::Pending;
                }
            }
        }
        self.last = Some((event.pointer, event.phase));
        self.recognizer.handle_event(event);
        self.decide(event.pointer, event.phase)
    }

    fn dispatch(&mut self, action: GestureAction) {
        if let GestureAction::ArenaAccepted { pointer } = action
            && let Some(accepted) = self.pointers.get_mut(&pointer)
        {
            *accepted = true;
            self.recognizer.accept_gesture(pointer);
            // Resolutions made while accepting apply to the recognizer.
            let _ = self.decide(pointer, PointerPhase::Move);
            // Accepted on its release, the pointer has no events left.
            if matches!(
                self.last,
                Some((last, PointerPhase::Up | PointerPhase::Cancel)) if last == pointer
            ) {
                self.pointers.remove(&pointer);
            }
        }
    }

    fn cancel(&mut self) {
        if let Some((pointer, _)) = self.last
            && self.pointers.get(&pointer) == Some(&false)
        {
            self.pointers.remove(&pointer);
            self.recognizer.reject_gesture(pointer);
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
