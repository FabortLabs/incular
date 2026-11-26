//! Flutter's `StadiumBorder` (`painting/stadium_border.dart`), `StarBorder`
//! (`painting/star_border.dart`) and `RoundedSuperellipseBorder`
//! (`painting/rounded_rectangle_border.dart`), and the circle, oval,
//! beveled, continuous and linear borders.

use std::{
    f64::consts::PI,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

use incular_config::{EdgeInsets, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, CornerRadii, FillRule, Paint, PaintStyle, Path, Stroke};
use smart_default::SmartDefault;

use super::decoration::{Border, BorderSide, BorderStyle, BoxShape};
use super::geometry::{BorderRadius, BorderRadiusGeometry, Radius};

/// Flutter's `ShapeBorder` for the outlined shapes Incular paints itself:
/// the shape's outline and the area inside its stroke, and the stroke.
pub trait ShapeBorder {
    /// The space consumed along each edge before a nested border is painted.
    fn dimensions(&self, _direction: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::ZERO
    }

    /// Flutter `getOuterPath`: the shape's outline in `rect`.
    fn outer_path(&self, rect: Rect) -> Path;

    /// Resolves the outline for callers that supply an inherited direction.
    /// Direction-independent borders can keep the default implementation.
    fn outer_path_with_direction(&self, rect: Rect, _direction: Option<TextDirection>) -> Path {
        self.outer_path(rect)
    }

    /// Flutter `getInnerPath`: the outline inset by the stroke's inner
    /// width.
    fn inner_path(&self, rect: Rect) -> Path;

    fn inner_path_with_direction(&self, rect: Rect, _direction: Option<TextDirection>) -> Path {
        self.inner_path(rect)
    }

    /// Whether the interior has an equivalent specialized painting path.
    fn prefer_paint_interior(&self) -> bool {
        false
    }

    /// Specialized alternative to filling the outer path. Called only when
    /// `prefer_paint_interior` is true.
    fn paint_interior(
        &self,
        _canvas: &mut Canvas,
        _rect: Rect,
        _paint: &Paint,
        _direction: Option<TextDirection>,
    ) {
        panic!("preferred ShapeBorder interior has no paint implementation");
    }

    /// Flutter `paint`: strokes the side along the shape.
    fn paint(&self, canvas: &mut Canvas, rect: Rect);

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        _direction: Option<TextDirection>,
    ) {
        self.paint(canvas, rect);
    }
}

/// Incompatible shape borders layered from outside to inside, like Flutter's
/// compound shape border. Every inner border is inset by its outer neighbors.
#[derive(Clone)]
pub struct CompoundBorder {
    borders: Vec<Rc<dyn ShapeBorder>>,
}

impl CompoundBorder {
    #[must_use]
    pub fn new(outside: impl ShapeBorder + 'static, inside: impl ShapeBorder + 'static) -> Self {
        Self {
            borders: vec![Rc::new(outside), Rc::new(inside)],
        }
    }
}

impl ShapeBorder for CompoundBorder {
    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        self.borders.iter().fold(EdgeInsets::ZERO, |total, border| {
            let next = border.dimensions(direction);
            EdgeInsets::only(
                total.left + next.left,
                total.top + next.top,
                total.right + next.right,
                total.bottom + next.bottom,
            )
        })
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.borders[0].outer_path_with_direction(rect, direction)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, mut rect: Rect, direction: Option<TextDirection>) -> Path {
        let (inside, outside) = self
            .borders
            .split_last()
            .expect("a compound border has layers");
        for border in outside {
            rect = deflate_border(rect, border.dimensions(direction));
        }
        inside.inner_path_with_direction(rect, direction)
    }

    fn prefer_paint_interior(&self) -> bool {
        self.borders
            .iter()
            .all(|border| border.prefer_paint_interior())
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        direction: Option<TextDirection>,
    ) {
        self.borders[0].paint_interior(canvas, rect, paint, direction);
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        mut rect: Rect,
        direction: Option<TextDirection>,
    ) {
        for border in &self.borders {
            border.paint_with_direction(canvas, rect, direction);
            rect = deflate_border(rect, border.dimensions(direction));
        }
    }
}

/// The result of adding two physical box borders. Compatible sides merge
/// without allocating or painting an extra stroke; incompatible sides
/// stay as a physical pair with the right operand on the outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorderComposition {
    Merged(Border),
    Layered { outside: Border, inside: Border },
}

impl std::ops::Add for Border {
    type Output = BorderComposition;

    fn add(self, outside: Self) -> Self::Output {
        if self.top.can_merge(&outside.top)
            && self.right.can_merge(&outside.right)
            && self.bottom.can_merge(&outside.bottom)
            && self.left.can_merge(&outside.left)
        {
            BorderComposition::Merged(self.merge(outside))
        } else {
            BorderComposition::Layered {
                outside,
                inside: self,
            }
        }
    }
}

impl ShapeBorder for Border {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        Border::dimensions(*self)
    }

    fn outer_path(&self, rect: Rect) -> Path {
        Border::outer_path(*self, rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        Border::inner_path(*self, rect)
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _: Option<TextDirection>,
    ) {
        if let (PaintStyle::Fill, Brush::Solid(color)) = (paint.style_value(), paint.brush_value())
        {
            canvas.rect(rect, *color);
        } else {
            canvas.draw_path(Arc::new(rect_path(rect)), paint, FillRule::NonZero);
        }
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        let _ = Border::paint(*self, canvas, rect, BoxShape::Rectangle, None);
    }
}

