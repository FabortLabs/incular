//! Flutter's explicit transitions (`widgets/transitions.dart`) and
//! `DualTransitionBuilder` (`widgets/dual_transition_builder.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_animation::{
    AlwaysStoppedAnimation, AnimationController, AnimationStatus, Curve, DoubleAnimation,
    OptionalColorTween,
};
use incular_config::{Alignment, AlignmentDirectional, Axis, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Brush, GradientStops};
use incular_text::TextStyle;

use crate::explicit_animations::animated_widget_for;
use crate::implicit_animations::PropertyTween;
use crate::tree::RetainedState;
use crate::{
    Align, BoxDecoration, BoxShadow, BuildContext, ClipRect, DecoratedBox, DecoratedShapeBox,
    DecorationPosition, DefaultTextStyle, Positioned, ShapeDecoration, Transform, Widget,
};

/// Flutter's `RelativeRect`: a rectangle given by its distances from the
/// edges of a container, as a [`Positioned`] child is placed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RelativeRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl RelativeRect {
    /// Flutter `RelativeRect.fill`: no inset on any side.
    pub const FILL: Self = Self::from_ltrb(0.0, 0.0, 0.0, 0.0);

    #[must_use]
    pub const fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Flutter `RelativeRect.fromSize`: `rect` inside a container of `size`
    /// whose origin is the container's.
    #[must_use]
    pub fn from_size(rect: Rect, size: Size) -> Self {
        Self::from_ltrb(
            rect.origin.x,
            rect.origin.y,
            size.width - (rect.origin.x + rect.size.width),
            size.height - (rect.origin.y + rect.size.height),
        )
    }

    /// Flutter `RelativeRect.fromRect`: `rect` inside `container`.
    #[must_use]
    pub fn from_rect(rect: Rect, container: Rect) -> Self {
        Self::from_ltrb(
            rect.origin.x - container.origin.x,
            rect.origin.y - container.origin.y,
            container.origin.x + container.size.width - (rect.origin.x + rect.size.width),
            container.origin.y + container.size.height - (rect.origin.y + rect.size.height),
        )
    }

    /// Resolves start/end in the supplied text direction.
    #[must_use]
    pub const fn from_directional(
        start: f32,
        top: f32,
        end: f32,
        bottom: f32,
        text_direction: TextDirection,
    ) -> Self {
        match text_direction {
            TextDirection::Ltr => Self::from_ltrb(start, top, end, bottom),
            TextDirection::Rtl => Self::from_ltrb(end, top, start, bottom),
        }
    }

    /// Flutter `RelativeRect.toRect`: the rectangle in the container's local
    /// coordinates. The container's origin is not applied.
    #[must_use]
    pub fn to_rect(self, container: Rect) -> Rect {
        Rect::from_origin_size(
            Offset::new(self.left, self.top),
            // Unlike a layout size, an intersection may have inverted edges.
            Size {
                width: (container.size.width - self.right) - self.left,
                height: (container.size.height - self.bottom) - self.top,
            },
        )
    }

    /// Converts to the container's enclosing coordinate space, applying its
    /// origin. This is the inverse of `from_rect`.
    #[must_use]
    pub fn to_global_rect(self, container: Rect) -> Rect {
        let rect = self.to_rect(container);
        Rect::from_origin_size(container.origin + rect.origin, rect.size)
    }

    /// Returns signed geometric extents, not a constrained layout size.
    #[must_use]
    pub fn to_size(self, container: Size) -> Size {
        Size {
            width: container.width - self.left - self.right,
            height: container.height - self.top - self.bottom,
        }
    }

    /// Intersects four inset distances without normalizing disjoint edges.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let inward_edge = |a: f32, b: f32| {
            if a.is_nan() || b.is_nan() {
                f32::NAN
            } else if a.total_cmp(&b).is_lt() {
                b
            } else {
                a
            }
        };
        Self::from_ltrb(
            inward_edge(self.left, other.left),
            inward_edge(self.top, other.top),
            inward_edge(self.right, other.right),
            inward_edge(self.bottom, other.bottom),
        )
    }

    /// Flutter `RelativeRect.shift`.
    #[must_use]
    pub fn shift(self, offset: Offset) -> Self {
        Self::from_ltrb(
            self.left + offset.x,
            self.top + offset.y,
            self.right - offset.x,
            self.bottom - offset.y,
        )
    }

    /// Flutter `RelativeRect.inflate`.
    #[must_use]
    pub fn inflate(self, delta: f32) -> Self {
        Self::from_ltrb(
            self.left - delta,
            self.top - delta,
            self.right - delta,
            self.bottom - delta,
        )
    }

    /// Flutter `RelativeRect.deflate`.
    #[must_use]
    pub fn deflate(self, delta: f32) -> Self {
        self.inflate(-delta)
    }

    /// Flutter `RelativeRect.hasInsets`.
    #[must_use]
    pub fn has_insets(self) -> bool {
        self.left > 0.0 || self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0
    }

    /// Interpolates a missing endpoint as `FILL`, as documented by Flutter.
    /// Two missing endpoints return `None`.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        let scale = |rect: Self, k: f32| {
            Self::from_ltrb(rect.left * k, rect.top * k, rect.right * k, rect.bottom * k)
        };
        match (begin, end) {
            (None, None) => None,
            (None, Some(end)) => Some(scale(end, t)),
            (Some(begin), None) => Some(scale(begin, 1.0 - t)),
            (Some(begin), Some(end)) => Some(begin.lerp(&end, t)),
        }
    }
}

