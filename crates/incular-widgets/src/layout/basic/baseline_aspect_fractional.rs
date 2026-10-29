//! Baseline, aspect-ratio, fractional-sizing, and fitted-box primitives.

use incular_config::Alignment;
use typed_builder::TypedBuilder;

use crate::tree::BoxFit;
use crate::{Widget, WidgetKind, macros::widget_from_kind};

/// Sizes child as a fraction of incoming parent constraints.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct FractionallySizedBox {
    #[builder(
        default,
        setter(transform = |factor: f32| Some(factor.max(0.0)))
    )]
    width_factor: Option<f32>,
    #[builder(
        default,
        setter(transform = |factor: f32| Some(factor.max(0.0)))
    )]
    height_factor: Option<f32>,
    #[builder(default = Alignment::CENTER)]
    alignment: Alignment,
    #[builder(setter(into))]
    child: Widget,
}

impl FractionallySizedBox {
    /// Creates a FractionallySizedBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            alignment: Alignment::CENTER,
            child: child.into(),
        }
    }

    /// Where the child sits when it is smaller or larger than the box.
    /// Defaults to center, as in Flutter.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
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

/// Positions child baseline at an explicit vertical offset.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Baseline {
    #[builder(setter(transform = |baseline: f32| baseline.max(0.0)))]
    baseline: f32,
    #[builder(setter(into))]
    child: Widget,
}

impl Baseline {
    /// Creates a Baseline widget.
    #[must_use]
    pub fn new(baseline: f32, child: impl Into<Widget>) -> Self {
        Self {
            baseline: baseline.max(0.0),
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
    #[builder(default = Alignment::CENTER)]
    alignment: Alignment,
    #[builder(setter(into))]
    child: Widget,
}

impl FittedBox {
    /// Creates a FittedBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            fit: BoxFit::Contain,
            alignment: Alignment::CENTER,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn fit(mut self, fit: BoxFit) -> Self {
        self.fit = fit;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl From<FittedBox> for Widget {
    fn from(value: FittedBox) -> Self {
        Widget::fitted_box(value.fit, value.alignment, value.child)
    }
}