impl ShapeBorder for BorderComposition {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        match self {
            Self::Merged(border) => Border::dimensions(*border),
            Self::Layered { outside, inside } => {
                let a = Border::dimensions(*outside);
                let b = Border::dimensions(*inside);
                EdgeInsets::only(
                    a.left + b.left,
                    a.top + b.top,
                    a.right + b.right,
                    a.bottom + b.bottom,
                )
            }
        }
    }

    fn outer_path(&self, rect: Rect) -> Path {
        rect_path(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        match self {
            Self::Merged(border) => Border::inner_path(*border, rect),
            Self::Layered { outside, inside } => {
                Border::inner_path(*inside, deflate_border(rect, Border::dimensions(*outside)))
            }
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
        let outside = match self {
            Self::Merged(border)
            | Self::Layered {
                outside: border, ..
            } => border,
        };
        outside.paint_interior(canvas, rect, paint, direction);
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        match self {
            Self::Merged(border) => ShapeBorder::paint(border, canvas, rect),
            Self::Layered { outside, inside } => {
                ShapeBorder::paint(outside, canvas, rect);
                ShapeBorder::paint(
                    inside,
                    canvas,
                    deflate_border(rect, Border::dimensions(*outside)),
                );
            }
        }
    }
}

fn deflate_border(rect: Rect, insets: EdgeInsets) -> Rect {
    Rect::from_origin_size(
        Offset::new(rect.origin.x + insets.left, rect.origin.y + insets.top),
        Size::new(
            (rect.size.width - insets.left - insets.right).max(0.),
            (rect.size.height - insets.top - insets.bottom).max(0.),
        ),
    )
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

/// Signed edge coordinates preserve inverted inner bounds. Flutter's
/// deflated RRect can have these when a stroke is larger than its box.
fn inflated_edges(rect: Rect, delta: f32) -> [f32; 4] {
    [
        rect.origin.x - delta,
        rect.origin.y - delta,
        rect.origin.x + rect.size.width + delta,
        rect.origin.y + rect.size.height + delta,
    ]
}

// Native addOval/addRRect order their signed edges. Point/line bounds
// remain available to path queries even when nothing is painted.
fn ordered_inflate(rect: Rect, delta: f32) -> Option<Rect> {
    let [left, top, right, bottom] = inflated_edges(rect, delta);
    if ![left, top, right, bottom].into_iter().all(f32::is_finite) {
        return None;
    }
    Some(Rect::from_origin_size(
        Offset::new(left.min(right), top.min(bottom)),
        Size::maybe_new((right - left).abs(), (bottom - top).abs())?,
    ))
}

fn non_flat(rect: &Rect) -> bool {
    rect.size.width > 0. && rect.size.height > 0.
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
#[derive(Clone, Copy, Debug, PartialEq, Hash, SmartDefault)]
pub struct StadiumBorder {
    #[default(BorderSide::NONE)]
    pub side: BorderSide,
}

impl StadiumBorder {
    #[must_use]
    pub const fn new(side: BorderSide) -> Self {
        Self { side }
    }

    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>) -> Self {
        Self::new(side.unwrap_or(self.side))
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
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        rounded_rect_path(rect, Self::radii(rect))
    }

    fn inner_path(&self, rect: Rect) -> Path {
        let inset = self.side.stroke_inset();
        ordered_inflate(rect, -inset).map_or_else(
            || Path::builder().build(),
            |inner| rounded_rect_path(inner, inflate_radii(Self::radii(rect), -inset)),
        )
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let delta = self.side.stroke_offset() / 2.0;
        if let Some(contour) = ordered_inflate(rect, delta).filter(non_flat) {
            stroke(
                canvas,
                &self.side,
                rounded_rect_path(contour, inflate_radii(Self::radii(rect), delta)),
            );
        }
    }
}

impl incular_core::Lerp for StadiumBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(self.side.lerp(&other.side, t))
    }
}

/// Flutter's `RoundedRectangleBorder`: a rectangle with rounded corners.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct RoundedRectangleBorder<R = BorderRadius> {
    pub side: BorderSide,
    pub border_radius: R,
}

impl Default for RoundedRectangleBorder<BorderRadius> {
    fn default() -> Self {
        Self::new(BorderSide::NONE, BorderRadius::ZERO)
    }
}

impl<R> RoundedRectangleBorder<R> {
    #[must_use]
    pub const fn new(side: BorderSide, border_radius: R) -> Self {
        Self {
            side,
            border_radius,
        }
    }

    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>, border_radius: Option<R>) -> Self {
        Self::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }
}

