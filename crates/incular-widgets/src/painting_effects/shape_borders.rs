//! Flutter's `StadiumBorder` (`painting/stadium_border.dart`), `StarBorder`
//! (`painting/star_border.dart`) and `RoundedSuperellipseBorder`
//! (`painting/rounded_rectangle_border.dart`).

use std::{f64::consts::PI, sync::Arc};

use incular_core::{Offset, Rect, Size};
use incular_rendering::{Canvas, CornerRadii, Path, Stroke};

use super::decoration::{BorderSide, BorderStyle};
use super::geometry::BorderRadius;

/// Flutter's `ShapeBorder` for the outlined shapes Incular paints itself:
/// the shape's outline and the area inside its stroke, and the stroke.
pub trait ShapeBorder {
    /// Flutter `getOuterPath`: the shape's outline in `rect`.
    fn outer_path(&self, rect: Rect) -> Path;

    /// Flutter `getInnerPath`: the outline inset by the stroke's inner
    /// width.
    fn inner_path(&self, rect: Rect) -> Path;

    /// Flutter `paint`: strokes the side along the shape.
    fn paint(&self, canvas: &mut Canvas, rect: Rect);
}

/// Flutter `Rect.inflate`; a negative `delta` deflates.
fn inflate(rect: Rect, delta: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(rect.origin.x - delta, rect.origin.y - delta),
        Size::new(
            rect.size.width + 2.0 * delta,
            rect.size.height + 2.0 * delta,
        ),
    )
}

/// Flutter `RRect.inflate`: the radii grow with the rectangle and stop at
/// zero.
fn inflate_radii(radii: CornerRadii, delta: f32) -> CornerRadii {
    CornerRadii {
        top_left: (radii.top_left + delta).max(0.0),
        top_right: (radii.top_right + delta).max(0.0),
        bottom_right: (radii.bottom_right + delta).max(0.0),
        bottom_left: (radii.bottom_left + delta).max(0.0),
    }
}

fn shortest_side(rect: Rect) -> f32 {
    rect.size.width.min(rect.size.height)
}

fn rounded_rect_path(rect: Rect, radii: CornerRadii) -> Path {
    let mut builder = Path::builder();
    builder.rounded_rect(rect, radii);
    builder.build()
}

fn rect_path(rect: Rect) -> Path {
    rounded_rect_path(rect, CornerRadii::ZERO)
}

/// `BorderSide.toPaint` applied to a path.
fn stroke(canvas: &mut Canvas, side: &BorderSide, path: Path) {
    canvas.stroke_path(
        Arc::new(path),
        side.color,
        Stroke {
            width: side.width,
            ..Stroke::default()
        },
    );
}

/// Flutter's `StadiumBorder`: semicircular ends on the shorter sides.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StadiumBorder {
    pub side: BorderSide,
}

impl StadiumBorder {
    #[must_use]
    pub const fn new(side: BorderSide) -> Self {
        Self { side }
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t))
    }

    fn radii(rect: Rect) -> CornerRadii {
        CornerRadii::uniform(shortest_side(rect) / 2.0)
    }
}

impl ShapeBorder for StadiumBorder {
    fn outer_path(&self, rect: Rect) -> Path {
        rounded_rect_path(rect, Self::radii(rect))
    }

    fn inner_path(&self, rect: Rect) -> Path {
        let inset = self.side.stroke_inset();
        rounded_rect_path(
            inflate(rect, -inset),
            inflate_radii(Self::radii(rect), -inset),
        )
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let delta = self.side.stroke_offset() / 2.0;
        stroke(
            canvas,
            &self.side,
            rounded_rect_path(
                inflate(rect, delta),
                inflate_radii(Self::radii(rect), delta),
            ),
        );
    }
}

impl incular_core::Lerp for StadiumBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(self.side.lerp(&other.side, t))
    }
}

/// Flutter's `RoundedSuperellipseBorder`: a rectangle whose corners are
/// rounded superellipses, the continuous corners of iOS.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedSuperellipseBorder {
    pub side: BorderSide,
    /// Circular corner radii; Incular's corner geometry has no elliptical
    /// radii, so each corner uses its horizontal radius.
    pub border_radius: BorderRadius,
}

impl RoundedSuperellipseBorder {
    #[must_use]
    pub const fn new(side: BorderSide, border_radius: BorderRadius) -> Self {
        Self {
            side,
            border_radius,
        }
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        let radius =
            |r: super::geometry::Radius| super::geometry::Radius::elliptical(r.x * t, r.y * t);
        Self::new(
            self.side.scale(t),
            BorderRadius::only(
                radius(self.border_radius.top_left),
                radius(self.border_radius.top_right),
                radius(self.border_radius.bottom_right),
                radius(self.border_radius.bottom_left),
            ),
        )
    }

