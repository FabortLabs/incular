use crate::geometry::CornerRadii;
use crate::paint::FillRule;
use incular_core::{Offset, Rect, Size, Transform};
use kurbo::{BezPath, Ellipse, Point, RoundedRect, RoundedRectRadii, Shape, Vec2};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(1);

fn next_path_id() -> PathId {
    PathId(
        NEXT_PATH_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("path identity space exhausted"),
    )
}

/// Stable identity for immutable path geometry. It deliberately does not
/// encode paint, placement, or transforms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PathId(u64);
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    id: PathId,
    path: Arc<BezPath>,
    bounds: Option<Rect>,
}
impl Path {
    #[must_use]
    pub fn builder() -> PathBuilder {
        PathBuilder::default()
    }
    #[must_use]
    pub fn bez_path(&self) -> &BezPath {
        &self.path
    }
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }
    #[must_use]
    pub const fn id(&self) -> PathId {
        self.id
    }
    /// Renderer-neutral filled-path containment delegated to Kurbo's exact
    /// segment winding implementation. It is intentionally independent from
    /// GPU tessellation so pointer targeting follows the retained geometry.
    #[must_use]
    pub fn contains(&self, point: Offset, rule: FillRule) -> bool {
        let winding = self
            .path
            .winding(Point::new(f64::from(point.x), f64::from(point.y)));
        match rule {
            FillRule::NonZero => winding != 0,
            FillRule::EvenOdd => winding.unsigned_abs() % 2 == 1,
        }
    }

    /// Wraps an already-built Bézier path with fresh identity and bounds.
    /// Clip fallbacks convert analytic shapes through here; every other
    /// construction keeps its existing entry point.
    pub(crate) fn from_bez_path(path: BezPath) -> Self {
        let bounds = (!path.is_empty() && path.is_finite()).then(|| {
            let bounds = path.bounding_box();
            Rect::from_origin_size(
                Offset::new(bounds.x0 as f32, bounds.y0 as f32),
                incular_core::Size::new(
                    (bounds.x1 - bounds.x0) as f32,
                    (bounds.y1 - bounds.y0) as f32,
                ),
            )
        });
        Self {
            id: next_path_id(),
            path: Arc::new(path),
            bounds,
        }
    }

    /// Returns a copy of this path with an affine transform applied to its
    /// geometry.  Path identity is intentionally not reused: the transformed
    /// geometry is a distinct immutable mesh candidate.  This is used by
    /// higher-level icon primitives to fit a canonical 24px path into the
    /// requested logical size without changing pointer/layout coordinates.
    #[must_use]
    pub fn transformed(&self, transform: Transform) -> Self {
        let mut path = (*self.path).clone();
        path.apply_affine(transform.to_kurbo());
        let bounds = (!path.is_empty() && path.is_finite()).then(|| {
            let bounds = path.bounding_box();
            Rect::from_origin_size(
                Offset::new(bounds.x0 as f32, bounds.y0 as f32),
                Size::new(
                    (bounds.x1 - bounds.x0) as f32,
                    (bounds.y1 - bounds.y0) as f32,
                ),
            )
        });
        Self {
            id: next_path_id(),
            path: Arc::new(path),
            bounds,
        }
    }
}
impl Default for Path {
    fn default() -> Self {
        PathBuilder::default().build()
    }
}
#[derive(Default)]
pub struct PathBuilder {
    path: BezPath,
}
impl PathBuilder {
    pub fn move_to(&mut self, p: Offset) -> &mut Self {
        self.path.move_to(point(p));
        self
    }
    pub fn line_to(&mut self, p: Offset) -> &mut Self {
        self.path.line_to(point(p));
        self
    }
    pub fn quadratic_to(&mut self, c: Offset, p: Offset) -> &mut Self {
        self.path.quad_to(point(c), point(p));
        self
    }
    pub fn cubic_to(&mut self, a: Offset, b: Offset, p: Offset) -> &mut Self {
        self.path.curve_to(point(a), point(b), point(p));
        self
    }
    /// Flutter `Path.conicTo`: a rational quadratic Bézier with weight
    /// `weight`, drawn as cubics. The conic is halved twice (Skia's
    /// `SkConic::chop`) and each quarter becomes the cubic whose handles
    /// sit `4w / (3(1 + w))` of the way to its control point: for a
    /// circular arc of angle θ (weight `cos(θ/2)`) that is the usual
    /// `(4/3)·tan(θ/4)·r` handle length.
    pub fn conic_to(&mut self, control: Offset, end: Offset, weight: f32) -> &mut Self {
        let Some(start) = self
            .path
            .elements()
            .last()
            .and_then(|element| element.end_point())
        else {
            return self.move_to(end);
        };
        let weight = f64::from(weight);
        if !(weight.is_finite() && weight > 0.0) {
            return self.line_to(end);
        }
        let mut pieces = vec![(start, point(control), point(end), weight)];
        for _ in 0..2 {
            pieces = pieces
                .into_iter()
                .flat_map(|(p0, p1, p2, w)| {
                    let scale = 1.0 / (1.0 + w);
                    let mid = ((p0.to_vec2() + p1.to_vec2() * (2.0 * w) + p2.to_vec2())
                        * (0.5 * scale))
                        .to_point();
                    let half = ((1.0 + w) / 2.0).sqrt();
                    [
                        (
                            p0,
                            ((p0.to_vec2() + p1.to_vec2() * w) * scale).to_point(),
                            mid,
                            half,
                        ),
                        (
                            mid,
                            ((p1.to_vec2() * w + p2.to_vec2()) * scale).to_point(),
                            p2,
                            half,
                        ),
                    ]
                })
                .collect();
        }
        for (p0, p1, p2, w) in pieces {
            let k = 4.0 * w / (3.0 * (1.0 + w));
            self.path.curve_to(p0.lerp(p1, k), p2.lerp(p1, k), p2);
        }
        self
    }

