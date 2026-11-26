use crate::geometry::CornerRadii;
use crate::paint::FillRule;
use incular_core::{IdGenerator, Offset, Rect, Size, Transform};
use kurbo::{BezPath, Ellipse, PathEl, PathSeg, Point, RoundedRect, RoundedRectRadii, Shape, Vec2};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, PoisonError, Weak};

static NEXT_PATH_ID: IdGenerator = IdGenerator::new();

fn next_path_id() -> PathId {
    PathId(NEXT_PATH_ID.next())
}

/// Stable identity for immutable path geometry. It deliberately does not
/// encode paint, placement, or transforms.
///
/// Identity follows geometry: while a path is alive, building the same
/// segments again yields the same identity, so a repaint that records
/// unchanged shapes keeps every cache keyed by it (tessellated meshes, GPU
/// buffers, retained pictures) instead of minting replacements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PathId(u64);
#[derive(Clone, Debug)]
pub struct Path {
    id: PathId,
    geometry: Arc<PathGeometry>,
}
#[derive(Debug)]
struct PathGeometry {
    id: PathId,
    path: BezPath,
    tight_bounds: Option<Rect>,
    control_bounds: OnceLock<Option<Rect>>,
}
/// One identity is one geometry, so comparing identities never walks a
/// segment.
impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

/// Geometry hashes are already mixed; the table uses them as they are.
#[derive(Default)]
struct PassThroughHasher(u64);

impl Hasher for PassThroughHasher {
    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("geometry hashes are written as one u64");
    }
    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Process-wide table of live path geometry, one entry per geometry hash.
/// Entries hold weak references: a geometry stays interned exactly as long
/// as some [`Path`] owns it, and dead entries are swept as the table grows.
#[derive(Default)]
struct InternTable {
    paths: HashMap<u64, Weak<PathGeometry>, BuildHasherDefault<PassThroughHasher>>,
    sweep_at: usize,
}

impl InternTable {
    /// Entries the table may hold before its first sweep.
    const MIN_SWEEP: usize = 256;

    /// The live path owning `path`'s geometry, or `path` itself under a
    /// fresh identity.
    fn intern(&mut self, path: BezPath) -> Path {
        let hash = geometry_hash(path.elements());
        if let Some(geometry) = self.paths.get(&hash).and_then(Weak::upgrade)
            && geometry.path.elements() == path.elements()
        {
            return Path {
                id: geometry.id,
                geometry,
            };
        }
        if self.paths.len() >= self.sweep_at.max(Self::MIN_SWEEP) {
            // Drop entries whose geometry no path owns any more. Sweeping
            // only once the table has doubled keeps inserts amortized O(1).
            self.paths
                .retain(|_, geometry| geometry.strong_count() != 0);
            self.sweep_at = self.paths.len().saturating_mul(2);
        }
        let geometry = Arc::new(PathGeometry {
            id: next_path_id(),
            tight_bounds: tight_path_bounds(&path),
            control_bounds: OnceLock::new(),
            path,
        });
        // A dead entry is replaced. So is a live one whose geometry merely
        // shares the hash, without changing its existing identity.
        self.paths.insert(hash, Arc::downgrade(&geometry));
        Path {
            id: geometry.id,
            geometry,
        }
    }
}

static INTERNED_PATHS: LazyLock<Mutex<InternTable>> = LazyLock::new(Mutex::default);

/// Hash of the segments' kinds and coordinate bit patterns. Geometry that
/// differs only in the sign of a zero hashes apart and keeps its own
/// identity.
fn geometry_hash(elements: &[PathEl]) -> u64 {
    const SEED: u64 = 0x517c_c1b7_2722_0a95;
    fn add(hash: u64, word: u64) -> u64 {
        (hash.rotate_left(5) ^ word).wrapping_mul(SEED)
    }
    fn point(hash: u64, point: Point) -> u64 {
        add(add(hash, point.x.to_bits()), point.y.to_bits())
    }
    let mut hash = elements.len() as u64;
    for element in elements {
        hash = match *element {
            PathEl::MoveTo(p) => point(add(hash, 1), p),
            PathEl::LineTo(p) => point(add(hash, 2), p),
            PathEl::QuadTo(a, p) => point(point(add(hash, 3), a), p),
            PathEl::CurveTo(a, b, p) => point(point(point(add(hash, 4), a), b), p),
            PathEl::ClosePath => add(hash, 5),
        };
    }
    hash ^ (hash >> 32)
}