    fn path(&self, rect: Rect, delta: f32) -> Path {
        if self.border_radius == BorderRadius::ZERO {
            return rect_path(inflate(rect, delta));
        }
        let radii = inflate_radii(self.border_radius.to_corner_radii(), delta);
        let mut builder = Path::builder();
        builder.rounded_superellipse(inflate(rect, delta), radii);
        builder.build()
    }
}

impl ShapeBorder for RoundedSuperellipseBorder {
    fn outer_path(&self, rect: Rect) -> Path {
        self.path(rect, 0.0)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.path(rect, -self.side.stroke_inset())
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let offset = (self.side.stroke_outset() - self.side.stroke_inset()) / 2.0;
        stroke(canvas, &self.side, self.path(rect, offset));
    }
}

impl incular_core::Lerp for RoundedSuperellipseBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            self.side.lerp(&other.side, t),
            self.border_radius.lerp(&other.border_radius, t),
        )
    }
}

/// Flutter's `StarBorder`: a star, or with [`StarBorder::polygon`] a regular
/// polygon, with optionally rounded points and valleys.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarBorder {
    pub side: BorderSide,
    points: f32,
    /// `None` for a polygon, whose inner radius is its incircle's.
    inner_radius_ratio: Option<f32>,
    point_rounding: f32,
    valley_rounding: f32,
    rotation_radians: f32,
    squash: f32,
}

impl Default for StarBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl StarBorder {
    /// A five-pointed star with inner radius ratio 0.4.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            side: BorderSide::NONE,
            points: 5.0,
            inner_radius_ratio: Some(0.4),
            point_rounding: 0.0,
            valley_rounding: 0.0,
            rotation_radians: 0.0,
            squash: 0.0,
        }
    }

    /// Flutter `StarBorder.polygon`: five sides by default.
    #[must_use]
    pub const fn polygon() -> Self {
        Self {
            inner_radius_ratio: None,
            ..Self::new()
        }
    }

    #[must_use]
    pub const fn side(mut self, side: BorderSide) -> Self {
        self.side = side;
        self
    }

    /// The number of points, or of a polygon's sides; at least 2 and
    /// possibly fractional.
    #[must_use]
    pub fn points(mut self, points: f32) -> Self {
        assert!(points >= 2.0, "a star has at least two points");
        self.points = points;
        self
    }

    /// The valleys' radius as a fraction of the points', from 0 to 1.
    #[must_use]
    pub fn inner_radius_ratio(mut self, ratio: f32) -> Self {
        assert!((0.0..=1.0).contains(&ratio));
        self.inner_radius_ratio = Some(ratio);
        self
    }

    /// From 0 to 1; with the valley rounding at most 1.
    #[must_use]
    pub fn point_rounding(mut self, rounding: f32) -> Self {
        assert!((0.0..=1.0).contains(&rounding));
        assert!(rounding + self.valley_rounding <= 1.0);
        self.point_rounding = rounding;
        self
    }

    /// From 0 to 1; with the point rounding at most 1.
    #[must_use]
    pub fn valley_rounding(mut self, rounding: f32) -> Self {
        assert!((0.0..=1.0).contains(&rounding));
        assert!(rounding + self.point_rounding <= 1.0);
        self.valley_rounding = rounding;
        self
    }

    /// Clockwise rotation in degrees.
    #[must_use]
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation_radians = degrees.to_radians();
        self
    }

    /// How far the star stretches to the rectangle's longer side, from 0
    /// (kept round) to 1 (filling it).
    #[must_use]
    pub fn squash(mut self, squash: f32) -> Self {
        assert!((0.0..=1.0).contains(&squash));
        self.squash = squash;
        self
    }

    #[must_use]
    pub const fn point_count(&self) -> f32 {
        self.points
    }

    /// The valleys' radius ratio; a polygon's is `cos(π / sides)`.
    #[must_use]
    pub fn inner_ratio(&self) -> f32 {
        self.inner_radius_ratio
            .unwrap_or_else(|| (std::f32::consts::PI / self.points).cos())
    }

    /// The rotation in degrees.
    #[must_use]
    pub fn rotation_degrees(&self) -> f32 {
        self.rotation_radians.to_degrees()
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self {
            side: self.side.scale(t),
            inner_radius_ratio: Some(self.inner_ratio()),
            ..*self
        }
    }

    fn generate(&self, rect: Rect) -> Path {
        StarGenerator {
            points: f64::from(self.points),
            inner_radius_ratio: f64::from(self.inner_ratio()),
            point_rounding: f64::from(self.point_rounding),
            valley_rounding: f64::from(self.valley_rounding),
            rotation: f64::from(self.rotation_radians),
            squash: f64::from(self.squash),
        }
        .generate(rect)
    }
}

