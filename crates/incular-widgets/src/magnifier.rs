//! The renderer-independent parts of Flutter's magnifier
//! (`widgets/magnifier.dart`): the gesture information a text magnifier
//! follows, its configuration and decoration, and the backdrop transform a
//! `RawMagnifier` applies.
//!
//! Incular's backdrop filters are Gaussian blurs only, so the magnified
//! backdrop itself is not drawn yet: [`RawMagnifier`] describes the
//! magnifier and computes the transform its backdrop filter would use.

use std::rc::Rc;

use incular_config::Clip;
use incular_core::{Offset, Rect, Size, Transform};

use crate::{BoxShadow, RoundedRectangleBorder, ShapeBorder, Widget};

const ZERO_RECT: Rect = Rect {
    origin: Offset::ZERO,
    size: Size::ZERO,
};

/// Flutter `MagnifierInfo`: where the gesture driving a text magnifier is,
/// in global coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MagnifierInfo {
    pub global_gesture_position: Offset,
    /// The bounds of the line the gesture is on.
    pub current_line_boundaries: Rect,
    pub caret_rect: Rect,
    /// The bounds of the text field.
    pub field_bounds: Rect,
}

impl MagnifierInfo {
    /// Flutter `MagnifierInfo.empty`.
    pub const EMPTY: Self = Self {
        global_gesture_position: Offset::ZERO,
        current_line_boundaries: ZERO_RECT,
        caret_rect: ZERO_RECT,
        field_bounds: ZERO_RECT,
    };
}

/// Builds a text magnifier for the gesture, or `None` for no magnifier.
pub type MagnifierBuilder = Rc<dyn Fn(MagnifierInfo) -> Option<Widget>>;

/// Flutter `TextMagnifierConfiguration`.
#[derive(Clone)]
pub struct TextMagnifierConfiguration {
    magnifier_builder: Option<MagnifierBuilder>,
    should_display_handles_in_magnifier: bool,
}

impl Default for TextMagnifierConfiguration {
    fn default() -> Self {
        Self::disabled()
    }
}

impl TextMagnifierConfiguration {
    /// Flutter `TextMagnifierConfiguration.disabled`: builds no magnifier
    /// and shows handles in it.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            magnifier_builder: None,
            should_display_handles_in_magnifier: true,
        }
    }

    #[must_use]
    pub fn new(builder: impl Fn(MagnifierInfo) -> Option<Widget> + 'static) -> Self {
        Self {
            magnifier_builder: Some(Rc::new(builder)),
            ..Self::disabled()
        }
    }

    /// Whether the selection handles show inside the magnifier. Defaults to
    /// true.
    #[must_use]
    pub const fn should_display_handles_in_magnifier(mut self, display: bool) -> Self {
        self.should_display_handles_in_magnifier = display;
        self
    }

    #[must_use]
    pub const fn displays_handles_in_magnifier(&self) -> bool {
        self.should_display_handles_in_magnifier
    }

    /// The magnifier for `info`; `None` without a builder.
    #[must_use]
    pub fn build(&self, info: MagnifierInfo) -> Option<Widget> {
        self.magnifier_builder
            .as_ref()
            .and_then(|builder| builder(info))
    }
}

/// Flutter `MagnifierController`'s geometry.
pub struct MagnifierController;

impl MagnifierController {
    /// Flutter `shiftWithinBounds`: moves `rect`, which must fit, the least
    /// distance that puts it inside `bounds`.
    #[must_use]
    pub fn shift_within_bounds(rect: Rect, bounds: Rect) -> Rect {
        let right = |r: Rect| r.origin.x + r.size.width;
        let bottom = |r: Rect| r.origin.y + r.size.height;
        let mut shift = Offset::ZERO;
        if rect.origin.x < bounds.origin.x {
            shift.x = bounds.origin.x - rect.origin.x;
        } else if right(rect) > right(bounds) {
            shift.x = right(bounds) - right(rect);
        }
        if rect.origin.y < bounds.origin.y {
            shift.y = bounds.origin.y - rect.origin.y;
        } else if bottom(rect) > bottom(bounds) {
            shift.y = bottom(bounds) - bottom(rect);
        }
        Rect::from_origin_size(rect.origin + shift, rect.size)
    }
}

/// Flutter `MagnifierDecoration`.
#[derive(Clone)]
pub struct MagnifierDecoration {
    /// Defaults to 1.
    pub opacity: f32,
    pub shadows: Vec<BoxShadow>,
    /// Defaults to a square-cornered [`RoundedRectangleBorder`].
    pub shape: Rc<dyn ShapeBorder>,
}

impl Default for MagnifierDecoration {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            shadows: Vec::new(),
            shape: Rc::new(RoundedRectangleBorder::default()),
        }
    }
}

/// Flutter `RawMagnifier`'s configuration.
#[derive(Clone)]
pub struct RawMagnifier {
    pub child: Option<Widget>,
    pub decoration: MagnifierDecoration,
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
    /// Where the magnifier's center points, relative to its center.
    pub focal_point_offset: Offset,
    /// Defaults to 1, true to size.
    pub magnification_scale: f32,
    pub size: Size,
}

impl RawMagnifier {
    #[must_use]
    pub fn new(size: Size) -> Self {
        Self {
            child: None,
            decoration: MagnifierDecoration::default(),
            clip_behavior: Clip::None,
            focal_point_offset: Offset::ZERO,
            magnification_scale: 1.0,
            size,
        }
    }

    /// Sets the magnification; zero is undefined.
    #[must_use]
    pub fn magnification_scale(mut self, scale: f32) -> Self {
        assert!(
            scale != 0.0,
            "Magnification scale of 0 results in undefined behavior."
        );
        self.magnification_scale = scale;
        self
    }

    #[must_use]
    pub fn focal_point_offset(mut self, offset: Offset) -> Self {
        self.focal_point_offset = offset;
        self
    }

    #[must_use]
    pub fn decoration(mut self, decoration: MagnifierDecoration) -> Self {
        self.decoration = decoration;
        self
    }

    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// `_RenderMagnification.paint`'s backdrop matrix for a magnifier whose
    /// top-left corner is at `origin`: scaled by the magnification about the
    /// focal point, which lands on the magnifier's center.
    #[must_use]
    pub fn backdrop_transform(&self, origin: Offset) -> Transform {
        let scale = self.magnification_scale;
        let center = Offset::new(
            origin.x + self.size.width / 2.0,
            origin.y + self.size.height / 2.0,
        );
        let translation = Offset::new(
            scale * (-self.focal_point_offset.x - center.x) + center.x,
            scale * (-self.focal_point_offset.y - center.y) + center.y,
        );
        Transform::translation(translation).then(Transform::scale(scale))
    }
}
