//! Color interpolation contracts from Flutter 3.50.0's `painting/colors.dart`.
//! Paired reference evidence: audits/flutter-3.50.0-parity/color-clamp-comparisons.md.

use incular_core::{HslColor, HsvColor, Lerp};

#[test]
fn hsv_lerp_takes_the_short_arc_across_zero_degrees() {
    let start = HsvColor::new(350.0, 1.0, 1.0, 1.0);
    let end = HsvColor::new(10.0, 1.0, 1.0, 1.0);

    assert_eq!(start.lerp(&end, 0.5).hue, 0.0);
}

#[test]
fn hsl_lerp_takes_the_short_arc_across_zero_degrees() {
    let start = HslColor::new(350.0, 1.0, 0.5, 1.0);
    let end = HslColor::new(10.0, 1.0, 0.5, 1.0);

    assert_eq!(start.lerp(&end, 0.5).hue, 0.0);
}

#[test]
fn hsl_lerp_extrapolates_hue_past_the_end() {
    let start = HslColor::new(20.0, 1.0, 0.5, 1.0);
    let end = HslColor::new(60.0, 1.0, 0.5, 1.0);

    assert_eq!(start.lerp(&end, 0.5).hue, 40.0);
    assert_eq!(start.lerp(&end, 1.5).hue, 80.0);
}
