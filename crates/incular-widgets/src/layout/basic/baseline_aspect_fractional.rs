//! Baseline, aspect-ratio, fractional-sizing, and fitted-box primitives.

use incular_config::{Alignment, Clip};
use typed_builder::TypedBuilder;

use crate::tree::BoxFit;
use crate::{AlignmentGeometry, Widget, WidgetKind, macros::widget_from_kind};

/// Sizes child as a fraction of incoming parent constraints.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct FractionallySizedBox {
    #[builder(
        default,
        setter(transform = |factor: f32| Some(factor.max(0.0)))
    )]
    pub(crate) width_factor: Option<f32>,
    #[builder(
        default,
        setter(transform = |factor: f32| Some(factor.max(0.0)))
    )]
    pub(crate) height_factor: Option<f32>,
    #[builder(default = AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    pub(crate) alignment: AlignmentGeometry,
    #[builder(default, setter(strip_option, into))]
    pub(crate) child: Option<Widget>,
}

impl Default for FractionallySizedBox {
    fn default() -> Self {
        Self::empty()
    }
}

impl FractionallySizedBox {
    /// Creates a FractionallySizedBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::empty().child(child)
    }

    /// Creates a childless box that still reserves its fraction of each bounded axis.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            alignment: AlignmentGeometry::Physical(Alignment::CENTER),
            child: None,
        }
    }

    /// Sets the child sized by the incoming constraints and configured factors.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Where the child sits when it is smaller or larger than the box.
    /// Defaults to center, as in Flutter.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    #[must_use]
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor.max(0.0));
        self
    }

    #[must_use]
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor.max(0.0));
        self
    }
}

widget_from_kind!(FractionallySizedBox => Fractional { width_factor, height_factor, alignment, child });

/// Positions the child baseline at an explicit, signed vertical offset.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Baseline {
    #[builder(setter(transform = |baseline: f32| incular_core::finite_or_zero(baseline)))]
    baseline: f32,
    #[builder(setter(into))]
    child: Widget,
}

impl Baseline {
    /// Creates a Baseline widget.
    #[must_use]
    pub fn new(baseline: f32, child: impl Into<Widget>) -> Self {
        Self {
            baseline: incular_core::finite_or_zero(baseline),
            child: child.into(),
        }
    }
}

widget_from_kind!(Baseline => Baseline { baseline, child });

/// Sizes its child to match a specified aspect ratio.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct AspectRatio {
    #[builder(setter(transform = |ratio: f32| ratio.max(f32::EPSILON)))]
    aspect_ratio: f32,
    #[builder(setter(into))]
    child: Widget,
}

impl AspectRatio {
    /// Creates an AspectRatio widget.
    #[must_use]
    pub fn new(aspect_ratio: f32, child: impl Into<Widget>) -> Self {
        Self {
            aspect_ratio: aspect_ratio.max(f32::EPSILON),
            child: child.into(),
        }
    }

    /// The configured ratio of width to height.
    #[must_use]
    pub const fn ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Changes the configured aspect ratio while retaining the child.
    #[must_use]
    pub fn with_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = ratio.max(f32::EPSILON);
        self
    }
}

impl From<AspectRatio> for Widget {
    fn from(value: AspectRatio) -> Self {
        Widget::from_kind(WidgetKind::AspectRatio {
            ratio: value.aspect_ratio,
            child: value.child,
        })
    }
}

/// Scales and positions its child according to [`BoxFit`] and [`Alignment`].
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct FittedBox {
    #[builder(default = BoxFit::Contain)]
    fit: BoxFit,
    #[builder(default = AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    alignment: AlignmentGeometry,
    #[builder(default = Clip::None)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl FittedBox {
    /// Creates a FittedBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            fit: BoxFit::Contain,
            alignment: Alignment::CENTER.into(),
            clip_behavior: Clip::None,
            child: Some(child.into()),
        }
    }

    #[must_use]
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

impl FittedBox {
    /// Clips visual overflow using this mode. Content that fits is not clipped.
    #[must_use]
    pub fn clip_behavior(mut self, behavior: Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }
}

impl From<FittedBox> for Widget {
    fn from(value: FittedBox) -> Self {
        Widget::fitted_box(value.fit, value.alignment, value.clip_behavior, value.child)
    }
}
