//! Continuous interpolation between rounded rectangle and circle outlines.

use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Lerp, Offset, Rect, Size};
use incular_rendering::{Canvas, FillRule, Paint, Path};

use super::{
    BorderRadius, BorderRadiusGeometry, BorderSide, BorderStyle, CircleBorder,
    EllipticalRoundedRect, Radius, RoundedRectangleBorder, RoundedSuperellipseBorder, ShapeBorder,
    StadiumBorder,
};

/// A rounded rectangle, rounded superellipse, circle, or transition to
/// or from a circle.
/// Intermediate contours retain their geometric parameters so successive
/// interpolations do not have to interpolate already-fitted paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundedCircleOutline(Outline);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outline {
    Rounded(RoundedRectangleBorder<BorderRadiusGeometry>),
    Superellipse(RoundedSuperellipseBorder<BorderRadiusGeometry>),
    Circle(CircleBorder),
    Stadium(StadiumBorder),
    StadiumMorph(StadiumMorph),
    Morph(Morph),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Morph {
    family: CornerFamily,
    side: BorderSide,
    radius: BorderRadiusGeometry,
    circularity: f64,
    eccentricity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StadiumMorph {
    side: BorderSide,
    fraction: f64,
    target: StadiumTarget,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StadiumTarget {
    Circle { eccentricity: f32 },
    Rounded { radius: BorderRadiusGeometry },
}

#[derive(Clone, Copy, Debug, PartialEq, Hash)]
enum CornerFamily {
    Rounded,
    Superellipse,
}

impl<R: Copy + Into<BorderRadiusGeometry>> From<RoundedRectangleBorder<R>>
    for RoundedCircleOutline
{
    fn from(value: RoundedRectangleBorder<R>) -> Self {
        Self(Outline::Rounded(value.into_radius_geometry()))
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> From<RoundedSuperellipseBorder<R>>
    for RoundedCircleOutline
{
    fn from(value: RoundedSuperellipseBorder<R>) -> Self {
        Self(Outline::Superellipse(value.into_radius_geometry()))
    }
}

impl From<CircleBorder> for RoundedCircleOutline {
    fn from(value: CircleBorder) -> Self {
        Self(Outline::Circle(value))
    }
}

impl From<StadiumBorder> for RoundedCircleOutline {
    fn from(value: StadiumBorder) -> Self {
        Self(Outline::Stadium(value))
    }
}

impl RoundedCircleOutline {
    #[must_use]
    pub fn side(self) -> BorderSide {
        match self.0 {
            Outline::Rounded(shape) => shape.side,
            Outline::Superellipse(shape) => shape.side,
            Outline::Circle(shape) => shape.side,
            Outline::Stadium(shape) => shape.side,
            Outline::StadiumMorph(shape) => shape.side,
            Outline::Morph(shape) => shape.side,
        }
    }

    /// Zero is the original rounded rectangle; one is the fitted circle.
    #[must_use]
    pub fn circularity(self) -> f64 {
        match self.0 {
            Outline::Rounded(_) | Outline::Superellipse(_) => 0.,
            Outline::Circle(_) => 1.,
            Outline::Morph(shape) => shape.circularity,
            Outline::StadiumMorph(StadiumMorph {
                target: StadiumTarget::Circle { .. },
                fraction,
                ..
            }) => fraction,
            Outline::Stadium(_) | Outline::StadiumMorph(_) => 0.,
        }
    }

    fn radius(self) -> Option<BorderRadiusGeometry> {
        match self.0 {
            Outline::Rounded(shape) => Some(shape.border_radius),
            Outline::Superellipse(shape) => Some(shape.border_radius),
            Outline::Circle(_) | Outline::Stadium(_) | Outline::StadiumMorph(_) => None,
            Outline::Morph(shape) => Some(shape.radius),
        }
    }

    fn family(self) -> Option<CornerFamily> {
        match self.0 {
            Outline::Rounded(_) => Some(CornerFamily::Rounded),
            Outline::Superellipse(_) => Some(CornerFamily::Superellipse),
            Outline::Circle(_) | Outline::Stadium(_) | Outline::StadiumMorph(_) => None,
            Outline::Morph(shape) => Some(shape.family),
        }
    }

    pub(super) fn star_transition_segments(self) -> Option<u8> {
        match self.0 {
            Outline::Stadium(_) => Some(2),
            Outline::Rounded(_) => Some(3),
            _ => None,
        }
    }

    pub(super) fn as_circle(self) -> Option<CircleBorder> {
        let Outline::Circle(circle) = self.0 else {
            return None;
        };
        Some(circle)
    }

    pub(crate) fn try_interpolate(self, other: Self, t: f32) -> Option<Self> {
        self.interpolate_supported(other, f64::from(t))
    }

    pub(super) fn scale(self, factor: f32) -> Self {
        Self(match self.0 {
            Outline::Rounded(shape) => Outline::Rounded(shape.scale(factor)),
            Outline::Superellipse(shape) => Outline::Superellipse(shape.scale(factor)),
            Outline::Circle(shape) => Outline::Circle(shape.scale(factor)),
            Outline::Stadium(shape) => Outline::Stadium(shape.scale(factor)),
            Outline::StadiumMorph(shape) => Outline::StadiumMorph(StadiumMorph {
                side: shape.side.scale(factor),
                fraction: f64::from(factor),
                target: match shape.target {
                    StadiumTarget::Rounded { radius } => StadiumTarget::Rounded {
                        radius: radius * factor,
                    },
                    target => target,
                },
            }),
            Outline::Morph(shape) => Outline::Morph(Morph {
                side: shape.side.scale(factor),
                radius: shape.radius * factor,
                circularity: f64::from(factor),
                ..shape
            }),
        })
    }

    fn has_stadium(self) -> bool {
        matches!(self.0, Outline::Stadium(_) | Outline::StadiumMorph(_))
    }

    fn stadium_target(self) -> Option<(StadiumTarget, f64)> {
        match self.0 {
            Outline::Circle(circle) => Some((
                StadiumTarget::Circle {
                    eccentricity: circle.eccentricity(),
                },
                1.,
            )),
            Outline::Rounded(rounded) => Some((
                StadiumTarget::Rounded {
                    radius: rounded.border_radius,
                },
                1.,
            )),
            Outline::StadiumMorph(morph) => Some((morph.target, morph.fraction)),
            _ => None,
        }
    }

    fn interpolate_stadium(self, other: Self, t: f64) -> Option<Self> {
        let side = self.side().lerp(&other.side(), t as f32);
        let (target, fraction) = match (self.0, other.0) {
            (Outline::Stadium(_), Outline::Stadium(_)) => {
                return Some(StadiumBorder::new(side).into());
            }
            (Outline::Stadium(_), _) => {
                let (target, fraction) = other.stadium_target()?;
                (target, fraction * t)
            }
            (_, Outline::Stadium(_)) => {
                let (target, fraction) = self.stadium_target()?;
                (target, fraction * (1. - t))
            }
            _ => {
                let (a, from) = self.stadium_target()?;
                let (b, to) = other.stadium_target()?;
                let target = match (a, b) {
                    (
                        StadiumTarget::Circle { eccentricity: a },
                        StadiumTarget::Circle { eccentricity: b },
                    ) => {
                        let eccentricity = match (self.0, other.0) {
                            (Outline::Circle(_), _) => a,
                            (_, Outline::Circle(_)) => b,
                            _ => a.lerp(&b, t as f32),
                        };
                        StadiumTarget::Circle { eccentricity }
                    }
                    (
                        StadiumTarget::Rounded { radius: a },
                        StadiumTarget::Rounded { radius: b },
                    ) => {
                        let radius = match (self.0, other.0) {
                            (Outline::Rounded(_), _) => b,
                            (_, Outline::Rounded(_)) => a,
                            _ => a.lerp(&b, t as f32),
                        };
                        StadiumTarget::Rounded { radius }
                    }
                    _ => return None,
                };
                (target, from * (1. - t) + to * t)
            }
        };
        Some(Self(Outline::StadiumMorph(StadiumMorph {
            side,
            target,
            fraction,
        })))
    }

    /// Interpolates the unfitted geometry at double precision. In particular,
    /// a rectangle's authored radius is not scaled down to zero on the way
    /// to a circle: it is blended against the circle fitted to the paint box.
    #[must_use]
    pub fn interpolate(self, other: Self, t: f64) -> Self {
        self.interpolate_supported(other, t)
            .unwrap_or(if t < 0.5 { self } else { other })
    }

    fn interpolate_supported(self, other: Self, t: f64) -> Option<Self> {
        if self.has_stadium() || other.has_stadium() {
            return self.interpolate_stadium(other, t);
        }
        match (self.0, other.0) {
            (Outline::Rounded(a), Outline::Rounded(b)) => {
                return Some(Self(Outline::Rounded(a.lerp(&b, t as f32))));
            }
            (Outline::Superellipse(a), Outline::Superellipse(b)) => {
                return Some(Self(Outline::Superellipse(a.lerp(&b, t as f32))));
            }
            (Outline::Circle(a), Outline::Circle(b)) => {
                return Some(Self(Outline::Circle(a.lerp(&b, t as f32))));
            }
            _ => {}
        }
        if self == other {
            return Some(self);
        }
        let family = match (self.family(), other.family()) {
            (Some(a), Some(b)) if a != b => return None,
            (Some(family), _) | (_, Some(family)) => family,
            (None, None) => unreachable!("two circles are handled above"),
        };
        let (a, b) = (self.circularity(), other.circularity());
        let circularity = match (self.0, other.0) {
            (Outline::Circle(_), _) => b + (1. - b) * (1. - t),
            (_, Outline::Circle(_)) => a + (1. - a) * t,
            _ => a * (1. - t) + b * t,
        };
        let eccentricity = match (self.0, other.0) {
            (Outline::Circle(circle), _) | (_, Outline::Circle(circle)) => circle.eccentricity(),
            (_, Outline::Morph(shape)) | (Outline::Morph(shape), _) => shape.eccentricity,
            _ => unreachable!("two rounded rectangles are handled above"),
        };
        let radius = match (self.radius(), other.radius()) {
            (Some(a), Some(b)) if a == b => a,
            (Some(a), Some(b)) => a.lerp(&b, t as f32),
            (Some(radius), None) | (None, Some(radius)) => radius,
            (None, None) => unreachable!("two circles are handled above"),
        };
        Some(Self(Outline::Morph(Morph {
            family,
            side: self.side().lerp(&other.side(), t as f32),
            radius,
            circularity,
            eccentricity,
        })))
    }
}

impl Lerp for RoundedCircleOutline {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self.interpolate(*other, f64::from(t))
    }
}

impl Hash for RoundedCircleOutline {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(&self.0).hash(state);
        self.side().hash(state);
        self.radius().hash(state);
        self.family().hash(state);
        let canonical_bits = |value: f64| if value == 0. { 0 } else { value.to_bits() };
        canonical_bits(self.circularity()).hash(state);
        match self.0 {
            Outline::Circle(shape) => canonical_bits(f64::from(shape.eccentricity())).hash(state),
            Outline::Morph(shape) => canonical_bits(f64::from(shape.eccentricity)).hash(state),
            Outline::StadiumMorph(morph) => {
                canonical_bits(morph.fraction).hash(state);
                std::mem::discriminant(&morph.target).hash(state);
                match morph.target {
                    StadiumTarget::Circle { eccentricity } => {
                        canonical_bits(f64::from(eccentricity)).hash(state);
                    }
                    StadiumTarget::Rounded { radius } => radius.hash(state),
                }
            }
            Outline::Rounded(_) | Outline::Superellipse(_) | Outline::Stadium(_) => {}
        }
    }
}

impl Morph {
    fn geometry(
        self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> Option<(Rect, BorderRadius)> {
        let (width, height) = (rect.size.width, rect.size.height);
        let eccentricity = self.eccentricity;
        let delta =
            (self.circularity * f64::from((width - height).abs()) * f64::from(1. - eccentricity)
                / 2.) as f32;
        let fitted = if width < height {
            (
                Offset::new(rect.origin.x, rect.origin.y + delta),
                width,
                height - 2. * delta,
            )
        } else {
            (
                Offset::new(rect.origin.x + delta, rect.origin.y),
                width - 2. * delta,
                height,
            )
        };
        let size = Size::maybe_new(fitted.1, fitted.2)?;
        let resolved = self.radius.resolve(direction.unwrap_or(TextDirection::Ltr));
        let radius = if eccentricity == 0. {
            Radius::circular(width.min(height) / 2.)
        } else if width < height {
            Radius::elliptical(width / 2., (0.5 + eccentricity / 2.) * height / 2.)
        } else {
            Radius::elliptical((0.5 + eccentricity / 2.) * width / 2., height / 2.)
        };
        let radius = resolved.lerp(&BorderRadius::all(radius), self.circularity as f32);
        Some((Rect::from_origin_size(fitted.0, size), radius))
    }

    fn path(self, rect: Rect, direction: Option<TextDirection>, inflation: f32) -> Path {
        let Some((fitted, radius)) = self.geometry(rect, direction) else {
            return Path::builder().build();
        };
        match self.family {
            CornerFamily::Rounded => radius
                .to_rounded_rect(fitted)
                .inflated_paint_path(EdgeInsets::all(inflation)),
            CornerFamily::Superellipse => {
                RoundedSuperellipseBorder::new(self.side, radius).path(fitted, inflation, direction)
            }
        }
    }
}

impl StadiumMorph {
    fn geometry(
        self,
        rect: Rect,
        direction: Option<TextDirection>,
    ) -> Option<EllipticalRoundedRect> {
        let stadium = BorderRadius::circular(rect.size.width.min(rect.size.height) / 2.);
        match self.target {
            StadiumTarget::Circle { eccentricity } => {
                let morph = Morph {
                    family: CornerFamily::Rounded,
                    side: self.side,
                    radius: stadium.into(),
                    circularity: self.fraction,
                    eccentricity,
                };
                let (rect, radius) = morph.geometry(rect, direction)?;
                Some(radius.to_rounded_rect(rect))
            }
            StadiumTarget::Rounded { radius } => {
                let resolved = radius.resolve(direction.unwrap_or(TextDirection::Ltr));
                Some(
                    stadium
                        .lerp(&resolved, self.fraction as f32)
                        .to_rounded_rect(rect),
                )
            }
        }
    }

    fn inner_inset(self) -> f32 {
        match self.target {
            StadiumTarget::Circle { .. } => self.side.stroke_inset(),
            StadiumTarget::Rounded { .. } => self.side.width * (1. - self.side.stroke_align),
        }
    }

    fn path(self, rect: Rect, direction: Option<TextDirection>, inflation: f32) -> Path {
        self.geometry(rect, direction).map_or_else(
            || Path::builder().build(),
            |rounded| rounded.inflated_paint_path(EdgeInsets::all(inflation)),
        )
    }
}

impl ShapeBorder for RoundedCircleOutline {
    fn equality_value(&self) -> Option<super::shape_borders::ShapeBorderValue<'_>> {
        Some(super::shape_borders::ShapeBorderValue::RoundedOutline(*self))
    }

    fn rounded_circle_outline(&self) -> Option<Self> {
        Some(*self)
    }

    fn needs_text_direction(&self) -> bool {
        let radius = match self.0 {
            Outline::StadiumMorph(StadiumMorph {
                target: StadiumTarget::Rounded { radius },
                ..
            }) => Some(radius),
            _ => self.radius(),
        };
        radius.is_some_and(|radius| !matches!(radius, BorderRadiusGeometry::Physical(_)))
    }

    fn hit_test(&self, rect: Rect, position: Offset, direction: Option<TextDirection>) -> bool {
        match self.0 {
            Outline::Rounded(shape) => shape.hit_test(rect, position, direction),
            Outline::Superellipse(shape) => shape.hit_test(rect, position, direction),
            Outline::Circle(shape) => shape.hit_test(rect, position, direction),
            Outline::Stadium(shape) => shape.hit_test(rect, position, direction),
            Outline::StadiumMorph(shape) => shape
                .geometry(rect, direction)
                .is_some_and(|shape| shape.contains(position)),
            Outline::Morph(shape) if shape.family == CornerFamily::Rounded => shape
                .geometry(rect, direction)
                .is_some_and(|(rect, radius)| radius.to_rounded_rect(rect).contains(position)),
            Outline::Morph(shape) => shape
                .path(rect, direction, 0.)
                .contains(position, FillRule::NonZero),
        }
    }

    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side().stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        match self.0 {
            Outline::Rounded(shape) => shape.outer_path_with_direction(rect, direction),
            Outline::Superellipse(shape) => shape.outer_path_with_direction(rect, direction),
            Outline::Circle(shape) => shape.outer_path(rect),
            Outline::Stadium(shape) => shape.outer_path(rect),
            Outline::StadiumMorph(shape) => shape.path(rect, direction, 0.),
            Outline::Morph(shape) => shape.path(rect, direction, 0.),
        }
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        match self.0 {
            Outline::Rounded(shape) => shape.inner_path_with_direction(rect, direction),
            Outline::Superellipse(shape) => shape.inner_path_with_direction(rect, direction),
            Outline::Circle(shape) => shape.inner_path(rect),
            Outline::Stadium(shape) => shape.inner_path(rect),
            Outline::StadiumMorph(shape) => shape.path(rect, direction, -shape.inner_inset()),
            Outline::Morph(shape) => {
                // Flutter's transitional shape uses the raw alignment in
                // this interpolation, unlike the ordinary outline inset.
                shape.path(
                    rect,
                    direction,
                    -shape.side.width * (1. - shape.side.stroke_align),
                )
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
    ) {
        match self.0 {
            Outline::Rounded(shape) => shape.paint_with_direction(canvas, rect, direction),
            Outline::Superellipse(shape) => shape.paint_with_direction(canvas, rect, direction),
            Outline::Circle(shape) => shape.paint(canvas, rect),
            Outline::Stadium(shape) => shape.paint(canvas, rect),
            Outline::StadiumMorph(shape) if shape.side.style == BorderStyle::Solid => {
                let path = shape.path(rect, direction, shape.side.stroke_offset() / 2.);
                canvas.draw_path(Arc::new(path), &shape.side.to_paint(), FillRule::NonZero);
            }
            Outline::StadiumMorph(_) => {}
            Outline::Morph(shape) if shape.side.style == BorderStyle::Solid => {
                let path = shape.path(rect, direction, shape.side.stroke_offset() / 2.);
                canvas.draw_path(Arc::new(path), &shape.side.to_paint(), FillRule::NonZero);
            }
            Outline::Morph(_) => {}
        }
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        direction: Option<TextDirection>,
    ) {
        canvas.draw_path(
            Arc::new(self.outer_path_with_direction(rect, direction)),
            paint,
            FillRule::NonZero,
        );
    }
}

impl std::fmt::Display for RoundedCircleOutline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shape = match self.0 {
            Outline::Morph(shape) => shape,
            Outline::StadiumMorph(shape) => {
                let percent = shape.fraction * 100.;
                write!(f, "StadiumBorder({}, ", shape.side)?;
                match shape.target {
                    StadiumTarget::Circle { eccentricity } => {
                        write!(f, "{percent:.1}% of the way to being a CircleBorder")?;
                        if eccentricity != 0. {
                            write!(f, " that is {:.1}% oval", eccentricity * 100.)?;
                        }
                    }
                    StadiumTarget::Rounded { radius } => {
                        write!(
                            f,
                            "{radius}, {percent:.1}% of the way to being a RoundedRectangleBorder"
                        )?;
                    }
                }
                return f.write_str(")");
            }
            _ => return std::fmt::Debug::fmt(&self.0, f),
        };
        let name = match shape.family {
            CornerFamily::Rounded => "RoundedRectangleBorder",
            CornerFamily::Superellipse => "RoundedSuperellipseBorder",
        };
        write!(f, "{name}(")?;
        if shape.side.style == BorderStyle::None {
            write!(f, "BorderSide(width: {:.1}, style: none)", shape.side.width)?;
        } else {
            write!(f, "{:?}", shape.side)?;
        }
        if let BorderRadiusGeometry::Physical(radius) = shape.radius
            && radius == BorderRadius::all(radius.top_left)
            && radius.top_left.x == radius.top_left.y
        {
            write!(f, ", BorderRadius.circular({:.1})", radius.top_left.x)?;
        } else {
            write!(f, ", {:?}", shape.radius)?;
        }
        write!(
            f,
            ", {:.1}% of the way to being a CircleBorder",
            shape.circularity * 100.
        )?;
        if shape.eccentricity != 0. {
            write!(f, " that is {:.1}% oval", shape.eccentricity * 100.)?;
        }
        write!(f, ")")
    }
}