impl Lerp for RelativeRect {
    /// Flutter `RelativeRect.lerp`: each side independently, not clamped.
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| {
            if a == b || (a.is_nan() && b.is_nan()) {
                return a;
            }
            debug_assert!(a.is_finite() && b.is_finite() && t.is_finite());
            a * (1.0 - t) + b * t
        };
        Self::from_ltrb(
            mix(self.left, other.left),
            mix(self.top, other.top),
            mix(self.right, other.right),
            mix(self.bottom, other.bottom),
        )
    }
}

/// Flutter's `RelativeRectTween`: a named `Tween<RelativeRect>` that
/// interpolates each side independently, as [`RelativeRect::lerp`] does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeRectTween {
    begin: RelativeRect,
    end: RelativeRect,
}

impl RelativeRectTween {
    #[must_use]
    pub const fn new(begin: RelativeRect, end: RelativeRect) -> Self {
        Self { begin, end }
    }

    #[must_use]
    pub const fn begin(&self) -> RelativeRect {
        self.begin
    }

    #[must_use]
    pub const fn end(&self) -> RelativeRect {
        self.end
    }

    /// The rectangle at `t`, where 0 is `begin` and 1 is `end`.
    #[must_use]
    pub fn lerp(&self, t: f32) -> RelativeRect {
        self.begin.lerp(&self.end, t)
    }

    /// `RelativeRectTween.animate(parent)`: this tween driven by `parent`.
    #[must_use]
    pub fn animate(self, parent: impl Into<DoubleAnimation>) -> TweenAnimation<RelativeRect> {
        TweenAnimation::new(parent, self.begin, self.end)
    }
}

/// Flutter's `AlignTransition`: an [`Align`] whose alignment follows
/// `alignment`.
#[derive(Clone)]
pub struct AlignTransition {
    alignment: TweenAnimation<Alignment>,
    width_factor: Option<f32>,
    height_factor: Option<f32>,
    child: Widget,
}

impl AlignTransition {
    /// A fixed [`Alignment`] converts into a stopped animation.
    #[must_use]
    pub fn new(alignment: impl Into<TweenAnimation<Alignment>>, child: impl Into<Widget>) -> Self {
        Self {
            alignment: alignment.into(),
            width_factor: None,
            height_factor: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn width_factor(mut self, factor: f32) -> Self {
        self.width_factor = Some(factor);
        self
    }

    #[must_use]
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = Some(factor);
        self
    }
}

impl From<AlignTransition> for Widget {
    fn from(value: AlignTransition) -> Self {
        let parent = value.alignment.parent().clone();
        animated_widget_for(parent, move |_| {
            let mut align = Align::new(value.alignment.value(), value.child.clone());
            if let Some(factor) = value.width_factor {
                align = align.width_factor(factor);
            }
            if let Some(factor) = value.height_factor {
                align = align.height_factor(factor);
            }
            align.into()
        })
    }
}

/// Flutter's `SizeTransition`: clips the child to `size_factor` of its size
/// along `axis`, aligned by `axis_alignment` (-1 is the start, 1 the end).
#[derive(Clone)]
pub struct SizeTransition {
    axis: Axis,
    size_factor: DoubleAnimation,
    axis_alignment: f32,
    fixed_cross_axis_size_factor: Option<f32>,
    child: Widget,
}

impl SizeTransition {
    /// A fixed factor converts into a stopped animation.
    #[must_use]
    pub fn new(size_factor: impl Into<DoubleAnimation>, child: impl Into<Widget>) -> Self {
        Self {
            axis: Axis::Vertical,
            size_factor: size_factor.into(),
            axis_alignment: 0.0,
            fixed_cross_axis_size_factor: None,
            child: child.into(),
        }
    }

