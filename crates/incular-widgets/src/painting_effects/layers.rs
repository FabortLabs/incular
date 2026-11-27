//! Retained layer, clipping, physical-model, and grid descriptors.

use std::{rc::Rc, sync::Arc};

use incular_config::Clip;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, CornerRadii, DisplayList, FillRule, Path, Stroke};
use typed_builder::TypedBuilder;

use super::{
    BorderRadiusGeometry, OvalBorder, RoundedSuperellipseBorder, ShapeBorder,
    decoration::{BorderSide, BoxShape},
};
use crate::{
    ClipOval, ClipPath, ClipRRect, CustomPaint, CustomPainter, DecoratedBox, DropShadow, Positioned,
    Stack, StackFit, Widget,
};

/// Renders its child into a retained raster snapshot for performance optimization.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SnapshotWidget {
    #[builder(setter(into))]
    child: Widget,
}

impl SnapshotWidget {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl From<SnapshotWidget> for Widget {
    fn from(value: SnapshotWidget) -> Self {
        crate::layout::RepaintBoundary::new(value.child).into()
    }
}

/// Clips its child using a rounded superellipse (squircle) shape.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ClipRSuperellipse {
    #[builder(
        default = 0.0,
        setter(transform = |radius: f32| radius.max(0.0))
    )]
    pub(super) radius: f32,
    #[builder(default, setter(strip_option, into))]
    border_radius: Option<BorderRadiusGeometry>,
    #[builder(default = Clip::AntiAlias)]
    pub(super) clip_behavior: Clip,
    #[builder(setter(into))]
    child: Widget,
}

impl ClipRSuperellipse {
    #[must_use]
    pub fn new(radius: f32, child: impl Into<Widget>) -> Self {
        Self {
            radius: radius.max(0.0),
            border_radius: None,
            clip_behavior: Clip::AntiAlias,
            child: child.into(),
        }
    }

    /// Replaces the uniform radius with physical, directional or mixed
    /// corners, preserving both axes.
    #[must_use]
    pub fn border_radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> Self {
        self.border_radius = Some(radius.into());
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<ClipRSuperellipse> for Widget {
    fn from(value: ClipRSuperellipse) -> Self {
        let radius = value.border_radius
            .unwrap_or_else(|| BorderRadiusGeometry::circular(value.radius));
        // The clipper is evaluated for the child's actual layout size, not
        // the maximum incoming constraints. The shared outline also retains
        // its identity between ordinary rebuilds.
        // Flutter does not narrow a default superellipse's box hit test
        // to the painted corner curve.
        ClipPath::shape_with_hit_policy(
            Rc::new(RoundedSuperellipseBorder::new(BorderSide::NONE, radius)),
            value.clip_behavior,
            Some(value.child),
            false,
        )
    }
}

/// A physical layer widget with elevation, shadow, and corner radius.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct PhysicalModel {
    pub(crate) color: Color,
    #[builder(default = Color::BLACK)]
    pub(crate) shadow_color: Color,
    #[builder(
        default = 0.0,
        setter(transform = |elevation: f32| elevation.max(0.0))
    )]
    pub(crate) elevation: f32,
    #[builder(default = CornerRadii::default())]
    pub(crate) border_radius: CornerRadii,
    /// Flutter `shape`: a rectangle with `border_radius`, or a circle.
    #[builder(default = BoxShape::Rectangle)]
    pub(crate) shape: BoxShape,
    #[builder(default = Clip::None)]
    pub(crate) clip_behavior: Clip,
    #[builder(setter(into))]
    pub(crate) child: Widget,
}

impl PhysicalModel {
    #[must_use]
    pub fn new(color: Color, child: impl Into<Widget>) -> Self {
        Self {
            color,
            shadow_color: Color::BLACK,
            elevation: 0.0,
            border_radius: CornerRadii::default(),
            shape: BoxShape::Rectangle,
            clip_behavior: Clip::None,
            child: child.into(),
        }
    }

    /// Flutter `PhysicalModel.shape`; circles ignore `border_radius`.
    #[must_use]
    pub fn shape(mut self, shape: BoxShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation.max(0.0);
        self
    }