impl<R: Copy + std::ops::Mul<f32, Output = R>> RoundedRectangleBorder<R> {
    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t), self.border_radius * t)
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> RoundedRectangleBorder<R> {
    /// Keeps physical and directional components until paint-time resolution.
    #[must_use]
    pub fn into_radius_geometry(self) -> RoundedRectangleBorder<BorderRadiusGeometry> {
        RoundedRectangleBorder::new(self.side, self.border_radius.into())
    }

    fn path(&self, rect: Rect, delta: f32, direction: Option<TextDirection>) -> Path {
        let radius = self
            .border_radius
            .into()
            .resolve(direction.unwrap_or(TextDirection::Ltr));
        radius
            .to_rounded_rect(rect)
            .inflated_paint_path(EdgeInsets::all(delta))
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> ShapeBorder for RoundedRectangleBorder<R> {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, 0., direction)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, -self.side.stroke_inset(), direction)
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

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
    ) {
        if self.side.style == BorderStyle::None {
            return;
        }
        if self.side.width == 0. {
            stroke(
                canvas,
                &self.side,
                self.outer_path_with_direction(rect, direction),
            );
            return;
        }
        // A positive-width Flutter DrawDRRect fills two independently
        // normalized contours. Stroking one centerline is not equivalent
        // when radii clamp or the inner rectangle becomes flat or inverted.
        super::box_painter::fill_between(
            canvas,
            self.path(rect, self.side.stroke_outset(), direction),
            self.inner_path_with_direction(rect, direction),
            self.side.color,
        );
    }
}

impl<R: Copy + incular_core::Lerp> incular_core::Lerp for RoundedRectangleBorder<R> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0. {
            return *self;
        }
        if t == 1. {
            return *other;
        }
        Self::new(
            self.side.lerp(&other.side, t),
            self.border_radius.lerp(&other.border_radius, t),
        )
    }
}

/// Flutter's `RoundedSuperellipseBorder`: a rectangle whose corners are
/// rounded superellipses, the continuous corners of iOS.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct RoundedSuperellipseBorder<R = BorderRadius> {
    pub side: BorderSide,
    pub border_radius: R,
}

impl Default for RoundedSuperellipseBorder<BorderRadius> {
    fn default() -> Self {
        Self::new(BorderSide::NONE, BorderRadius::ZERO)
    }
}

impl<R> RoundedSuperellipseBorder<R> {
    #[must_use]
    pub const fn new(side: BorderSide, border_radius: R) -> Self {
        Self { side, border_radius }
    }

    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>, border_radius: Option<R>) -> Self {
        Self::new(side.unwrap_or(self.side), border_radius.unwrap_or(self.border_radius))
    }
}

impl<R: Copy + std::ops::Mul<f32, Output = R>> RoundedSuperellipseBorder<R> {
    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t), self.border_radius * t)
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> RoundedSuperellipseBorder<R> {
    #[must_use]
    pub fn into_radius_geometry(self) -> RoundedSuperellipseBorder<BorderRadiusGeometry> {
        RoundedSuperellipseBorder::new(self.side, self.border_radius.into())
    }

    pub(super) fn path(&self, rect: Rect, delta: f32, direction: Option<TextDirection>) -> Path {
        let Some(contour) = ordered_inflate(rect, delta) else {
            return Path::builder().build();
        };
        let radius = self.border_radius.into().resolve(direction.unwrap_or(TextDirection::Ltr));
        if radius == BorderRadius::ZERO {
            return rect_path(contour);
        }
        let radii = [radius.top_left, radius.top_right, radius.bottom_right, radius.bottom_left]
            .map(|corner| Size {
                width: (corner.x + delta).max(0.),
                height: (corner.y + delta).max(0.),
            });
        let mut builder = Path::builder();
        builder.rounded_superellipse_elliptical(contour, radii);
        builder.build()
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> ShapeBorder for RoundedSuperellipseBorder<R> {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, 0., direction)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, -self.side.stroke_inset(), direction)
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint_interior(&self, canvas: &mut Canvas, rect: Rect, paint: &Paint, direction: Option<TextDirection>) {
        canvas.draw_path(Arc::new(self.outer_path_with_direction(rect, direction)), paint, FillRule::NonZero);
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(&self, canvas: &mut Canvas, rect: Rect, direction: Option<TextDirection>) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let offset = (self.side.stroke_outset() - self.side.stroke_inset()) / 2.0;
        stroke(canvas, &self.side, self.path(rect, offset, direction));
    }
}

impl<R: Copy + incular_core::Lerp> incular_core::Lerp for RoundedSuperellipseBorder<R> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0. { return *self; }
        if t == 1. { return *other; }
        Self::new(
            self.side.lerp(&other.side, t),
            self.border_radius.lerp(&other.border_radius, t),
        )
    }
}

/// Flutter's CircleBorder, a circle at eccentricity zero and an oval
/// filling the whole rectangle at eccentricity one.
#[derive(Clone, Copy, Debug, PartialEq, SmartDefault)]
pub struct CircleBorder {
    #[default(BorderSide::NONE)]
    pub side: BorderSide,
    eccentricity: f32,
}

impl CircleBorder {
    #[must_use]
    pub const fn new(side: BorderSide) -> Self {
        Self {
            side,
            eccentricity: 0.,
        }
    }

    /// # Panics
    /// In debug builds, panics unless the eccentricity is in `0..=1`.
    #[must_use]
    pub fn with_eccentricity(mut self, value: f32) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&value),
            "eccentricity must be in 0..=1"
        );
        self.eccentricity = if value.is_nan() {
            0.
        } else {
            value.clamp(0., 1.)
        };
        self
    }

    #[must_use]
    pub const fn eccentricity(self) -> f32 {
        self.eccentricity
    }

    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>, eccentricity: Option<f32>) -> Self {
        Self::new(side.unwrap_or(self.side))
            .with_eccentricity(eccentricity.unwrap_or(self.eccentricity))
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self {
            side: self.side.scale(t),
            ..*self
        }
    }

    fn fitted_rect(self, rect: Rect) -> Rect {
        let width = rect.size.width;
        let height = rect.size.height;
        let margin = (1. - self.eccentricity) * (width - height).abs() / 2.;
        if width < height {
            Rect::from_origin_size(
                Offset::new(rect.origin.x, rect.origin.y + margin),
                Size::new(width, height - 2. * margin),
            )
        } else {
            Rect::from_origin_size(
                Offset::new(rect.origin.x + margin, rect.origin.y),
                Size::new(width - 2. * margin, height),
            )
        }
    }
}