    /// Defaults to [`Axis::Vertical`].
    #[must_use]
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Where the child sits along the axis while clipped. Defaults to 0.
    #[must_use]
    pub fn axis_alignment(mut self, alignment: f32) -> Self {
        self.axis_alignment = alignment;
        self
    }

    /// Flutter `fixedCrossAxisSizeFactor`: sizes the cross axis to this
    /// fraction of the child instead of the child's own size.
    ///
    /// # Panics
    /// If `factor` is negative, as Flutter's constructor assertion rejects it.
    #[must_use]
    pub fn fixed_cross_axis_size_factor(mut self, factor: f32) -> Self {
        assert!(
            factor >= 0.0,
            "fixedCrossAxisSizeFactor must be non-negative, got {factor}"
        );
        self.fixed_cross_axis_size_factor = Some(factor);
        self
    }
}

impl From<SizeTransition> for Widget {
    fn from(value: SizeTransition) -> Self {
        let parent = value.size_factor.clone();
        animated_widget_for(parent, move |context| {
            let factor = value.size_factor.value().max(0.0);
            let alignment = match value.axis {
                Axis::Horizontal => AlignmentDirectional::new(value.axis_alignment, -1.0),
                Axis::Vertical => AlignmentDirectional::new(-1.0, value.axis_alignment),
            }
            .resolve(context.text_direction().unwrap_or(TextDirection::Ltr));
            let mut align = Align::new(alignment, value.child.clone());
            let (width, height) = match value.axis {
                Axis::Horizontal => (Some(factor), value.fixed_cross_axis_size_factor),
                Axis::Vertical => (value.fixed_cross_axis_size_factor, Some(factor)),
            };
            if let Some(width) = width {
                align = align.width_factor(width);
            }
            if let Some(height) = height {
                align = align.height_factor(height);
            }
            ClipRect::new(align).into()
        })
    }
}

/// Flutter's `PositionedTransition`: a [`Positioned`] child of a
/// [`Stack`](crate::Stack) whose insets follow `rect`.
#[derive(Clone)]
pub struct PositionedTransition {
    rect: TweenAnimation<RelativeRect>,
    child: Widget,
}

impl PositionedTransition {
    #[must_use]
    pub fn new(rect: impl Into<TweenAnimation<RelativeRect>>, child: impl Into<Widget>) -> Self {
        Self {
            rect: rect.into(),
            child: child.into(),
        }
    }
}

impl From<PositionedTransition> for Widget {
    fn from(value: PositionedTransition) -> Self {
        let parent = value.rect.parent().clone();
        animated_widget_for(parent, move |_| {
            let rect = value.rect.value();
            Positioned::new(value.child.clone())
                .left(rect.left)
                .top(rect.top)
                .right(rect.right)
                .bottom(rect.bottom)
                .into()
        })
    }
}

/// Flutter's `RelativePositionedTransition`: a [`Positioned`] child placed
/// at `rect` inside a stack of `size`.
#[derive(Clone)]
pub struct RelativePositionedTransition {
    rect: TweenAnimation<Rect>,
    size: Size,
    child: Widget,
}

impl RelativePositionedTransition {
    #[must_use]
    pub fn new(
        rect: impl Into<TweenAnimation<Rect>>,
        size: Size,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            rect: rect.into(),
            size,
            child: child.into(),
        }
    }