fn as_rect(bounds: kurbo::Rect) -> Rect {
    Rect::from_origin_size(
        Offset::new(bounds.x0 as f32, bounds.y0 as f32),
        Size::new(
            (bounds.x1 - bounds.x0) as f32,
            (bounds.y1 - bounds.y0) as f32,
        ),
    )
}

fn tight_path_bounds(path: &BezPath) -> Option<Rect> {
    (!path.is_empty() && path.is_finite()).then(|| as_rect(path.bounding_box()))
}

// Exact edge fallback for a single closed line contour. Repeated segments
// and intersections are left to winding; one isolated edge is the boundary
// between a filled and an unfilled side for either fill rule.
fn on_single_polygon_edge(path: &BezPath, point: Point) -> bool {
    let [PathEl::MoveTo(_), edges @ .., PathEl::ClosePath] = path.elements() else {
        return false;
    };
    if edges.len() < 2 || edges.iter().any(|edge| !matches!(edge, PathEl::LineTo(_))) {
        return false;
    }
    path.segments()
        .filter(|segment| {
            let PathSeg::Line(line) = segment else {
                return false;
            };
            let direction = line.p1 - line.p0;
            let offset = point - line.p0;
            let distance = offset.dot(direction);
            direction.cross(offset) == 0. && distance > 0. && distance < direction.hypot2()
        })
        .take(2)
        .count()
        == 1
}

