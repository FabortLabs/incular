//! User-offset and release-velocity assertions from Flutter 3.50.0
//! widgets/scroll_physics_test.dart.

use incular_scroll::{ScrollDecelerationRate, ScrollPhysics};

fn finger_offset(physics: ScrollPhysics, pixels: f32, max: f32, finger_delta: f32) -> f32 {
    // Flutter's API returns finger-coordinate offset. Incular consumes and
    // returns scroll-coordinate deltas, which have the opposite sign.
    -physics
        .with_device_pixel_ratio(3.0)
        .apply_delta_in_viewport(pixels, -finger_delta, 0.0, max, 100.0)
        .consumed
}

fn in_upstream_resistance_range(offset: f32) {
    assert!(offset > 1.0 && offset < 20.0, "offset: {offset}");
}

#[test]
fn creating_ballistic_simulations_preserves_the_release_velocity() {
    // Upstream line 112. Page physics resolve their page size from the
    // viewport; the same viewport is immaterial to clamp/bounce.
    let clamping = ScrollPhysics::CLAMPING;
    for (name, physics) in [
        ("bounce", ScrollPhysics::BOUNCING),
        ("clamp", clamping),
        ("page", clamping.page()),
    ] {
        let simulation = physics
            .with_device_pixel_ratio(3.0)
            .release_simulation(20.0, 1000.0, 0.0, 100.0, 500.0)
            .unwrap_or_else(|| panic!("{name} did not start a ballistic simulation"));
        assert_eq!(simulation.dx(0.0), 1000.0, "{name}");
    }
}

#[test]
fn pushing_further_out_is_progressively_harder() {
    // Upstream line 140.
    let less = finger_offset(ScrollPhysics::BOUNCING, -20.0, 1000.0, 10.0);
    let more = finger_offset(ScrollPhysics::BOUNCING, -40.0, 1000.0, 10.0);
    in_upstream_resistance_range(less);
    in_upstream_resistance_range(more);
    assert!(less.abs() > more.abs(), "{less} vs {more}");
}

#[test]
fn normal_overscroll_eases_more_freely_than_it_tensions() {
    // Upstream line 212.
    let normal = ScrollPhysics::BOUNCING;
    let easing = finger_offset(normal, -20.0, 1000.0, -10.0);
    let tension = finger_offset(normal, -20.0, 1000.0, 10.0);
    assert!(easing.abs() > tension.abs(), "{easing} vs {tension}");
}

#[test]
fn fast_overscroll_has_no_easing_resistance() {
    // Upstream line 234.
    let fast = ScrollPhysics::CLAMPING.bouncing_with(ScrollDecelerationRate::Fast);
    let easing = finger_offset(fast, -20.0, 1000.0, -10.0);
    let tension = finger_offset(fast, -20.0, 1000.0, 10.0);
    assert!(tension.abs() < easing.abs(), "{tension} vs {easing}");
    assert_eq!(easing, -10.0);
}

#[test]
fn leading_edge_resistance_does_not_depend_on_the_trailing_extent() {
    // Upstream line 252.
    let small = finger_offset(ScrollPhysics::BOUNCING, -20.0, 10.0, 10.0);
    let large = finger_offset(ScrollPhysics::BOUNCING, -20.0, 1000.0, 10.0);
    assert_eq!(small, large);
    in_upstream_resistance_range(small);
}
