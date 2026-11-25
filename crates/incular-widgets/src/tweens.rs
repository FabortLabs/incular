//! Flutter's named tweens for widget-layer values, as [`LerpTween`]
//! aliases: each interpolates with its type's `lerp`, like Flutter's.

use crate::{Alignment, AlignmentDirectional, Border, BorderRadius, BoxDecoration, TextStyle};
pub use incular_animation::LerpTween;
use incular_animation::{NullableTween, OptionalTween, Tween};
use incular_config::{EdgeInsets, EdgeInsetsDirectional};

/// Flutter's `AlignmentTween`.
pub type AlignmentTween = LerpTween<Alignment>;
/// Flutter's `AlignmentGeometryTween` for directional alignments.
pub type AlignmentDirectionalTween = LerpTween<AlignmentDirectional>;
/// Flutter's `BorderRadiusTween`.
pub type BorderRadiusTween = LerpTween<BorderRadius>;
/// Flutter's `BorderTween`.
pub type BorderTween = LerpTween<Border>;
/// Flutter's nullable `BorderTween`.
pub type OptionalBorderTween = OptionalTween<Border>;
/// Flutter's nullable `BorderRadiusTween` for physical corners.
pub type OptionalBorderRadiusTween = OptionalTween<BorderRadius>;
/// Flutter's `BoxConstraintsTween`.
pub type BoxConstraintsTween = LerpTween<incular_config::Constraints>;
/// Flutter's `DecorationTween` over box decorations.
pub type DecorationTween = LerpTween<BoxDecoration>;
/// Flutter's `EdgeInsetsTween`.
pub type EdgeInsetsTween = LerpTween<EdgeInsets>;
/// Flutter's `EdgeInsetsGeometryTween` for directional insets.
pub type EdgeInsetsDirectionalTween = LerpTween<EdgeInsetsDirectional>;
/// Flutter's `TextStyleTween`.
pub type TextStyleTween = LerpTween<TextStyle>;

impl Tween for Border {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        incular_core::Lerp::lerp(&from, &to, t)
    }
}

impl NullableTween for Border {
    fn absent_like(present: Self) -> Self {
        present.scale(0.)
    }

    fn interpolate_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        Self::lerp_optional(begin, end, t)
    }
}

impl Tween for BorderRadius {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        incular_core::Lerp::lerp(&from, &to, t)
    }
}

impl NullableTween for BorderRadius {
    fn absent_like(_: Self) -> Self {
        Self::ZERO
    }

    fn interpolate_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        Self::lerp_optional(begin, end, t)
    }
}
