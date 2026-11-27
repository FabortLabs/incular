use incular_animation::Matrix4Tween;
use incular_core::{Matrix4, Transform};

fn assert_matrix(actual: Matrix4, expected: [f64; 16]) {
    for (index, (&actual, expected)) in actual.columns().iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < 1e-12,
            "column-major entry {index}: {actual} != {expected}"
        );
    }
}

#[test]
fn matrix4_tween_translates_and_scales_all_three_axes_then_blends_rotation() {
    let a = Matrix4::IDENTITY;
    let b = Matrix4::translation(6., -8., 0.).multiply(Matrix4::scale(0.5, 1., 5.));
    let tween = Matrix4Tween::new(a, b);
    assert_eq!(tween.lerp(0.), a);
    assert_eq!(tween.lerp(1.), b);
    assert_eq!(
        tween.lerp(0.5),
        Matrix4::translation(3., -4., 0.).multiply(Matrix4::scale(0.75, 1., 3.)),
    );

    let c = Matrix4::rotation_z(1.);
    let rotation = Matrix4Tween::new(a, c);
    assert_matrix(rotation.lerp(0.), *a.columns());
    assert_matrix(rotation.lerp(1.), *c.columns());
    assert_matrix(rotation.lerp(0.5), *Matrix4::rotation_z(0.5).columns());
}

#[test]
fn real_matrix_tweens_match_reflections_and_three_axis_translation_rotation_and_scale() {
    let a = Matrix4::translation(2., -5., 7.).multiply(Matrix4::scale(2., 3., 4.));
    let b = Matrix4::translation(-9., 11., 13.)
        .multiply(Matrix4::rotation_y(2.4))
        .multiply(Matrix4::rotation_x(-0.4))
        .multiply(Matrix4::scale(-3., 0.5, 8.));
    let tween: Matrix4Tween<Matrix4> = Matrix4Tween::new(a, b);
    assert_eq!(tween.transform(0.), a);
    assert_eq!(tween.transform(1.), b);
    assert_matrix(
        tween.lerp(0.25),
        [
            0.6425838040254901,
            0.06868908426300949,
            -0.3806151133458049,
            0.,
            -0.2690136225163055,
            2.359545123961154,
            -0.02834570323344445,
            0.,
            2.515456826394721,
            0.33854074452528704,
            4.307884297290315,
            0.,
            -0.75,
            -1.,
            8.5,
            1.,
        ],
    );
}

#[test]
fn rotations_follow_flutter_normalized_linear_blend_without_shortest_path_flip() {
    let tween = Matrix4Tween::new(Matrix4::rotation_x(2.9), Matrix4::rotation_y(-2.9));
    assert_matrix(
        tween.lerp(0.5),
        [
            0., 1., 0., 0., 1., 0., 0., 0., 0., 0., -1., 0., 0., 0., 0., 1.,
        ],
    );
}

#[test]
fn exact_endpoints_preserve_perspective_while_the_interior_recomposes_affine_components() {
    let mut a = *Matrix4::IDENTITY.columns();
    a[11] = -0.02;
    let a = Matrix4::from_columns(a);
    let mut b = *Matrix4::translation(2., 4., 6.).columns();
    b[3] = 0.01;
    let b = Matrix4::from_columns(b);
    let tween = Matrix4Tween::new(a, b);
    assert_eq!(tween.transform(0.), a);
    assert_eq!(tween.transform(1.), b);
    assert_eq!(tween.lerp(0.5), Matrix4::translation(1., 2., 3.));

    // Retaining the previous default type keeps Incular's affine usage valid.
    let affine: Matrix4Tween = Matrix4Tween::new(Transform::IDENTITY, Transform::IDENTITY);
    assert_eq!(affine.evaluate(0.5), Transform::IDENTITY);
}
