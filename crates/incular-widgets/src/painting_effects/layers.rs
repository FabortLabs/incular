//! Retained layer, clipping, physical-model, and grid descriptors.

use std::sync::Arc;

use incular_config::Clip;
use incular_core::{Color, Offset, Rect};
use incular_rendering::{Canvas, CornerRadii, FillRule, Path};
use typed_builder::TypedBuilder;

use super::decoration::{BoxDecoration, BoxShape};
use crate::{
    ClipOval, ClipPath, ClipRRect, CustomPaint, DecoratedBox, DropShadow, LayoutBuilder,
    Positioned, Stack, StackFit, Widget,
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
            clip_behavior: Clip::AntiAlias,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<ClipRSuperellipse> for Widget {
    /// Flutter `RenderClipRSuperellipse`: the child is clipped to the
    /// rounded-superellipse outline (the squircle profile of
    /// `Path.addRSuperellipse`), not to circular arcs. The outline is built
    /// for the largest size the constraints allow, as [`ClipPath::shape`]
    /// does, so it matches the child under tight constraints.
    fn from(value: ClipRSuperellipse) -> Self {
        let ClipRSuperellipse {
            radius,
            clip_behavior,
            child,
        } = value;
        LayoutBuilder::new(move |_, constraints| {
            let rect = Rect::from_origin_size(Offset::ZERO, constraints.biggest());
            let mut builder = Path::builder();
            builder.rounded_superellipse(rect, CornerRadii::uniform(radius));
            ClipPath::new(Arc::new(builder.build()), child.clone())
                .clip_behavior(clip_behavior)
                .into()
        })
        .into()
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
        // Flutter clips the child to the model's shape when `clipBehavior`
        // is not `none` (`RenderPhysicalModel.paint`).
        let child = match (value.shape, value.clip_behavior) {
            (_, Clip::None) => value.child,
            (BoxShape::Circle, clip) => ClipOval::new(value.child).clip_behavior(clip).into(),
            (BoxShape::Rectangle, clip) => ClipRRect::new(value.border_radius, value.child)
                .clip_behavior(clip)
                .into(),
        };
        let surface: Widget = match value.shape {
            BoxShape::Circle => DecoratedBox::new(child)
                .decoration(
                    BoxDecoration::new()
                        .color(value.color)
                        .shape(BoxShape::Circle),
                )
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
        let child = match clip_behavior {
            Clip::None => child,
            clip => ClipPath::new(clipper.clone(), child)
                .clip_behavior(clip)
                .into(),
        };
        // The path is already local geometry. A tightly positioned
        // CustomPaint adopts the Stack size without a fresh layout closure.
        let mut canvas = Canvas::default();
        canvas.fill_path(clipper, color, FillRule::NonZero);
        let surface: Widget = CustomPaint::new(incular_core::Size::ZERO, canvas.finish()).into();
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

/// Draws an engineering grid paper over its background.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct GridPaper {
    #[builder(default = Color::rgba(195, 232, 243, 127))]
    color: Color,
    #[builder(
        default = 100.0,
        setter(transform = |interval: f32| interval.max(1.0))
    )]
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
        self.interval = interval.max(1.0);
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

impl From<GridPaper> for Widget {
    fn from(value: GridPaper) -> Self {
        value
            .child
            .unwrap_or_else(|| crate::layout::SizedBox::shrink().into())
    }
}
