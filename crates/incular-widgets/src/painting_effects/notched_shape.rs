//! Neutral, renderer-independent notched paths.

use incular_core::{Offset, Rect};
use incular_rendering::{Path, PathBuilder};

/// The outline of a host that can make room for an optional guest.
pub trait NotchedShape {
    fn outer_path(&self, host: Rect, guest: Option<Rect>) -> Path;
}

/// A rectangular outline with a smooth circular notch in its top edge,
/// or in its bottom edge when inverted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct CircularNotchedRectangle {
    pub inverted: bool,
}

impl CircularNotchedRectangle {
    #[must_use]
    pub const fn new() -> Self {
        Self { inverted: false }
    }

    #[must_use]
    pub const fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl NotchedShape for CircularNotchedRectangle {
    fn outer_path(&self, host: Rect, guest: Option<Rect>) -> Path {
        let left = host.origin.x;
        let top = host.origin.y;
        let right = left + host.size.width;
        let bottom = top + host.size.height;
        let mut path = Path::builder();
        path.move_to(Offset::new(left, top));
        let Some(guest) = guest.filter(|guest| host.intersects(*guest)) else {
            path.line_to(Offset::new(right, top))
                .line_to(Offset::new(right, bottom))
                .line_to(Offset::new(left, bottom))
                .close();
            return path.build();
        };

        let radius = f64::from(guest.size.width) / 2.;
        let center_x = f64::from(guest.origin.x) + radius;
        let center_y = f64::from(guest.origin.y) + f64::from(guest.size.height) / 2.;
        // Flutter joins the circle's tangent with quadratic shoulder curves.
        // The control point sits one logical pixel past the circle, and the
        // shoulder starts fifteen pixels farther along the rectangular edge.
        let a = -radius - 1.;
        let b = f64::from(if self.inverted { bottom } else { top }) - center_y;
        let denominator = a * a + b * b;
        let square_radius = radius * radius;
        let root = (b * b * square_radius * (denominator - square_radius)).sqrt();
        let x_a = (a * square_radius - root) / denominator;
        let x_b = (a * square_radius + root) / denominator;
        let sign = if self.inverted { -1. } else { 1. };
        let y_a = sign * (square_radius - x_a * x_a).max(0.).sqrt();
        let y_b = sign * (square_radius - x_b * x_b).max(0.).sqrt();
        let choose_a = if b < 0. { y_a < y_b } else { y_a > y_b };
        let (tangent_x, tangent_y) = if choose_a { (x_a, y_a) } else { (x_b, y_b) };
        let point = |x, y| ((center_x + x), (center_y + y));
        let p0 = point(a - 15., b);
        let p1 = point(a, b);
        let p2 = point(tangent_x, tangent_y);
        let p3 = point(-tangent_x, tangent_y);
        let p4 = point(-a, b);
        let p5 = point(15. - a, b);

        if self.inverted {
            path.line_to(Offset::new(right, top))
                .line_to(Offset::new(right, bottom))
                .line_to(offset(p5))
                .quadratic_to(offset(p4), offset(p3));
            append_notch_arc(&mut path, p3, p2, radius);
            path.quadratic_to(offset(p1), offset(p0))
                .line_to(Offset::new(left, bottom));
        } else {
            path.line_to(offset(p0))
                .quadratic_to(offset(p1), offset(p2));
            append_notch_arc(&mut path, p2, p3, radius);
            path.quadratic_to(offset(p4), offset(p5))
                .line_to(Offset::new(right, top))
                .line_to(Offset::new(right, bottom))
                .line_to(Offset::new(left, bottom));
        }
        path.close();
        path.build()
    }
}

fn offset((x, y): (f64, f64)) -> Offset {
    Offset::new(x as f32, y as f32)
}

/// The endpoints are at the same height. Use the small counter-clockwise
/// arc, matching Path.arcToPoint, and split at its cardinal midpoint for
/// two stable rational quadratics even when the arc is a semicircle.
fn append_notch_arc(path: &mut PathBuilder, from: (f64, f64), to: (f64, f64), radius: f64) {
    let half_chord = (to.0 - from.0) / 2.;
    if radius == 0. || half_chord == 0. {
        return;
    }
    let direction = half_chord.signum();
    let cx = (from.0 + to.0) / 2.;
    let cy = from.1 - direction * (radius * radius - half_chord * half_chord).max(0.).sqrt();
    let start = (from.1 - cy).atan2(from.0 - cx);
    let end = (to.1 - cy).atan2(to.0 - cx);
    let middle = direction * std::f64::consts::FRAC_PI_2;
    for (a, b, endpoint) in [
        (start, middle, (cx, cy + direction * radius)),
        (middle, end, to),
    ] {
        let weight = ((b - a) / 2.).cos();
        let angle = (a + b) / 2.;
        let distance = radius / weight;
        path.conic_to(
            offset((cx + angle.cos() * distance, cy + angle.sin() * distance)),
            offset(endpoint),
            weight as f32,
        );
    }
}
