//! Size, intrinsic measurement, and constraint-manipulation primitives.

use std::rc::Rc;

use incular_config::{Alignment, Axis, Clip, ConstraintError, Constraints, TextDirection};
use incular_core::{Color, Size};
use typed_builder::TypedBuilder;

use super::Align;
use crate::tree::{ChildLayoutMode, ConstraintMapper};
use crate::{Widget, WidgetKind, macros::widget_from_kind};

/// A box with a specified size.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SizedBox {
    #[builder(default, setter(strip_option))]
    width: Option<f32>,
    #[builder(default, setter(strip_option))]
    height: Option<f32>,
    #[builder(default, setter(strip_option, into))]
    pub(super) child: Option<Widget>,
}

impl Default for SizedBox {
    fn default() -> Self {
        Self::new()
    }
}

impl SizedBox {
    /// Creates an empty or unconstrained SizedBox.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width: None,
            height: None,
            child: None,
        }
    }

    /// Creates a SizedBox with explicit dimensions and optional child.
    #[must_use]
    pub fn from_dimensions(
        width: Option<f32>,
        height: Option<f32>,
        child: Option<impl Into<Widget>>,
    ) -> Self {
        Self {
            width,
            height,
            child: child.map(Into::into),
        }
    }

    /// Sets the child widget.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets explicit width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets explicit height.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Convenience constructor for square box.
    #[must_use]
    pub fn square(dimension: f32) -> Self {
        Self {
            width: Some(dimension),
            height: Some(dimension),
            child: None,
        }
    }

    /// Convenience constructor with `Size`.
    #[must_use]
    pub fn from_size(size: Size) -> Self {
        Self {
            width: Some(size.width),
            height: Some(size.height),
            child: None,
        }
    }

    /// Shrinks to zero size or tight minimum child constraints.
    #[must_use]
    pub const fn shrink() -> Self {
        Self {
            width: Some(0.0),
            height: Some(0.0),
            child: None,
        }
    }

    /// Expands to match parent constraints.
    #[must_use]
    pub const fn expand() -> Self {
        Self {
            width: Some(f32::INFINITY),
            height: Some(f32::INFINITY),
            child: None,
        }
    }

    /// Creates an empty SizedBox.
    #[must_use]
    pub fn empty() -> Self {
        Self::new()
    }
}

impl From<SizedBox> for Widget {
    fn from(value: SizedBox) -> Self {
        let w = value.width;
        let h = value.height;
        let is_expand = w == Some(f32::INFINITY) && h == Some(f32::INFINITY);
        if is_expand && value.child.is_none() {
            return Align::new(Alignment::CENTER, SizedBox::shrink()).into();
        }
        let child = value.child.unwrap_or_else(|| {
            Widget::from_kind(WidgetKind::Box {
                size: Size::ZERO,
                color: Color::TRANSPARENT,
            })
        });
        let mut constraints = Constraints::unbounded();
        let mut has_constraints = false;
        if let Some(w_val) = w
            && w_val.is_finite()
        {
            has_constraints = true;
            constraints = Constraints::new(
                w_val.max(0.0),
                w_val.max(0.0),
                constraints.min_height(),
                constraints.max_height(),
            );
        }
        if let Some(h_val) = h
            && h_val.is_finite()
        {
            has_constraints = true;
            constraints = Constraints::new(
                constraints.min_width(),
                constraints.max_width(),
                h_val.max(0.0),
                h_val.max(0.0),
            );
        }
        // Flutter's `BoxConstraints.tightFor(width: infinity)` is enforced
        // against the incoming constraints, so an infinite axis fills the
        // parent's maximum. That bound is only known during layout.
        let fill_width = w == Some(f32::INFINITY);
        let fill_height = h == Some(f32::INFINITY);
        if fill_width || fill_height {
            return LayoutBuilder::new(move |_, incoming| {
                let mut fitted = constraints;
                if fill_width && incoming.is_width_bounded() {
                    fitted = Constraints::new(
                        incoming.max_width(),
                        incoming.max_width(),
                        fitted.min_height(),
                        fitted.max_height(),
                    );
                }
                if fill_height && incoming.is_height_bounded() {
                    fitted = Constraints::new(
                        fitted.min_width(),
                        fitted.max_width(),
                        incoming.max_height(),
                        incoming.max_height(),
                    );
                }
                ConstrainedBox::new(fitted, child.clone()).into()
            })
            .into();
        }
        if has_constraints {
            ConstrainedBox::new(constraints, child).into()
        } else {
            child
        }
    }
}

/// Imposes additional layout constraints on its child.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ConstrainedBox {
    constraints: Constraints,
    #[builder(setter(into))]
    child: Widget,
}

