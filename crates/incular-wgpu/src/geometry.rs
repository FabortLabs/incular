use super::prelude::*;
use kurbo::{BezPath, Point};

/// Flattening tolerance for path outlines, in path units.
const FLATTEN_TOLERANCE: f64 = 0.05;
/// Accuracy of the stroke-to-outline expansion, in path units.
const STROKE_TOLERANCE: f64 = 0.01;
/// Longest antialiasing miter, in half-pixels, at a sharp outline corner.
const MAX_FRINGE_MITER: f32 = 4.;

/// CPU tessellation output. It deliberately contains no GPU objects so the
/// renderer may cache/upload it independently of Incular's Path type.
///
/// The mesh is antialiased by coverage, not multisampling: every outline
/// vertex carries an outward miter direction in [`Self::extrusions`], and
/// the vertex stage moves it by half a physical pixel along that direction
/// (outward for a fringe's outer edge, inward for the interior and the
/// fringe's inner edge). [`Self::coverage`] then ramps from 1 to 0 across
/// the one-pixel fringe, at any scale or transform the mesh is drawn with.
/// In path units the fringe has no area, so [`Self::vertices`] alone is the
/// exact filled shape (what a stencil clip mask uses).
#[derive(Clone, Debug, Default)]
pub struct PathMesh {
    pub vertices: Vec<[f32; 2]>,
    /// Per-vertex extrusion direction in path units; its length is the miter
    /// scale of the outline corner it belongs to.
    pub extrusions: Vec<[f32; 2]>,
    /// Per-vertex coverage: 1 inside the shape, 0 on the fringe's outer edge.
    pub coverage: Vec<f32>,
    pub indices: Vec<u32>,
}

impl PathMesh {
    /// Interleaved `[x, y, extrude_x, extrude_y, coverage]` vertices, the
    /// layout of the retained path vertex buffer.
    pub(crate) fn gpu_vertices(&self) -> Vec<[f32; 5]> {
        self.vertices
            .iter()
            .zip(&self.extrusions)
            .zip(&self.coverage)
            .map(|((position, extrusion), coverage)| {
                [
                    position[0],
                    position[1],
                    extrusion[0],
                    extrusion[1],
                    *coverage,
                ]
            })
            .collect()
    }

    fn push(&mut self, position: Point, extrusion: [f32; 2], coverage: f32) -> u32 {
        let index = self.vertices.len() as u32;
        self.vertices.push([position.x as f32, position.y as f32]);
        self.extrusions.push(extrusion);
        self.coverage.push(coverage);
        index
    }
}

/// Tessellates a fill, or a stroke expanded to its filled outline, into an
/// antialiased [`PathMesh`].
pub fn tessellate_path(
    path: &incular_painting::Path,
    fill_rule: incular_painting::FillRule,
    stroke: Option<incular_painting::Stroke>,
) -> Option<PathMesh> {
    let (outline, fill_rule) = match stroke {
        Some(stroke) => (stroke_outline(path.bez_path(), stroke)?, FillRule::NonZero),
        None => (path.bez_path().clone(), fill_rule),
    };
    let even_odd = fill_rule == FillRule::EvenOdd;
    let contours = flatten_contours(&outline);
    if contours.is_empty() {
        return None;
    }
    let miters: Vec<Vec<[f32; 2]>> = contours
        .iter()
        .map(|contour| contour_miters(contour, outward_sign(contour, &contours, even_odd)))
        .collect();

    // Interior: the flattened outline, each endpoint carrying its miter so
    // the vertex stage can pull the interior half a pixel inside the edge.
    let mut builder = LyonPath::builder_with_attributes(2);
    for (contour, miters) in contours.iter().zip(&miters) {
        builder.begin(lyon_point(contour[0]), &miters[0][..]);
        for (point, miter) in contour.iter().zip(miters).skip(1) {
            builder.line_to(lyon_point(*point), &miter[..]);
        }
        builder.end(true);
    }
    let lyon = builder.build();
    let mut interior: VertexBuffers<([f32; 2], [f32; 2]), u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &lyon,
            &FillOptions::default().with_fill_rule(if even_odd {
                LyonFillRule::EvenOdd
            } else {
                LyonFillRule::NonZero
            }),
            &mut lyon_tessellation::BuffersBuilder::new(
                &mut interior,
                |mut vertex: lyon_tessellation::FillVertex| {
                    let position = vertex.position();
                    let attributes = vertex.interpolated_attributes();
                    (
                        [position.x, position.y],
                        [
                            attributes.first().copied().unwrap_or(0.),
                            attributes.get(1).copied().unwrap_or(0.),
                        ],
                    )
                },
            ),
        )
        .ok()?;
    if interior.indices.is_empty() {
        return None;
    }
    let mut mesh = PathMesh::default();
    for (position, miter) in interior.vertices {
        mesh.push(
            Point::new(f64::from(position[0]), f64::from(position[1])),
            [-miter[0], -miter[1]],
            1.,
        );
    }
    mesh.indices = interior.indices;

    // Fringe: a quad strip straddling every outline edge, opaque on its
    // inner side and transparent on its outer side.
    for (contour, miters) in contours.iter().zip(&miters) {
        let base = mesh.vertices.len() as u32;
        for (point, miter) in contour.iter().zip(miters) {
            mesh.push(*point, [-miter[0], -miter[1]], 1.);
            mesh.push(*point, *miter, 0.);
        }
        let count = contour.len() as u32;
        for index in 0..count {
            let next = (index + 1) % count;
            let (inner, outer) = (base + index * 2, base + index * 2 + 1);
            let (next_inner, next_outer) = (base + next * 2, base + next * 2 + 1);
            mesh.indices
                .extend([inner, outer, next_outer, inner, next_outer, next_inner]);
        }
    }
    Some(mesh)
}

