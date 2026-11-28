use incular_core::{Offset, PointerDeviceKind, time::Instant};
use incular_gestures::{
    DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, ForcePressDetails,
    GestureDetailDiagnostics, LongPressDownDetails, LongPressEndDetails,
    LongPressMoveUpdateDetails, LongPressStartDetails, ScaleEndDetails, ScaleStartDetails,
    ScaleUpdateDetails, SerialTapCancelDetails, SerialTapDownDetails, SerialTapUpDetails,
    TapDownDetails, TapDragDownDetails, TapDragEndDetails, TapDragStartDetails, TapDragUpDetails,
    TapDragUpdateDetails, TapUpDetails, Velocity,
};

const ZERO: Offset = Offset::ZERO;
const UNKNOWN: PointerDeviceKind = PointerDeviceKind::Unknown;
const PRESS_NONE: &[&str] = &["kind: null"];
const PRESS_UNKNOWN: &[&str] = &["kind: unknown"];
const VELOCITY: &str = "velocity: Velocity(0.0, 0.0)";

fn properties(details: impl GestureDetailDiagnostics) -> Vec<String> {
    details
        .diagnostic_properties()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn positioned(details: impl GestureDetailDiagnostics, rest: &[&str]) {
    let leading = [
        "globalPosition: Offset(0.0, 0.0)",
        "localPosition: Offset(0.0, 0.0)",
    ];
    assert_eq!(
        properties(details),
        leading
            .into_iter()
            .chain(rest.iter().copied())
            .collect::<Vec<_>>()
    );
}

#[test]
fn drag_force_and_long_press_defaults_keep_their_expected_property_order() {
    positioned(DragDownDetails::default(), &[]);
    positioned(
        DragStartDetails::default(),
        &["sourceTimeStamp: null", "kind: null"],
    );
    positioned(
        DragUpdateDetails {
            source_time: None,
            delta: ZERO,
            primary_delta: None,
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
        },
        &[
            "sourceTimeStamp: null",
            "delta: Offset(0.0, 0.0)",
            "primaryDelta: null",
        ],
    );
    positioned(
        DragEndDetails::default(),
        &[VELOCITY, "primaryVelocity: null"],
    );
    positioned(
        ForcePressDetails {
            global_position: ZERO,
            local_position: ZERO,
            pressure: 1.,
        },
        &["pressure: 1.0"],
    );
    positioned(LongPressDownDetails::default(), PRESS_NONE);
    positioned(
        LongPressStartDetails {
            global_position: ZERO,
            local_position: ZERO,
        },
        &[],
    );
    positioned(
        LongPressMoveUpdateDetails {
            global_position: ZERO,
            local_position: ZERO,
            offset_from_origin: ZERO,
            local_offset_from_origin: ZERO,
        },
        &[
            "offsetFromOrigin: Offset(0.0, 0.0)",
            "localOffsetFromOrigin: Offset(0.0, 0.0)",
        ],
    );
    positioned(
        LongPressEndDetails {
            global_position: ZERO,
            local_position: ZERO,
            velocity: Velocity::ZERO,
        },
        &[VELOCITY],
    );
}

#[test]
fn default_scale_details_are_identity_with_zero_contacts_not_two_fingers() {
    assert_eq!(
        properties(ScaleStartDetails::default()),
        [
            "focalPoint: Offset(0.0, 0.0)",
            "localFocalPoint: Offset(0.0, 0.0)",
            "pointerCount: 0",
            "sourceTimeStamp: null",
        ]
    );
    assert_eq!(
        properties(ScaleUpdateDetails::default()),
        [
            "focalPointDelta: Offset(0.0, 0.0)",
            "focalPoint: Offset(0.0, 0.0)",
            "localFocalPoint: Offset(0.0, 0.0)",
            "scale: 1.0",
            "horizontalScale: 1.0",
            "verticalScale: 1.0",
            "rotation: 0.0",
            "pointerCount: 0",
            "sourceTimeStamp: null",
        ]
    );
    assert_eq!(
        properties(ScaleEndDetails::default()),
        [VELOCITY, "scaleVelocity: 0.0", "pointerCount: 0"]
    );
    assert_eq!(
        ScaleUpdateDetails::new(ZERO, 2.).pointer_count,
        2,
        "explicit native two-contact shortcut stays available"
    );
}

#[test]
fn tap_families_preserve_count_device_and_axis_metadata_in_order() {
    positioned(
        SerialTapDownDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            buttons: 0,
            count: 1,
        },
        &["kind: unknown", "buttons: 0", "count: 1"],
    );
    assert_eq!(
        properties(SerialTapCancelDetails { count: 1 }),
        ["count: 1"]
    );
    positioned(
        SerialTapUpDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            count: 1,
        },
        &["kind: unknown", "count: 1"],
    );
    positioned(
        TapDownDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
        },
        PRESS_UNKNOWN,
    );
    positioned(
        TapUpDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
        },
        PRESS_UNKNOWN,
    );
    positioned(
        TapDragDownDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            consecutive_tap_count: 1,
        },
        &["kind: unknown", "consecutiveTapCount: 1"],
    );
    positioned(
        TapDragUpDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            consecutive_tap_count: 1,
        },
        &["kind: unknown", "consecutiveTapCount: 1"],
    );
    positioned(
        TapDragStartDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            source_time: None,
            consecutive_tap_count: 1,
        },
        &[
            "sourceTimeStamp: null",
            "kind: unknown",
            "consecutiveTapCount: 1",
        ],
    );
    positioned(
        TapDragUpdateDetails {
            global_position: ZERO,
            local_position: ZERO,
            kind: UNKNOWN,
            source_time: None,
            delta: ZERO,
            primary_delta: None,
            offset_from_origin: ZERO,
            local_offset_from_origin: ZERO,
            consecutive_tap_count: 1,
        },
        &[
            "sourceTimeStamp: null",
            "delta: Offset(0.0, 0.0)",
            "primaryDelta: null",
            "kind: unknown",
            "offsetFromOrigin: Offset(0.0, 0.0)",
            "localOffsetFromOrigin: Offset(0.0, 0.0)",
            "consecutiveTapCount: 1",
        ],
    );
    positioned(
        TapDragEndDetails {
            global_position: ZERO,
            local_position: ZERO,
            velocity: Velocity::ZERO,
            primary_velocity: None,
            consecutive_tap_count: 1,
        },
        &[VELOCITY, "primaryVelocity: null", "consecutiveTapCount: 1"],
    );
}

