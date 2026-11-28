//! Optional interpolation, value replacement, and diagnostics for box shadows.

use std::{
    borrow::Cow,
    fmt,
    hash::{Hash, Hasher},
};

use incular_config::Clip;
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{BlendMode, DisplayList};

use crate::{CustomPaint, Opacity, Stack, Widget};

use super::{BlurStyle, BoxShadow};

impl BoxShadow {
    /// The raster bounds after translating/inflating the shadow. Flutter's
    /// painter sorts an axis when a negative spread crosses both edges.
    pub(super) fn paint_bounds(&self, rect: Rect) -> Rect {
        let spread = self.spread_radius;
        let width = rect.size.width + 2.0 * spread;
        let height = rect.size.height + 2.0 * spread;
        Rect::from_origin_size(
            rect.origin + self.offset - Offset::new(spread, spread)
                + Offset::new(width.min(0.0), height.min(0.0)),
            Size::new(width.abs(), height.abs()),
        )
    }

    /// Replaces only the supplied shadow values.
    #[must_use]
    pub fn copy_with(
        self,
        color: Option<Color>,
        offset: Option<Offset>,
        blur_radius: Option<f32>,
        spread_radius: Option<f32>,
        blur_style: Option<BlurStyle>,
    ) -> Self {
        Self {
            color: color.unwrap_or(self.color),
            offset: offset.unwrap_or(self.offset),
            blur_radius: blur_radius.unwrap_or(self.blur_radius),
            spread_radius: spread_radius.unwrap_or(self.spread_radius),
            blur_style: blur_style.unwrap_or(self.blur_style),
        }
    }

    /// Interpolates optional shadows, preserving the identity of a shared
    /// input and scaling the present shadow when the other input is absent.
    #[must_use]
    pub fn lerp_ref<'a>(a: Option<&'a Self>, b: Option<&'a Self>, t: f32) -> Option<Cow<'a, Self>> {
        match (a, b) {
            (None, None) => None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => Some(Cow::Borrowed(a)),
            (Some(a), Some(b)) => Some(Cow::Owned(a.lerp(b, t))),
            (None, Some(b)) => Some(Cow::Owned(b.scale(t))),
            (Some(a), None) => Some(Cow::Owned(a.scale(1.0 - t))),
        }
    }

    /// Interpolates optional shadow lists, borrowing a shared list unchanged.
    #[must_use]
    pub fn lerp_list_ref<'a>(
        a: Option<&'a [Self]>,
        b: Option<&'a [Self]>,
        t: f32,
    ) -> Option<Cow<'a, [Self]>> {
        match (a, b) {
            (None, None) => None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => Some(Cow::Borrowed(a)),
            (a, b) => Some(Cow::Owned(Self::lerp_list(
                a.unwrap_or_default(),
                b.unwrap_or_default(),
                t,
            ))),
        }
    }
}

impl Hash for BoxShadow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.color.hash(state);
        for value in [
            self.offset.x,
            self.offset.y,
            self.blur_radius,
            self.spread_radius,
        ] {
            super::hash_paint_float(value, state);
        }
        self.blur_style.hash(state);
    }
}

impl fmt::Display for BoxShadow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fraction = |channel: u8| f64::from(channel) / 255.0;
        let style = match self.blur_style {
            BlurStyle::Normal => "normal",
            BlurStyle::Solid => "solid",
            BlurStyle::Outer => "outer",
            BlurStyle::Inner => "inner",
        };
        write!(
            formatter,
            "BoxShadow(Color(alpha: {:.4}, red: {:.4}, green: {:.4}, blue: {:.4}, colorSpace: ColorSpace.sRGB), Offset({:.1}, {:.1}), {:?}, {:?}, BlurStyle.{style})",
            fraction(self.color.alpha),
            fraction(self.color.red),
            fraction(self.color.green),
            fraction(self.color.blue),
            self.offset.x,
            self.offset.y,
            self.blur_radius,
            self.spread_radius,
        )
    }
}

/// Builds a blurred box or shape shadow from a single opaque mask. A solid
/// blur unions the source with the blur; outer and inner use subtraction
/// and intersection respectively. The shadow alpha applies once.
pub(super) fn styled_shadow(
    size: Size,
    shadow: &BoxShadow,
    paint: impl FnOnce(&BoxShadow) -> DisplayList,
) -> Widget {
    let sigma = crate::utilities::banner_shadow_sigma(shadow.blur_radius);
    if sigma <= f32::EPSILON {
        // A zero-sigma Flutter blur does not install a mask filter.
        return CustomPaint::new(size, paint(shadow)).into();
    }
    if shadow.blur_style == BlurStyle::Normal {
        return Widget::blur(sigma, CustomPaint::new(size, paint(shadow)).into());
    }
    let opaque = BoxShadow {
        color: Color {
            alpha: 255,
            ..shadow.color
        },
        ..*shadow
    };
    let shape: Widget = CustomPaint::new(size, paint(&opaque)).into();
    let blurred = Widget::blur(sigma, shape.clone());
    let blend = match shadow.blur_style {
        BlurStyle::Outer => Some(BlendMode::DstOut),
        BlurStyle::Inner => Some(BlendMode::DstIn),
        BlurStyle::Solid => None,
        BlurStyle::Normal => unreachable!("handled before composing a mask"),
    };
    let mask = if let Some(mode) = blend {
        Widget::blend(mode, shape)
    } else {
        shape
    };
    let group: Widget = Stack::new([blurred, mask]).clip_behavior(Clip::None).into();
    if shadow.color.alpha < 255 {
        Opacity::new(f32::from(shadow.color.alpha) / 255.0, group).into()
    } else if blend.is_some() {
        // Alpha-one opacity layers are flattened. An explicit source-over
        // group isolates these destination reads from earlier content.
        Widget::blend(BlendMode::SrcOver, group)
    } else {
        group
    }
}
