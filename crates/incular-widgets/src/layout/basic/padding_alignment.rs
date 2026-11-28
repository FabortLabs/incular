//! Padding and alignment layout primitives.

use incular_config::{Alignment, AlignmentGeometry, EdgeInsets, TextDirection};
use typed_builder::TypedBuilder;

use crate::{Widget, WidgetKind, macros::widget_from_kind};

/// Insets its child by given [`EdgeInsets`].
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Padding {
    #[builder(setter(into))]
    pub(crate) padding: EdgeInsets,
    #[builder(setter(into))]
    pub(crate) child: Widget,
}

impl Padding {
    /// Creates a padding widget wrapping a child with the given insets.
    #[must_use]
    pub fn new(padding: impl Into<EdgeInsets>, child: impl Into<Widget>) -> Self {
        Self {
            padding: padding.into(),
            child: child.into(),
        }
    }

    /// Convenience constructor with uniform padding on all edges.
    #[must_use]
    pub fn all(value: f32, child: impl Into<Widget>) -> Self {
        Self::new(EdgeInsets::all(value), child)
    }

    /// Convenience constructor with symmetric padding.
    #[must_use]
    pub fn symmetric(horizontal: f32, vertical: f32, child: impl Into<Widget>) -> Self {
        Self::new(EdgeInsets::symmetric(horizontal, vertical), child)
    }

    /// Convenience constructor with zero padding.
    #[must_use]
    pub fn zero(child: impl Into<Widget>) -> Self {
        Self::new(EdgeInsets::ZERO, child)
    }
}

widget_from_kind!(Padding => Padding { padding, child });

/// Positions a child inside itself with fractional alignment and optional size factors.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Align {
    #[builder(default = AlignmentGeometry::CENTER, setter(into))]
    pub(crate) alignment: AlignmentGeometry,
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
    #[builder(default, setter(strip_option, into))]
    pub(crate) child: Option<Widget>,
}

impl Align {
    /// Creates an align widget with a physical or direction-relative alignment.
    #[must_use]
    pub fn new(alignment: impl Into<AlignmentGeometry>, child: impl Into<Widget>) -> Self {
        Self::builder().alignment(alignment).child(child).build()
    }

    /// Creates an Align with no child.
    #[must_use]
    pub const fn empty(alignment: Alignment) -> Self {
        Self {
            alignment: AlignmentGeometry::Physical(alignment),
            width_factor: None,
            height_factor: None,
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets the width scaling factor relative to child width.
    #[must_use]
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor.max(0.0));
        self
    }

    /// Sets the height scaling factor relative to child height.
    #[must_use]
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor.max(0.0));
        self
    }

    /// Returns the physical alignment with a left-to-right fallback for
    /// callers of the original physical-only getter. To inspect directional
    /// values before layout, use [Self::get_alignment_geometry].
    #[must_use]
    pub fn get_alignment(&self) -> Alignment {
        self.alignment.resolve(TextDirection::Ltr)
    }

    /// Returns the description before it is resolved against Directionality.
    #[must_use]
    pub const fn get_alignment_geometry(&self) -> AlignmentGeometry {
        self.alignment
    }
}

widget_from_kind!(Align => Align { alignment, width_factor, height_factor, child });

/// Centers its child within itself.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Center {
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
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Center {
    /// Creates a Center widget with a child.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::empty().child(child)
    }

    /// Creates a Center with no child.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets the width scaling factor relative to child width.
    #[must_use]
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor.max(0.0));
        self
    }

    /// Sets the height scaling factor relative to child height.
    #[must_use]
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor.max(0.0));
        self
    }
}

impl From<Center> for Widget {
    fn from(value: Center) -> Self {
        let mut align = Align::empty(Alignment::CENTER);
        align.child = value.child;
        if let Some(wf) = value.width_factor {
            align = align.width_factor(wf);
        }
        if let Some(hf) = value.height_factor {
            align = align.height_factor(hf);
        }
        align.into()
    }
}