    /// Flutter's `Animation<Rect?>` form: a `None` endpoint is the zero rectangle.
    /// Flutter's `Rect.lerp` scales a null endpoint from the origin, which is
    /// the same as interpolating from or to the zero rectangle; a sample that is
    /// null at both ends is therefore the zero rectangle, as in Flutter.
    #[must_use]
    pub fn from_optional_rects(
        parent: impl Into<DoubleAnimation>,
        begin: Option<Rect>,
        end: Option<Rect>,
        size: Size,
        child: impl Into<Widget>,
    ) -> Self {
        let zero = Rect::from_origin_size(incular_core::Offset::ZERO, Size::ZERO);
        Self::new(
            TweenAnimation::new(parent, begin.unwrap_or(zero), end.unwrap_or(zero)),
            size,
            child,
        )
    }
}

impl From<RelativePositionedTransition> for Widget {
    fn from(value: RelativePositionedTransition) -> Self {
        let parent = value.rect.parent().clone();
        animated_widget_for(parent, move |_| {
            let offsets = RelativeRect::from_size(value.rect.value(), value.size);
            Positioned::new(value.child.clone())
                .left(offsets.left)
                .top(offsets.top)
                .right(offsets.right)
                .bottom(offsets.bottom)
                .into()
        })
    }
}

/// Flutter's `Decoration` as a transition animates it: a [`BoxDecoration`]
/// or a [`ShapeDecoration`].
///
/// Two values of the same kind interpolate. Two kinds that do not
/// interpolate with each other switch at the midpoint, as Flutter's
/// `Decoration.lerp` falls back to `t < 0.5 ? a : b`.
#[derive(Clone)]
pub enum Decoration {
    Box(BoxDecoration),
    Shape(ShapeDecoration),
}

impl From<BoxDecoration> for Decoration {
    fn from(value: BoxDecoration) -> Self {
        Self::Box(value)
    }
}

impl From<ShapeDecoration> for Decoration {
    fn from(value: ShapeDecoration) -> Self {
        Self::Shape(value)
    }
}

impl Lerp for Decoration {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if t == 0.0 {
            return self.clone();
        }
        if t == 1.0 {
            return other.clone();
        }
        match (self, other) {
            (Self::Box(a), Self::Box(b)) => Self::Box(a.lerp(b, t)),
            (Self::Shape(a), Self::Shape(b)) => Self::Shape(lerp_shape_decoration(a, b, t)),
            _ if t < 0.5 => self.clone(),
            _ => other.clone(),
        }
    }
}

/// The color-to-gradient case uses a matching uniform gradient. Images,
/// border shape morphs and transitions between two gradient geometries are
/// still handled by the existing midpoint fallback.
fn lerp_shape_decoration(a: &ShapeDecoration, b: &ShapeDecoration, t: f32) -> ShapeDecoration {
    let switches = t >= 0.5;
    let gradient = match (&a.gradient, &b.gradient) {
        (None, Some(gradient)) => Some(blend_shape_gradient(gradient, a.color, t, true)),
        (Some(gradient), None) => Some(blend_shape_gradient(gradient, b.color, t, false)),
        (a, b) => {
            if switches { b.clone() } else { a.clone() }
        }
    };
    ShapeDecoration {
        color: if gradient.is_none() {
            OptionalColorTween::new(a.color, b.color).lerp(t)
        } else {
            None
        },
        gradient,
        shadows: BoxShadow::lerp_list(&a.shadows, &b.shadows, t),
        image: if switches {
            b.image.clone()
        } else {
            a.image.clone()
        },
        shape: if switches {
            b.shape.clone()
        } else {
            a.shape.clone()
        },
    }
}

fn blend_shape_gradient(
    template: &Brush,
    solid: Option<Color>,
    t: f32,
    toward_gradient: bool,
) -> Brush {
    let stops = |gradient: &GradientStops| {
        let Some(color) = solid else {
            return gradient.scale_opacity(if toward_gradient { t } else { 1. - t });
        };
        let uniform = gradient.with_color(color);
        if toward_gradient {
            uniform.interpolate(gradient, t)
        } else {
            gradient.interpolate(&uniform, t)
        }
    };
    match template {
        Brush::Solid(color) => {
            let (a, b) = if toward_gradient {
                (solid, Some(*color))
            } else {
                (Some(*color), solid)
            };
            Brush::Solid(OptionalColorTween::new(a, b).lerp(t).unwrap_or(Color::TRANSPARENT))
        }
        Brush::LinearGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::LinearGradient(gradient)
        }
        Brush::RadialGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::RadialGradient(gradient)
        }
        Brush::SweepGradient(gradient) => {
            let mut gradient = gradient.clone();
            gradient.stops = stops(&gradient.stops);
            Brush::SweepGradient(gradient)
        }
    }
}

