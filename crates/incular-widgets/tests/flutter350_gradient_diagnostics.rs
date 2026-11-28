use incular_config::Alignment;
use incular_core::Color;
use incular_rendering::GradientStops;
use incular_widgets::BoxLinearGradient;

const COLORS: [Color; 2] = [Color::rgba(51, 51, 51, 51), Color::rgba(102, 102, 102, 102)];

#[test]
fn describes_a_rotated_linear_gradient_without_implied_stop_positions() {
    let gradient = BoxLinearGradient::new(
        Alignment::TOP_LEFT,
        Alignment::BOTTOM_LEFT,
        GradientStops::from_colors(&COLORS, None).unwrap(),
    )
    .rotated(1.6);
    let first = "Color(alpha: 0.2000, red: 0.2000, green: 0.2000, blue: 0.2000, colorSpace: ColorSpace.sRGB)";
    let second = "Color(alpha: 0.4000, red: 0.4000, green: 0.4000, blue: 0.4000, colorSpace: ColorSpace.sRGB)";
    assert_eq!(
        gradient.to_string(),
        format!(
            "LinearGradient(begin: Alignment.topLeft, end: Alignment.bottomLeft, colors: [{first}, {second}], tileMode: TileMode.clamp, transform: GradientRotation(radians: 1.6))"
        ),
    );
    assert!(!gradient.scale(0.5).to_string().contains("stops:"));
}

#[test]
fn explicit_and_inferred_stops_render_the_same_but_describe_the_declared_input() {
    let implicit = GradientStops::even(&COLORS);
    let explicit = GradientStops::from_colors(&COLORS, Some(&[0., 1.])).unwrap();
    assert_eq!(implicit, explicit);
    assert!(!implicit.has_explicit_positions());
    assert!(explicit.has_explicit_positions());
    let gradient = BoxLinearGradient::new(Alignment::TOP_LEFT, Alignment::BOTTOM_LEFT, explicit);
    assert!(
        gradient
            .to_string()
            .contains("], stops: [0.0, 1.0], tileMode: TileMode.clamp)")
    );
    assert!(
        gradient
            .scale(0.5)
            .to_string()
            .contains("], stops: [0.0, 1.0],")
    );
}
