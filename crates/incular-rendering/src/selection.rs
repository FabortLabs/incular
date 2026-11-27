//! Rectangle ordering and offset adjustment for selectable fragments.

use incular_core::{Offset, Rect, TextDirection};

/// The next step after handling a selection event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionResult {
    /// Continue to the next selectable in screen order.
    Next,
    /// The event lies before this selectable in screen order.
    Previous,
    /// This selectable owns the selection edge.
    End,
    /// The result cannot be decided in the current frame.
    Pending,
    /// This event does not use a selection result.
    None,
}

/// Geometry helpers for extending a selection across laid-out fragments.
pub struct SelectionUtils;

impl SelectionUtils {
    /// Classifies a point relative to the rectangle in screen order.
    #[must_use]
    pub fn get_result_based_on_rect(rect: Rect, point: Offset) -> SelectionResult {
        let (left, top, right, bottom) = edges(rect);
        if contains(point, left, top, right, bottom) {
            SelectionResult::End
        } else if point.y < top {
            SelectionResult::Previous
        } else if point.y > bottom || point.x >= right {
            SelectionResult::Next
        } else {
            SelectionResult::Previous
        }
    }

    /// Leaves interior offsets alone and snaps other points to the first or
    /// last corner in the paragraph's writing direction.
    #[must_use]
    pub fn adjust_drag_offset(rect: Rect, point: Offset, direction: TextDirection) -> Offset {
        let (left, top, right, bottom) = edges(rect);
        if contains(point, left, top, right, bottom) {
            return point;
        }
        let first = point.y <= top || (point.y <= bottom && point.x <= left);
        match (first, direction) {
            (true, TextDirection::Ltr) => Offset::new(left, top),
            (true, TextDirection::Rtl) => Offset::new(right, top),
            (false, TextDirection::Ltr) => Offset::new(right, bottom),
            (false, TextDirection::Rtl) => Offset::new(left, bottom),
        }
    }
}

fn edges(rect: Rect) -> (f32, f32, f32, f32) {
    let (left, top) = (rect.origin.x, rect.origin.y);
    (left, top, left + rect.size.width, top + rect.size.height)
}

fn contains(point: Offset, left: f32, top: f32, right: f32, bottom: f32) -> bool {
    // Flutter Rect.contains is half-open; the general Ink/hit-test rectangle
    // in Incular includes its bottom and right edges.
    point.x >= left && point.x < right && point.y >= top && point.y < bottom
}