/// Flutter's `DecoratedBoxTransition`: a [`DecoratedBox`] whose decoration
/// follows `decoration`, rebuilt on every change of its animation.
///
/// Accepts a tween of [`BoxDecoration`] or of [`ShapeDecoration`] (through
/// [`Decoration`]). A shape decoration always paints behind the child, so
/// [`DecoratedBoxTransition::position`] applies to box decorations only.
#[derive(Clone)]
pub struct DecoratedBoxTransition {
    decoration: TweenAnimation<Decoration>,
    position: DecorationPosition,
    child: Widget,
}

impl DecoratedBoxTransition {
    #[must_use]
    pub fn new(
        decoration: impl Into<TweenAnimation<Decoration>>,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            decoration: decoration.into(),
            position: DecorationPosition::Background,
            child: child.into(),
        }
    }

    /// Defaults to [`DecorationPosition::Background`]. Box decorations only.
    #[must_use]
    pub fn position(mut self, position: DecorationPosition) -> Self {
        self.position = position;
        self
    }
}

impl From<TweenAnimation<BoxDecoration>> for TweenAnimation<Decoration> {
    fn from(value: TweenAnimation<BoxDecoration>) -> Self {
        TweenAnimation {
            parent: value.parent,
            curve: value.curve,
            tween: PropertyTween {
                begin: Decoration::Box(value.tween.begin),
                end: Decoration::Box(value.tween.end),
            },
        }
    }
}

impl From<TweenAnimation<ShapeDecoration>> for TweenAnimation<Decoration> {
    fn from(value: TweenAnimation<ShapeDecoration>) -> Self {
        TweenAnimation {
            parent: value.parent,
            curve: value.curve,
            tween: PropertyTween {
                begin: Decoration::Shape(value.tween.begin),
                end: Decoration::Shape(value.tween.end),
            },
        }
    }
}

impl From<DecoratedBoxTransition> for Widget {
    fn from(value: DecoratedBoxTransition) -> Self {
        let parent = value.decoration.parent().clone();
        animated_widget_for(parent, move |_| match value.decoration.value() {
            Decoration::Box(decoration) => DecoratedBox::new(value.child.clone())
                .decoration(decoration)
                .position(value.position)
                .into(),
            Decoration::Shape(decoration) => {
                DecoratedShapeBox::new(decoration, value.child.clone()).into()
            }
        })
    }
}

/// Flutter's `DefaultTextStyleTransition`: a [`DefaultTextStyle`] whose
/// style follows `style`, rebuilt on every change of its animation.
///
/// Incular's [`DefaultTextStyle`] carries only the style, so Flutter's
/// `textAlign`, `softWrap`, `overflow` and `maxLines` have no counterpart.
#[derive(Clone)]
pub struct DefaultTextStyleTransition {
    style: TweenAnimation<TextStyle>,
    child: Widget,
}

impl DefaultTextStyleTransition {
    #[must_use]
    pub fn new(style: TweenAnimation<TextStyle>, child: impl Into<Widget>) -> Self {
        Self {
            style,
            child: child.into(),
        }
    }
}

impl From<DefaultTextStyleTransition> for Widget {
    fn from(value: DefaultTextStyleTransition) -> Self {
        let parent = value.style.parent().clone();
        animated_widget_for(parent, move |_| {
            DefaultTextStyle::new(value.style.value(), value.child.clone()).into()
        })
    }
}

/// Flutter's `MatrixTransition`: a [`Transform`] whose matrix is
/// `on_transform(animation)`, about `alignment` (the center by default).
#[derive(Clone)]
pub struct MatrixTransition {
    animation: DoubleAnimation,
    on_transform: Rc<dyn Fn(f32) -> CoreTransform>,
    alignment: Alignment,
    filter_quality: Option<incular_rendering::FilterQuality>,
    child: Widget,
}

