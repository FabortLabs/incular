use incular_core::{Offset, Rect, Size};
use incular_rendering::{FillRule, Path};
use incular_widgets::{CircularNotchedRectangle, NotchedShape};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

fn host() -> Rect {
    rect(0., 100., 300., 200.)
}

fn verify_rect_samples(path: &Path, inflation: f32, count: u32) {
    for y in 0..count {
        for x in 0..count {
            let x = -inflation + (x as f32 + 0.5) / count as f32 * (300. + 2. * inflation);
            let y = 100. - inflation + (y as f32 + 0.5) / count as f32 * (200. + 2. * inflation);
            let point = Offset::new(x, y);
            assert_eq!(
                path.contains(point, FillRule::NonZero),
                host().contains(point),
                "{point:?}"
            );
        }
    }
}

#[test]
fn a_guest_without_overlap_leaves_the_rectangle_untouched() {
    let path = CircularNotchedRectangle::new().outer_path(host(), Some(rect(50., 50., 10., 10.)));
    verify_rect_samples(&path, 5., 40);
}

#[test]
fn omitting_the_guest_leaves_the_rectangle_untouched() {
    let path = CircularNotchedRectangle::new().outer_path(host(), None);
    verify_rect_samples(&path, 800., 100);
}

#[test]
fn circles_above_and_below_either_host_edge_are_excluded() {
    for (inverted, center_y) in [(false, 95.), (false, 105.), (true, 295.), (true, 305.)] {
        let path = CircularNotchedRectangle::new()
            .inverted(inverted)
            .outer_path(host(), Some(rect(190., center_y - 10., 20., 20.)));
        assert!(path.bez_path().is_finite());
        for sector in 0..=40 {
            let theta = f64::from(sector) * std::f64::consts::PI / 20.;
            for step in 0..100 {
                let radius = f64::from(step) / 10.;
                let point = Offset::new(
                    (200. + radius * theta.cos()) as f32,
                    (f64::from(center_y) + radius * theta.sin()) as f32,
                );
                assert!(
                    !path.contains(point, FillRule::NonZero),
                    "{inverted} {center_y} {point:?}"
                );
            }
        }
        // The rectangular body still exists on both sides and across the
        // opposite edge; excluding a circle must not erase the host.
        for point in [
            Offset::new(10., 200.),
            Offset::new(290., 200.),
            Offset::new(200., 200.),
        ] {
            assert!(path.contains(point, FillRule::NonZero));
        }
    }
}
