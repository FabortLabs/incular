use std::time::Instant;

use incular_core::{Offset, PointerDeviceKind as Kind, PointerPhase, Transform};
use incular_gestures::{
    DeviceGestureSettings, RawPointerEvent, compute_hit_slop, compute_pan_slop, compute_scale_slop,
    is_single_button, nth_mouse_button, nth_stylus_button, smallest_button,
};

#[test]
fn mouse_and_stylus_button_numbers_use_different_starting_bits() {
    assert_eq!(nth_mouse_button(2), Some(0x2));
    assert_eq!(nth_stylus_button(2), Some(0x4));
    assert_eq!(nth_mouse_button(1), Some(0x1));
    assert_eq!(nth_stylus_button(1), Some(0x2));
    assert_eq!(nth_mouse_button(32), Some(1 << 31));
    assert_eq!(nth_stylus_button(31), Some(1 << 31));
    for (mouse, stylus) in [(0, 0), (33, 32), (u32::MAX, u32::MAX)] {
        assert_eq!(nth_mouse_button(mouse), None);
        assert_eq!(nth_stylus_button(stylus), None);
    }
}

#[test]
fn least_significant_button_is_selected_from_a_chord() {
    for (input, expected) in [
        (0, 0),
        (1, 1),
        (0x200, 0x200),
        (0x220, 0x20),
        (1 << 31, 1 << 31),
    ] {
        assert_eq!(smallest_button(input), expected);
    }
    assert_eq!(smallest_button(u32::MAX), 1);
}

#[test]
fn single_button_rejects_empty_masks_and_chords() {
    for (input, expected) in [
        (0, false),
        (1, true),
        (0x200, true),
        (0x220, false),
        (1 << 31, true),
    ] {
        assert_eq!(is_single_button(input), expected);
    }
}

#[test]
fn default_slop_by_pointer_device_matches_the_pinned_values() {
    let defaults = DeviceGestureSettings::default();
    for (device, hit, pan, scale) in [
        (Kind::Mouse, 1., 2., 1.),
        (Kind::Stylus, 18., 36., 18.),
        (Kind::InvertedStylus, 18., 36., 18.),
        (Kind::Touch, 18., 36., 18.),
        (Kind::Unknown, 18., 36., 18.),
        (Kind::Trackpad, 18., 36., 18.),
    ] {
        assert_eq!(compute_hit_slop(device, defaults), hit);
        assert_eq!(compute_pan_slop(device, defaults), pan);
        assert_eq!(compute_scale_slop(device), scale);
    }
}

#[test]
fn touch_slop_override_scales_the_pan_threshold_and_leaves_mouse_precise() {
    let settings = DeviceGestureSettings::new(Some(1.));
    for (device, hit, pan) in [
        (Kind::Mouse, 1., 2.),
        (Kind::Stylus, 1., 2.),
        (Kind::InvertedStylus, 1., 2.),
        (Kind::Touch, 1., 2.),
        (Kind::Unknown, 1., 2.),
        (Kind::Trackpad, 1., 2.),
    ] {
        assert_eq!(compute_hit_slop(device, settings), hit);
        assert_eq!(compute_pan_slop(device, settings), pan);
    }
    // Use a value other than the mouse's intrinsic 1/2 to verify the split.
    let settings = DeviceGestureSettings::new(Some(7.));
    assert_eq!(compute_hit_slop(Kind::Mouse, settings), 1.);
    assert_eq!(compute_pan_slop(Kind::Mouse, settings), 2.);
    assert_eq!(compute_hit_slop(Kind::Trackpad, settings), 7.);
    assert_eq!(compute_pan_slop(Kind::Trackpad, settings), 14.);
}

