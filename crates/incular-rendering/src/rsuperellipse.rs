//! The rounded superellipse path of Flutter's engine
//! (`lib/web_ui/lib/rsuperellipse_param.dart`, mirroring
//! `round_superellipse_param.cc`): each corner is a superellipse arc joined
//! to a circular arc, both drawn as cubic Béziers.

use incular_core::{Offset, Rect};

use crate::geometry::CornerRadii;
use crate::paths::PathBuilder;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    const ZERO: Self = Self { x: 0.0, y: 0.0 };

    const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    fn distance(self) -> f64 {
        self.x.hypot(self.y)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn offset(self) -> Offset {
        Offset::new(self.x as f32, self.y as f32)
    }
}

/// An affine map built from translations, axis scales and the x/y flip
/// (`_Transform`).
#[derive(Clone, Copy, Debug)]
struct Affine {
    /// `p -> (xx * p.x + xy * p.y + tx, yx * p.x + yy * p.y + ty)`.
    xx: f64,
    xy: f64,
    yx: f64,
    yy: f64,
    tx: f64,
    ty: f64,
}

impl Affine {
    const fn translate(offset: Point) -> Self {
        Self {
            xx: 1.0,
            xy: 0.0,
            yx: 0.0,
            yy: 1.0,
            tx: offset.x,
            ty: offset.y,
        }
    }

    const fn scale(scale: Point) -> Self {
        Self {
            xx: scale.x,
            xy: 0.0,
            yx: 0.0,
            yy: scale.y,
            tx: 0.0,
            ty: 0.0,
        }
    }

    const FLIP: Self = Self {
        xx: 0.0,
        xy: 1.0,
        yx: 1.0,
        yy: 0.0,
        tx: 0.0,
        ty: 0.0,
    };

    fn apply(self, p: Point) -> Point {
        Point::new(
            self.xx * p.x + self.xy * p.y + self.tx,
            self.yx * p.x + self.yy * p.y + self.ty,
        )
    }

    /// `makeComposite(second, first)`: `first`, then `self`.
    fn then_after(self, first: Self) -> Self {
        Self {
            xx: self.xx * first.xx + self.xy * first.yx,
            xy: self.xx * first.xy + self.xy * first.yy,
            yx: self.yx * first.xx + self.yy * first.yx,
            yy: self.yx * first.xy + self.yy * first.yy,
            tx: self.xx * first.tx + self.xy * first.ty + self.tx,
            ty: self.yx * first.tx + self.yy * first.ty + self.ty,
        }
    }
}

const GAP_FACTOR: f64 = 0.292_893_218_81;

/// `_RSuperellipseOctant`.
#[derive(Clone, Copy, Debug)]
struct Octant {
    offset: Point,
    se_a: f64,
    se_n: f64,
    circle_start: Point,
    circle_center: Point,
    circle_max_angle: f64,
}

impl Octant {
    fn new(center: Point, a: f64, radius: f64) -> Self {
        if radius <= 0.0 {
            return Self {
                offset: center,
                se_a: a,
                se_n: 0.0,
                circle_start: Point::ZERO,
                circle_center: Point::ZERO,
                circle_max_angle: 0.0,
            };
        }
        let ratio = a * 2.0 / radius;
        let g = GAP_FACTOR * radius;
        let (n, xj_over_a) = compute_n_and_xj(ratio);
        let xj = xj_over_a * a;
        let yj = (1.0 - xj_over_a.powf(n)).powf(1.0 / n) * a;
        let tan_phi_j = (xj / yj).powf(n - 1.0);
        let d = (xj - tan_phi_j * yj) / (1.0 - tan_phi_j);
        let r = (a - d - g) * std::f64::consts::SQRT_2;
        let point_m = Point::new(a - g, a - g);
        let point_j = Point::new(xj, yj);
        let circle_center = find_circle_center(point_j, point_m, r);
        let circle_max_angle = angle_to(point_m.sub(circle_center), point_j.sub(circle_center));
        Self {
            offset: center,
            se_a: a,
            se_n: n,
            circle_start: point_j,
            circle_center,
            circle_max_angle,
        }
    }

