use incular_config::Axis;
use incular_core::{Color, Offset, Rect, Size};

use crate::controller::ScrollController;

/// Visual configuration for an overlay scrollbar; `width` is its thickness
/// across the scroll axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarStyle {
    pub width: f32,
    pub min_thumb_extent: f32,
    pub track_color: Color,
    pub thumb_color: Color,
}
impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            // Flutter `RawScrollbar` `_kScrollbarThickness` (scrollbar.dart:42).
            width: 6.,
            // Flutter `RawScrollbar` `_kMinThumbExtent` (scrollbar.dart:40).
            min_thumb_extent: 18.,
            track_color: Color::TRANSPARENT,
            // Flutter `RawScrollbar` `thumbColor ?? const Color(0x66BCBCBC)`.
            thumb_color: Color::rgba(0xBC, 0xBC, 0xBC, 0x66),
        }
    }
}

/// Deterministic geometry for an overlay scrollbar.
///
/// A vertical scrollbar runs along the viewport's trailing (right) edge and
/// a horizontal one along its bottom edge, as Flutter's `ScrollbarPainter`
/// places them for `AxisDirection.down` and `AxisDirection.right`. The
/// `thumb_*` helpers measure along that main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarGeometry {
    pub visible: bool,
    pub track: Rect,
    pub thumb: Rect,
    pub max_scroll_extent: f32,
    pub thumb_travel: f32,
    /// The scroll axis the track runs along.
    pub axis: Axis,
    /// Whether scroll offset zero puts the thumb at the track's far end, as
    /// for a reversed scroll view.
    pub reverse: bool,
}
impl ScrollbarGeometry {
    fn track_start(self) -> f32 {
        match self.axis {
            Axis::Vertical => self.track.origin.y,
            Axis::Horizontal => self.track.origin.x,
        }
    }

    /// The thumb's leading main-axis coordinate (its top for a vertical
    /// scrollbar, its left edge for a horizontal one) at `offset`.
    #[must_use]
    pub fn thumb_start_for_offset(self, offset: f32) -> f32 {
        let mut normalized = if self.max_scroll_extent > 0. && offset.is_finite() {
            (offset / self.max_scroll_extent).clamp(0., 1.)
        } else {
            0.
        };
        if self.reverse {
            normalized = 1. - normalized;
        }
        self.track_start() + normalized * self.thumb_travel
    }

    /// The scroll offset that puts the thumb's leading main-axis edge at
    /// `thumb_start`.
    #[must_use]
    pub fn offset_for_thumb_start(self, thumb_start: f32) -> f32 {
        if self.thumb_travel <= 0. || self.max_scroll_extent <= 0. || !thumb_start.is_finite() {
            return 0.;
        }
        let mut normalized =
            (thumb_start - self.track_start()).clamp(0., self.thumb_travel) / self.thumb_travel;
        if self.reverse {
            normalized = 1. - normalized;
        }
        normalized * self.max_scroll_extent
    }

    /// The thumb's main-axis start, extent and the point's main-axis
    /// coordinate, for pointer handling along either axis.
    #[must_use]
    pub fn main_axis(self, point: Offset) -> (f32, f32, f32) {
        match self.axis {
            Axis::Vertical => (self.thumb.origin.y, self.thumb.size.height, point.y),
            Axis::Horizontal => (self.thumb.origin.x, self.thumb.size.width, point.x),
        }
    }

    /// [`Self::thumb_start_for_offset`] under its vertical name.
    #[must_use]
    pub fn thumb_top_for_offset(self, offset: f32) -> f32 {
        self.thumb_start_for_offset(offset)
    }

    /// [`Self::offset_for_thumb_start`] under its vertical name.
    #[must_use]
    pub fn offset_for_thumb_top(self, thumb_top: f32) -> f32 {
        self.offset_for_thumb_start(thumb_top)
    }
}