impl Path {
    #[must_use]
    pub fn builder() -> PathBuilder {
        PathBuilder::default()
    }
    #[must_use]
    pub fn bez_path(&self) -> &BezPath {
        &self.geometry.path
    }
    /// Flutter's `Path.getBounds`: encloses the endpoints and Bézier control
    /// points. Returns no rectangle for an empty or non-finite path.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        *self.geometry.control_bounds.get_or_init(|| {
            let path = self.bez_path();
            (!path.elements().is_empty() && path.is_finite()).then(|| as_rect(path.control_box()))
        })
    }
    /// Tight bounds of the actual curve geometry, for culling and paint damage.
    #[must_use]
    pub fn tight_bounds(&self) -> Option<Rect> {
        self.geometry.tight_bounds
    }
    #[must_use]
    pub const fn id(&self) -> PathId {
        self.id
    }
    /// Renderer-neutral filled-path containment. Single rectangular edges
    /// and extremal joins of a lone filled contour are inclusive as in
    /// Flutter; other closed polygon edges are inclusive when they separate
    /// filled and unfilled regions.
    #[must_use]
    pub fn contains(&self, point: Offset, rule: FillRule) -> bool {
        let offset = point;
        let point = Point::new(f64::from(point.x), f64::from(point.y));
        if let [
            PathEl::MoveTo(a),
            PathEl::LineTo(b),
            PathEl::LineTo(c),
            PathEl::LineTo(d),
            PathEl::ClosePath,
        ] = self.bez_path().elements()
            && ((a.x == b.x && b.y == c.y && c.x == d.x && d.y == a.y)
                || (a.y == b.y && b.x == c.x && c.y == d.y && d.x == a.x))
            && a.x != c.x
            && a.y != c.y
            && a.is_finite()
            && c.is_finite()
        {
            return point.x >= a.x.min(c.x)
                && point.x <= a.x.max(c.x)
                && point.y >= a.y.min(c.y)
                && point.y <= a.y.max(c.y);
        }
        let winding = self.bez_path().winding(point);
        let inside = match rule {
            FillRule::NonZero => winding != 0,
            FillRule::EvenOdd => winding.unsigned_abs() % 2 == 1,
        };
        if inside {
            return true;
        }

        // Winding deliberately skips tangent joins. Preserve Flutter's
        // inclusive cardinal boundary on an oval (and equivalent single
        // closed contours) without filling a degenerate path or turning
        // matching edges of two cancelling contours into solid pixels.
        let Some(bounds) = self.tight_bounds() else {
            return false;
        };
        if bounds.size.width <= 0. || bounds.size.height <= 0. || !bounds.contains(offset) {
            return false;
        }
        if on_single_polygon_edge(self.bez_path(), point) {
            return true;
        }
        if !(offset.x == bounds.origin.x
            || offset.x == bounds.origin.x + bounds.size.width
            || offset.y == bounds.origin.y
            || offset.y == bounds.origin.y + bounds.size.height)
        {
            return false;
        }
        let elements = self.bez_path().elements();
        matches!(elements, [PathEl::MoveTo(_), _, .., PathEl::ClosePath])
            && elements
                .iter()
                .any(|segment| segment.end_point() == Some(point))
            && !elements[1..]
                .iter()
                .any(|segment| matches!(segment, PathEl::MoveTo(_)))
            && self.bez_path().area().abs() > 0.
    }

    /// Wraps an already-built Bézier path. Geometry some live path already
    /// owns is shared with it, identity and bounds included; anything else
    /// gets a fresh identity. Clip fallbacks convert analytic shapes through
    /// here; every other construction keeps its existing entry point.
    pub(crate) fn from_bez_path(path: BezPath) -> Self {
        // Poisoning cannot leave the table inconsistent: every entry is
        // complete before it is inserted.
        INTERNED_PATHS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .intern(path)
    }

    /// The part of `outer` outside `inner` under the non-zero rule: Flutter's
    /// even-odd `Path()..addRect(outer)..addPath(inner)` for an `inner` path
    /// of one winding inside `outer`. `inner` is wound against the rectangle.
    #[must_use]
    pub fn rect_excluding(outer: Rect, inner: &Self) -> Self {
        let left = f64::from(outer.origin.x);
        let top = f64::from(outer.origin.y);
        let right = left + f64::from(outer.size.width);
        let bottom = top + f64::from(outer.size.height);
        let mut path = BezPath::new();
        path.move_to((left, top));
        path.line_to((right, top));
        path.line_to((right, bottom));
        path.line_to((left, bottom));
        path.close_path();
        let mut hole = inner.bez_path().clone();
        if hole.area().signum() == path.area().signum() {
            hole = hole.reverse_subpaths();
        }
        path.extend(hole);
        Self::from_bez_path(path)
    }

    /// Returns a copy of this path with an affine transform applied to its
    /// geometry.  The result is a distinct immutable mesh candidate, sharing
    /// identity only with live paths of that same geometry.  This is used by
    /// higher-level icon primitives to fit a canonical 24px path into the
    /// requested logical size without changing pointer/layout coordinates.
    #[must_use]
    pub fn transformed(&self, transform: Transform) -> Self {
        let mut path = self.bez_path().clone();
        path.apply_affine(transform.to_kurbo());
        Self::from_bez_path(path)
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
        fn append_conic(
            path: &mut BezPath,
            p0: Point,
            p1: Point,
            p2: Point,
            weight: f64,
            depth: u8,
        ) {
            if depth == 0 {
                let k = 4.0 * weight / (3.0 * (1.0 + weight));
                path.curve_to(p0.lerp(p1, k), p2.lerp(p1, k), p2);
                return;
            }
            let scale = 1.0 / (1.0 + weight);
            let middle = ((p0.to_vec2() + p1.to_vec2() * (2.0 * weight) + p2.to_vec2())
                * (0.5 * scale))
                .to_point();
            let half = ((1.0 + weight) / 2.0).sqrt();
            let left_control = ((p0.to_vec2() + p1.to_vec2() * weight) * scale).to_point();
            let right_control = ((p1.to_vec2() * weight + p2.to_vec2()) * scale).to_point();
            append_conic(path, p0, left_control, middle, half, depth - 1);
            append_conic(path, middle, right_control, p2, half, depth - 1);
        }
        append_conic(&mut self.path, start, point(control), point(end), weight, 2);
        self
    }

    /// Adds a closed rounded rectangle, its arcs as cubics within 0.01
    /// logical pixels.
    pub fn rounded_rect(&mut self, rect: Rect, radii: CornerRadii) -> &mut Self {
        let radii = radii.normalized(rect.size);
        if radii.is_zero() {
            let shape = kurbo::Rect::new(
                f64::from(rect.origin.x),
                f64::from(rect.origin.y),
                f64::from(rect.origin.x + rect.size.width),
                f64::from(rect.origin.y + rect.size.height),
            )
            .abs();
            self.path.extend(shape.path_elements(0.01));
            return self;
        }
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

    /// Adds an RSuperellipse with independent x/y radii. Corners are
    /// specified clockwise starting at the upper left.
    pub fn rounded_superellipse_elliptical(
        &mut self,
        rect: Rect,
        radii: [incular_core::Size; 4],
    ) -> &mut Self {
        crate::rsuperellipse::rounded_superellipse_elliptical(self, rect, radii);
        self
    }

    /// Adds every contour of `path`, as Flutter `Path.addPath` with no
    /// offset.
    pub fn append(&mut self, path: &Path) -> &mut Self {
        self.path.extend(path.bez_path().elements().iter().copied());
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