impl ShapeBorder for StarBorder {
    fn outer_path(&self, rect: Rect) -> Path {
        self.generate(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.generate(inflate(rect, -self.side.stroke_inset()))
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let path = self.generate(inflate(rect, self.side.stroke_offset() / 2.0));
        stroke(canvas, &self.side, path);
    }
}

impl incular_core::Lerp for StarBorder {
    /// Flutter's `StarBorder` to `StarBorder` lerp.
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0.0 {
            return *self;
        }
        if t == 1.0 {
            return *other;
        }
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            side: self.side.lerp(&other.side, t),
            points: mix(self.points, other.points),
            inner_radius_ratio: Some(mix(self.inner_ratio(), other.inner_ratio())),
            point_rounding: mix(self.point_rounding, other.point_rounding),
            valley_rounding: mix(self.valley_rounding, other.valley_rounding),
            rotation_radians: mix(self.rotation_radians, other.rotation_radians),
            squash: mix(self.squash, other.squash),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
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
}

/// `_PointInfo`.
#[derive(Clone, Copy, Debug)]
struct PointInfo {
    valley: Point,
    point: Point,
    valley_arc1: Point,
    point_arc1: Point,
    point_arc2: Point,
    valley_arc2: Point,
}

/// `_StarGenerator`.
struct StarGenerator {
    points: f64,
    inner_radius_ratio: f64,
    point_rounding: f64,
    valley_rounding: f64,
    rotation: f64,
    squash: f64,
}

impl StarGenerator {
    #[allow(clippy::cast_possible_truncation)]
    fn generate(&self, rect: Rect) -> Path {
        let width = f64::from(rect.size.width);
        let height = f64::from(rect.size.height);
        let shortest = width.min(height);
        let radius = shortest / 2.0;
        let center = Point::new(
            f64::from(rect.origin.x) + width / 2.0,
            f64::from(rect.origin.y) + height / 2.0,
        );
        const MIN_INNER_RADIUS_RATIO: f64 = 0.002;
        let mapped_inner_radius_ratio =
            self.inner_radius_ratio * (1.0 - MIN_INNER_RADIUS_RATIO) + MIN_INNER_RADIUS_RATIO;
        let mut points = Vec::new();
        let max_diameter = 2.0
            * self.generate_points(
                &mut points,
                center,
                radius,
                radius * mapped_inner_radius_ratio,
            );
        let mut scale = Point::new(width / max_diameter, height / max_diameter);
        if shortest == width {
            scale = Point::new(
                scale.x,
                self.squash * scale.y + (1.0 - self.squash) * scale.x,
            );
        } else {
            scale = Point::new(
                self.squash * scale.x + (1.0 - self.squash) * scale.y,
                scale.y,
            );
        }
        // translate(center) · scale · rotateZ(rotation) · translate(-center).
        let (sine, cosine) = self.rotation.sin_cos();
        let map = |p: Point| {
            let local = p.sub(center);
            let rotated = Point::new(
                local.x * cosine - local.y * sine,
                local.x * sine + local.y * cosine,
            );
            let mapped = Point::new(rotated.x * scale.x, rotated.y * scale.y).add(center);
            Offset::new(mapped.x as f32, mapped.y as f32)
        };
        let mut builder = Path::builder();
        self.draw_points(&mut builder, &points, map);
        builder.build()
    }

