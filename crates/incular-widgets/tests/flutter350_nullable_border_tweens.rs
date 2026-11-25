//! Flutter 3.50.0's nullable BorderTween and BorderRadiusTween test cases.

use incular_animation::DoubleAnimation;
use incular_core::Color;
use incular_widgets::{
    Border, BorderRadius, BorderSide, OptionalBorderRadiusTween, OptionalBorderTween,
};

#[test]
fn nullable_border_tween_uses_absent_side_scale_but_preserves_transform_endpoints() {
    let tween = OptionalBorderTween::default();
    assert_eq!(tween.lerp(0.), None);
    assert_eq!(tween.lerp(1.), None);

    let border = Border {
        top: BorderSide::default(),
        ..Border::default()
    };
    let tween = OptionalBorderTween::new(None, Some(border));
    let half = Border {
        top: BorderSide {
            width: 0.5,
            ..BorderSide::default()
        },
        ..Border::default()
    };
    assert_eq!(tween.lerp(0.), Some(Border::default()));
    assert_eq!(tween.lerp(0.5), Some(half));
    assert_eq!(tween.lerp(1.), Some(border));

    // The typed tween can also drive an Animation<Option<Border>>.
    assert_eq!(tween.transform(0.), None);
    assert_eq!(tween.transform(1.), Some(border));
    assert_eq!(DoubleAnimation::from(0.5).drive(tween).value(), Some(half));
}

#[test]
fn absent_border_end_scales_without_replacing_the_present_color() {
    let painted = Border::new(4., Color::rgba(13, 44, 204, 255));
    let halfway = OptionalBorderTween::new(Some(painted), None)
        .lerp(0.5)
        .unwrap();
    assert_eq!(halfway.top.color, painted.top.color);
    assert_eq!(halfway.top.width, 2.);
}

#[test]
fn nullable_radius_tween_returns_no_radius_for_two_absent_values() {
    let tween = OptionalBorderRadiusTween::default();
    assert_eq!(tween.transform(0.), None);
    assert_eq!(tween.transform(1.), None);
    assert_eq!(tween.lerp(0.), None);

    let radius = BorderRadius::circular(20.);
    let tween = OptionalBorderRadiusTween::new(Some(radius), None);
    assert_eq!(tween.transform(0.), Some(radius));
    assert_eq!(tween.transform(1.), None);
    assert_eq!(tween.lerp(1.), Some(BorderRadius::ZERO));
    assert_eq!(tween.lerp(1.5), Some(BorderRadius::circular(-10.)));
}
