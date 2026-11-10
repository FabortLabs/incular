//! Flutter's named tweens for widget-layer values, as [`LerpTween`]
//! aliases: each interpolates with its type's `lerp`, like Flutter's.

use crate::{Alignment, AlignmentDirectional, Border, BorderRadius, BoxDecoration, TextStyle};
pub use incular_animation::LerpTween;
use incular_config::{EdgeInsets, EdgeInsetsDirectional};

/// Flutter's `AlignmentTween`.
pub type AlignmentTween = LerpTween<Alignment>;
/// Flutter's `AlignmentGeometryTween` for directional alignments.
pub type AlignmentDirectionalTween = LerpTween<AlignmentDirectional>;
/// Flutter's `BorderRadiusTween`.
pub type BorderRadiusTween = LerpTween<BorderRadius>;
/// Flutter's `BorderTween`.
pub type BorderTween = LerpTween<Border>;
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