    #[must_use]
    pub fn border_radius(mut self, radius: CornerRadii) -> Self {
        self.border_radius = radius;
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<PhysicalModel> for Widget {
    fn from(value: PhysicalModel) -> Self {
        // Keep the wrapper mounted even at Clip.none: toggling physical
        // paint clipping must not rebuild the child's focus or local state.
        let child: Widget = match value.shape {
            BoxShape::Circle => ClipOval::new(value.child)
                .retain_rectangular_hit_test()
                .clip_behavior(value.clip_behavior)
                .into(),
            BoxShape::Rectangle => ClipRRect::new(value.border_radius, value.child)
                .clip_behavior(value.clip_behavior)
                .into(),
        };
        let surface: Widget = match value.shape {
            BoxShape::Circle => CustomPaint::from_painter(Size::ZERO, PhysicalOval(value.color))
                .child(child)
                .into(),
            BoxShape::Rectangle => DecoratedBox::new(child)
                .background(value.color)
                .corner_radii(value.border_radius)
                .into(),
        };
        // Keep the same retained proxy when elevation turns on or off.
        elevation_shadow(value.elevation, value.shadow_color, surface)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct PhysicalOval(Color);

impl CustomPainter for PhysicalOval {
    fn paint(&self, size: Size) -> DisplayList {
        if size.width <= 0. || size.height <= 0. {
            return DisplayList::default();
        }
        // Unlike BoxDecoration's circle, RenderPhysicalModel's circle
        // uses half the width AND half the height as its ellipse radii.
        let rect = Rect::from_origin_size(Offset::ZERO, size);
        let path = OvalBorder::new(BorderSide::NONE).outer_path(rect);
        let mut canvas = Canvas::default();
        canvas.fill_path(Arc::new(path), self.0, FillRule::NonZero);
        canvas.finish()
    }

    fn should_repaint(&self, old: &Self) -> bool {
        self != old
    }

    fn hit_test(&self, _: Offset) -> Option<bool> {
        // A physical model paints a backing for its child; the paint is
        // not a hit target in its own right.
        Some(false)
    }
}

/// The elevation shadow both physical widgets cast: an offset, blurred copy
/// of the surface. Flutter's `Canvas.drawShadow` uses a different
/// ambient/spot model; this keeps the existing approximation for both.
fn elevation_shadow(elevation: f32, shadow_color: Color, surface: impl Into<Widget>) -> Widget {
    DropShadow::new(
        Offset::new(0.0, elevation * 0.18),
        if elevation > 0.0 {
            (elevation * 0.55).max(1.0)
        } else {
            0.0
        },
        if elevation > 0.0 {
            shadow_color
        } else {
            Color::TRANSPARENT
        },
        surface,
    )
    .into()
}

/// A physical layer widget with custom path shape and elevation.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct PhysicalShape {
    #[builder(setter(into))]
    clipper: Arc<Path>,
    color: Color,
    #[builder(default = Color::BLACK)]
    shadow_color: Color,
    #[builder(
        default = 0.0,
        setter(transform = |elevation: f32| elevation.max(0.0))
    )]
    pub(super) elevation: f32,
    /// Flutter `clipBehavior`, default `Clip::None`: the child is clipped to
    /// the path only when this is set.
    #[builder(default = Clip::None)]
    clip_behavior: Clip,
    #[builder(setter(into))]
    child: Widget,
}

impl PhysicalShape {
    #[must_use]
    pub fn new(clipper: impl Into<Arc<Path>>, color: Color, child: impl Into<Widget>) -> Self {
        Self {
            clipper: clipper.into(),
            color,
            shadow_color: Color::BLACK,
            elevation: 0.0,
            clip_behavior: Clip::None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation.max(0.0);
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<PhysicalShape> for Widget {
    /// Flutter `RenderPhysicalShape.paint`: the elevation shadow and the
    /// filled path paint behind the child, which is clipped to the path only
    /// when `clip_behavior` is set. The fill is a layer sized to the child, as
    /// the decoration painter sizes its layers.
    fn from(value: PhysicalShape) -> Self {
        let PhysicalShape {
            clipper,
            color,
            shadow_color,
            elevation,
            clip_behavior,
            child,
        } = value;
        // The custom outline always restricts hit testing, even when it
        // does not clip painted descendants. Keep the same mounted child
        // when only the paint mode changes.
        let child: Widget = ClipPath::new(clipper.clone(), child)
            .clip_behavior(clip_behavior)
            .into();
        // The path is already local geometry. A tightly positioned
        // CustomPaint adopts the Stack size without a fresh layout closure.
        let surface: Widget = CustomPaint::from_painter(
            Size::ZERO,
            PhysicalPathSurface { path: clipper, color },
        ).into();
        let fill = if elevation > 0.0 {
            elevation_shadow(elevation, shadow_color, surface)
        } else {
            surface
        };
        Stack::new([Positioned::fill(fill).into(), child])
            .fit(StackFit::Passthrough)
            .clip_behavior(Clip::None)
            .into()
    }
}

#[derive(Clone, PartialEq)]
struct PhysicalPathSurface {
    path: Arc<Path>,
    color: Color,
}

impl CustomPainter for PhysicalPathSurface {
    fn paint(&self, _: Size) -> DisplayList {
        let mut canvas = Canvas::default();
        canvas.fill_path(self.path.clone(), self.color, FillRule::NonZero);
        canvas.finish()
    }

    fn should_repaint(&self, old: &Self) -> bool {
        self != old
    }

    fn hit_test(&self, _: Offset) -> Option<bool> {
        Some(false)
    }
}

/// Draws a foreground engineering grid without intercepting pointer hits.
///
/// An interval must be finite and positive and both division counts must
/// be nonzero. Invalid parameters or grids requiring more than 16,384
/// lines leave the content visible without a grid.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct GridPaper {
    #[builder(default = Color::rgba(195, 232, 243, 127))]
    color: Color,
    #[builder(default = 100.0)]
    interval: f32,
    #[builder(default = 2)]
    divisions: usize,
    #[builder(default = 5)]
    subdivisions: usize,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Default for GridPaper {
    fn default() -> Self {
        Self::new()
    }
}

impl GridPaper {
    #[must_use]
    pub fn new() -> Self {
        Self {
            color: Color::rgba(195, 232, 243, 127),
            interval: 100.0,
            divisions: 2,
            subdivisions: 5,
            child: None,
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn interval(mut self, interval: f32) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the positive number of divisions per interval.
    #[must_use]
    pub fn divisions(mut self, divisions: usize) -> Self {
        self.divisions = divisions;
        self
    }

    /// Sets the positive number of subdivisions.
    #[must_use]
    pub fn subdivisions(mut self, subdivisions: usize) -> Self {
        self.subdivisions = subdivisions;
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GridPaperPainter {
    color: Color,
    interval: f32,
    divisions: usize,
    subdivisions: usize,
}

impl CustomPainter for GridPaperPainter {
    fn paint(&self, size: Size) -> DisplayList {
        const MAX_GRID_LINES: u32 = 16_384;
        let interval = f64::from(self.interval);
        if !interval.is_finite() || interval <= 0. || self.divisions == 0 || self.subdivisions == 0
        {
            return DisplayList::default();
        }
        let step = interval / (self.divisions as f64 * self.subdivisions as f64);
        let medium = interval / self.subdivisions as f64;
        let estimated_lines =
            (f64::from(size.width) / step).floor() + (f64::from(size.height) / step).floor() + 2.;
        if estimated_lines > f64::from(MAX_GRID_LINES) {
            return DisplayList::default();
        }
        let mut canvas = Canvas::default();
        let mut remaining = MAX_GRID_LINES;
        for (extent, vertical) in [(size.width, true), (size.height, false)] {
            let mut coordinate = 0.;
            while coordinate <= f64::from(extent) {
                // Guard the actual count as well: accumulated rounding can
                // place one more stroke on the final edge than division predicts.
                if remaining == 0 {
                    return DisplayList::default();
                }
                remaining -= 1;
                let width = if coordinate % interval == 0. {
                    1.
                } else if coordinate % medium == 0. {
                    0.5
                } else {
                    0.25
                };
                // Accumulate in double precision as Flutter does; only the
                // final points use the renderer-neutral logical pixel type.
                let position = coordinate as f32;
                let (start, end) = if vertical {
                    (
                        Offset::new(position, 0.),
                        Offset::new(position, size.height),
                    )
                } else {
                    (Offset::new(0., position), Offset::new(size.width, position))
                };
                let mut path = Path::builder();
                path.move_to(start).line_to(end);
                canvas.stroke_path(
                    Arc::new(path.build()),
                    self.color,
                    Stroke {
                        width,
                        ..Stroke::default()
                    },
                );
                coordinate += step;
            }
        }
        canvas.finish()
    }

    fn should_repaint(&self, old: &Self) -> bool {
        self != old
    }

    fn hit_test(&self, _: Offset) -> Option<bool> {
        Some(false)
    }
}

impl From<GridPaper> for Widget {
    fn from(value: GridPaper) -> Self {
        let paint = CustomPaint::new(Size::ZERO, DisplayList::default()).foreground_painter(
            GridPaperPainter {
                color: value.color,
                interval: value.interval,
                divisions: value.divisions,
                subdivisions: value.subdivisions,
            },
        );
        if let Some(child) = value.child {
            paint.child(child).into()
        } else {
            paint.into()
        }
    }
}
