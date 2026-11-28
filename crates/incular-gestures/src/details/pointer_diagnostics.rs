//! Single-line position and sample descriptions for routed pointer events.

use std::fmt::{self, Display, Formatter};

use incular_core::{Offset, PointerPhase};

use super::{
    RawPointerEvent,
    drag_diagnostics::{DiagnosticOffset, device_name},
};

impl RawPointerEvent {
    fn diagnostic_name(&self) -> &'static str {
        if self.scale_signal.is_some() {
            return "PointerScaleEvent";
        }
        if self.scroll_delta != Offset::ZERO {
            return "PointerScrollEvent";
        }
        match self.phase {
            PointerPhase::Down => "PointerDownEvent",
            PointerPhase::Up => "PointerUpEvent",
            PointerPhase::Cancel => "PointerCancelEvent",
            PointerPhase::Enter => "PointerEnterEvent",
            PointerPhase::Exit => "PointerExitEvent",
            PointerPhase::Move if self.down => "PointerMoveEvent",
            PointerPhase::Move => "PointerHoverEvent",
        }
    }

    /// Describes the recorded sample in one line, including the device,
    /// contact state and any distinct local coordinates.
    #[must_use]
    pub fn to_string_full(&self) -> String {
        FullPointerEvent(self).to_string()
    }
}

impl Display for RawPointerEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(position: {})",
            self.diagnostic_name(),
            DiagnosticOffset(self.position)
        )
    }
}

struct FullPointerEvent<'a>(&'a RawPointerEvent);

impl Display for FullPointerEvent<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let event = self.0;
        write!(
            f,
            "{}(position: {}",
            event.diagnostic_name(),
            DiagnosticOffset(event.position),
        )?;
        if event.local_position != event.position {
            write!(
                f,
                ", localPosition: {}",
                DiagnosticOffset(event.local_position)
            )?;
        }
        if event.delta != Offset::ZERO {
            write!(f, ", delta: {}", DiagnosticOffset(event.delta))?;
        }
        if event.local_delta != event.delta {
            write!(f, ", localDelta: {}", DiagnosticOffset(event.local_delta))?;
        }
        write!(
            f,
            ", timeStamp: {:?}, pointer: {}, kind: {}",
            event.time,
            event.pointer,
            device_name(Some(event.kind)),
        )?;
        if event.device != 0 {
            write!(f, ", device: {}", event.device)?;
        }
        if event.buttons != 0 {
            write!(f, ", buttons: {}", event.buttons)?;
        }
        if let Some(button) = event.button {
            write!(f, ", changedButton: {button}")?;
        }
        write!(f, ", down: {}", event.down)?;
        if event.sample != Default::default() {
            write!(f, ", sample: {:?}", event.sample)?;
        }
        if event.scroll_delta != Offset::ZERO {
            write!(f, ", scrollDelta: {}", DiagnosticOffset(event.scroll_delta))?;
            if event.local_scroll_delta != event.scroll_delta {
                write!(
                    f,
                    ", localScrollDelta: {}",
                    DiagnosticOffset(event.local_scroll_delta)
                )?;
            }
        }
        if let Some(scale) = event.scale_signal {
            write!(f, ", scale: {scale}")?;
        }
        if event.synthesized {
            f.write_str(", synthesized")?;
        }
        f.write_str(")")
    }
}