#[test]
fn optional_affine_transform_maps_positions_but_not_global_event_metadata() {
    let position = Offset::new(20., 30.);
    assert_eq!(
        RawPointerEvent::transform_position(None, position),
        position
    );
    assert_eq!(
        RawPointerEvent::transform_position(Some(Transform::IDENTITY), position),
        position
    );
    let transform = Transform::translation(Offset::new(10., 20.));
    assert_eq!(
        RawPointerEvent::transform_position(Some(transform), position),
        Offset::new(30., 50.)
    );
    let raw = RawPointerEvent::new(
        3,
        4,
        Kind::Mouse,
        1,
        position,
        PointerPhase::Down,
        Instant::now(),
    );
    let changed = raw.transformed(Some(transform));
    assert_eq!(
        changed,
        RawPointerEvent {
            local_position: Offset::new(30., 50.),
            ..raw
        }
    );
    assert_eq!(raw.transformed(None), raw);
}

#[test]
fn pointer_delta_uses_a_supplied_end_point_and_ignores_translation() {
    let end = Offset::new(20., 30.);
    let delta = Offset::new(5., 5.);
    let transform = Some(Transform::scale(2.));
    let projected = RawPointerEvent::transform_delta_via_positions;
    assert_eq!(
        projected(transform, end, None, delta),
        Offset::new(10., 10.)
    );
    assert_eq!(
        projected(transform, end, Some(Offset::new(40., 60.)), delta),
        Offset::new(10., 10.)
    );
    assert_eq!(
        projected(None, end, Some(Offset::new(40., 60.)), delta),
        delta
    );

    let huge = Transform::translation(Offset::new(33_554_432., -33_554_432.));
    let raw = RawPointerEvent {
        delta: Offset::new(1., -1.),
        ..RawPointerEvent::new(
            3,
            4,
            Kind::Mouse,
            1,
            end,
            PointerPhase::Move,
            Instant::now(),
        )
    };
    let local = raw.transformed(Some(huge));
    assert_eq!(local.local_delta, raw.delta);
    assert_eq!(local.local_position, huge.transform_point(raw.position));
}

#[test]
fn the_pointer_plane_clears_z_without_mutating_the_paint_matrix() {
    // gestures/events_test.dart:239.
    use incular_core::Matrix4;

    let clean = RawPointerEvent::remove_perspective_transform;
    assert_eq!(clean(Matrix4::IDENTITY), Matrix4::IDENTITY);
    let paint = Matrix4::scale(3., 3., 3.);
    assert_eq!(clean(paint), Matrix4::scale(3., 3., 1.));
    assert_eq!(paint.columns()[10], 3.);
}

#[test]
fn projective_pointer_delta_uses_both_positions_without_mutating_globals() {
    use incular_core::{Matrix4, NormalizedPressure, PointerSampleMetadata};

    let matrix = Matrix4::from_columns([
        2., 0., 0., 0.01, 0., 3., 0., 0., 7., 8., 9., 10., 10., 20., 30., 1.,
    ]);
    let pointer = RawPointerEvent::remove_perspective_transform(matrix);
    assert_eq!(
        *pointer.columns(),
        [2., 0., 0., 0.01, 0., 3., 0., 0., 0., 0., 1., 0., 10., 20., 0., 1.]
    );
    let mut original = RawPointerEvent::new(
        34,
        10,
        Kind::Stylus,
        1,
        Offset::new(20., 30.),
        PointerPhase::Move,
        Instant::now(),
    )
    .with_sample(PointerSampleMetadata {
        pressure: NormalizedPressure::new(0.4),
        stylus: None,
    });
    original.delta = Offset::new(5., 5.);
    assert_eq!(original.transformed_matrix4(None), original);
    let actual = original.transformed_matrix4(Some(pointer));
    assert_eq!(actual.in_global_coordinates(), original.in_global_coordinates());
    let check = |actual: f32, expected: f32| {
        assert!((actual - expected).abs() < 0.000_02, "{actual} != {expected}");
    };
    check(actual.local_position.x, 50. / 1.2);
    check(actual.local_position.y, 110. / 1.2);
    check(actual.local_delta.x, 50. / 1.2 - 40. / 1.15);
    check(actual.local_delta.y, 110. / 1.2 - 95. / 1.15);
}
