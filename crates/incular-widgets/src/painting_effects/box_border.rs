//! Owned physical, directional, and custom box border interpolation.

use incular_core::Lerp;

use super::{Border, BorderDirectional, BorderSide, BoxBorderValue, CustomBoxBorderHandle};

/// A single box outline, without early resolution of start and end.
///
/// Use [Self::lerp] when the outline kind can change over the animation.
/// A [super::BoxBorderStack] instead preserves multiple painted outlines.
#[derive(Clone, Debug, PartialEq)]
pub enum BoxBorderOutline {
    Physical(Border),
    Directional(BorderDirectional),
    Custom(CustomBoxBorderHandle),
}

impl From<Border> for BoxBorderOutline {
    fn from(border: Border) -> Self {
        Self::Physical(border)
    }
}

impl From<BorderDirectional> for BoxBorderOutline {
    fn from(border: BorderDirectional) -> Self {
        Self::Directional(border)
    }
}

impl From<CustomBoxBorderHandle> for BoxBorderOutline {
    fn from(border: CustomBoxBorderHandle) -> Self {
        Self::Custom(border)
    }
}

impl BoxBorderOutline {
    fn value(&self) -> BoxBorderValue<'_> {
        match self {
            Self::Physical(border) => BoxBorderValue::Physical(*border),
            Self::Directional(border) => BoxBorderValue::Directional(*border),
            Self::Custom(border) => BoxBorderValue::Custom(border.behavior()),
        }
    }

    /// Flutter's `BoxBorder.lerp`, including nullable and extrapolated
    /// endpoints. Physical and logical horizontal sides do not draw over
    /// each other; when both exist, they switch kinds at the midpoint.
    #[must_use]
    pub fn lerp(a: Option<&Self>, b: Option<&Self>, t: f32) -> Option<Self> {
        use BoxBorderOutline::{Custom, Directional, Physical};
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return Some(a.clone());
        }
        match (a, b) {
            (None, None) => None,
            (Some(Physical(a)), Some(Physical(b))) => Some(Physical(a.lerp(b, t))),
            (Some(Physical(a)), None) => Some(Physical(a.scale(1. - t))),
            (None, Some(Physical(b))) => Some(Physical(b.scale(t))),
            (Some(Directional(a)), Some(Directional(b))) => Some(Directional(a.lerp(b, t))),
            (Some(Directional(a)), None) => Some(Directional(a.scale(1. - t))),
            (None, Some(Directional(b))) => Some(Directional(b.scale(t))),
            (Some(Physical(a)), Some(Directional(b))) => Some(Self::mixed(*a, *b, t)),
            (Some(Directional(a)), Some(Physical(b))) => Some(Self::mixed(*b, *a, 1. - t)),
            (Some(Custom(a)), Some(Custom(b))) if a.same_instance(b) && a == b => {
                Some(Custom(a.clone()))
            }
            _ => {
                let to = if let Some(Custom(to)) = b {
                    to.behavior().lerp_from(a.map(Self::value), t)
                } else {
                    None
                };
                let result = to.or_else(|| {
                    if let Some(Custom(from)) = a {
                        from.behavior().lerp_to(b.map(Self::value), t)
                    } else {
                        None
                    }
                });
                result
                    .map(Custom)
                    .or_else(|| (if t < 0.5 { a } else { b }).cloned())
            }
        }
    }

    fn mixed(physical: Border, logical: BorderDirectional, t: f32) -> Self {
        let absent = BorderSide::NONE;
        let top = physical.top.lerp(&logical.top, t);
        let bottom = physical.bottom.lerp(&logical.bottom, t);
        let no_physical_sides = physical.left == absent && physical.right == absent;
        let no_logical_sides = logical.start == absent && logical.end == absent;
        if no_logical_sides || (!no_physical_sides && t < 0.5) {
            let progress = if no_logical_sides { t } else { t * 2. };
            return Self::Physical(Border {
                top,
                bottom,
                left: physical.left.lerp(&absent, progress),
                right: physical.right.lerp(&absent, progress),
            });
        }
        let progress = if no_physical_sides { t } else { (t - 0.5) * 2. };
        Self::Directional(BorderDirectional::only(
            top,
            absent.lerp(&logical.start, progress),
            absent.lerp(&logical.end, progress),
            bottom,
        ))
    }
}