impl Hash for CircleBorder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.side.hash(state);
        super::hash_paint_float(self.eccentricity, state);
    }
}

impl ShapeBorder for CircleBorder {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        ellipse_path(self.fitted_rect(rect))
    }

    fn inner_path(&self, rect: Rect) -> Path {
        ordered_inflate(self.fitted_rect(rect), -self.side.stroke_inset())
            .map_or_else(|| Path::builder().build(), ellipse_path)
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _: Option<TextDirection>,
    ) {
        canvas.draw_path(Arc::new(self.outer_path(rect)), paint, FillRule::NonZero);
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let delta = self.side.stroke_offset() / 2.0;
        let fitted = self.fitted_rect(rect);
        // Unlike DrawOval, DrawCircle with a nonpositive radius does not
        // reflect the contour through the center and draw another circle.
        if self.eccentricity == 0. && shortest_side(fitted) + 2. * delta <= 0. {
            return;
        }
        if let Some(contour) = ordered_inflate(fitted, delta).filter(non_flat) {
            stroke(canvas, &self.side, ellipse_path(contour));
        }
    }
}

impl incular_core::Lerp for CircleBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(self.side.lerp(&other.side, t))
            .with_eccentricity(self.eccentricity.lerp(&other.eccentricity, t).clamp(0., 1.))
    }
}

/// Flutter's `OvalBorder`: an ellipse inscribed in the rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Hash, SmartDefault)]
pub struct OvalBorder {
    #[default(BorderSide::NONE)]
    pub side: BorderSide,
}

impl OvalBorder {
    #[must_use]
    pub const fn new(side: BorderSide) -> Self {
        Self { side }
    }

    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>) -> Self {
        Self::new(side.unwrap_or(self.side))
    }

    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t))
    }
}

impl ShapeBorder for OvalBorder {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        ellipse_path(rect)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        ordered_inflate(rect, -self.side.stroke_inset())
            .map_or_else(|| Path::builder().build(), ellipse_path)
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let delta = self.side.stroke_offset() / 2.0;
        if let Some(contour) = ordered_inflate(rect, delta).filter(non_flat) {
            stroke(canvas, &self.side, ellipse_path(contour));
        }
    }
}

impl incular_core::Lerp for OvalBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(self.side.lerp(&other.side, t))
    }
}

/// Flutter's `BeveledRectangleBorder`: a rectangle whose corners are cut as
/// straight segments. The default parameter keeps existing physical-radius
/// callers unchanged; directional or mixed geometry resolves at paint time.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct BeveledRectangleBorder<R = BorderRadius> {
    pub side: BorderSide,
    pub border_radius: R,
}

impl Default for BeveledRectangleBorder {
    fn default() -> Self {
        Self::new(BorderSide::NONE, BorderRadius::ZERO)
    }
}

impl<R> BeveledRectangleBorder<R> {
    #[must_use]
    pub const fn new(side: BorderSide, border_radius: R) -> Self {
        Self {
            side,
            border_radius,
        }
    }

    /// Retains properties whose replacement is omitted.
    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>, border_radius: Option<R>) -> Self {
        Self::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }
}

impl<R: Copy + std::ops::Mul<f32, Output = R>> BeveledRectangleBorder<R> {
    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t), self.border_radius * t)
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> BeveledRectangleBorder<R> {
    fn path(&self, rect: Rect, delta: f32, direction: Option<TextDirection>) -> Path {
        // The inner rectangle may invert for a thick border. Keep its
        // signed edge coordinates rather than constructing a negative Size.
        let radius = self
            .border_radius
            .into()
            .resolve(direction.unwrap_or(TextDirection::Ltr));
        bevel_path(inflated_edges(rect, delta), radius, delta)
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> ShapeBorder for BeveledRectangleBorder<R> {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, 0.0, direction)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, -self.side.stroke_inset(), direction)
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
        if self.side.style == BorderStyle::None || rect.size.width == 0. || rect.size.height == 0. {
            return;
        }
        let mut path = Path::builder();
        path.append(&self.path(rect, self.side.stroke_outset(), direction));
        path.append(&self.inner_path_with_direction(rect, direction));
        stroke(canvas, &self.side, path.build());
    }
}

impl<R: incular_core::Lerp> incular_core::Lerp for BeveledRectangleBorder<R> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            self.side.lerp(&other.side, t),
            self.border_radius.lerp(&other.border_radius, t),
        )
    }
}

/// A closed ellipse inscribed in `rect`, as four quarter conics.
fn ellipse_path(rect: Rect) -> Path {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (right, bottom) = (left + rect.size.width, top + rect.size.height);
    let (cx, cy) = (left + rect.size.width / 2.0, top + rect.size.height / 2.0);
    let weight = std::f32::consts::FRAC_1_SQRT_2;
    let mut builder = Path::builder();
    builder.move_to(Offset::new(right, cy));
    builder.conic_to(Offset::new(right, bottom), Offset::new(cx, bottom), weight);
    builder.conic_to(Offset::new(left, bottom), Offset::new(left, cy), weight);
    builder.conic_to(Offset::new(left, top), Offset::new(cx, top), weight);
    builder.conic_to(Offset::new(right, top), Offset::new(right, cy), weight);
    builder.close();
    builder.build()
}