    /// Adds a closed rounded rectangle, its arcs as cubics within 0.01
    /// logical pixels.
    pub fn rounded_rect(&mut self, rect: Rect, radii: CornerRadii) -> &mut Self {
        let radii = radii.normalized(rect.size);
        let shape = RoundedRect::new(
            f64::from(rect.origin.x),
            f64::from(rect.origin.y),
            f64::from(rect.origin.x + rect.size.width),
            f64::from(rect.origin.y + rect.size.height),
            RoundedRectRadii::new(
                f64::from(radii.top_left),
                f64::from(radii.top_right),
                f64::from(radii.bottom_right),
                f64::from(radii.bottom_left),
            ),
        );
        self.path.extend(shape.path_elements(0.01));
        self
    }

    /// Flutter `Path.addRSuperellipse` for circular corner radii: a closed
    /// rounded superellipse, the corner shape of iOS.
    pub fn rounded_superellipse(&mut self, rect: Rect, radii: CornerRadii) -> &mut Self {
        crate::rsuperellipse::rounded_superellipse(self, rect, radii);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        if !self.path.elements().is_empty() {
            self.path.close_path();
        }
        self
    }
    #[must_use]
    pub fn build(self) -> Path {
        Path::from_bez_path(self.path)
    }
}

fn point(offset: Offset) -> Point {
    Point::new(f64::from(offset.x), f64::from(offset.y))
}

/// Target world-space curve deviation for analytic clip fallbacks: a
/// tenth of a logical pixel after the world transform is applied.
const CLIP_PATH_WORLD_TOLERANCE: f64 = 0.1;

/// Local-space arc-subdivision tolerance for one world transform,
/// targeting [`CLIP_PATH_WORLD_TOLERANCE`] in world space. kurbo
/// subdivides arcs into cubics in the shape's local space, so the local
/// tolerance divides the target by the transform's magnification bound;
/// the floor bounds segment counts where magnification exceeds 100x,
/// past which the world bound relaxes linearly instead. Tolerance is a
/// pure function of the world transform, so memoized fallback paths stay
/// valid exactly while their world does.
pub(crate) fn fallback_tolerance(world: Transform) -> f64 {
    (CLIP_PATH_WORLD_TOLERANCE / f64::from(world.magnification_bound())).clamp(1e-3, 1.0)
}

/// Exact local-space path of an axis-aligned rect: straight edges only,
/// so no tolerance applies.
pub(crate) fn rect_as_path(rect: Rect) -> Path {
    let mut builder = PathBuilder::default();
    builder
        .move_to(rect.origin)
        .line_to(Offset::new(rect.origin.x + rect.size.width, rect.origin.y))
        .line_to(Offset::new(
            rect.origin.x + rect.size.width,
            rect.origin.y + rect.size.height,
        ))
        .line_to(Offset::new(rect.origin.x, rect.origin.y + rect.size.height))
        .close();
    builder.build()
}

/// Local-space path of a rounded rect, arcs subdivided into cubics
/// within `tolerance`. Used only where uniform corner radii cannot
/// survive the world transform; an approximation by construction, never
/// an exact curve.
pub(crate) fn rrect_as_path(rect: Rect, radii: CornerRadii, tolerance: f64) -> Path {
    let shape = RoundedRect::new(
        f64::from(rect.origin.x),
        f64::from(rect.origin.y),
        f64::from(rect.origin.x + rect.size.width),
        f64::from(rect.origin.y + rect.size.height),
        RoundedRectRadii::new(
            f64::from(radii.top_left),
            f64::from(radii.top_right),
            f64::from(radii.bottom_right),
            f64::from(radii.bottom_left),
        ),
    );
    Path::from_bez_path(shape.to_path(tolerance))
}

/// Local-space path of the ellipse inscribed in `rect`, arcs subdivided
/// into cubics within `tolerance`. Used only where the world transform
/// would rotate the oval.
pub(crate) fn ellipse_as_path(rect: Rect, tolerance: f64) -> Path {
    let shape = Ellipse::new(
        Point::new(
            f64::from(rect.origin.x) + f64::from(rect.size.width) * 0.5,
            f64::from(rect.origin.y) + f64::from(rect.size.height) * 0.5,
        ),
        Vec2::new(
            f64::from(rect.size.width) * 0.5,
            f64::from(rect.size.height) * 0.5,
        ),
        0.0,
    );
    Path::from_bez_path(shape.to_path(tolerance))
}
