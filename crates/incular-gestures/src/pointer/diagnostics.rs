//! Diagnostic projection of the tap branch of the shared recognizer.

use std::fmt::{self, Display, Formatter};

use incular_core::{Offset, PRIMARY_POINTER_BUTTON};

use super::PointerGestureRecognizer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapTrackingState {
    Ready,
    Possible,
    Defunct,
}

/// A snapshot of the live tap branch, never a second gesture state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapRecognizerDiagnostics {
    pub state: TapTrackingState,
    pub buttons: Option<u32>,
    pub tap_down_sent: bool,
    pub won_arena: bool,
    pub final_position: Option<Offset>,
    pub final_local_position: Option<Offset>,
}

impl PointerGestureRecognizer {
    /// Fields corresponding to Flutter's tap recognizer diagnostic output.
    #[must_use]
    pub fn tap_diagnostics(&self) -> TapRecognizerDiagnostics {
        let release = self.pending_tap_release.get();
        let active = self.down.is_some() || release.is_some();
        TapRecognizerDiagnostics {
            state: if !active {
                TapTrackingState::Ready
            } else if self.tap_rejected {
                TapTrackingState::Defunct
            } else {
                TapTrackingState::Possible
            },
            buttons: active.then_some(self.down_buttons.unwrap_or(PRIMARY_POINTER_BUTTON)),
            tap_down_sent: active
                && (self.tap_down_sent || release.is_some_and(|release| release.down_announced)),
            won_arena: active && self.tap_accepted,
            final_position: release.map(|release| release.up.global_position),
            final_local_position: release.map(|release| release.up.local_position),
        }
    }
}

impl Display for TapRecognizerDiagnostics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use crate::details::drag_diagnostics::DiagnosticOffset;

        let state = match self.state {
            TapTrackingState::Ready => "ready",
            TapTrackingState::Possible => "possible",
            TapTrackingState::Defunct => "defunct",
        };
        write!(f, "TapGestureRecognizer(state: {state}")?;
        if self.won_arena {
            f.write_str(", won arena")?;
        }
        if let Some(position) = self.final_position {
            write!(f, ", finalPosition: {}", DiagnosticOffset(position))?;
        }
        if let Some(local) = self.final_local_position
            && Some(local) != self.final_position
        {
            write!(f, ", finalLocalPosition: {}", DiagnosticOffset(local))?;
        }
        if let Some(buttons) = self.buttons {
            write!(f, ", button: {buttons}")?;
        }
        if self.tap_down_sent {
            f.write_str(", sent tap down")?;
        }
        f.write_str(")")
    }
}