/// Flutter `BeveledRectangleBorder._getPath`: the eight vertices where each
/// corner's radii meet the sides, closed as a polygon. Radii are clamped to
/// zero and not scaled to fit, matching the vertex formulas.
fn bevel_path([left, top, right, bottom]: [f32; 4], radii: BorderRadius, delta: f32) -> Path {
    let (cx, cy) = (left + (right - left) / 2.0, top + (bottom - top) / 2.0);
    let [tl, tr, br, bl] = [
        radii.top_left,
        radii.top_right,
        radii.bottom_right,
        radii.bottom_left,
    ]
    .map(|radius| Radius::elliptical((radius.x + delta).max(0.0), (radius.y + delta).max(0.0)));
    let vertices = [
        Offset::new(left, cy.min(top + tl.y)),
        Offset::new(cx.min(left + tl.x), top),
        Offset::new(cx.max(right - tr.x), top),
        Offset::new(right, cy.min(top + tr.y)),
        Offset::new(right, cy.max(bottom - br.y)),
        Offset::new(cx.max(right - br.x), bottom),
        Offset::new(cx.min(left + bl.x), bottom),
        Offset::new(left, cy.max(bottom - bl.y)),
    ];
    let mut builder = Path::builder();
    builder.move_to(vertices[0]);
    let mut previous = vertices[0];
    for vertex in vertices[1..].iter().copied() {
        if vertex != previous {
            builder.line_to(vertex);
            previous = vertex;
        }
    }
    builder.close();
    builder.build()
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

/// Why a [`StarBorder`] value was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StarBorderError {
    /// The point count was below 2 or NaN.
    TooFewPoints,
    /// The inner radius ratio was outside `0.0..=1.0`.
    InnerRadiusRatioOutOfRange,
    /// A rounding was outside `0.0..=1.0`.
    RoundingOutOfRange,
    /// The point and valley roundings sum to more than 1.
    RoundingSumExceedsOne,
    /// The squash was outside `0.0..=1.0`.
    SquashOutOfRange,
}

impl std::fmt::Display for StarBorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooFewPoints => "a star has at least two points",
            Self::InnerRadiusRatioOutOfRange => "inner radius ratio must be within 0..=1",
            Self::RoundingOutOfRange => "rounding must be within 0..=1",
            Self::RoundingSumExceedsOne => "point and valley rounding must sum to at most 1",
            Self::SquashOutOfRange => "squash must be within 0..=1",
        })
    }
}

impl std::error::Error for StarBorderError {}