impl ConstrainedBox {
    /// Creates a ConstrainedBox.
    #[must_use]
    pub fn new(constraints: Constraints, child: impl Into<Widget>) -> Self {
        Self {
            constraints,
            child: child.into(),
        }
    }
}

widget_from_kind!(ConstrainedBox => Constrained { constraints, child });

/// Applies maximum dimensions only when incoming constraints are unbounded.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct LimitedBox {
    #[builder(
        default = f32::INFINITY,
        setter(transform = |limit: f32| limit.max(0.0))
    )]
    max_width: f32,
    #[builder(
        default = f32::INFINITY,
        setter(transform = |limit: f32| limit.max(0.0))
    )]
    max_height: f32,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Default for LimitedBox {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            child: None,
        }
    }
}

impl LimitedBox {
    /// Creates a LimitedBox with a child.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::default().child(child)
    }

    /// Creates a LimitedBox without a child. It takes the parent's minimum size.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets the child to constrain.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Sets the limit for unbounded width.
    #[must_use]
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width.max(0.0);
        self
    }

    /// Sets the limit for unbounded height.
    #[must_use]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(0.0);
        self
    }
}

widget_from_kind!(LimitedBox => Limited { max_width, max_height, child });

/// Determines the box size when the child does not fill its parent's bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverflowBoxFit {
    /// Reserves the full finite space permitted by the parent.
    #[default]
    Max,
    /// Sizes to its child within the parent's bounds.
    DeferToChild,
}

/// Allows a child to overflow its bounds. Defaults to reserving the parent's
/// maximum size; select [OverflowBoxFit::DeferToChild] to shrink-wrap it.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct OverflowBox {
    #[builder(default, setter(strip_option))]
    min_width: Option<f32>,
    #[builder(default, setter(strip_option))]
    max_width: Option<f32>,
    #[builder(default, setter(strip_option))]
    min_height: Option<f32>,
    #[builder(default, setter(strip_option))]
    max_height: Option<f32>,
    #[builder(default = crate::AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    alignment: crate::AlignmentGeometry,
    #[builder(default)]
    fit: OverflowBoxFit,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl OverflowBox {
    /// Creates an OverflowBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::empty().child(child)
    }

    /// Creates an OverflowBox without a child.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            alignment: Alignment::CENTER.into(),
            fit: OverflowBoxFit::Max,
            child: None,
        }
    }

    /// Sets or replaces the child.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Chooses how to size this box independently of the child constraints.
    #[must_use]
    pub fn fit(mut self, fit: OverflowBoxFit) -> Self {
        self.fit = fit;
        self
    }

    /// Aligns the child within the box. Defaults to centered, as in Flutter's
    /// `OverflowBox`; an overflowing child is shifted by half of its excess.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<crate::AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    #[must_use]
    pub fn min_width(mut self, value: f32) -> Self {
        self.min_width = Some(value);
        self
    }

    #[must_use]
    pub fn max_width(mut self, value: f32) -> Self {
        self.max_width = Some(value);
        self
    }

    #[must_use]
    pub fn min_height(mut self, value: f32) -> Self {
        self.min_height = Some(value);
        self
    }

    #[must_use]
    pub fn max_height(mut self, value: f32) -> Self {
        self.max_height = Some(value);
        self
    }
}

widget_from_kind!(OverflowBox => Overflow { min_width, max_width, min_height, max_height, alignment, fit, child });

/// Allows its child to size naturally without parent constraints.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct UnconstrainedBox {
    #[builder(default, setter(strip_option))]
    constrained_axis: Option<Axis>,
    #[builder(default = crate::AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    alignment: crate::AlignmentGeometry,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(default = Clip::None)]
    clip_behavior: Clip,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Default for UnconstrainedBox {
    fn default() -> Self {
        Self::empty()
    }
}

impl UnconstrainedBox {
    /// Creates an UnconstrainedBox.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::empty().child(child)
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            constrained_axis: None,
            alignment: crate::AlignmentGeometry::Physical(Alignment::CENTER),
            text_direction: None,
            clip_behavior: Clip::None,
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Aligns the child within the box. Defaults to centered, as in Flutter's
    /// `UnconstrainedBox`; an overflowing child is shifted by half of its excess.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<crate::AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Retains constraints along a specific axis while loosening the orthogonal axis.
    #[must_use]
    pub fn constrained_axis(mut self, axis: Axis) -> Self {
        self.constrained_axis = Some(axis);
        self
    }
    /// Clips only when the unconstrained child overflows this box.
    /// Overrides the direction used to resolve logical alignment.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, behavior: Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }
}

