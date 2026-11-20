//! Host-owned pacing of presentation retries.

use incular_wgpu::FrameOutcome;
/// Delay before the next attempt after this many consecutive retryable
/// skips. The first retry is immediate: a single transient timeout or an
/// inline surface recovery usually succeeds at once. Further consecutive
/// failures back off exponentially to a quarter-second cap, so repeated
/// failures stay responsive without spinning full-frame work.
fn retry_delay(consecutive_skips: u32) -> std::time::Duration {
    match consecutive_skips {
        0 | 1 => std::time::Duration::ZERO,
        2 => std::time::Duration::from_millis(16),
        3 => std::time::Duration::from_millis(32),
        4 => std::time::Duration::from_millis(64),
        5 => std::time::Duration::from_millis(128),
        _ => std::time::Duration::from_millis(250),
    }
}

/// Mutually exclusive scheduling states for one window's presentation
/// retries. There is deliberately no separate "armed" boolean: the state
/// itself says whether a retry is owed, queued, or parked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RetryState {
    /// Nothing owed; demand decides.
    #[default]
    Idle,
    /// A retry is owed at the deadline.
    Waiting {
        deadline: incular_core::time::Instant,
    },
    /// A redraw was dispatched for the owed retry and the attempt has not
    /// run yet. Holds no deadline: repeated maintenance must neither
    /// re-dispatch nor wake the loop for it.
    Dispatched,
    /// Parked without a deadline (unconfigured, occluded). Only an
    /// explicit recovery event leaves this state.
    Dormant,
}

/// Host-owned pacing for presentation retries: one per window (normal and
/// transient alike), driven by settled frame outcomes and explicit clock
/// readings. The event loop owns deadlines and window lifetime; the
/// runtime and renderer hold no retry state. All decisions are pure over
/// the injected `now`, so headless tests drive exact dispatch times with
/// a fake clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationRetry {
    consecutive_skips: u32,
    state: RetryState,
}

impl PresentationRetry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Settles one frame outcome at `now`: the attempt ran, so any queued
    /// or waiting retry resolves. Presentation resets to idle; retryable
    /// skips (timeouts, inline recoveries) arm a deadline — immediate for
    /// the first, then backing off; dormant reasons (unconfigured,
    /// occluded) park without deadlines until an explicit recovery event.
    /// Skips never settle waiters: pending work stays eligible for the
    /// armed attempt.
    pub fn note_outcome(&mut self, outcome: &FrameOutcome, now: incular_core::time::Instant) {
        match outcome {
            FrameOutcome::Presented(_) => {
                *self = Self::new();
            }
            FrameOutcome::Skipped(reason) => {
                if reason.should_request_retry() {
                    self.consecutive_skips = self.consecutive_skips.saturating_add(1);
                    self.state = RetryState::Waiting {
                        deadline: now + retry_delay(self.consecutive_skips),
                    };
                } else {
                    self.consecutive_skips = 0;
                    self.state = RetryState::Dormant;
                }
            }
        }
    }

    /// Recovery events that make attempts useful again — non-zero resize,
    /// unocclusion: dormant, waiting, or dispatched windows become
    /// immediately eligible. New application demand deliberately does not
    /// reset: gated demand waits for the armed deadline instead of
    /// bypassing it.
    pub fn note_recovered(&mut self) {
        *self = Self::new();
    }

    /// Settles a completed, non-retryable failure: clears obsolete retry
    /// debt and backoff so no stale automatic retry survives the failure,
    /// while later application demand stays eligible. This resets to idle,
    /// not dormant — dormancy would park the window until an explicit
    /// resize/unocclusion recovery event, blocking corrected content from
    /// rendering on plain demand.
    pub fn note_failed(&mut self) {
        *self = Self::new();
    }

    /// Armed retry deadline, if any. `Some` exactly while a retry is
    /// owed and undispatched; idle, dispatched, and dormant windows
    /// report `None`.
    #[must_use]
    pub const fn retry_at(&self) -> Option<incular_core::time::Instant> {
        match self.state {
            RetryState::Waiting { deadline } => Some(deadline),
            RetryState::Idle | RetryState::Dispatched | RetryState::Dormant => None,
        }
    }

    /// Whether a requested attempt may run now: idle always, an owed retry
    /// once due, a dispatched redraw already in flight, never while
    /// dormant. Gates demand scheduling and redraw handling. Visibility is
    /// enforced separately by the host's own checks plus [`Self::poll`];
    /// this gate only answers for the retry lifecycle itself.
    #[must_use]
    pub fn attempt_due(&self, now: incular_core::time::Instant) -> bool {
        match self.state {
            RetryState::Idle | RetryState::Dispatched => true,
            RetryState::Dormant => false,
            RetryState::Waiting { deadline } => now >= deadline,
        }
    }

    /// Advances scheduling at `now` for a window the host reports as
    /// runnable (visible and live). This is the one authoritative decision
    /// for both retry dispatch and wake deadlines: it returns whether the
    /// host must request a redraw now, plus the next wake deadline the
    /// loop should wait on, if any.
    ///
    /// - `Idle` / `Dormant`: nothing owed — `(false, None)`. Dormant stays
    ///   parked even past any clock reading; only [`Self::note_recovered`]
    ///   wakes it.
    /// - `Waiting`: hidden windows contribute neither a redraw nor a
    ///   deadline — the owed retry and its backoff progress are preserved
    ///   silently instead of waking the loop for nothing. Visible windows
    ///   dispatch once due (transitioning to `Dispatched`, consuming the
    ///   deadline) and otherwise contribute their deadline.
    /// - `Dispatched`: the redraw is already queued — `(false, None)`,
    ///   however often maintenance polls. If the window stops being
    ///   runnable before the attempt runs, the retry falls back to
    ///   `Waiting` due immediately, so restoring runnability dispatches
    ///   one prompt attempt; the attempt's outcome then settles normally.
    pub fn poll(
        &mut self,
        now: incular_core::time::Instant,
        runnable: bool,
    ) -> (bool, Option<incular_core::time::Instant>) {
        match self.state {
            RetryState::Idle | RetryState::Dormant => (false, None),
            RetryState::Waiting { deadline } => {
                if !runnable {
                    (false, None)
                } else if now >= deadline {
                    self.state = RetryState::Dispatched;
                    (true, None)
                } else {
                    (false, Some(deadline))
                }
            }
            RetryState::Dispatched => {
                if !runnable {
                    self.state = RetryState::Waiting { deadline: now };
                }
                (false, None)
            }
        }
    }
}
