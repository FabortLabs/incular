//! Flutter-style stack and positioning layout descriptors.

use incular_config::{Alignment, AlignmentDirectional, Clip, StackFit, TextDirection};
use incular_core::Rect;
use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

use crate::{
    AlignmentGeometry, PositionedDirectional, RelativeRect, Widget, WidgetKind,
    macros::widget_from_kind,
};

/// Overlays children in paint order with alignment, sizing fit, and clipping.
///
/// The default places children at the top and ambient start edge, falling
/// back to left when no direction is provided. Physical alignment is not
/// mirrored; directional alignment resolves against the stack direction.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct Stack {
    #[builder(default, setter(into))]
    children: Vec<Widget>,
    #[builder(default, setter(strip_option, into))]
    alignment: Option<AlignmentGeometry>,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(default = StackFit::Loose)]
    fit: StackFit,
    #[builder(default = Clip::HardEdge)]
    clip_behavior: Clip,
}

impl Stack {
    /// Creates a new Stack overlay.
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Creates a Stack with explicit alignment.
    #[must_use]
    pub fn aligned(
        alignment: Alignment,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        Self::new(children).alignment(alignment)
    }

    /// Sets the alignment for non-positioned children.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// Overrides the direction used by this stack's own alignment.
    /// Does not change the ambient direction for its descendants.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    /// Sets the sizing policy for non-positioned children.
    #[must_use]
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets clipping behavior for children overflowing stack bounds.
    /// The clip bounds raster output to the stack bounds only; layout,
    /// hit testing, and semantic bounds are unaffected, and `Clip::None`
    /// disables clipping entirely.
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    #[must_use]
    pub fn get_alignment(&self) -> Alignment {
        self.alignment
            .unwrap_or_else(|| AlignmentDirectional::TOP_START.into())
            .resolve(self.text_direction.unwrap_or(TextDirection::Ltr))
    }

    #[must_use]
    pub fn get_fit(&self) -> StackFit {
        self.fit
    }

    #[must_use]
    pub fn get_clip_behavior(&self) -> Clip {
        self.clip_behavior
    }
}

widget_from_kind!(Stack => Stack { alignment, text_direction, fit, clip_behavior, children });

/// Positions a child inside a [`Stack`].
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Positioned {
    #[builder(default, setter(strip_option))]
    pub(crate) left: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(crate) top: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(crate) right: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(crate) bottom: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(crate) width: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub(crate) height: Option<f32>,
    #[builder(setter(into))]
    pub(crate) child: Widget,
}

impl Positioned {
    /// Creates a new unconstrained positioned child.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self::builder().child(child).build()
    }

    /// Creates a positioned child with the rect's left/top and size.
    #[must_use]
    pub fn from_rect(rect: Rect, child: impl Into<Widget>) -> Self {
        Self::new(child)
            .left(rect.origin.x)
            .top(rect.origin.y)
            .width(rect.size.width)
            .height(rect.size.height)
    }

    /// Creates a positioned child with all four insets from the rect.
    #[must_use]
    pub fn from_relative_rect(rect: RelativeRect, child: impl Into<Widget>) -> Self {
        Self::new(child)
            .left(rect.left)
            .top(rect.top)
            .right(rect.right)
            .bottom(rect.bottom)
    }

    /// Resolves directional positioning using a fixed direction, independent
    /// of any ambient direction where the result is mounted.
    #[must_use]
    pub fn directional(direction: TextDirection, positioned: PositionedDirectional) -> Self {
        positioned.resolve(direction)
    }

    /// Creates a positioned child filled to all four edges.
    #[must_use]
    pub fn fill(child: impl Into<Widget>) -> Self {
        Self::builder()
            .left(0.0)
            .top(0.0)
            .right(0.0)
            .bottom(0.0)
            .child(child)
            .build()
    }

    /// Returns the configured left inset.
    #[must_use]
    pub fn get_left(&self) -> Option<f32> {
        self.left
    }

    /// Returns the configured top inset.
    #[must_use]
    pub fn get_top(&self) -> Option<f32> {
        self.top
    }

    /// Returns the configured right inset.
    #[must_use]
    pub fn get_right(&self) -> Option<f32> {
        self.right
    }

    /// Returns the configured bottom inset.
    #[must_use]
    pub fn get_bottom(&self) -> Option<f32> {
        self.bottom
    }

    /// Returns the configured width.
    #[must_use]
    pub fn get_width(&self) -> Option<f32> {
        self.width
    }

    /// Returns the configured height.
    #[must_use]
    pub fn get_height(&self) -> Option<f32> {
        self.height
    }

    /// Sets the left edge offset.
    #[must_use]
    pub fn left(mut self, left: f32) -> Self {
        self.left = Some(left);
        self
    }

    /// Sets the top edge offset.
    #[must_use]
    pub fn top(mut self, top: f32) -> Self {
        self.top = Some(top);
        self
    }

    /// Sets the right edge offset.
    #[must_use]
    pub fn right(mut self, right: f32) -> Self {
        self.right = Some(right);
        self
    }

    /// Sets the bottom edge offset.
    #[must_use]
    pub fn bottom(mut self, bottom: f32) -> Self {
        self.bottom = Some(bottom);
        self
    }

    /// Sets an explicit width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets an explicit height.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

widget_from_kind!(Positioned => Positioned { left, top, right, bottom, width, height, child });

/// A Stack that shows at most one child while keeping all children laid out.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct IndexedStack {
    #[builder(default = Some(0), setter(into))]
    #[default(Some(0))]
    index: Option<usize>,
    #[builder(default, setter(strip_option, into))]
    alignment: Option<AlignmentGeometry>,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(default = StackFit::Loose)]
    fit: StackFit,
    #[builder(default = Clip::HardEdge)]
    clip_behavior: Clip,
    #[builder(default, setter(into))]
    children: Vec<Widget>,
}

impl IndexedStack {
    /// Creates a new IndexedStack.
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Sets the active index. Pass `None` to hide every child without unmounting them.
    /// In debug builds, an index outside the children is rejected, except
    /// for the default index zero on an empty stack.
    #[must_use]
    pub fn index(mut self, index: impl Into<Option<usize>>) -> Self {
        self.index = index.into();
        self
    }

    /// Sets alignment for children.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// Overrides the direction used by this stack's own alignment.
    /// Does not change the ambient direction for its descendants.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    /// Sets the stack fit policy.
    #[must_use]
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the clipping behavior.
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Returns the configured index, or zero if no index is selected.
    /// Use `get_selected_index` to distinguish an absent selection from index zero.
    #[must_use]
    pub fn get_index(&self) -> usize {
        self.index.unwrap_or(0)
    }

    /// Returns the configured selection. `None` hides all children.
    #[must_use]
    pub fn get_selected_index(&self) -> Option<usize> {
        self.index
    }
}

widget_from_kind!(IndexedStack => IndexedStack { alignment, text_direction, index, fit, clip_behavior, children });