/// Derives display geometry for a vertical, unreversed scroll view without
/// creating renderer state.
#[must_use]
pub fn scrollbar_geometry(
    size: Size,
    controller: &ScrollController,
    style: ScrollbarStyle,
) -> ScrollbarGeometry {
    scrollbar_geometry_for_axis(size, controller, style, Axis::Vertical, false)
}

/// Derives display geometry for a scroll view on `axis`: a vertical thumb at
/// the trailing edge or a horizontal thumb along the bottom edge. `reverse`
/// starts the thumb at the far end of the track, as a reversed scroll view
/// starts at its far end.
#[must_use]
pub fn scrollbar_geometry_for_axis(
    size: Size,
    controller: &ScrollController,
    style: ScrollbarStyle,
    axis: Axis,
    reverse: bool,
) -> ScrollbarGeometry {
    let viewport = controller.viewport_extent();
    let content = controller.content_extent();
    let (main, cross) = match axis {
        Axis::Vertical => (size.height, size.width),
        Axis::Horizontal => (size.width, size.height),
    };
    if !viewport.is_finite()
        || !content.is_finite()
        || content <= viewport
        || viewport <= 0.
        || !main.is_finite()
        || main <= 0.
        || !cross.is_finite()
    {
        return ScrollbarGeometry::default();
    }
    let thickness = style.width.min(cross).max(0.);
    let extent = (main * (viewport / content)).clamp(style.min_thumb_extent.min(main), main);
    let (track, thumb) = match axis {
        Axis::Vertical => (
            Rect::from_origin_size(
                Offset::new(size.width - thickness, 0.),
                Size::new(thickness, size.height),
            ),
            Rect::from_origin_size(
                Offset::new(size.width - thickness, 0.),
                Size::new(thickness, extent),
            ),
        ),
        Axis::Horizontal => (
            Rect::from_origin_size(
                Offset::new(0., size.height - thickness),
                Size::new(size.width, thickness),
            ),
            Rect::from_origin_size(
                Offset::new(0., size.height - thickness),
                Size::new(extent, thickness),
            ),
        ),
    };
    let mut geometry = ScrollbarGeometry {
        visible: true,
        track,
        thumb,
        max_scroll_extent: controller.max_offset().max(0.),
        thumb_travel: (main - extent).max(0.),
        axis,
        reverse,
    };
    let start = geometry.thumb_start_for_offset(controller.offset());
    match axis {
        Axis::Vertical => geometry.thumb.origin.y = start,
        Axis::Horizontal => geometry.thumb.origin.x = start,
    }
    geometry
}

impl ScrollController {
    /// Returns the style used when a retained viewport paints its overlay
    /// scrollbar.
    #[must_use]
    pub fn scrollbar_style(&self) -> ScrollbarStyle {
        self.state.borrow().scrollbar_style
    }

    /// Installs the style used by a retained viewport's overlay scrollbar.
    ///
    /// The controller is shared by the viewport and any themed scrollbar
    /// wrapper, so the style is applied before layout/paint without creating
    /// a parallel visual widget tree.
    pub fn set_scrollbar_style(&self, style: ScrollbarStyle) -> bool {
        let mut state = self.state.borrow_mut();
        if state.scrollbar_style == style {
            return false;
        }
        state.scrollbar_style = style;
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        true
    }

    /// Whether the thumb is painted even while the pointer is away from the
    /// scrollbar. When false, the retained viewport still owns hit testing so
    /// the thumb reveals on hover and remains draggable.
    #[must_use]
    pub fn scrollbar_thumb_visibility(&self) -> bool {
        self.state.borrow().scrollbar_thumb_visibility
    }

    /// Controls whether the scrollbar thumb is always visible. The default is
    /// hover-reveal, matching compact desktop controls.
    pub fn set_scrollbar_thumb_visibility(&self, visible: bool) -> bool {
        let mut state = self.state.borrow_mut();
        if state.scrollbar_thumb_visibility == visible {
            return false;
        }
        state.scrollbar_thumb_visibility = visible;
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        true
    }
}
