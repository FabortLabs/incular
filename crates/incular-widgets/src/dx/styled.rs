//! Chainable layout and decoration modifiers, after the `styled_widget`
//! package: `Text::new("Hi").padding_all(8.).background(color).on_tap(..)`.
//!
//! Each modifier wraps the receiver in the widget Flutter code would write
//! by hand, so the result is ordinary composition: `a.padding_all(8.)` is
//! `Padding::all(8., a)`. Modifiers apply inside out, as written.

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Color, Offset};

use crate::{
    Align, AspectRatio, BoxDecoration, Center, ClipRRect, ConstrainedBox, DecoratedBox, Expanded,
    FittedBox, Flexible, GestureDetector, IgnorePointer, ImageFiltered, Opacity, Padding,
    Positioned, SafeArea, Semantics, SizedBox, Transform, Visibility, Widget,
};

/// Chainable modifiers for anything that becomes a [`Widget`].
///
/// Method names avoid the inherent builders of common widgets (`Text::align`
/// takes a `TextAlign`, for instance), so `aligned` and `centered` are used
/// for alignment.
pub trait WidgetExt: Into<Widget> + Sized {
    /// `Padding(padding: padding, child: self)`.
    #[must_use]
    fn padding(self, padding: EdgeInsets) -> Widget {
        Padding::new(padding, self).into()
    }

    /// `Padding(padding: EdgeInsets.all(value))`.
    #[must_use]
    fn padding_all(self, value: f32) -> Widget {
        Padding::all(value, self).into()
    }

    /// `Padding(padding: EdgeInsets.symmetric(...))`.
    #[must_use]
    fn padding_symmetric(self, horizontal: f32, vertical: f32) -> Widget {
        Padding::symmetric(horizontal, vertical, self).into()
    }

    /// `Padding(padding: EdgeInsets.only(...))`, left, top, right, bottom.
    #[must_use]
    fn padding_only(self, left: f32, top: f32, right: f32, bottom: f32) -> Widget {
        Padding::new(EdgeInsets::only(left, top, right, bottom), self).into()
    }

    /// `Center(child: self)`.
    #[must_use]
    fn centered(self) -> Widget {
        Center::new(self).into()
    }

    /// `Align(alignment: alignment, child: self)`.
    #[must_use]
    fn aligned(self, alignment: Alignment) -> Widget {
        Align::new(alignment, self).into()
    }

    /// `Expanded(child: self)`.
    #[must_use]
    fn expanded(self) -> Widget {
        Expanded::new(self).into()
    }

    /// `Expanded(flex: flex, child: self)`.
    #[must_use]
    fn expanded_flex(self, flex: u32) -> Widget {
        Expanded::flex_factor(flex, self).into()
    }

    /// `Flexible(child: self)`.
    #[must_use]
    fn flexible(self) -> Widget {
        Flexible::new(self).into()
    }

    /// `SizedBox(width: width, height: height, child: self)`.
    #[must_use]
    fn sized(self, width: f32, height: f32) -> Widget {
        SizedBox::new()
            .width(width)
            .height(height)
            .child(self)
            .into()
    }

    /// `SizedBox(width: width, child: self)`.
    #[must_use]
    fn with_width(self, width: f32) -> Widget {
        SizedBox::new().width(width).child(self).into()
    }

    /// `SizedBox(height: height, child: self)`.
    #[must_use]
    fn with_height(self, height: f32) -> Widget {
        SizedBox::new().height(height).child(self).into()
    }

    /// `ConstrainedBox(constraints: constraints, child: self)`.
    #[must_use]
    fn constrained(self, constraints: Constraints) -> Widget {
        ConstrainedBox::new(constraints, self).into()
    }

    /// `AspectRatio(aspectRatio: ratio, child: self)`.
    #[must_use]
    fn aspect_ratio(self, ratio: f32) -> Widget {
        AspectRatio::new(ratio, self).into()
    }

    /// `FittedBox(child: self)`.
    #[must_use]
    fn fitted(self) -> Widget {
        FittedBox::new(self).into()
    }

