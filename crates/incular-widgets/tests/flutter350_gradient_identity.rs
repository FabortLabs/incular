use std::borrow::Cow;
use std::sync::Arc;

use incular_config::Alignment;
use incular_core::{Color, Lerp};
use incular_rendering::{GradientStop, GradientStops};
use incular_widgets::{
    AlignedShapeGradient, BoxLinearGradient, BoxRadialGradient, BoxSweepGradient,
};

fn stops() -> GradientStops {
    GradientStops::even(&[Color::BLACK, Color::WHITE])
}

#[test]
fn linear_lerp_preserves_the_identical_descriptor() {
    let gradient = BoxLinearGradient::new(Alignment::CENTER_LEFT, Alignment::CENTER_RIGHT, stops());
    let returned = BoxLinearGradient::lerp_ref(Some(&gradient), Some(&gradient), 0.5);
    assert!(matches!(returned, Some(Cow::Borrowed(value)) if std::ptr::eq(value, &gradient)));
    assert!(BoxLinearGradient::lerp_ref(None, None, 0.5).is_none());
}

#[test]
fn radial_lerp_preserves_the_identical_descriptor() {
    let gradient = BoxRadialGradient::new(stops()).radius(0.7);
    let returned = BoxRadialGradient::lerp_ref(Some(&gradient), Some(&gradient), 0.5);
    assert!(matches!(returned, Some(Cow::Borrowed(value)) if std::ptr::eq(value, &gradient)));
    assert!(BoxRadialGradient::lerp_ref(None, None, 0.5).is_none());
}

#[test]
fn sweep_lerp_preserves_the_identical_descriptor() {
    let gradient = BoxSweepGradient::new(stops()).center(Alignment::CENTER_RIGHT);
    let returned = BoxSweepGradient::lerp_ref(Some(&gradient), Some(&gradient), 0.5);
    assert!(matches!(returned, Some(Cow::Borrowed(value)) if std::ptr::eq(value, &gradient)));
    assert!(BoxSweepGradient::lerp_ref(None, None, 0.5).is_none());
}

#[test]
fn distinct_endpoints_still_union_stops_and_absent_endpoints_fade() {
    let a = BoxLinearGradient::new(Alignment::CENTER_LEFT, Alignment::CENTER_RIGHT, stops());
    let b = BoxLinearGradient::new(
        Alignment::CENTER_LEFT,
        Alignment::CENTER_RIGHT,
        GradientStops::new(vec![
            GradientStop {
                offset: 0.,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 0.25,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 1.,
                color: Color::WHITE,
            },
        ]),
    );
    let Some(Cow::Owned(result)) = BoxLinearGradient::lerp_ref(Some(&a), Some(&b), 0.) else {
        panic!("distinct endpoints recompute the union even at zero");
    };
    assert_eq!(
        result
            .stops
            .as_slice()
            .iter()
            .map(|stop| stop.offset)
            .collect::<Vec<_>>(),
        [0., 0.25, 1.]
    );
    assert_eq!(
        result.stops.as_slice()[1].color,
        Color::rgba(64, 64, 64, 255)
    );
    let fade = BoxLinearGradient::lerp_ref(Some(&a), None, 0.75).unwrap();
    assert_eq!(
        fade.stops.as_slice()[1].color,
        Color::rgba(255, 255, 255, 64)
    );
}

#[test]
fn mounted_shape_gradient_wrapper_keeps_the_existing_arc() {
    let shader = Arc::new(BoxRadialGradient::new(stops()));
    let left = AlignedShapeGradient::Radial(shader.clone());
    let right = AlignedShapeGradient::Radial(shader.clone());
    let AlignedShapeGradient::Radial(result) = left.lerp(&right, 0.5) else {
        unreachable!()
    };
    assert!(Arc::ptr_eq(&result, &shader));
}
