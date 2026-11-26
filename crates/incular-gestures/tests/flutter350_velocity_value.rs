use incular_core::Offset;
use incular_gestures::Velocity;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn hash(velocity: Velocity) -> u64 {
    let mut hasher = DefaultHasher::new();
    velocity.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn velocity_value_has_vector_arithmetic_and_one_line_description() {
    let first = Velocity::new(Offset::new(7., 0.));
    let second = Velocity::new(Offset::new(12., 0.));
    assert_eq!(first, Velocity::new(Offset::new(7., 0.)));
    assert_ne!(first, second);
    assert_eq!(second - first, Velocity::new(Offset::new(5., 0.)));
    assert_eq!((-first).pixels_per_second, Offset::new(-7., 0.));
    assert_eq!(first + second, Velocity::new(Offset::new(19., 0.)));
    assert_ne!(hash(first), hash(second));
    assert_eq!(hash(first), hash(Velocity::new(Offset::new(7., -0.))));
    assert_eq!(first.to_string(), "Velocity(7.0, 0.0)");
    assert_eq!(format!("{first:#?}"), "Velocity(7.0, 0.0)");
    assert_eq!(
        Velocity::new(Offset::new(-1.25, 2.25)).to_string(),
        "Velocity(-1.3, 2.3)"
    );
}

#[test]
fn magnitude_clamp_preserves_direction_and_in_range_bits() {
    let vector = Velocity::new(Offset::new(-3., 4.));
    assert_eq!(vector.clamp_magnitude(0., 10.), vector);
    assert_eq!(
        vector.clamp_magnitude(6., 10.).pixels_per_second,
        Offset::new(-3.6, 4.8)
    );
    assert_eq!(
        vector.clamp_magnitude(0., 2.5).pixels_per_second,
        Offset::new(-1.5, 2.)
    );
    assert_eq!(Velocity::ZERO.clamp_magnitude(0., 0.), Velocity::ZERO);
}