    fn add_to_path(&self, path: &mut PathBuilder, external: Affine, reverse: bool, flip: bool) {
        let mut transform = external.then_after(Affine::translate(self.offset));
        if flip {
            transform = transform.then_after(Affine::FLIP);
        }
        let circle = self.circular_arc_points();
        let se = self.superellipse_arc_points();
        let mut cubic = |a: Point, b: Point, c: Point| {
            path.cubic_to(
                transform.apply(a).offset(),
                transform.apply(b).offset(),
                transform.apply(c).offset(),
            );
        };
        if reverse {
            cubic(circle[2], circle[1], circle[0]);
            cubic(se[2], se[1], se[0]);
        } else {
            cubic(se[1], se[2], se[3]);
            cubic(circle[1], circle[2], circle[3]);
        }
    }

    fn superellipse_arc_points(&self) -> [Point; 4] {
        let start = Point::new(0.0, self.se_a);
        let end = self.circle_start;
        let start_tangent = Point::new(1.0, 0.0);
        let circle_start_vector = self.circle_start.sub(self.circle_center);
        let end_tangent = Point::new(-circle_start_vector.y, circle_start_vector.x)
            .scale(1.0 / circle_start_vector.distance());
        let (start_factor, end_factor) = superellipse_bezier_factors(self.se_n);
        [
            start,
            start.add(start_tangent.scale(start_factor * self.se_a)),
            end.add(end_tangent.scale(end_factor * self.se_a)),
            end,
        ]
    }

    fn circular_arc_points(&self) -> [Point; 4] {
        let start_vector = self.circle_start.sub(self.circle_center);
        let end_vector = rotate(start_vector, -self.circle_max_angle);
        let circle_end = self.circle_center.add(end_vector);
        let start_tangent =
            Point::new(start_vector.y, -start_vector.x).scale(1.0 / start_vector.distance());
        let end_tangent =
            Point::new(-end_vector.y, end_vector.x).scale(1.0 / end_vector.distance());
        let bezier_factor = (self.circle_max_angle / 4.0).tan() * 4.0 / 3.0;
        let radius = start_vector.distance();
        [
            self.circle_start,
            self.circle_start
                .add(start_tangent.scale(bezier_factor * radius)),
            circle_end.add(end_tangent.scale(bezier_factor * radius)),
            circle_end,
        ]
    }
}

fn rotate(p: Point, radians: f64) -> Point {
    let (sine, cosine) = radians.sin_cos();
    Point::new(p.x * cosine - p.y * sine, p.x * sine + p.y * cosine)
}

fn angle_to(a: Point, b: Point) -> f64 {
    (a.x * b.y - a.y * b.x).atan2(a.x * b.x + a.y * b.y)
}

