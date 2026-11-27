//! The pinned Flutter native rounded superellipse geometry
//! (`round_superellipse_param.cc`): two tangent conics lead into a circular
//! cubic in each octant. The native fit avoids the older web cubic's
//! protrusion over the straight edge at some radius-to-size ratios.

use incular_core::{Offset, Rect, Size};

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

    #[expect(
        clippy::cast_possible_truncation,
        reason = "path geometry is f32 while the Flutter superellipse math is f64"
    )]
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
const NATIVE_GEOMETRY_TOLERANCE: f64 = 1e-3;

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
        if radius <= NATIVE_GEOMETRY_TOLERANCE {
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
        let [from_edge, to_circle] = self.superellipse_conics();
        let map = |p: Point| transform.apply(p).offset();
        if reverse {
            path.cubic_to(map(circle[2]), map(circle[1]), map(circle[0]));
            path.conic_to(
                map(to_circle.control),
                map(to_circle.from),
                to_circle.weight,
            );
            path.conic_to(
                map(from_edge.control),
                map(from_edge.from),
                from_edge.weight,
            );
        } else {
            path.conic_to(map(from_edge.control), map(from_edge.to), from_edge.weight);
            path.conic_to(map(to_circle.control), map(to_circle.to), to_circle.weight);
            path.cubic_to(map(circle[1]), map(circle[2]), map(circle[3]));
        }
    }

    fn superellipse_conics(&self) -> [Conic; 2] {
        let start = Point::new(0.0, self.se_a);
        let joint = self.circle_start;
        let (first, second, split_y) =
            superellipse_conic_factors(self.se_n, joint.x / self.se_a, joint.y / self.se_a);
        let split = Point::new(
            (1.0 - split_y.powf(self.se_n)).powf(1.0 / self.se_n) * self.se_a,
            split_y * self.se_a,
        );
        let slope = |p: Point| -(p.x / p.y).powf(self.se_n - 1.0);
        [
            Conic {
                from: start,
                control: tangent_intersection(start, 0.0, split, slope(split)),
                to: split,
                weight: first as f32,
            },
            Conic {
                from: split,
                control: tangent_intersection(split, slope(split), joint, slope(joint)),
                to: joint,
                weight: second as f32,
            },
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

    fn contains(&self, point: Point) -> bool {
        if point.x < 0. || point.y < 0. || point.y < point.x {
            return true;
        }
        if point.x <= self.circle_start.x {
            let x = point.x / self.se_a;
            let y = point.y / self.se_a;
            return x.powf(self.se_n) + y.powf(self.se_n) <= 1.;
        }
        let delta = point.sub(self.circle_center);
        let radius = self.circle_start.sub(self.circle_center);
        delta.x * delta.x + delta.y * delta.y < radius.x * radius.x + radius.y * radius.y
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

struct Conic {
    from: Point,
    control: Point,
    to: Point,
    weight: f32,
}

fn tangent_intersection(a: Point, slope_a: f64, b: Point, slope_b: f64) -> Point {
    if (slope_a - slope_b).abs() < NATIVE_GEOMETRY_TOLERANCE {
        return a.add(b).scale(0.5);
    }
    let x = (slope_a * a.x - slope_b * b.x + b.y - a.y) / (slope_a - slope_b);
    Point::new(x, a.y + slope_a * (x - a.x))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the conic table index is clamped to the finite table"
)]
fn superellipse_conic_factors(n: f64, joint_x: f64, joint_y: f64) -> (f64, f64, f64) {
    // Native control-weight fits for exponents 2 through 14.
    const WEIGHTS: [(f64, f64); 13] = [
        (0.7078, 8.3194),
        (0.7895, 2.4523),
        (0.8379, 1.8528),
        (0.8701, 1.6891),
        (0.8932, 1.5806),
        (0.9107, 1.5043),
        (0.9244, 1.4470),
        (0.9355, 1.4037),
        (0.9448, 1.3701),
        (0.9526, 1.3431),
        (0.9594, 1.3212),
        (0.9653, 1.3032),
        (0.9705, 1.2880),
    ];
    let exponent = n.clamp(2.0, 14.0);
    let steps = exponent - 2.0;
    let lower = (steps.floor() as usize).min(WEIGHTS.len() - 2);
    let mix = steps - lower as f64;
    let root = exponent.sqrt();
    let (lower_first, lower_second) = WEIGHTS[lower];
    let (upper_first, upper_second) = WEIGHTS[lower + 1];
    (
        (1.0 - mix) * lower_first + mix * upper_first * root,
        (1.0 - mix) * lower_second + mix * upper_second * joint_x,
        (root + joint_y) / (root + 1.0),
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "Flutter superellipse tables are f64 and the table index is clamped"
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

/// `_RSuperellipseQuadrant`, normalized to an affine circular basis.
struct Quadrant {
    offset: Point,
    signed_scale: Point,
    top: Octant,
    right: Octant,
}

impl Quadrant {
    fn new(center: Point, corner: Point, radius: Point, sign: Point) -> Self {
        let corner_vector = corner.sub(center);
        let radius = Point::new(
            radius.x.abs().min(corner_vector.x.abs()),
            radius.y.abs().min(corner_vector.y.abs()),
        );
        let norm_radius = radius.x.min(radius.y);
        let scale = if norm_radius > 0. {
            Point::new(radius.x / norm_radius, radius.y / norm_radius)
        } else {
            Point::new(1., 1.)
        };
        let norm_half = Point::new(
            corner_vector.x.abs() / scale.x,
            corner_vector.y.abs() / scale.y,
        );
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

    fn contains(&self, point: Point, reflect: bool) -> bool {
        let delta = point.sub(self.offset);
        let mut normalized =
            Point::new(delta.x / self.signed_scale.x, delta.y / self.signed_scale.y);
        if reflect {
            normalized = Point::new(normalized.x.abs(), normalized.y.abs());
        } else if normalized.x < 0. || normalized.y < 0. {
            return true;
        }

        if self.is_sharp_corner() {
            let x = self.right.offset.x + self.right.se_a - normalized.x;
            let y = self.top.offset.y + self.top.se_a - normalized.y;
            return (x > 0. || (x == 0. && self.signed_scale.x < 0.))
                && (y > 0. || (y == 0. && self.signed_scale.y < 0.));
        }
        let right = normalized.sub(self.right.offset);
        self.top.contains(normalized.sub(self.top.offset))
            && self.right.contains(Point::new(right.y, right.x))
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

fn corner_quadrants(rect: Rect, [tl, tr, br, bl]: [Point; 4]) -> (Point, [Quadrant; 4]) {
    let left = f64::from(rect.origin.x);
    let top = f64::from(rect.origin.y);
    let right = left + f64::from(rect.size.width);
    let bottom = top + f64::from(rect.size.height);
    let top_split = split(left, right, tl.x, tr.x);
    let right_split = split(top, bottom, tr.y, br.y);
    let bottom_split = split(left, right, bl.x, br.x);
    let left_split = split(top, bottom, tl.y, bl.y);
    (
        Point::new(top_split, top),
        [
            Quadrant::new(
                Point::new(top_split, right_split),
                Point::new(right, top),
                tr,
                Point::new(1., -1.),
            ),
            Quadrant::new(
                Point::new(bottom_split, right_split),
                Point::new(right, bottom),
                br,
                Point::new(1., 1.),
            ),
            Quadrant::new(
                Point::new(bottom_split, left_split),
                Point::new(left, bottom),
                bl,
                Point::new(-1., 1.),
            ),
            Quadrant::new(
                Point::new(top_split, left_split),
                Point::new(left, top),
                tl,
                Point::new(-1., -1.),
            ),
        ],
    )
}

/// Native Flutter's analytical rounded-superellipse containment. Radii are
/// clockwise starting at top left. Hit testing uses the raw radii with
/// per-quadrant limits; painting normalizes all four together. Oversized
/// radii can therefore give different results from the painted path.
#[must_use]
pub fn rounded_superellipse_contains(rect: Rect, radii: [Size; 4], position: Offset) -> bool {
    let left = f64::from(rect.origin.x);
    let top = f64::from(rect.origin.y);
    let right = left + f64::from(rect.size.width);
    let bottom = top + f64::from(rect.size.height);
    let point = Point::new(f64::from(position.x), f64::from(position.y));
    if !(left <= point.x && point.x < right && top <= point.y && point.y < bottom) {
        return false;
    }
    let radii = radii.map(|radii| Point::new(f64::from(radii.width), f64::from(radii.height)));
    let radius = radii[0];
    if radius.x > 0. && radius.y > 0. && radii.iter().all(|&other| other == radius) {
        let center = Point::new((left + right) / 2., (top + bottom) / 2.);
        return Quadrant::new(center, Point::new(right, top), radius, Point::new(1., -1.))
            .contains(point, true);
    }
    let (_, corners) = corner_quadrants(rect, radii);
    corners.iter().all(|corner| corner.contains(point, false))
}

/// Builds a circular-corner path through the same axis-aware implementation.
pub(crate) fn rounded_superellipse(builder: &mut PathBuilder, rect: Rect, radii: CornerRadii) {
    rounded_superellipse_elliptical(
        builder,
        rect,
        [
            radii.top_left,
            radii.top_right,
            radii.bottom_right,
            radii.bottom_left,
        ]
        .map(|radius| Size {
            width: radius,
            height: radius,
        }),
    );
}

/// Flutter normalizes all corner radii together, dropping a flat corner
/// before its unused axis can force neighboring corners to shrink.
pub(crate) fn rounded_superellipse_elliptical(
    builder: &mut PathBuilder,
    rect: Rect,
    radii: [Size; 4],
) {
    let [tl, tr, br, bl] = if rect.size.width <= 0.
        || rect.size.height <= 0.
        || radii
            .iter()
            .any(|r| !r.width.is_finite() || !r.height.is_finite())
    {
        [Point::ZERO; 4]
    } else {
        radii.map(|r| {
            if r.width <= 0. || r.height <= 0. {
                Point::ZERO
            } else {
                Point::new(f64::from(r.width), f64::from(r.height))
            }
        })
    };
    let width = f64::from(rect.size.width);
    let height = f64::from(rect.size.height);
    let scale = [
        (width, tl.x + tr.x),
        (width, bl.x + br.x),
        (height, tl.y + bl.y),
        (height, tr.y + br.y),
    ]
    .into_iter()
    .fold(1.0_f64, |scale, (dimension, sum)| {
        if sum > dimension {
            scale.min(dimension / sum)
        } else {
            scale
        }
    });
    let radii = [tl, tr, br, bl].map(|radius| radius.scale(scale));
    let (start, corners) = corner_quadrants(rect, radii);
    builder.move_to(start.offset());
    for (index, corner) in corners.iter().enumerate() {
        corner.add_to_path(builder, index % 2 == 1);
    }
    builder.line_to(start.offset());
    builder.close();
}
