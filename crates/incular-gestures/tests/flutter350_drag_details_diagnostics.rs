use incular_core::{Offset, PointerDeviceKind};
use incular_gestures::{
    DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails, Velocity,
};

#[test]
fn default_drag_details_have_ordered_one_line_descriptions() {
    let position = "globalPosition: Offset(0.0, 0.0), localPosition: Offset(0.0, 0.0)";
    assert_eq!(
        DragDownDetails::default().to_string(),
        format!("DragDownDetails({position})")
    );
    assert_eq!(
        DragStartDetails::default().to_string(),
        format!("DragStartDetails({position}, sourceTimeStamp: null, kind: null)")
    );
    assert_eq!(
        DragUpdateDetails {
            source_time: None,
            delta: Offset::ZERO,
            primary_delta: None,
            global_position: Offset::ZERO,
            local_position: Offset::ZERO,
            kind: PointerDeviceKind::Unknown,
        }
        .to_string(),
        format!(
            "DragUpdateDetails({position}, sourceTimeStamp: null, delta: Offset(0.0, 0.0), primaryDelta: null)"
        )
    );
    assert_eq!(
        DragEndDetails::default().to_string(),
        format!("DragEndDetails({position}, velocity: Velocity(0.0, 0.0), primaryVelocity: null)")
    );
}

#[test]
fn drag_diagnostics_read_actual_positions_device_and_velocity() {
    let global = Offset::new(8., -2.);
    let local = Offset::new(3., 4.);
    let start = DragStartDetails {
        global_position: global,
        local_position: local,
        source_time: None,
        kind: Some(PointerDeviceKind::InvertedStylus),
    };
    let end = DragEndDetails {
        global_position: global,
        local_position: local,
        velocity: Velocity::new(Offset::new(30., 0.)),
        primary_velocity: Some(30.),
    };
    assert_eq!(
        format!("{start:#?}"),
        "DragStartDetails(globalPosition: Offset(8.0, -2.0), localPosition: Offset(3.0, 4.0), sourceTimeStamp: null, kind: invertedStylus)"
    );
    assert_eq!(
        end.to_string(),
        "DragEndDetails(globalPosition: Offset(8.0, -2.0), localPosition: Offset(3.0, 4.0), velocity: Velocity(30.0, 0.0), primaryVelocity: 30.0)"
    );
}