fn check_rounding(rounding: f32, other: f32) -> Result<(), StarBorderError> {
    if !(0.0..=1.0).contains(&rounding) {
        return Err(StarBorderError::RoundingOutOfRange);
    }
    if rounding + other > 1.0 {
        return Err(StarBorderError::RoundingSumExceedsOne);
    }
    Ok(())
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
    ///
    /// # Panics
    ///
    /// Panics when `points` is below 2 or NaN. Use [`Self::try_points`] to
    /// handle that case.
    #[must_use]
    pub fn points(self, points: f32) -> Self {
        self.try_points(points)
            .expect("a star has at least two points; use try_points")
    }

    /// Sets the number of points, or reports a count below 2.
    ///
    /// # Errors
    ///
    /// Returns [`StarBorderError::TooFewPoints`] when `points` is below 2 or
    /// NaN.
    pub fn try_points(mut self, points: f32) -> Result<Self, StarBorderError> {
        if points.is_nan() || points < 2.0 {
            return Err(StarBorderError::TooFewPoints);
        }
        self.points = points;
        Ok(self)
    }

    /// The valleys' radius as a fraction of the points', from 0 to 1.
    ///
    /// # Panics
    ///
    /// Panics when `ratio` is outside `0.0..=1.0`. Use
    /// [`Self::try_inner_radius_ratio`] to handle that case.
    #[must_use]
    pub fn inner_radius_ratio(self, ratio: f32) -> Self {
        self.try_inner_radius_ratio(ratio)
            .expect("inner radius ratio must be within 0..=1; use try_inner_radius_ratio")
    }

    /// Sets the valleys' radius ratio, or reports one outside `0.0..=1.0`.
    ///
    /// # Errors
    ///
    /// Returns [`StarBorderError::InnerRadiusRatioOutOfRange`].
    pub fn try_inner_radius_ratio(mut self, ratio: f32) -> Result<Self, StarBorderError> {
        if !(0.0..=1.0).contains(&ratio) {
            return Err(StarBorderError::InnerRadiusRatioOutOfRange);
        }
        self.inner_radius_ratio = Some(ratio);
        Ok(self)
    }

    /// From 0 to 1; with the valley rounding at most 1.
    ///
    /// # Panics
    ///
    /// Panics when `rounding` is outside `0.0..=1.0` or the point and valley
    /// roundings sum above 1. Use [`Self::try_point_rounding`] to handle
    /// those cases.
    #[must_use]
    pub fn point_rounding(self, rounding: f32) -> Self {
        self.try_point_rounding(rounding)
            .expect("point rounding must be within 0..=1 and sum to at most 1 with valley rounding; use try_point_rounding")
    }

    /// Sets the point rounding, or reports an invalid value.
    ///
    /// # Errors
    ///
    /// Returns [`StarBorderError::RoundingOutOfRange`] or
    /// [`StarBorderError::RoundingSumExceedsOne`].
    pub fn try_point_rounding(mut self, rounding: f32) -> Result<Self, StarBorderError> {
        check_rounding(rounding, self.valley_rounding)?;
        self.point_rounding = rounding;
        Ok(self)
    }

    /// From 0 to 1; with the point rounding at most 1.
    ///
    /// # Panics
    ///
    /// Panics when `rounding` is outside `0.0..=1.0` or the point and valley
    /// roundings sum above 1. Use [`Self::try_valley_rounding`] to handle
    /// those cases.
    #[must_use]
    pub fn valley_rounding(self, rounding: f32) -> Self {
        self.try_valley_rounding(rounding)
            .expect("valley rounding must be within 0..=1 and sum to at most 1 with point rounding; use try_valley_rounding")
    }

    /// Sets the valley rounding, or reports an invalid value.
    ///
    /// # Errors
    ///
    /// Returns [`StarBorderError::RoundingOutOfRange`] or
    /// [`StarBorderError::RoundingSumExceedsOne`].
    pub fn try_valley_rounding(mut self, rounding: f32) -> Result<Self, StarBorderError> {
        check_rounding(rounding, self.point_rounding)?;
        self.valley_rounding = rounding;
        Ok(self)
    }

    /// Clockwise rotation in degrees.
    #[must_use]
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation_radians = degrees.to_radians();
        self
    }

    /// How far the star stretches to the rectangle's longer side, from 0
    /// (kept round) to 1 (filling it).
    ///
    /// # Panics
    ///
    /// Panics when `squash` is outside `0.0..=1.0`. Use [`Self::try_squash`]
    /// to handle that case.
    #[must_use]
    pub fn squash(self, squash: f32) -> Self {
        self.try_squash(squash)
            .expect("squash must be within 0..=1; use try_squash")
    }

    /// Sets the squash, or reports one outside `0.0..=1.0`.
    ///
    /// # Errors
    ///
    /// Returns [`StarBorderError::SquashOutOfRange`].
    pub fn try_squash(mut self, squash: f32) -> Result<Self, StarBorderError> {
        if !(0.0..=1.0).contains(&squash) {
            return Err(StarBorderError::SquashOutOfRange);
        }
        self.squash = squash;
        Ok(self)
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
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.stroke_inset().max(0.))
    }

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
    #[expect(
        clippy::cast_possible_truncation,
        reason = "path coordinates are f32; f64 math is narrowed on output"
    )]
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
        let mut points = Vec::with_capacity(self.points.ceil() as usize + 1);
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

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Offset coordinates and weights are f32 by design"
    )]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparisons mirror Flutter's degenerate-angle checks"
    )]
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
#[expect(
    clippy::float_cmp,
    reason = "exact comparisons mirror Flutter's degenerate-angle checks"
)]
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

/// Flutter's `ContinuousRectangleBorder`: a rectangle whose corners
/// blend into the sides along cubic curves with both controls at the
/// corner. Logical corner values resolve against the text direction.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct ContinuousRectangleBorder<R = BorderRadius> {
    pub side: BorderSide,
    pub border_radius: R,
}

impl Default for ContinuousRectangleBorder {
    fn default() -> Self {
        Self::new(BorderSide::NONE, BorderRadius::ZERO)
    }
}

impl<R> ContinuousRectangleBorder<R> {
    #[must_use]
    pub const fn new(side: BorderSide, border_radius: R) -> Self {
        Self {
            side,
            border_radius,
        }
    }

    /// Copies the border while keeping unspecified properties.
    #[must_use]
    pub fn copy_with(self, side: Option<BorderSide>, border_radius: Option<R>) -> Self {
        Self::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }
}

impl<R: Copy + std::ops::Mul<f32, Output = R>> ContinuousRectangleBorder<R> {
    #[must_use]
    pub fn scale(&self, t: f32) -> Self {
        Self::new(self.side.scale(t), self.border_radius * t)
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> ContinuousRectangleBorder<R> {
    /// `_getPath` over `rect` inflated by `delta` (`RRect.inflate`, radii
    /// clamped at zero). Radii clamp to the shortest side; as in Flutter,
    /// each corner's x radius sets its vertical extent and its y radius the
    /// horizontal one.
    fn path(&self, rect: Rect, delta: f32, direction: Option<TextDirection>) -> Path {
        let [left, top, right, bottom] = inflated_edges(rect, delta);
        let shortest = (right - left).min(bottom - top).max(0.0);
        let r = |value: f32| (value + delta).max(0.0).min(shortest);
        let radius = self
            .border_radius
            .into()
            .resolve(direction.unwrap_or(TextDirection::Ltr));
        let (tl, tr, br, bl) = (
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        );
        let at = Offset::new;
        let mut builder = Path::builder();
        builder.move_to(at(left, top + r(tl.x)));
        builder.cubic_to(at(left, top), at(left, top), at(left + r(tl.y), top));
        builder.line_to(at(right - r(tr.x), top));
        builder.cubic_to(at(right, top), at(right, top), at(right, top + r(tr.y)));
        builder.line_to(at(right, bottom - r(br.x)));
        builder.cubic_to(
            at(right, bottom),
            at(right, bottom),
            at(right - r(br.y), bottom),
        );
        builder.line_to(at(left + r(bl.x), bottom));
        builder.cubic_to(
            at(left, bottom),
            at(left, bottom),
            at(left, bottom - r(bl.y)),
        );
        builder.close();
        builder.build()
    }
}

impl<R: Copy + Into<BorderRadiusGeometry>> ShapeBorder for ContinuousRectangleBorder<R> {
    fn dimensions(&self, _: Option<TextDirection>) -> EdgeInsets {
        EdgeInsets::all(self.side.width)
    }

    fn outer_path(&self, rect: Rect) -> Path {
        self.outer_path_with_direction(rect, None)
    }

    fn outer_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, 0.0, direction)
    }