fn lyon_point(at: Point) -> lyon_tessellation::math::Point {
    point(at.x as f32, at.y as f32)
}

/// The filled outline a stroke covers, so strokes share the fill's
/// antialiasing.
fn stroke_outline(path: &BezPath, stroke: incular_painting::Stroke) -> Option<BezPath> {
    if !(stroke.width.is_finite() && stroke.width > 0.) {
        return None;
    }
    let style = kurbo::Stroke::new(f64::from(stroke.width))
        .with_caps(match stroke.cap {
            LineCap::Butt => kurbo::Cap::Butt,
            LineCap::Round => kurbo::Cap::Round,
            LineCap::Square => kurbo::Cap::Square,
        })
        .with_join(match stroke.join {
            LineJoin::Miter => kurbo::Join::Miter,
            LineJoin::Round => kurbo::Join::Round,
            LineJoin::Bevel => kurbo::Join::Bevel,
        })
        .with_miter_limit(f64::from(stroke.miter_limit.max(1.01)));
    Some(kurbo::stroke(
        path.iter(),
        &style,
        &kurbo::StrokeOpts::default(),
        STROKE_TOLERANCE,
    ))
}

/// Closed polylines of the outline. Fill semantics close open subpaths;
/// repeated points and contours without area are dropped.
fn flatten_contours(outline: &BezPath) -> Vec<Vec<Point>> {
    fn finish(current: &mut Vec<Point>, contours: &mut Vec<Vec<Point>>) {
        let mut contour = std::mem::take(current);
        contour.dedup_by(|a, b| a.distance(*b) < 1e-6);
        while contour.len() > 1
            && contour
                .first()
                .zip(contour.last())
                .is_some_and(|(first, last)| first.distance(*last) < 1e-6)
        {
            contour.pop();
        }
        if contour.len() >= 3
            && contour
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
        {
            contours.push(contour);
        }
    }
    let mut contours = Vec::new();
    let mut current = Vec::new();
    kurbo::flatten(outline.iter(), FLATTEN_TOLERANCE, |element| match element {
        PathEl::MoveTo(point) => {
            finish(&mut current, &mut contours);
            current.push(point);
        }
        PathEl::LineTo(point) => current.push(point),
        PathEl::ClosePath => finish(&mut current, &mut contours),
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    finish(&mut current, &mut contours);
    contours
}

/// `1.` when the filled side of `contour` is to the left of its edges (so
/// its outward normals point right), `-1.` otherwise. Decided at the
/// longest edge against the whole outline and fill rule, which also orients
/// holes and self-overlapping stroke outlines correctly.
fn outward_sign(contour: &[Point], contours: &[Vec<Point>], even_odd: bool) -> f64 {
    let count = contour.len();
    let (start, end) = (0..count)
        .map(|index| (contour[index], contour[(index + 1) % count]))
        .max_by(|(a0, a1), (b0, b1)| a0.distance(*a1).total_cmp(&b0.distance(*b1)))
        .expect("contours have edges");
    let delta = end - start;
    let length = delta.hypot();
    if length <= 0. {
        return 1.;
    }
    let left = kurbo::Vec2::new(-delta.y, delta.x) / length;
    let probe = start.midpoint(end) + left * (length * 1e-3).clamp(1e-5, 1e-2);
    let winding = winding_number(probe, contours);
    let filled = if even_odd {
        winding % 2 != 0
    } else {
        winding != 0
    };
    if filled { 1. } else { -1. }
}

fn winding_number(point: Point, contours: &[Vec<Point>]) -> i32 {
    let mut winding = 0;
    for contour in contours {
        let count = contour.len();
        for index in 0..count {
            let (a, b) = (contour[index], contour[(index + 1) % count]);
            let side = (b - a).cross(point - a);
            if a.y <= point.y {
                if b.y > point.y && side > 0. {
                    winding += 1;
                }
            } else if b.y <= point.y && side < 0. {
                winding -= 1;
            }
        }
    }
    winding
}

/// Outward miter vector at every vertex of `contour`: the bisector of its
/// two edges' outward normals, lengthened so the fringe keeps half a pixel
/// of width along both edges, and capped at sharp corners.
fn contour_miters(contour: &[Point], sign: f64) -> Vec<[f32; 2]> {
    let count = contour.len();
    let normal = |index: usize| {
        let delta = contour[(index + 1) % count] - contour[index];
        let length = delta.hypot();
        if length <= 0. {
            kurbo::Vec2::ZERO
        } else {
            kurbo::Vec2::new(delta.y, -delta.x) * (sign / length)
        }
    };
    (0..count)
        .map(|index| {
            let previous = normal((index + count - 1) % count);
            let next = normal(index);
            let sum = previous + next;
            let length = sum.hypot();
            let miter = if length < 1e-6 {
                if previous.hypot() > 0. {
                    previous
                } else {
                    next
                }
            } else {
                let bisector = sum / length;
                let cosine = bisector.dot(previous).max(f64::from(1. / MAX_FRINGE_MITER));
                bisector / cosine
            };
            [miter.x as f32, miter.y as f32]
        })
        .collect()
}