    fn generate_points(
        &self,
        point_list: &mut Vec<PointInfo>,
        center: Point,
        radius: f64,
        inner_radius: f64,
    ) -> f64 {
        let step = PI / self.points;
        let mut angle = -PI / 2.0 - step;
        let mut valley = Point::new(
            center.x + angle.cos() * inner_radius,
            center.y + angle.sin() * inner_radius,
        );
        let add_point = |point_list: &mut Vec<PointInfo>,
                         valley: &mut Point,
                         point_angle: f64,
                         point_step: f64,
                         point_radius: f64,
                         point_inner_radius: f64| {
            let mut point_angle = point_angle + point_step;
            let point = Point::new(
                center.x + point_angle.cos() * point_radius,
                center.y + point_angle.sin() * point_radius,
            );
            point_angle += point_step;
            let next_valley = Point::new(
                center.x + point_angle.cos() * point_inner_radius,
                center.y + point_angle.sin() * point_inner_radius,
            );
            let valley_arc1 = valley.add(point.sub(*valley).scale(self.valley_rounding));
            let point_arc1 = point.add(valley.sub(point).scale(self.point_rounding));
            let point_arc2 = point.add(next_valley.sub(point).scale(self.point_rounding));
            let valley_arc2 = next_valley.add(point.sub(next_valley).scale(self.valley_rounding));
            point_list.push(PointInfo {
                valley: *valley,
                point,
                valley_arc1,
                point_arc1,
                point_arc2,
                valley_arc2,
            });
            *valley = next_valley;
            point_angle
        };
        let remainder = self.points - self.points.trunc();
        let has_integer_sides = remainder < 1e-6;
        let whole_sides = self.points - if has_integer_sides { 0.0 } else { 1.0 };
        let mut i = 0.0;
        while i < whole_sides {
            angle = add_point(point_list, &mut valley, angle, step, radius, inner_radius);
            i += 1.0;
        }
        let this_point = point_list[0];
        let next_point = point_list[1];
        let point_midpoint = curve_midpoint(
            this_point.valley,
            this_point.point,
            next_point.valley,
            this_point.point_arc1,
            this_point.point_arc2,
        );
        let valley_midpoint = curve_midpoint(
            this_point.point,
            next_point.valley,
            next_point.point,
            this_point.valley_arc2,
            next_point.valley_arc1,
        );
        let valley_radius = valley_midpoint.sub(center).distance();
        let point_radius = point_midpoint.sub(center).distance();
        if !has_integer_sides {
            let effective_inner_radius = valley_radius.max(inner_radius);
            let ending_radius =
                effective_inner_radius + remainder * (radius - effective_inner_radius);
            add_point(
                point_list,
                &mut valley,
                angle,
                step * remainder,
                ending_radius,
                inner_radius,
            );
        }
        valley_radius
            .max(point_radius)
            .clamp(f64::MIN_POSITIVE, f64::MAX)
    }

    #[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
    fn draw_points(
        &self,
        builder: &mut incular_rendering::PathBuilder,
        points: &[PointInfo],
        map: impl Fn(Point) -> Offset,
    ) {
        builder.move_to(map(points[0].point_arc1));
        let point_angle = get_angle(points[0].valley, points[0].point, points[1].valley);
        let point_weight = get_weight(point_angle) as f32;
        let valley_angle = get_angle(points[1].point, points[1].valley, points[0].point);
        let valley_weight = get_weight(valley_angle) as f32;
        // Flutter compares these radian angles with 180 as well as 0.
        for (i, point) in points.iter().enumerate() {
            let next_point = points[(i + 1) % points.len()];
            builder.line_to(map(point.point_arc1));
            if point_angle != 180.0 && point_angle != 0.0 {
                builder.conic_to(map(point.point), map(point.point_arc2), point_weight);
            } else {
                builder.line_to(map(point.point_arc2));
            }
            builder.line_to(map(point.valley_arc2));
            if valley_angle != 180.0 && valley_angle != 0.0 {
                builder.conic_to(
                    map(next_point.valley),
                    map(next_point.valley_arc1),
                    valley_weight,
                );
            } else {
                builder.line_to(map(next_point.valley_arc1));
            }
        }
        builder.close();
    }
}

/// The rational quadratic Bézier's midpoint, which gives the radius the
/// rounded points and valleys actually reach.
fn curve_midpoint(a: Point, b: Point, c: Point, a1: Point, c1: Point) -> Point {
    let w = get_weight(get_angle(a, b, c)) / 2.0;
    a1.scale(0.25)
        .add(b.scale(w))
        .add(c1.scale(0.25))
        .scale(1.0 / (0.5 + w))
}

fn get_weight(angle: f64) -> f64 {
    ((angle / 2.0) % (PI / 2.0)).cos()
}

/// The included angle between points ABC in radians.
#[allow(clippy::float_cmp)]
fn get_angle(a: Point, b: Point, c: Point) -> f64 {
    if a == c || b == c || b == a {
        return 0.0;
    }
    let u = a.sub(b);
    let v = c.sub(b);
    let dot = u.x * v.x + u.y * v.y;
    let m1 = if b.x == a.x {
        f64::INFINITY
    } else {
        -u.y / -u.x
    };
    let m2 = if b.x == c.x {
        f64::INFINITY
    } else {
        -v.y / -v.x
    };
    let mut angle = (m1 - m2).atan2(1.0 + m1 * m2).abs();
    if dot < 0.0 {
        angle += PI;
    }
    angle
}