fn find_circle_center(a: Point, b: Point, r: f64) -> Point {
    let a_to_b = b.sub(a);
    let m = a.add(b).scale(0.5);
    let c_to_m = Point::new(-a_to_b.y, a_to_b.x);
    let distance_am = a_to_b.distance() / 2.0;
    let distance_cm = (r * r - distance_am * distance_am).sqrt();
    m.sub(c_to_m.scale(distance_cm / c_to_m.distance()))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn superellipse_bezier_factors(n: f64) -> (f64, f64) {
    const PRECOMPUTED: [(f64, f64); 14] = [
        (0.013_394_48, 0.059_949_73),
        (0.136_641_15, 0.135_920_82),
        (0.245_455_46, 0.140_995_16),
        (0.323_531_51, 0.128_080_21),
        (0.390_930_68, 0.117_262_64),
        (0.448_478_00, 0.108_082_78),
        (0.498_174_52, 0.100_261_75),
        (0.541_055_83, 0.093_444_29),
        (0.578_125_78, 0.087_489_84),
        (0.610_509_61, 0.082_247_22),
        (0.639_039_89, 0.077_596_39),
        (0.664_163_38, 0.073_465_30),
        (0.686_753_38, 0.069_749_96),
        (0.706_780_34, 0.065_295_12),
    ];
    const STEP: f64 = 1.0;
    const MIN_N: f64 = 2.0;
    let records = PRECOMPUTED.len();
    let max_n = MIN_N + (records - 1) as f64 * STEP;
    if n >= max_n {
        return (
            1.07 - 1.307_649_835_f64.exp() * n.powf(-0.856_851_673_1),
            -0.01 + (-0.928_769_032_2_f64).exp() * n.powf(-0.612_090_139_8),
        );
    }
    let steps = ((n - MIN_N) / STEP).clamp(0.0, (records - 1) as f64);
    let left = (steps.floor() as usize).min(records - 2);
    let frac = steps - left as f64;
    (
        (1.0 - frac) * PRECOMPUTED[left].0 + frac * PRECOMPUTED[left + 1].0,
        (1.0 - frac) * PRECOMPUTED[left].1 + frac * PRECOMPUTED[left + 1].1,
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn compute_n_and_xj(ratio: f64) -> (f64, f64) {
    const PRECOMPUTED: [(f64, f64); 11] = [
        (2.000_000_00, 1.132_766_76),
        (2.183_498_05, 1.203_119_21),
        (2.338_886_62, 1.286_987_96),
        (2.486_605_75, 1.363_519_41),
        (2.622_265_96, 1.447_179_76),
        (2.751_489_90, 1.533_858_19),
        (3.362_982_65, 1.982_882_83),
        (4.086_499_29, 2.238_118_46),
        (4.854_811_34, 2.475_634_63),
        (5.629_455_51, 2.729_485_97),
        (6.430_237_96, 2.980_204_21),
    ];
    const MIN_RATIO: f64 = 2.0;
    const FIRST_STEP_INVERSE: f64 = 10.0;
    const FIRST_MAX_RATIO: f64 = 2.5;
    const FIRST_NUM_RECORDS: f64 = 6.0;
    const SECOND_STEP_INVERSE: f64 = 2.0;
    const SECOND_MAX_RATIO: f64 = 5.0;
    const THIRD_N_SLOPE: f64 = 1.559_599_389;
    const THIRD_FACTOR_XJ_SLOPE: f64 = 0.522_807_185;
    let records = PRECOMPUTED.len();
    if ratio > SECOND_MAX_RATIO {
        let last = PRECOMPUTED[records - 1];
        let n = THIRD_N_SLOPE * (ratio - SECOND_MAX_RATIO) + last.0;
        let factor_xj = THIRD_FACTOR_XJ_SLOPE * (ratio - SECOND_MAX_RATIO) + last.1;
        return (n, 1.0 - 1.0 / factor_xj);
    }
    let ratio = ratio.clamp(MIN_RATIO, SECOND_MAX_RATIO);
    let steps = if ratio < FIRST_MAX_RATIO {
        (ratio - MIN_RATIO) * FIRST_STEP_INVERSE
    } else {
        (ratio - FIRST_MAX_RATIO) * SECOND_STEP_INVERSE + FIRST_NUM_RECORDS - 1.0
    };
    let left = (steps.floor().max(0.0) as usize).min(records - 2);
    let frac = steps - left as f64;
    let n = (1.0 - frac) * PRECOMPUTED[left].0 + frac * PRECOMPUTED[left + 1].0;
    let factor_xj = (1.0 - frac) * PRECOMPUTED[left].1 + frac * PRECOMPUTED[left + 1].1;
    (n, 1.0 - 1.0 / factor_xj)
}

/// `_RSuperellipseQuadrant` for a circular corner radius.
struct Quadrant {
    offset: Point,
    signed_scale: Point,
    top: Octant,
    right: Octant,
}

impl Quadrant {
    fn new(center: Point, corner: Point, radius: f64, sign: Point) -> Self {
        let corner_vector = corner.sub(center);
        let norm_radius = radius.abs();
        let norm_half = Point::new(corner_vector.x.abs(), corner_vector.y.abs());
        let finite_or =
            |value: f64, fallback: f64| if value.is_finite() { value } else { fallback };
        let signed_scale = Point::new(
            finite_or(corner_vector.x / norm_half.x, sign.x),
            finite_or(corner_vector.y / norm_half.y, sign.y),
        );
        let c = norm_half.x - norm_half.y;
        Self {
            offset: center,
            signed_scale,
            top: Octant::new(Point::new(0.0, -c), norm_half.x, norm_radius),
            right: Octant::new(Point::new(c, 0.0), norm_half.y, norm_radius),
        }
    }

    fn is_sharp_corner(&self) -> bool {
        self.top.se_n < 2.0 || self.right.se_n < 2.0
    }

    fn add_to_path(&self, path: &mut PathBuilder, reverse: bool) {
        let transform = Affine::translate(self.offset).then_after(Affine::scale(self.signed_scale));
        if self.is_sharp_corner() {
            let (octant, first, second) = if reverse {
                let a = self.top.se_a;
                (self.top, Point::new(a, a), Point::new(0.0, a))
            } else {
                let a = self.right.se_a;
                (self.right, Point::new(a, a), Point::new(a, 0.0))
            };
            let transform = transform.then_after(Affine::translate(octant.offset));
            path.line_to(transform.apply(first).offset());
            path.line_to(transform.apply(second).offset());
            return;
        }
        if reverse {
            self.right.add_to_path(path, transform, false, true);
            self.top.add_to_path(path, transform, true, false);
        } else {
            self.top.add_to_path(path, transform, false, false);
            self.right.add_to_path(path, transform, true, true);
        }
    }
}

fn split(left: f64, right: f64, ratio_left: f64, ratio_right: f64) -> f64 {
    if ratio_left == 0.0 && ratio_right == 0.0 {
        return (left + right) / 2.0;
    }
    (left * ratio_right + right * ratio_left) / (ratio_left + ratio_right)
}

/// `_RSuperellipsePathBuilder.exact` for circular corner radii, which are
/// first scaled down to fit the rectangle.
pub(crate) fn rounded_superellipse(builder: &mut PathBuilder, rect: Rect, radii: CornerRadii) {
    let radii = radii.normalized(rect.size);
    let left = f64::from(rect.origin.x);
    let top = f64::from(rect.origin.y);
    let right = left + f64::from(rect.size.width);
    let bottom = top + f64::from(rect.size.height);
    let tl = f64::from(radii.top_left);
    let tr = f64::from(radii.top_right);
    let br = f64::from(radii.bottom_right);
    let bl = f64::from(radii.bottom_left);
    let top_split = split(left, right, tl, tr);
    let right_split = split(top, bottom, tr, br);
    let bottom_split = split(left, right, bl, br);
    let left_split = split(top, bottom, tl, bl);
    let start = Point::new(top_split, top);
    builder.move_to(start.offset());
    Quadrant::new(
        Point::new(top_split, right_split),
        Point::new(right, top),
        tr,
        Point::new(1.0, -1.0),
    )
    .add_to_path(builder, false);
    Quadrant::new(
        Point::new(bottom_split, right_split),
        Point::new(right, bottom),
        br,
        Point::new(1.0, 1.0),
    )
    .add_to_path(builder, true);
    Quadrant::new(
        Point::new(bottom_split, left_split),
        Point::new(left, bottom),
        bl,
        Point::new(-1.0, 1.0),
    )
    .add_to_path(builder, false);
    Quadrant::new(
        Point::new(top_split, left_split),
        Point::new(left, top),
        tl,
        Point::new(-1.0, -1.0),
    )
    .add_to_path(builder, true);
    builder.line_to(start.offset());
    builder.close();
}