impl MatrixTransition {
    #[must_use]
    pub fn new(
        animation: impl Into<DoubleAnimation>,
        on_transform: impl Fn(f32) -> CoreTransform + 'static,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            animation: animation.into(),
            on_transform: Rc::new(on_transform),
            alignment: Alignment::CENTER,
            filter_quality: None,
            child: child.into(),
        }
    }

    /// A fixed transform, as a stopped animation.
    #[must_use]
    pub fn fixed(transform: CoreTransform, child: impl Into<Widget>) -> Self {
        Self::new(0.0, move |_| transform, child)
    }

    /// The transform's origin within the child. Defaults to the center.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Flutter's `filterQuality`: while the animation runs, the child is
    /// rasterized and resampled through the matrix with this quality. It
    /// is ignored while the animation is dismissed or completed.
    #[must_use]
    pub fn filter_quality(mut self, quality: incular_rendering::FilterQuality) -> Self {
        self.filter_quality = Some(quality);
        self
    }
}

impl From<MatrixTransition> for Widget {
    fn from(value: MatrixTransition) -> Self {
        let parent = value.animation.clone();
        animated_widget_for(parent, move |_| {
            let transform = Transform::new(
                (value.on_transform)(value.animation.value()),
                value.child.clone(),
            )
            .alignment(value.alignment);
            // Flutter `MatrixTransition.build`: `animation.isAnimating ?
            // filterQuality : null`.
            match value.filter_quality {
                Some(quality) if value.animation.is_animating() => {
                    transform.filter_quality(quality)
                }
                _ => transform,
            }
            .into()
        })
    }
}

/// Builds one side of a [`DualTransitionBuilder`] from the context, the
/// animation value it should follow and the child.
pub type AnimatedTransitionBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget>;

/// Flutter's `DualTransitionBuilder`: `forward_builder` animates while the
/// animation runs forward and `reverse_builder` while it runs in reverse.
///
/// Each builder gets the value of the animation it follows. While the
/// animation is dismissed or running forward, the forward builder follows it
/// and the reverse builder sees 0 (`kAlwaysDismissedAnimation`). Once it
/// completes or runs in reverse, the forward builder sees 1
/// (`kAlwaysCompleteAnimation`) and the reverse builder follows the reversed
/// animation, `1 - value`. An animation that turns around midway keeps
/// playing the transition it started.
#[derive(Clone)]
pub struct DualTransitionBuilder {
    animation: AnimationController,
    forward_builder: AnimatedTransitionBuilder,
    reverse_builder: AnimatedTransitionBuilder,
    child: Option<Widget>,
}

impl DualTransitionBuilder {
    #[must_use]
    pub fn new(
        animation: AnimationController,
        forward_builder: impl for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget + 'static,
        reverse_builder: impl for<'a> Fn(&BuildContext<'a>, f32, Option<Widget>) -> Widget + 'static,
    ) -> Self {
        Self {
            animation,
            forward_builder: Rc::new(forward_builder),
            reverse_builder: Rc::new(reverse_builder),
            child: None,
        }
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// `_calculateEffectiveAnimationStatus`: a transition interrupted
    /// midway plays back the way it came instead of switching to the other
    /// transition.
    #[must_use]
    pub fn effective_status(last: AnimationStatus, current: AnimationStatus) -> AnimationStatus {
        match (current, last) {
            (AnimationStatus::Forward, AnimationStatus::Reverse)
            | (AnimationStatus::Reverse, AnimationStatus::Forward) => last,
            _ => current,
        }
    }

    /// `_updateAnimations`: the values the forward and reverse builders see
    /// for an effective status and the animation's value.
    #[must_use]
    pub fn builder_values(effective: AnimationStatus, value: f32) -> (f32, f32) {
        match effective {
            AnimationStatus::Dismissed | AnimationStatus::Forward => (value, 0.0),
            AnimationStatus::Reverse | AnimationStatus::Completed => (1.0, 1.0 - value),
        }
    }
}

struct DualState {
    listener: AnimationListenerState,
    effective: Cell<Option<AnimationStatus>>,
}

impl RetainedState for DualState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        self.listener.rebind_revision(revision);
    }
}

impl DualState {
    /// `_animationListener`.
    fn update(&self, current: AnimationStatus) -> AnimationStatus {
        let effective = match self.effective.get() {
            Some(last) => DualTransitionBuilder::effective_status(last, current),
            None => current,
        };
        self.effective.set(Some(effective));
        effective
    }
}