impl From<UnconstrainedBox> for Widget {
    fn from(value: UnconstrainedBox) -> Self {
        Widget::from_kind(WidgetKind::Unconstrained {
            constrained_axis: value.constrained_axis,
            alignment: value.alignment,
            text_direction: value.text_direction,
            transform: None,
            clip_behavior: value.clip_behavior,
            child: value.child,
        })
    }
}

/// Rebuilds a subtree when incoming layout constraints change.
#[derive(Clone)]
pub struct LayoutBuilder {
    builder: Rc<crate::tree::LayoutBuilderCallback>,
}

impl LayoutBuilder {
    /// Creates a LayoutBuilder with a constraint builder closure.
    #[must_use]
    pub fn new(
        builder: impl for<'a> Fn(&crate::BuildContext<'a>, Constraints) -> Widget + 'static,
    ) -> Self {
        Self {
            builder: Rc::new(builder),
        }
    }
}

impl From<LayoutBuilder> for Widget {
    fn from(value: LayoutBuilder) -> Self {
        Widget::from_kind(WidgetKind::LayoutBuilder {
            builder: value.builder,
            environment: None,
            environment_boundary: false,
            revision: None,
            constraint_dependent: true,
            retained: None,
            static_child: None,
            direction_requirement: None,
        })
    }
}

/// Builds a subtree from its [`BuildContext`](crate::BuildContext) alone.
///
/// Flutter's `Builder`: the child is rebuilt when this widget is replaced or
/// when an inherited value read through the context (for example with
/// [`BuildContext::depend_on`](crate::BuildContext::depend_on)) changes. Unlike
/// [`LayoutBuilder`], a change in incoming constraints does not rebuild it, so
/// a builder that only reads a theme costs nothing when its parent resizes.
#[derive(Clone)]
pub struct Builder {
    builder: Rc<crate::tree::LayoutBuilderCallback>,
    direction_requirement: Option<&'static crate::tree::DirectionRequirement>,
}

impl Builder {
    /// Creates a Builder with a context builder closure.
    #[must_use]
    pub fn new(builder: impl for<'a> Fn(&crate::BuildContext<'a>) -> Widget + 'static) -> Self {
        Self {
            builder: Rc::new(move |context, _| builder(context)),
            direction_requirement: None,
        }
    }

    #[must_use]
    pub(crate) fn require_directionality(
        mut self,
        requirement: &'static crate::tree::DirectionRequirement,
    ) -> Self {
        self.direction_requirement = Some(requirement);
        self
    }
}

impl From<Builder> for Widget {
    fn from(value: Builder) -> Self {
        Widget::from_kind(WidgetKind::LayoutBuilder {
            builder: value.builder,
            environment: None,
            environment_boundary: false,
            revision: None,
            constraint_dependent: false,
            retained: None,
            static_child: None,
            direction_requirement: value.direction_requirement,
        })
    }
}

fn intrinsic_step(step: f32) -> Option<f32> {
    debug_assert!(step >= 0., "intrinsic step must be nonnegative");
    Some(step)
}

/// Sizes its child to the child's intrinsic width.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct IntrinsicWidth {
    #[builder(default, setter(transform = |step: f32| intrinsic_step(step)))]
    step_width: Option<f32>,
    #[builder(default, setter(transform = |step: f32| intrinsic_step(step)))]
    step_height: Option<f32>,
    #[builder(setter(into))]
    child: Widget,
}

impl IntrinsicWidth {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            step_width: None,
            step_height: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn step_width(mut self, step: f32) -> Self {
        self.step_width = intrinsic_step(step);
        self
    }

    #[must_use]
    pub fn step_height(mut self, step: f32) -> Self {
        self.step_height = intrinsic_step(step);
        self
    }
}

impl From<IntrinsicWidth> for Widget {
    fn from(value: IntrinsicWidth) -> Self {
        let positive_step = |step: Option<f32>| step.filter(|s| *s > 0. && s.is_finite());
        Widget::from_kind(WidgetKind::ProxyLayout {
            mode: ChildLayoutMode::IntrinsicWidth {
                step_width: positive_step(value.step_width),
                step_height: positive_step(value.step_height),
            },
            child: value.child,
        })
    }
}

/// Sizes its child to the child's intrinsic height.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct IntrinsicHeight {
    #[builder(setter(into))]
    child: Widget,
}

