//! The renderer-neutral backdrop plan: which stages run, which capture and
//! which share a keyed capture (Flutter's `BackdropGroup`), and where
//! `ImageFilter.matrix` samples the backdrop in device pixels.

use incular_core::{Offset, Rect, Size, Transform};
use incular_rendering::{
    BackdropCaptures, BackdropKey, BackdropSample, BlendMode, DisplayList, GaussianBlur, LayerTree,
    PaintCommand, backdrop_filter_is_active, backdrop_matrix_sampling,
};

fn assert_point(actual: Offset, expected: Offset) {
    assert!(
        (actual.x - expected.x).abs() < 1e-4 && (actual.y - expected.y).abs() < 1e-4,
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn only_enabled_stages_with_a_matrix_or_a_blur_run() {
    let none = GaussianBlur::uniform(0.);
    assert!(!backdrop_filter_is_active(none, true, None));
    assert!(!backdrop_filter_is_active(
        GaussianBlur::uniform(4.),
        false,
        None
    ));
    assert!(backdrop_filter_is_active(
        GaussianBlur::uniform(4.),
        true,
        None
    ));
    assert!(backdrop_filter_is_active(
        GaussianBlur::new(0., 2.),
        true,
        None
    ));
    // A matrix runs even at sigma 0; disabling still wins.
    assert!(backdrop_filter_is_active(
        none,
        true,
        Some(Transform::scale(2.))
    ));
    assert!(!backdrop_filter_is_active(
        none,
        false,
        Some(Transform::scale(2.))
    ));
}

fn layers(count: usize) -> Vec<incular_rendering::LayerId> {
    let mut tree = LayerTree::new();
    (0..count)
        .map(|_| {
            tree.create_picture(
                DisplayList::new(),
                Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
            )
        })
        .collect()
}

#[test]
fn stages_sharing_a_key_sample_the_first_capture() {
    let ids = layers(4);
    let key = BackdropKey::new();
    let mut captures = BackdropCaptures::new();
    assert_eq!(
        captures.sample(ids[0], Some(key), 0),
        BackdropSample::Capture
    );
    // A later stage with the key does not capture again, so it cannot see
    // the first stage's output.
    assert_eq!(
        captures.sample(ids[1], Some(key), 0),
        BackdropSample::Shared(ids[0])
    );
    assert_eq!(
        captures.sample(ids[2], Some(key), 0),
        BackdropSample::Shared(ids[0])
    );
    // Another key starts its own capture.
    assert_eq!(
        captures.sample(ids[3], Some(BackdropKey::new()), 0),
        BackdropSample::Capture
    );
}

#[test]
fn unkeyed_stages_and_other_targets_capture_their_own_backdrop() {
    let ids = layers(4);
    let key = BackdropKey::new();
    let mut captures = BackdropCaptures::new();
    assert_eq!(captures.sample(ids[0], None, 0), BackdropSample::Capture);
    assert_eq!(captures.sample(ids[1], None, 0), BackdropSample::Capture);
    assert_eq!(
        captures.sample(ids[2], Some(key), 0),
        BackdropSample::Capture
    );
    // The same key in an isolated layer reads that layer.
    assert_eq!(
        captures.sample(ids[3], Some(key), 7),
        BackdropSample::Capture
    );
    // A new frame forgets the captures.
    captures.clear();
    assert_eq!(
        captures.sample(ids[3], Some(key), 0),
        BackdropSample::Capture
    );
}

#[test]
fn a_matrix_moves_the_backdrop_in_the_stage_coordinates() {
    // The stage sits at (10, 20) in a 2x device: device pixel d is logical
    // d / 2, stage point d / 2 - (10, 20), and with the backdrop scaled by 2
    // it shows the backdrop at half that stage point.
    let sampling = backdrop_matrix_sampling(
        Transform::translation(Offset::new(10., 20.)),
        Transform::scale(2.),
        2.,
    )
    .expect("invertible");
    // (60, 80) -> logical (30, 40) -> stage (20, 20) -> backdrop stage
    // point (10, 10) -> logical (20, 30) -> device (40, 60).
    assert_point(
        sampling.transform_point(Offset::new(60., 80.)),
        Offset::new(40., 60.),
    );
    // The stage origin is the fixed point of a scale.
    assert_point(
        sampling.transform_point(Offset::new(20., 40.)),
        Offset::new(20., 40.),
    );
}

#[test]
fn a_rotated_backdrop_is_sampled_through_the_inverse_rotation() {
    // Content at stage point p moves to R(90°) p, so the output at (1, 0)
    // shows what was at R(-90°) (1, 0) = (0, -1).
    let sampling = backdrop_matrix_sampling(
        Transform::IDENTITY,
        Transform::rotation(std::f32::consts::FRAC_PI_2),
        1.,
    )
    .expect("invertible");
    assert_point(
        sampling.transform_point(Offset::new(1., 0.)),
        Offset::new(0., -1.),
    );
}

#[test]
fn a_singular_matrix_leaves_nothing_to_sample() {
    let flatten = Transform::from_coefficients([1., 0., 0., 0., 0., 0.]);
    assert!(backdrop_matrix_sampling(Transform::IDENTITY, flatten, 1.).is_none());
}

#[test]
fn the_flattened_list_keeps_only_active_stages() {
    let mut tree = LayerTree::new();
    let root = tree.create_transform(Transform::IDENTITY);
    let mut stages = Vec::new();
    for sigma in [0., 3.] {
        let stage =
            tree.create_backdrop_filter(GaussianBlur::uniform(sigma), BlendMode::SrcOver, true);
        let picture = tree.create_picture(
            DisplayList::new(),
            Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
        );
        tree.set_children(stage, vec![picture]);
        stages.push(stage);
    }
    tree.set_children(root, stages);
    tree.set_root(root);
    let list = tree.flatten();
    let stages: Vec<f32> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushBackdropFilter { blur, .. } => Some(blur.sigma_x),
            _ => None,
        })
        .collect();
    assert_eq!(stages, [3.]);
}