impl From<DualTransitionBuilder> for Widget {
    fn from(value: DualTransitionBuilder) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| DualState {
                listener: AnimationListenerState::new(revision),
                effective: Cell::new(None),
            },
            move |state, context, _| {
                let animation = &config.animation;
                let weak = Rc::downgrade(state);
                let listened = animation.clone();
                state.listener.listen(animation, move || {
                    if let Some(state) = weak.upgrade() {
                        state.update(listened.status());
                    }
                });
                let effective = state.update(animation.status());
                let (forward, reverse) =
                    DualTransitionBuilder::builder_values(effective, animation.value());
                let reversed = (config.reverse_builder)(context, reverse, config.child.clone());
                let child = (config.forward_builder)(context, forward, Some(reversed));
                Widget::animation_ticker_with_revision(
                    animation.clone(),
                    false,
                    state.listener.revision(),
                    child,
                )
            },
            false,
        )
    }
}

/// Flutter's `Animation<T>` built by `Tween.animate(CurvedAnimation(...))`:
/// a double animation's value through a curve, then interpolated from
/// `begin` to `end` with [`Lerp`].
#[derive(Clone)]
pub struct TweenAnimation<T> {
    parent: DoubleAnimation,
    curve: Curve,
    tween: PropertyTween<T>,
}

impl<T: Lerp + Clone> TweenAnimation<T> {
    /// `Tween(begin:, end:).animate(parent)`; `parent` may be a controller
    /// or any [`DoubleAnimation`].
    #[must_use]
    pub fn new(parent: impl Into<DoubleAnimation>, begin: T, end: T) -> Self {
        Self {
            parent: parent.into(),
            curve: Curve::Linear,
            tween: PropertyTween { begin, end },
        }
    }

    /// A value that never changes: `AlwaysStoppedAnimation(value)`.
    #[must_use]
    pub fn constant(value: T) -> Self {
        Self::new(AlwaysStoppedAnimation(0.0), value.clone(), value)
    }

    /// The curve between the parent and the tween. Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// The animation driving the tween.
    #[must_use]
    pub fn parent(&self) -> &DoubleAnimation {
        &self.parent
    }

    /// The controllers whose ticks change the value.
    #[must_use]
    pub fn controllers(&self) -> Vec<AnimationController> {
        self.parent.controllers()
    }

    /// The tween evaluated at the curved parent value: exactly `begin` at 0
    /// and `end` at 1.
    #[must_use]
    pub fn value(&self) -> T {
        self.tween.evaluate(self.curve.apply(self.parent.value()))
    }

    /// The parent's status.
    #[must_use]
    pub fn status(&self) -> AnimationStatus {
        self.parent.status()
    }
}

impl<T: Lerp + Clone> From<T> for TweenAnimation<T> {
    fn from(value: T) -> Self {
        Self::constant(value)
    }
}

/// A controller listener that bumps a retained build's revision, so the
/// build reruns whenever the controller notifies: Flutter `AnimatedWidget`'s
/// `_handleChange`.
pub(crate) struct AnimationListenerState {
    revision: RefCell<Rc<Cell<u64>>>,
    listening: RefCell<Option<(AnimationController, usize)>>,
}

impl AnimationListenerState {
    pub(crate) fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            revision: RefCell::new(revision),
            listening: RefCell::new(None),
        }
    }

    pub(crate) fn revision(&self) -> Rc<Cell<u64>> {
        self.revision.borrow().clone()
    }

    pub(crate) fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }

    /// Listens to `controller`, dropping the listener of a replaced one
    /// (`didUpdateWidget`). `also` runs on every notification before the
    /// rebuild is scheduled.
    pub(crate) fn listen(&self, controller: &AnimationController, also: impl Fn() + 'static) {
        if matches!(&*self.listening.borrow(), Some((listened, _)) if listened == controller) {
            return;
        }
        if let Some((old, id)) = self.listening.borrow_mut().take() {
            old.remove_listener(id);
        }
        let revision = Rc::downgrade(&self.revision.borrow());
        let id = controller.add_listener(move |_| {
            also();
            if let Some(revision) = revision.upgrade() {
                revision.set(revision.get().wrapping_add(1));
            }
        });
        *self.listening.borrow_mut() = Some((controller.clone(), id));
    }
}

impl Drop for AnimationListenerState {
    fn drop(&mut self) {
        if let Some((controller, id)) = self.listening.borrow_mut().take() {
            controller.remove_listener(id);
        }
    }
}
