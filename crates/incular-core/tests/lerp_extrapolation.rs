//! paint-021: HSV hue interpolates over the raw range, so 300 to 360 passes
//! through 330. Size lerps extrapolate as Flutter's `Size.lerp` does.

use incular_core::{HsvColor, Lerp, Size};

#[test]
fn hsv_hue_does_not_wrap_through_zero_before_360() {
    let mid = HsvColor::new(300.0, 1.0, 1.0, 1.0).lerp(&HsvColor::new(360.0, 1.0, 1.0, 1.0), 0.5);
    assert_eq!(mid.hue, 330.0);
}

#[test]
fn size_lerp_extrapolates_negative_extents() {
    let begin = Size {
        width: 10.0,
        height: 10.0,
    };
    let end = Size {
        width: 0.0,
        height: 0.0,
    };
    let overshoot = begin.lerp(&end, 2.0);
    assert_eq!((overshoot.width, overshoot.height), (-10.0, -10.0));
}
