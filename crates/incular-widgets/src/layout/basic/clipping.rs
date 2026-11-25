//! Clipping layout primitives.

use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

use incular_config::Clip;
use incular_core::{Offset, Rect, Size};
use incular_rendering::{CornerRadii, Path, RRect};
use typed_builder::TypedBuilder;

use crate::{Widget, WidgetKind, macros::widget_from_kind};

/// The geometry a [`CustomClipper`] resolves for a laid-out size, in the
/// clipped box's local coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum ClipShape {
    /// An axis-aligned rectangle.
    Rect(Rect),
    /// A rounded rectangle.
    RRect(RRect),
    /// An arbitrary path, filled with the non-zero rule.
    Path(Arc<Path>),
}

impl ClipShape {
    /// The shape's bounding rectangle.
    #[must_use]
    pub(crate) fn bounds(&self) -> Rect {
        match self {
            Self::Rect(rect) => *rect,
            Self::RRect(rrect) => rrect.rect,
            Self::Path(path) => path
                .bounds()
                .unwrap_or(Rect::from_origin_size(Offset::ZERO, Size::ZERO)),
        }
    }
}

/// Flutter's `CustomClipper`: derives a clip from the laid-out size and
/// decides whether an existing clip must be rebuilt when the delegate changes.
pub trait CustomClipper: 'static {
    /// Returns the clip for a box of `size`.
    fn get_clip(&self, size: Size) -> ClipShape;

    /// Whether the clip must be recomputed when this delegate replaces `old`.
    /// Flutter's `shouldReclip`; a delegate that returns `false` keeps the
    /// current clip until the size changes.
    fn should_reclip(&self, old: &Self) -> bool
    where
        Self: Sized,
    {
        let _ = old;
        true
    }
}

struct FnClipper<F>(F);

impl<F: Fn(Size) -> ClipShape + 'static> CustomClipper for FnClipper<F> {
    fn get_clip(&self, size: Size) -> ClipShape {
        (self.0)(size)
    }
}

fn get_clip_erased<C: CustomClipper>(clipper: &dyn Any, size: Size) -> ClipShape {
    match clipper.downcast_ref::<C>() {
        Some(clipper) => clipper.get_clip(size),
        None => ClipShape::Rect(Rect::from_origin_size(Offset::ZERO, size)),
    }
}

fn should_reclip_erased<C: CustomClipper>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<C>(), old.downcast_ref::<C>()) {
        (Some(new), Some(old)) => new.should_reclip(old),
        _ => true,
    }
}

/// A type-erased [`CustomClipper`] retained by a clip widget.
#[derive(Clone)]
pub struct ClipDelegate {
    clipper: Rc<dyn Any>,
    get_clip: fn(&dyn Any, Size) -> ClipShape,
    should_reclip: fn(&dyn Any, &dyn Any) -> bool,
}

impl ClipDelegate {
    /// Retains a clipper delegate.
    #[must_use]
    pub fn new<C: CustomClipper>(clipper: C) -> Self {
        Self {
            clipper: Rc::new(clipper),
            get_clip: get_clip_erased::<C>,
            should_reclip: should_reclip_erased::<C>,
        }
    }

    /// Retains a closure that maps a laid-out size to a clip.
    #[must_use]
    pub fn from_fn(get_clip: impl Fn(Size) -> ClipShape + 'static) -> Self {
        Self::new(FnClipper(get_clip))
    }

    /// Resolves the clip for `size`.
    #[must_use]
    pub(crate) fn clip(&self, size: Size) -> ClipShape {
        (self.get_clip)(&*self.clipper, size)
    }

    /// Whether this delegate, replacing `old`, requires the clip to be rebuilt.
    #[must_use]
    pub(crate) fn reclips_from(&self, old: &Self) -> bool {
        (self.should_reclip)(&*self.clipper, &*old.clipper)
    }
}

impl std::fmt::Debug for ClipDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipDelegate").finish_non_exhaustive()
    }
}

impl PartialEq for ClipDelegate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.clipper, &other.clipper)
    }
}

