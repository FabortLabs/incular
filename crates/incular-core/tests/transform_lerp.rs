//! `Transform`'s `Lerp` follows Flutter's `Matrix4Tween.lerp`.

use incular_core::{Lerp, Offset, Transform};

fn assert_coeffs(actual: Transform, expected: [f64; 6]) {
    let actual = actual.to_kurbo().as_coeffs();
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn rotation_interpolates_through_the_normalized_quaternion() {
    // Identity is the quaternion (0, 0, 0, 1); a quarter turn is
    // (0, 0, √2/2, √2/2). Their normalized midpoint is the eighth turn.
    let quarter = Transform::rotation(std::f32::consts::FRAC_PI_2);
    let half_way = Transform::IDENTITY.lerp(&quarter, 0.5);
    let c = std::f64::consts::FRAC_1_SQRT_2;
    assert_coeffs(half_way, [c, c, -c, c, 0., 0.]);
}

#[test]
fn translation_and_scale_interpolate_linearly() {
    let begin = Transform::scale(2.);
    let end = Transform::translation(Offset::new(40., -8.)).then(Transform::scale(4.));
    // Scale 2 + (4 - 2) / 4 = 2.5; translation (40, -8) / 4 = (10, -2).
    assert_coeffs(begin.lerp(&end, 0.25), [2.5, 0., 0., 2.5, 10., -2.]);
}

#[test]
fn a_reflection_decomposes_into_a_negative_x_scale() {
    // det < 0, so `decompose` negates the x scale: the rotation is the
    // identity and the x scale runs from 1 to -1.
    let flipped = Transform::scale_non_uniform(-1., 1.);
    assert_coeffs(
        Transform::IDENTITY.lerp(&flipped, 0.25),
        [0.5, 0., 0., 1., 0., 0.],
    );
}
