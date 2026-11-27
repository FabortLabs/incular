//! Optional interpolation, value replacement, and diagnostics for box shadows.

use std::{
    borrow::Cow,
    fmt,
    hash::{Hash, Hasher},
};

use incular_core::{Color, Lerp, Offset};

use super::{BlurStyle, BoxShadow};

impl BoxShadow {
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
