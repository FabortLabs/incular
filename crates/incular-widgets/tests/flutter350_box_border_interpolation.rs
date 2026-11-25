use incular_core::Color;
use incular_widgets::{Border, BorderDirectional, BorderSide};

fn solid(width: f32) -> BorderSide {
    BorderSide::solid(Color::BLACK, width)
}

fn compare_nullable_border_samples<B: Copy + std::fmt::Debug + PartialEq>(
    make: impl Fn(BorderSide, BorderSide, BorderSide, BorderSide) -> B,
    lerp: impl Fn(Option<B>, Option<B>, f32) -> Option<B>,
) {
    let none = BorderSide::NONE;
    let top_border = make(solid(10.0), none, none, none);
    let a = make(none, solid(100.0), solid(200.0), none);
    let b = make(none, solid(200.0), solid(100.0), none);
    let top_at = |width: Option<f32>| make(width.map_or(none, solid), none, none, none);
    for (t, out_top, in_top, left, right) in [
        (-1.0, Some(20.0), None, 0.0, 300.0),
        (0.0, Some(10.0), None, 100.0, 200.0),
        (0.25, Some(7.5), Some(2.5), 125.0, 175.0),
        (0.75, Some(2.5), Some(7.5), 175.0, 125.0),
        (1.0, None, Some(10.0), 200.0, 100.0),
        (2.0, None, Some(20.0), 300.0, 0.0),
    ] {
        assert_eq!(lerp(None, None, t), None, "t={t}");
        assert_eq!(
            lerp(Some(top_border), None, t),
            Some(top_at(out_top)),
            "t={t}"
        );
        assert_eq!(
            lerp(None, Some(top_border), t),
            Some(top_at(in_top)),
            "t={t}"
        );
        // Exact zero in an extrapolated solid/solid pair is a hairline;
        // scaling an optional end to zero above is a no-paint side.
        assert_eq!(
            lerp(Some(a), Some(b), t),
            Some(make(none, solid(left), solid(right), none)),
            "t={t}",
        );
    }
}

#[test]
fn physical_border_lerp_preserves_none_and_scales_single_present_endpoint() {
    compare_nullable_border_samples(
        |top, left, right, bottom| Border::only(top, right, bottom, left),
        Border::lerp_optional,
    );
}

#[test]
fn directional_border_lerp_keeps_interpolating_start_and_end_as_logical_sides() {
    compare_nullable_border_samples(BorderDirectional::only, BorderDirectional::lerp_optional);
}
