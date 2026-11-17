//! Compositor-only free dragging (§7.1): the drag offset is a motion value on
//! the element's translate channel, so dragging never rebuilds or relayouts.

use incular_core::{Offset, Rect};

/// Which axes a drag moves along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragAxis {
    X,
    Y,
    Both,
}

/// Where a dragged element may go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bounds {
    /// Anywhere.
    None,
    /// Offsets limited to this rectangle (relative to the resting place).
    Offsets(Rect),
    /// Within the parent's box.
    Parent,
}

/// What happens when a drag is released.
#[derive(Clone, Debug, PartialEq)]
pub enum Release {
    /// Spring back to the resting place.
    SnapBack,
    /// Coast with the release velocity, decelerating, within the bounds.
    Momentum,
    /// Spring to the nearest of these offsets.
    SnapTo(Vec<Offset>),
    /// Stop where released.
    Stay,
}

/// A free-drag configuration for [`Motion::drag`](crate::motion::Motion::drag).
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub(crate) axis: DragAxis,
    pub(crate) bounds: Bounds,
    pub(crate) elastic: f32,
    pub(crate) release: Release,
    /// Only a `reorder_handle()` inside the element starts the drag.
    pub(crate) handle_only: bool,
}

impl Drag {
    const fn axis(axis: DragAxis) -> Self {
        Self {
            axis,
            bounds: Bounds::None,
            elastic: 0.,
            release: Release::SnapBack,
            handle_only: false,
        }
    }

    /// Horizontal only.
    #[must_use]
    pub const fn x() -> Self {
        Self::axis(DragAxis::X)
    }

    /// Vertical only.
    #[must_use]
    pub const fn y() -> Self {
        Self::axis(DragAxis::Y)
    }

    /// Both axes.
    #[must_use]
    pub const fn both() -> Self {
        Self::axis(DragAxis::Both)
    }

    #[must_use]
    pub fn within(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Rubber-band past the bounds: 0 is a hard stop, 1 follows the pointer.
    #[must_use]
    pub fn elastic(mut self, elastic: f32) -> Self {
        self.elastic = if elastic.is_finite() {
            elastic.clamp(0., 1.)
        } else {
            0.
        };
        self
    }

    #[must_use]
    pub fn release(mut self, release: Release) -> Self {
        self.release = release;
        self
    }
}
