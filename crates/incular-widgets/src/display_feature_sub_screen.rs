//! Flutter's `DisplayFeatureSubScreen`
//! (`widgets/display_feature_sub_screen.dart`): places its child on one
//! sub-screen of a view that hinges or folds divide.

use incular_config::{DisplayFeature, DisplayFeatureState, EdgeInsets, TextDirection};
use incular_core::{Offset, Rect, Size};

use crate::{BuildContext, Builder, MediaQuery, Padding, Widget};

/// Flutter's `DisplayFeatureSubScreen`: lays its child out on the
/// sub-screen closest to `anchor_point`, between the display features that
/// separate the view. Without an anchor the child goes on the start-most
/// sub-screen: top left, or top right in a right-to-left context.
pub struct DisplayFeatureSubScreen {
    anchor_point: Option<Offset>,
    child: Widget,
}

impl DisplayFeatureSubScreen {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            anchor_point: None,
            child: child.into(),
        }
    }

    /// The point, in view coordinates, the chosen sub-screen is closest to.
    #[must_use]
    pub fn anchor_point(mut self, anchor: Offset) -> Self {
        self.anchor_point = Some(anchor);
        self
    }

    /// `DisplayFeatureSubScreen.avoidBounds`: the features that divide the
    /// view. A zero-width fold divides it only while half opened.
    #[must_use]
    pub fn avoid_bounds(features: &[DisplayFeature]) -> Vec<Rect> {
        features
            .iter()
            .filter(|feature| divides_view(feature))
            .map(|feature| feature.bounds)
            .collect()
    }

    /// `DisplayFeatureSubScreen.subScreensInBounds`: the parts of
    /// `wanted_bounds` left on each side of every bound to avoid.
    #[must_use]
    pub fn sub_screens_in_bounds(wanted_bounds: Rect, avoid_bounds: &[Rect]) -> Vec<Rect> {
        split_sub_screens(wanted_bounds, avoid_bounds.iter().copied())
    }

    fn build(context: &BuildContext<'_>, anchor: Option<Offset>, child: Widget) -> Widget {
        let data = context.effective_media_query();
        let parent_size = data.viewport;
        let wanted_bounds = Rect::from_origin_size(Offset::ZERO, parent_size);
        let anchor = cap_offset(
            anchor.unwrap_or_else(|| fallback_anchor_point(context)),
            parent_size,
        );
        let sub_screens = split_sub_screens(
            wanted_bounds,
            data.display_features
                .iter()
                .filter(|feature| divides_view(feature))
                .map(|feature| feature.bounds),
        );
        let closest = closest_to_anchor_point(&sub_screens, anchor).unwrap_or(wanted_bounds);
        let (left, top, right, bottom) = ltrb(closest);
        let remaining_right = parent_size.width - right;
        let remaining_bottom = parent_size.height - bottom;
        let child = MediaQuery::new(data.remove_display_features(closest), child);
        Padding::new(
            EdgeInsets {
                left,
                top,
                right: remaining_right,
                bottom: remaining_bottom,
            },
            child,
        )
        .into()
    }
}

/// Whether a display feature divides the view: a zero-width fold divides it
/// only while half opened.
fn divides_view(feature: &DisplayFeature) -> bool {
    shortest_side(feature.bounds) > 0. || feature.state == DisplayFeatureState::PostureHalfOpened
}

fn split_sub_screens(
    wanted_bounds: Rect,
    avoid_bounds: impl IntoIterator<Item = Rect>,
) -> Vec<Rect> {
    let mut sub_screens = vec![wanted_bounds];
    for avoid in avoid_bounds {
        let (avoid_left, avoid_top, avoid_right, avoid_bottom) = ltrb(avoid);
        sub_screens = sub_screens
            .into_iter()
            .flat_map(|screen| {
                let (left, top, right, bottom) = ltrb(screen);
                let mut parts = Vec::new();
                if top >= avoid_top && bottom <= avoid_bottom {
                    if left < avoid_left {
                        parts.push(from_ltrb(left, top, avoid_left, bottom));
                    }
                    if right > avoid_right {
                        parts.push(from_ltrb(avoid_right, top, right, bottom));
                    }
                } else if left >= avoid_left && right <= avoid_right {
                    if top < avoid_top {
                        parts.push(from_ltrb(left, top, right, avoid_top));
                    }
                    if bottom > avoid_bottom {
                        parts.push(from_ltrb(left, avoid_bottom, right, bottom));
                    }
                } else {
                    parts.push(screen);
                }
                parts
            })
            .collect();
    }
    sub_screens
}

impl From<DisplayFeatureSubScreen> for Widget {
    fn from(value: DisplayFeatureSubScreen) -> Self {
        let anchor = value.anchor_point;
        let child = value.child;
        let builder = Builder::new(move |context| {
            DisplayFeatureSubScreen::build(context, anchor, child.clone())
        });
        if anchor.is_none() {
            builder
                .require_directionality(&crate::tree::DirectionRequirement {
                    widget: "DisplayFeatureSubScreen",
                    reason: "choosing a default sub-screen without an anchor",
                })
                .into()
        } else {
            builder.into()
        }
    }
}

fn ltrb(rect: Rect) -> (f32, f32, f32, f32) {
    (
        rect.origin.x,
        rect.origin.y,
        rect.origin.x + rect.size.width,
        rect.origin.y + rect.size.height,
    )
}

fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

fn shortest_side(rect: Rect) -> f32 {
    rect.size.width.abs().min(rect.size.height.abs())
}

/// `_fallbackAnchorPoint`.
fn fallback_anchor_point(context: &BuildContext<'_>) -> Offset {
    match context.depend_on::<TextDirection>() {
        Some(TextDirection::Rtl) => Offset::new(f32::MAX, 0.),
        _ => Offset::ZERO,
    }
}

/// `_capOffset`: clamps the anchor into the view.
fn cap_offset(offset: Offset, maximum: Size) -> Offset {
    Offset::new(
        offset.x.clamp(0., maximum.width.max(0.)),
        offset.y.clamp(0., maximum.height.max(0.)),
    )
}

/// `_closestToAnchorPoint`: the first sub-screen at the least distance.
fn closest_to_anchor_point(sub_screens: &[Rect], anchor: Offset) -> Option<Rect> {
    let mut closest = *sub_screens.first()?;
    let mut closest_distance = distance_from_point_to_rect(anchor, closest);
    for screen in sub_screens {
        let distance = distance_from_point_to_rect(anchor, *screen);
        if distance < closest_distance {
            closest = *screen;
            closest_distance = distance;
        }
    }
    Some(closest)
}

/// `_distanceFromPointToRect`.
fn distance_from_point_to_rect(point: Offset, rect: Rect) -> f32 {
    let (left, top, right, bottom) = ltrb(rect);
    let corner = |x: f32, y: f32| (point.x - x).hypot(point.y - y);
    if point.x < left {
        if point.y < top {
            corner(left, top)
        } else if point.y > bottom {
            corner(left, bottom)
        } else {
            left - point.x
        }
    } else if point.x > right {
        if point.y < top {
            corner(right, top)
        } else if point.y > bottom {
            corner(right, bottom)
        } else {
            point.x - right
        }
    } else if point.y < top {
        top - point.y
    } else if point.y > bottom {
        point.y - bottom
    } else {
        0.
    }
}