#[test]
fn callback_properties_read_nondefault_coordinates_pressure_and_primary_velocity() {
    assert_eq!(
        properties(ForcePressDetails {
            global_position: Offset::new(5.25, -2.),
            local_position: Offset::new(1., 6.),
            pressure: 0.75,
        }),
        [
            "globalPosition: Offset(5.3, -2.0)",
            "localPosition: Offset(1.0, 6.0)",
            "pressure: 0.8"
        ]
    );
    assert_eq!(
        properties(TapDragEndDetails {
            global_position: Offset::new(13., 8.),
            local_position: Offset::new(2., 3.),
            velocity: Velocity::new(Offset::new(-8., 0.)),
            primary_velocity: Some(-8.),
            consecutive_tap_count: 3,
        }),
        [
            "globalPosition: Offset(13.0, 8.0)",
            "localPosition: Offset(2.0, 3.0)",
            "velocity: Velocity(-8.0, 0.0)",
            "primaryVelocity: -8.0",
            "consecutiveTapCount: 3"
        ]
    );
    let at = Instant::now();
    let stamp = properties(DragStartDetails {
        global_position: ZERO,
        local_position: ZERO,
        source_time: Some(at),
        kind: Some(PointerDeviceKind::Stylus),
    });
    assert_eq!(stamp[2], format!("sourceTimeStamp: {at:?}"));
    assert_eq!(stamp[3], "kind: stylus");
}