    /// Flutter deflates by the full side width, whatever the stroke align.
    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        self.path(rect, -self.side.width, direction)
    }

    /// Flutter strokes the outer path with `side.toPaint()`.
    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
    ) {
        if self.side.style == BorderStyle::None || rect.size.width <= 0.0 || rect.size.height <= 0.0
        {
            return;
        }
        stroke(canvas, &self.side, self.outer_path_with_direction(rect, direction));
    }
}

impl<R: incular_core::Lerp> incular_core::Lerp for ContinuousRectangleBorder<R> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            self.side.lerp(&other.side, t),
            self.border_radius.lerp(&other.border_radius, t),
        )
    }
}

/// Flutter's `LinearBorderEdge`: the fraction of one side a
/// [`LinearBorder`] draws (`size`, 0 to 1) and where along the side it sits
/// (`alignment`, -1 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearBorderEdge {
    pub size: f32,
    pub alignment: f32,
}

impl Default for LinearBorderEdge {
    fn default() -> Self {
        Self::new(1.0, 0.0)
    }
}

impl LinearBorderEdge {
    /// Constructs an edge. Flutter rejects an out-of-range size in debug
    /// builds; release retains Incular's clamp for finite inputs.
    #[must_use]
    pub fn new(size: f32, alignment: f32) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&size),
            "linear border size must be in 0..=1"
        );
        Self {
            size: size.clamp(0.0, 1.0),
            alignment,
        }
    }

    /// Flutter `LinearBorderEdge.lerp`: a missing edge is a zero-size edge
    /// at the other edge's alignment.
    #[must_use]
    pub fn lerp(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a == b {
            return a;
        }
        let (a, b) = match (a, b) {
            (None, None) => return None,
            (Some(a), None) => (a, Self { size: 0.0, ..a }),
            (None, Some(b)) => (Self { size: 0.0, ..b }, b),
            (Some(a), Some(b)) => (a, b),
        };
        Some(Self::new(
            a.size + (b.size - a.size) * t,
            a.alignment + (b.alignment - a.alignment) * t,
        ))
    }
}

impl Hash for LinearBorderEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        super::hash_paint_float(self.size, state);
        super::hash_paint_float(self.alignment, state);
    }
}

/// Flutter's `LinearBorder` (`painting/linear_border.dart`): a rectangle
/// whose outline is drawn only along the chosen edges, each possibly
/// shortened and aligned. Painted for a left-to-right direction (`start` is
/// the left edge), as [`ShapeBorder::paint`] carries no text direction.
#[derive(Clone, Copy, Debug, PartialEq, SmartDefault)]
pub struct LinearBorder {
    #[default(BorderSide::NONE)]
    pub side: BorderSide,
    pub start: Option<LinearBorderEdge>,
    pub end: Option<LinearBorderEdge>,
    pub top: Option<LinearBorderEdge>,
    pub bottom: Option<LinearBorderEdge>,
}

impl LinearBorder {
    /// Flutter `LinearBorder.none`: no edges.
    pub const NONE: Self = Self {
        side: BorderSide::NONE,
        start: None,
        end: None,
        top: None,
        bottom: None,
    };

    #[must_use]
    pub fn start(side: BorderSide, edge: LinearBorderEdge) -> Self {
        Self {
            side,
            start: Some(edge),
            ..Self::NONE
        }
    }

    #[must_use]
    pub fn end(side: BorderSide, edge: LinearBorderEdge) -> Self {
        Self {
            side,
            end: Some(edge),
            ..Self::NONE
        }
    }

    #[must_use]
    pub fn top(side: BorderSide, edge: LinearBorderEdge) -> Self {
        Self {
            side,
            top: Some(edge),
            ..Self::NONE
        }
    }

    #[must_use]
    pub fn bottom(side: BorderSide, edge: LinearBorderEdge) -> Self {
        Self {
            side,
            bottom: Some(edge),
            ..Self::NONE
        }
    }

    /// Copies this border, retaining the side or edge for each omitted input.
    #[must_use]
    pub fn copy_with(
        self,
        side: Option<BorderSide>,
        start: Option<LinearBorderEdge>,
        end: Option<LinearBorderEdge>,
        top: Option<LinearBorderEdge>,
        bottom: Option<LinearBorderEdge>,
    ) -> Self {
        Self {
            side: side.unwrap_or(self.side),
            start: start.or(self.start),
            end: end.or(self.end),
            top: top.or(self.top),
            bottom: bottom.or(self.bottom),
        }
    }

    /// Flutter scales the side but clears all selected edges.
    #[must_use]
    pub fn scale(self, t: f32) -> Self {
        Self {
            side: self.side.scale(t),
            ..Self::NONE
        }
    }

