use std::fmt::{self, Display, Formatter};

use incular_core::{Offset, debug_double::format_double_fixed, debug_format_double, time::Instant};

use super::{
    DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, PointerDeviceKind,
};

pub(crate) struct DiagnosticOffset(pub(crate) Offset);

impl Display for DiagnosticOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Offset({}, {})",
            format_double_fixed(f64::from(self.0.x), 1),
            format_double_fixed(f64::from(self.0.y), 1)
        )
    }
}

struct DiagnosticPosition(Offset, Offset);

impl Display for DiagnosticPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "globalPosition: {}, localPosition: {}",
            DiagnosticOffset(self.0),
            DiagnosticOffset(self.1)
        )
    }
}

struct DiagnosticSourceTime(Option<Instant>);

impl Display for DiagnosticSourceTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(instant) => write!(f, "{instant:?}"),
            None => f.write_str("null"),
        }
    }
}

pub(super) fn device_name(kind: Option<PointerDeviceKind>) -> &'static str {
    match kind {
        None => "null",
        Some(PointerDeviceKind::Touch) => "touch",
        Some(PointerDeviceKind::Mouse) => "mouse",
        Some(PointerDeviceKind::Stylus) => "stylus",
        Some(PointerDeviceKind::InvertedStylus) => "invertedStylus",
        Some(PointerDeviceKind::Trackpad) => "trackpad",
        Some(PointerDeviceKind::Unknown) => "unknown",
    }
}

impl Display for DragDownDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DragDownDetails({})",
            DiagnosticPosition(self.global_position, self.local_position)
        )
    }
}

impl Display for DragStartDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DragStartDetails({}, sourceTimeStamp: {}, kind: {})",
            DiagnosticPosition(self.global_position, self.local_position),
            DiagnosticSourceTime(self.source_time),
            device_name(self.kind)
        )
    }
}

impl Display for DragUpdateDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DragUpdateDetails({}, sourceTimeStamp: {}, delta: {}, primaryDelta: {})",
            DiagnosticPosition(self.global_position, self.local_position),
            DiagnosticSourceTime(self.source_time),
            DiagnosticOffset(self.delta),
            debug_format_double(self.primary_delta.map(f64::from))
        )
    }
}

impl Display for DragEndDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DragEndDetails({}, velocity: {}, primaryVelocity: {})",
            DiagnosticPosition(self.global_position, self.local_position),
            self.velocity,
            debug_format_double(self.primary_velocity.map(f64::from))
        )
    }
}

macro_rules! debug_as_display {
    ($($ty:ty),+ $(,)?) => {$(
        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Display::fmt(self, f)
            }
        }
    )+};
}

debug_as_display!(
    DragDownDetails,
    DragStartDetails,
    DragUpdateDetails,
    DragEndDetails
);