    /// `DecoratedBox(decoration: BoxDecoration(color: color))`.
    #[must_use]
    fn background(self, color: Color) -> Widget {
        DecoratedBox::new(self)
            .decoration(BoxDecoration::new().color(color))
            .into()
    }

    /// `DecoratedBox(decoration: decoration, child: self)`.
    #[must_use]
    fn decorated(self, decoration: BoxDecoration) -> Widget {
        DecoratedBox::new(self).decoration(decoration).into()
    }

    /// `ClipRRect(borderRadius: BorderRadius.circular(radius))`.
    #[must_use]
    fn clip_rounded(self, radius: f32) -> Widget {
        ClipRRect::new(radius, self).into()
    }

    /// `Opacity(opacity: opacity, child: self)`.
    #[must_use]
    fn with_opacity(self, opacity: f32) -> Widget {
        Opacity::new(opacity, self).into()
    }

    /// `ImageFiltered(imageFilter: ImageFilter.blur(sigma, sigma))`.
    #[must_use]
    fn blurred(self, sigma: f32) -> Widget {
        ImageFiltered::blur(sigma, self).into()
    }

    /// `Transform.rotate(angle: turns * 2π, child: self)`.
    #[must_use]
    fn rotated(self, turns: f32) -> Widget {
        Transform::rotation(turns * std::f32::consts::TAU, self).into()
    }

    /// `Transform.scale(scale: scale, child: self)`.
    #[must_use]
    fn scaled(self, scale: f32) -> Widget {
        Transform::scale(scale, self).into()
    }

    /// `Transform.translate(offset: offset, child: self)`.
    #[must_use]
    fn translated(self, offset: Offset) -> Widget {
        Transform::translation(offset, self).into()
    }

    /// `GestureDetector(onTap: callback, child: self)`.
    #[must_use]
    fn on_tap(self, callback: impl Fn() + 'static) -> Widget {
        GestureDetector::new(self).on_tap(callback).into()
    }

    /// `GestureDetector(onDoubleTap: callback, child: self)`.
    #[must_use]
    fn on_double_tap(self, callback: impl Fn() + 'static) -> Widget {
        GestureDetector::new(self).on_double_tap(callback).into()
    }

    /// `GestureDetector(onLongPress: callback, child: self)`.
    #[must_use]
    fn on_long_press(self, callback: impl Fn() + 'static) -> Widget {
        GestureDetector::new(self).on_long_press(callback).into()
    }

    /// `IgnorePointer(child: self)`.
    #[must_use]
    fn ignore_pointer(self) -> Widget {
        IgnorePointer::new(self).into()
    }

    /// `Visibility(visible: visible, child: self)`.
    #[must_use]
    fn visible(self, visible: bool) -> Widget {
        Visibility::new(self).visible(visible).into()
    }

    /// `Semantics(label: label, child: self)`.
    #[must_use]
    fn semantics_label(self, label: impl Into<String>) -> Widget {
        Semantics::new(self).label(label).into()
    }

    /// `SafeArea(child: self)`.
    #[must_use]
    fn safe_area(self) -> Widget {
        SafeArea::new(self).into()
    }

    /// `Positioned(left: .., top: .., child: self)` for a `Stack` child.
    #[must_use]
    fn positioned(
        self,
        left: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
    ) -> Widget {
        let mut positioned = Positioned::new(self);
        if let Some(left) = left {
            positioned = positioned.left(left);
        }
        if let Some(top) = top {
            positioned = positioned.top(top);
        }
        if let Some(right) = right {
            positioned = positioned.right(right);
        }
        if let Some(bottom) = bottom {
            positioned = positioned.bottom(bottom);
        }
        positioned.into()
    }

    /// `Positioned.fill(child: self)`.
    #[must_use]
    fn positioned_fill(self) -> Widget {
        Positioned::fill(self).into()
    }
}

impl<T: Into<Widget>> WidgetExt for T {}
