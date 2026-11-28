//! Ordered inspector properties for gesture callback detail values.

use incular_core::{
    DiagnosticsNode, DiagnosticsProperty, Offset, PointerDeviceKind, time::Instant,
};

use super::drag_diagnostics::{DiagnosticOffset, device_name};
use crate::{
    DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, ForcePressDetails,
    LongPressDownDetails, LongPressEndDetails, LongPressMoveUpdateDetails, LongPressStartDetails,
    ScaleEndDetails, ScaleStartDetails, ScaleUpdateDetails, SerialTapCancelDetails,
    SerialTapDownDetails, SerialTapUpDetails, TapDownDetails, TapDragDownDetails,
    TapDragEndDetails, TapDragStartDetails, TapDragUpDetails, TapDragUpdateDetails, TapUpDetails,
    Velocity,
};

/// Flutter-style inspector properties for a gesture callback's actual data.
/// This only runs when a caller asks for diagnostics.
pub trait GestureDetailDiagnostics {
    #[must_use]
    fn diagnostic_properties(&self) -> Vec<DiagnosticsNode>;
}

fn text(name: &str, value: Option<String>) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::new(name, value))
}

fn offset(name: &str, value: Offset) -> DiagnosticsNode {
    text(name, Some(DiagnosticOffset(value).to_string()))
}

fn velocity(value: Velocity) -> DiagnosticsNode {
    text("velocity", Some(value.to_string()))
}

fn kind(value: Option<PointerDeviceKind>) -> DiagnosticsNode {
    text("kind", value.map(|value| device_name(Some(value)).into()))
}

fn stamp(value: Option<Instant>) -> DiagnosticsNode {
    text("sourceTimeStamp", value.map(|time| format!("{time:?}")))
}

fn number(name: &str, value: Option<f32>) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::double(name, value.map(f64::from)))
}

fn integer(name: &str, value: i64) -> DiagnosticsNode {
    DiagnosticsNode::property(&DiagnosticsProperty::integer(name, Some(value)))
}

fn pointer_count(value: usize) -> DiagnosticsNode {
    integer("pointerCount", i64::try_from(value).unwrap_or(i64::MAX))
}

fn tap_count(value: u32) -> DiagnosticsNode {
    integer("consecutiveTapCount", i64::from(value))
}

fn positioned<const N: usize>(
    global: Offset,
    local: Offset,
    details: [DiagnosticsNode; N],
) -> Vec<DiagnosticsNode> {
    [
        offset("globalPosition", global),
        offset("localPosition", local),
    ]
    .into_iter()
    .chain(details)
    .collect()
}

macro_rules! properties {
    ($type:ty, |$value:ident| $result:expr) => {
        impl GestureDetailDiagnostics for $type {
            fn diagnostic_properties(&self) -> Vec<DiagnosticsNode> {
                let $value = self;
                $result
            }
        }
    };
}

macro_rules! positioned_properties {
    ($type:ty, |$value:ident| [$($property:expr),* $(,)?]) => {
        properties!($type, |$value| positioned(
            $value.global_position, $value.local_position, [$($property),*],
        ));
    };
}

positioned_properties!(DragDownDetails, |d| []);
positioned_properties!(DragStartDetails, |d| [stamp(d.source_time), kind(d.kind)]);
positioned_properties!(DragUpdateDetails, |d| [
    stamp(d.source_time),
    offset("delta", d.delta),
    number("primaryDelta", d.primary_delta),
]);
positioned_properties!(DragEndDetails, |d| [
    velocity(d.velocity),
    number("primaryVelocity", d.primary_velocity),
]);
positioned_properties!(ForcePressDetails, |d| [number(
    "pressure",
    Some(d.pressure)
)]);
positioned_properties!(LongPressDownDetails, |d| [kind(d.kind)]);
positioned_properties!(LongPressStartDetails, |d| []);
positioned_properties!(LongPressMoveUpdateDetails, |d| [
    offset("offsetFromOrigin", d.offset_from_origin),
    offset("localOffsetFromOrigin", d.local_offset_from_origin),
]);
positioned_properties!(LongPressEndDetails, |d| [velocity(d.velocity)]);
positioned_properties!(SerialTapDownDetails, |d| [
    kind(Some(d.kind)),
    integer("buttons", i64::from(d.buttons)),
    integer("count", i64::from(d.count)),
]);
properties!(SerialTapCancelDetails, |d| [integer(
    "count",
    i64::from(d.count)
)]
.into());
positioned_properties!(SerialTapUpDetails, |d| [
    kind(Some(d.kind)),
    integer("count", i64::from(d.count)),
]);
properties!(ScaleStartDetails, |d| vec![
    offset("focalPoint", d.focal_point),
    offset("localFocalPoint", d.local_focal_point),
    pointer_count(d.pointer_count),
    stamp(d.source_time),
]);
properties!(ScaleUpdateDetails, |d| vec![
    offset("focalPointDelta", d.focal_point_delta),
    offset("focalPoint", d.focal_point),
    offset("localFocalPoint", d.local_focal_point),
    number("scale", Some(d.scale)),
    number("horizontalScale", Some(d.horizontal_scale)),
    number("verticalScale", Some(d.vertical_scale)),
    number("rotation", Some(d.rotation)),
    pointer_count(d.pointer_count),
    stamp(d.source_time),
]);
properties!(ScaleEndDetails, |d| vec![
    velocity(d.velocity),
    number("scaleVelocity", Some(d.scale_velocity)),
    pointer_count(d.pointer_count),
]);
positioned_properties!(TapDownDetails, |d| [kind(Some(d.kind))]);
positioned_properties!(TapUpDetails, |d| [kind(Some(d.kind))]);
positioned_properties!(TapDragDownDetails, |d| [
    kind(Some(d.kind)),
    tap_count(d.consecutive_tap_count)
]);
positioned_properties!(TapDragUpDetails, |d| [
    kind(Some(d.kind)),
    tap_count(d.consecutive_tap_count)
]);
positioned_properties!(TapDragStartDetails, |d| [
    stamp(d.source_time),
    kind(Some(d.kind)),
    tap_count(d.consecutive_tap_count),
]);
positioned_properties!(TapDragUpdateDetails, |d| [
    stamp(d.source_time),
    offset("delta", d.delta),
    number("primaryDelta", d.primary_delta),
    kind(Some(d.kind)),
    offset("offsetFromOrigin", d.offset_from_origin),
    offset("localOffsetFromOrigin", d.local_offset_from_origin),
    tap_count(d.consecutive_tap_count),
]);
positioned_properties!(TapDragEndDetails, |d| [
    velocity(d.velocity),
    number("primaryVelocity", d.primary_velocity),
    tap_count(d.consecutive_tap_count),
]);