/// Clips its child using a rectangular boundary.
///
/// The clip bounds raster output only. Hit testing follows layout
/// bounds through the clip (points outside the clip node's bounds miss
/// as usual), and semantic bounds stay whole. `Clip::None` disables
/// clipping entirely; use `Visibility` to also remove input.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ClipRect {
    #[builder(default = Clip::HardEdge)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option))]
    clipper: Option<ClipDelegate>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl ClipRect {
    /// Creates a ClipRect widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            clip_behavior: Clip::HardEdge,
            clipper: None,
            child: Some(child.into()),
        }
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Clips to a rectangle derived from the laid-out size. Non-rectangular
    /// shapes clip to their bounds.
    #[must_use]
    pub fn custom_clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

widget_from_kind!(ClipRect => ClipRect { clip_behavior, clipper, child });

/// Clips its child using a rounded-rectangular boundary.
///
/// Same raster-only contract as [`ClipRect`]: the rounded corners bound
/// pixels, while hit testing and semantic bounds keep the node's full
/// rectangular geometry.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ClipRRect {
    #[builder(default, setter(into))]
    radius: CornerRadii,
    #[builder(default, setter(strip_option))]
    directional_radius: Option<crate::BorderRadiusDirectional>,
    #[builder(default = Clip::AntiAlias)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option))]
    clipper: Option<ClipDelegate>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl ClipRRect {
    /// Creates a ClipRRect with uniform or corner radii.
    #[must_use]
    pub fn new(radius: impl Into<CornerRadii>, child: impl Into<Widget>) -> Self {
        Self {
            radius: radius.into(),
            directional_radius: None,
            clip_behavior: Clip::AntiAlias,
            clipper: None,
            child: Some(child.into()),
        }
    }

    /// Resolves start and end radii against the nearest Directionality.
    /// When set, this takes precedence over the physical corner radii.
    #[must_use]
    pub fn directional_radius(mut self, radius: crate::BorderRadiusDirectional) -> Self {
        self.directional_radius = Some(radius);
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Clips to a rounded rectangle derived from the laid-out size. A
    /// returned rectangle takes this widget's radius; a rounded rectangle
    /// takes its own radii.
    #[must_use]
    pub fn custom_clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

widget_from_kind!(ClipRRect => ClipRRect { radius, directional_radius, clip_behavior, clipper, child });

/// Clips its child using an elliptical / oval boundary.
///
/// Same raster-only contract as [`ClipRect`]: the oval bounds pixels,
/// while hit testing and semantic bounds keep the node's full
/// rectangular geometry.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ClipOval {
    #[builder(default = Clip::AntiAlias)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option))]
    clipper: Option<ClipDelegate>,
    #[builder(setter(into))]
    child: Widget,
}

impl ClipOval {
    /// Creates a ClipOval widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            clip_behavior: Clip::AntiAlias,
            clipper: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Clips to the oval inscribed in the bounds of the shape the clipper
    /// returns for the laid-out size.
    #[must_use]
    pub fn custom_clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

widget_from_kind!(ClipOval => ClipOval { clip_behavior, clipper, child });

/// Clips its child using an arbitrary [`Path`] boundary.
///
/// Without an explicit path or clipper, the clip follows the child's
/// layout rectangle. Explicit paths and custom clippers also narrow hit
/// testing. The paint path uses the non-zero fill rule.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ClipPath {
    #[builder(default, setter(strip_option, into))]
    path: Option<Arc<Path>>,
    #[builder(default = Clip::AntiAlias)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option))]
    clipper: Option<ClipDelegate>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl ClipPath {
    /// Creates a ClipPath widget.
    #[must_use]
    pub fn new(path: impl Into<Arc<Path>>, child: impl Into<Widget>) -> Self {
        Self {
            path: Some(path.into()),
            clip_behavior: Clip::AntiAlias,
            clipper: None,
            child: Some(child.into()),
        }
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Clips to the path the clipper returns for the laid-out size, replacing
    /// the fixed path. A returned rectangle or rounded rectangle clips to its
    /// outline.
    #[must_use]
    pub fn custom_clipper(mut self, clipper: impl CustomClipper) -> Self {
        self.clipper = Some(ClipDelegate::new(clipper));
        self
    }
}

widget_from_kind!(ClipPath => ClipPath { path, clip_behavior, clipper, child });