impl IntrinsicHeight {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl From<IntrinsicHeight> for Widget {
    fn from(value: IntrinsicHeight) -> Self {
        Widget::from_kind(WidgetKind::ProxyLayout {
            mode: ChildLayoutMode::IntrinsicHeight,
            child: value.child,
        })
    }
}

/// A box with specified size that allows its child to overflow with alignment.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SizedOverflowBox {
    size: Size,
    #[builder(default = crate::AlignmentGeometry::Physical(Alignment::CENTER), setter(into))]
    alignment: crate::AlignmentGeometry,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl SizedOverflowBox {
    #[must_use]
    pub fn new(size: Size, child: impl Into<Widget>) -> Self {
        Self::empty(size).child(child)
    }

    #[must_use]
    pub fn empty(size: Size) -> Self {
        Self {
            size,
            alignment: crate::AlignmentGeometry::Physical(Alignment::CENTER),
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<crate::AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

widget_from_kind!(SizedOverflowBox => SizedOverflow { size, alignment, child });

/// Transforms incoming layout constraints before passing them to its child.
#[derive(Clone)]
pub struct ConstraintsTransformBox {
    transform: ConstraintMapper,
    alignment: crate::AlignmentGeometry,
    text_direction: Option<TextDirection>,
    clip_behavior: Clip,
    child: Option<Widget>,
}

impl ConstraintsTransformBox {
    #[must_use]
    pub fn new(
        transform: impl Fn(Constraints) -> Constraints + 'static,
        child: impl Into<Widget>,
    ) -> Self {
        Self::empty(transform).child(child)
    }

    /// Creates a proxy without a child. It takes the incoming smallest
    /// size and does not invoke the constraint transform until a child mounts.
    #[must_use]
    pub fn empty(transform: impl Fn(Constraints) -> Constraints + 'static) -> Self {
        Self::from_mapper(move |constraints| Ok(transform(constraints)))
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Transforms constraints through a fallible callback. Invalid output
    /// is reported by the retained layout pass, before laying out the child.
    #[must_use]
    pub fn try_new(
        transform: impl Fn(Constraints) -> Result<Constraints, ConstraintError> + 'static,
        child: impl Into<Widget>,
    ) -> Self {
        Self::from_mapper(transform).child(child)
    }

    fn from_mapper(
        transform: impl Fn(Constraints) -> Result<Constraints, ConstraintError> + 'static,
    ) -> Self {
        Self {
            transform: ConstraintMapper(Rc::new(transform)),
            alignment: crate::AlignmentGeometry::Physical(Alignment::CENTER),
            text_direction: None,
            clip_behavior: Clip::None,
            child: None,
        }
    }

    #[must_use]
    pub const fn unmodified(constraints: Constraints) -> Constraints {
        constraints
    }

    #[must_use]
    pub fn unconstrained(_: Constraints) -> Constraints {
        Constraints::default()
    }

    #[must_use]
    pub fn width_unconstrained(constraints: Constraints) -> Constraints {
        Constraints::new(
            0.,
            f32::INFINITY,
            constraints.min_height(),
            constraints.max_height(),
        )
    }

    #[must_use]
    pub fn height_unconstrained(constraints: Constraints) -> Constraints {
        Constraints::new(
            constraints.min_width(),
            constraints.max_width(),
            0.,
            f32::INFINITY,
        )
    }

    #[must_use]
    pub fn max_width_unconstrained(constraints: Constraints) -> Constraints {
        Constraints::new(
            constraints.min_width(),
            f32::INFINITY,
            constraints.min_height(),
            constraints.max_height(),
        )
    }

    #[must_use]
    pub fn max_height_unconstrained(constraints: Constraints) -> Constraints {
        Constraints::new(
            constraints.min_width(),
            constraints.max_width(),
            constraints.min_height(),
            f32::INFINITY,
        )
    }

    #[must_use]
    pub fn max_unconstrained(constraints: Constraints) -> Constraints {
        Constraints::new(
            constraints.min_width(),
            f32::INFINITY,
            constraints.min_height(),
            f32::INFINITY,
        )
    }

    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<crate::AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
        self
    }

    /// Overrides the direction used to resolve logical alignment.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }
}

impl From<ConstraintsTransformBox> for Widget {
    fn from(value: ConstraintsTransformBox) -> Self {
        Widget::from_kind(WidgetKind::Unconstrained {
            constrained_axis: None,
            alignment: value.alignment,
            text_direction: value.text_direction,
            transform: Some(value.transform),
            clip_behavior: value.clip_behavior,
            child: value.child,
        })
    }
}

/// Informs parent of preferred size.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct PreferredSize {
    preferred_size: Size,
    #[builder(setter(into))]
    child: Widget,
}

impl PreferredSize {
    #[must_use]
    pub fn new(preferred_size: Size, child: impl Into<Widget>) -> Self {
        Self {
            preferred_size,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn preferred_size(&self) -> Size {
        self.preferred_size
    }
}

impl From<PreferredSize> for Widget {
    fn from(value: PreferredSize) -> Self {
        ConstrainedBox::new(
            Constraints::new(
                value.preferred_size.width,
                value.preferred_size.width,
                value.preferred_size.height,
                value.preferred_size.height,
            ),
            value.child,
        )
        .into()
    }
}