    /// Interpolates two nullable instances as Flutter's OutlinedBorder
    /// does for LinearBorder. An absent endpoint uses `scale`.
    #[must_use]
    pub fn lerp_optional(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1. - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) if a == b => Some(a),
            (Some(a), Some(b)) => Some(incular_core::Lerp::lerp(&a, &b, t)),
        }
    }

    /// Flutter `dimensions`: the side width on each edge that is present
    /// (left, top, right, bottom).
    fn insets(&self) -> [f32; 4] {
        let width = |edge: Option<LinearBorderEdge>| {
            if edge.is_some() { self.side.width } else { 0.0 }
        };
        [
            width(self.start),
            width(self.top),
            width(self.end),
            width(self.bottom),
        ]
    }
}

impl Hash for LinearBorder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.side.hash(state);
        self.start.hash(state);
        self.end.hash(state);
        self.top.hash(state);
        self.bottom.hash(state);
    }
}

fn hairline() -> Stroke {
    Stroke {
        width: 0.0,
        ..Stroke::default()
    }
}

impl ShapeBorder for LinearBorder {
    fn dimensions(&self, direction: Option<TextDirection>) -> EdgeInsets {
        let [start, top, end, bottom] = self.insets();
        match direction.unwrap_or(TextDirection::Ltr) {
            TextDirection::Ltr => EdgeInsets::only(start, top, end, bottom),
            TextDirection::Rtl => EdgeInsets::only(end, top, start, bottom),
        }
    }

    fn outer_path(&self, rect: Rect) -> Path {
        rounded_rect_path(rect, CornerRadii::ZERO)
    }

    fn inner_path(&self, rect: Rect) -> Path {
        self.inner_path_with_direction(rect, None)
    }

    fn inner_path_with_direction(&self, rect: Rect, direction: Option<TextDirection>) -> Path {
        rect_path(deflate_border(rect, self.dimensions(direction)))
    }

    /// Flutter `LinearBorder.paint` (left to right). As in Flutter, the
    /// start and end edges are offset from zero rather than from the
    /// rectangle's top, and the top and bottom edges from zero rather than
    /// its left; at the origin, where a shape decoration paints, they agree.
    fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        self.paint_with_direction(canvas, rect, None);
    }

    fn paint_with_direction(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        direction: Option<TextDirection>,
    ) {
        if self.side.style == BorderStyle::None {
            return;
        }
        let rtl = direction == Some(TextDirection::Rtl);
        let EdgeInsets {
            left,
            top,
            right,
            bottom,
        } = self.dimensions(direction);
        let color = self.side.color;
        let mut draw_edge = |edge: Rect| {
            let mut builder = Path::builder();
            let (x, y) = (edge.origin.x, edge.origin.y);
            builder.move_to(Offset::new(x, y));
            if edge.size.width == 0.0 {
                builder.line_to(Offset::new(x, y + edge.size.height));
                canvas.stroke_path(Arc::new(builder.build()), color, hairline());
            } else if edge.size.height == 0.0 {
                builder.line_to(Offset::new(x + edge.size.width, y));
                canvas.stroke_path(Arc::new(builder.build()), color, hairline());
            } else {
                builder
                    .line_to(Offset::new(x + edge.size.width, y))
                    .line_to(Offset::new(x + edge.size.width, y + edge.size.height))
                    .line_to(Offset::new(x, y + edge.size.height))
                    .close();
                canvas.fill_path(
                    Arc::new(builder.build()),
                    color,
                    incular_rendering::FillRule::NonZero,
                );
            }
        };
        let inset_height = rect.size.height - top - bottom;
        let vertical = |edge: LinearBorderEdge, x: f32, width: f32| {
            let height = inset_height * edge.size;
            let y = (inset_height - height) * ((edge.alignment + 1.0) / 2.0);
            Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
        };
        let horizontal = |edge: LinearBorderEdge, y: f32, height: f32| {
            let width = rect.size.width * edge.size;
            let start_x = (rect.size.width - width) * ((edge.alignment + 1.0) / 2.0);
            let x = if rtl {
                rect.size.width - start_x - width
            } else {
                start_x
            };
            Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
        };
        let left_edge = (rect.origin.x, left);
        let right_edge = (rect.origin.x + rect.size.width - right, right);
        if let Some(edge) = self.start.filter(|edge| edge.size != 0.0) {
            let (x, width) = if rtl { right_edge } else { left_edge };
            draw_edge(vertical(edge, x, width));
        }
        if let Some(edge) = self.end.filter(|edge| edge.size != 0.0) {
            let (x, width) = if rtl { left_edge } else { right_edge };
            draw_edge(vertical(edge, x, width));
        }
        if let Some(edge) = self.top.filter(|edge| edge.size != 0.0) {
            draw_edge(horizontal(edge, rect.origin.y, top));
        }
        if let Some(edge) = self.bottom.filter(|edge| edge.size != 0.0) {
            draw_edge(horizontal(
                edge,
                rect.origin.y + rect.size.height - bottom,
                self.side.width,
            ));
        }
    }
}

impl incular_core::Lerp for LinearBorder {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            side: self.side.lerp(&other.side, t),
            start: LinearBorderEdge::lerp(self.start, other.start, t),
            end: LinearBorderEdge::lerp(self.end, other.end, t),
            top: LinearBorderEdge::lerp(self.top, other.top, t),
            bottom: LinearBorderEdge::lerp(self.bottom, other.bottom, t),
        }
    }
}
