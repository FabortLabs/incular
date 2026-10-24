//! Flutter's Material `Magnifier` and `TextMagnifier`
//! (`material/magnifier.dart`): the Android text magnifier's look and the
//! rule that places it above the gesture.

use std::{rc::Rc, time::Duration};

use incular_config::{Clip, TargetPlatform};
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    BorderRadius, BorderSide, BoxShadow, MagnifierController, MagnifierDecoration, MagnifierInfo,
    RawMagnifier, RoundedRectangleBorder,
};

/// Flutter's Material `Magnifier`: a pill-shaped loupe magnifying 1.25
/// times, its focal point below it.
#[derive(Clone, Debug, PartialEq)]
pub struct Magnifier {
    pub additional_focal_point_offset: Offset,
    pub border_radius: BorderRadius,
    pub film_color: Color,
    pub shadows: Vec<BoxShadow>,
    pub clip_behavior: Clip,
    pub size: Size,
}

impl Default for Magnifier {
    fn default() -> Self {
        Self {
            additional_focal_point_offset: Offset::ZERO,
            border_radius: BorderRadius::circular(Self::BORDER_RADIUS),
            film_color: Color::rgba(158, 158, 158, 8),
            shadows: vec![BoxShadow::new(
                Color::rgba(0, 0, 0, 25),
                Offset::new(0.0, 2.0),
                1.5,
                0.75,
            )],
            clip_behavior: Clip::HardEdge,
            size: Self::default_magnifier_size(),
        }
    }
}

impl Magnifier {
    /// Flutter `kStandardVerticalFocalPointShift`: how far below the
    /// magnifier's bottom its focal point sits.
    pub const STANDARD_VERTICAL_FOCAL_POINT_SHIFT: f32 = 22.0;
    const BORDER_RADIUS: f32 = 40.0;
    /// Flutter `_magnification`.
    pub const MAGNIFICATION: f32 = 1.25;

    /// Flutter `kDefaultMagnifierSize`: 77.37 by 37.9.
    #[must_use]
    pub fn default_magnifier_size() -> Size {
        Size::new(77.37, 37.9)
    }

    /// The `RawMagnifier` this magnifier builds, its film color the child.
    #[must_use]
    pub fn raw_magnifier(&self) -> RawMagnifier {
        RawMagnifier::new(self.size)
            .decoration(MagnifierDecoration {
                shadows: self.shadows.clone(),
                shape: Rc::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    self.border_radius,
                )),
                ..MagnifierDecoration::default()
            })
            .clip_behavior(self.clip_behavior)
            .magnification_scale(Self::MAGNIFICATION)
            .focal_point_offset(
                self.additional_focal_point_offset
                    + Offset::new(
                        0.0,
                        Self::STANDARD_VERTICAL_FOCAL_POINT_SHIFT
                            + Self::default_magnifier_size().height / 2.0,
                    ),
            )
            .child(incular_widgets::ColoredBox::new(
                self.film_color,
                incular_widgets::SizedBox::shrink(),
            ))
    }
}

/// Where a [`TextMagnifier`] goes: its top-left corner and the extra focal
/// point offset for its [`Magnifier`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMagnifierPosition {
    pub position: Offset,
    pub focal_point_offset: Offset,
}

/// Flutter's Material `TextMagnifier`.
pub struct TextMagnifier;

impl TextMagnifier {
    /// Flutter `jumpBetweenLinesAnimationDuration`: the magnifier animates
    /// only when it moves to another line.
    pub const JUMP_BETWEEN_LINES_ANIMATION_DURATION: Duration = Duration::from_millis(70);

    /// Flutter `adaptiveMagnifierConfiguration`'s handle rule: handles show
    /// in the magnifier on iOS only. Its builder picks the platform's
    /// magnifier: Cupertino's on iOS, this one on Android, none elsewhere.
    #[must_use]
    pub fn displays_handles_in_magnifier(platform: TargetPlatform) -> bool {
        platform == TargetPlatform::Ios
    }

    /// `_determineMagnifierPositionAndFocalPoint`: centered above the
    /// gesture, held within its line horizontally and within the screen,
    /// with the focal point kept from showing beyond the field.
    #[must_use]
    pub fn position(info: &MagnifierInfo, screen: Size) -> TextMagnifierPosition {
        let size = Magnifier::default_magnifier_size();
        let screen_rect = Rect::from_origin_size(Offset::ZERO, screen);
        let basic_offset = Offset::new(
            size.width / 2.0,
            size.height + Magnifier::STANDARD_VERTICAL_FOCAL_POINT_SHIFT,
        );
        let line = info.current_line_boundaries;
        let magnifier_x = info
            .global_gesture_position
            .x
            .max(line.origin.x)
            .min(line.origin.x + line.size.width);
        let caret_center_y = info.caret_rect.origin.y + info.caret_rect.size.height / 2.0;
        let unadjusted = Rect::from_origin_size(
            Offset::new(magnifier_x, caret_center_y) - basic_offset,
            size,
        );
        let adjusted = MagnifierController::shift_within_bounds(unadjusted, screen_rect);
        let horizontal_inset = (size.width / 2.0) / Magnifier::MAGNIFICATION;
        let field = info.field_bounds;
        let adjusted_center_x = adjusted.origin.x + adjusted.size.width / 2.0;
        let focal_x = if field.size.width < horizontal_inset * 2.0 {
            field.origin.x + field.size.width / 2.0
        } else {
            adjusted_center_x
                .max(field.origin.x + horizontal_inset)
                .min(field.origin.x + field.size.width - horizontal_inset)
        };
        TextMagnifierPosition {
            position: adjusted.origin,
            focal_point_offset: Offset::new(
                focal_x - adjusted_center_x,
                unadjusted.origin.y - adjusted.origin.y,
            ),
        }
    }
}
