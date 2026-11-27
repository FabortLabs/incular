//! Logical and physical bounds for a retained rendering view.

use crate::{Constraints, ViewConstraints};
use incular_core::{Matrix4, Size};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ViewConfigurationError {
    #[error("the device pixel ratio must be positive and finite")]
    InvalidPixelRatio,
    #[error("the view constraints cannot be represented by layout bounds")]
    InvalidBounds,
}

/// The layout and presentation bounds of a render view. The pixel ratio is
/// kept at host precision, independently of the layout engine's f32 boxes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewConfiguration {
    logical_constraints: Constraints,
    physical_constraints: Constraints,
    device_pixel_ratio: f64,
}

impl ViewConfiguration {
    /// Creates distinct logical and physical bounds.
    /// # Errors
    /// The pixel ratio must be positive and finite.
    pub fn new(
        logical_constraints: Constraints,
        physical_constraints: Constraints,
        device_pixel_ratio: f64,
    ) -> Result<Self, ViewConfigurationError> {
        if !device_pixel_ratio.is_finite() || device_pixel_ratio <= 0. {
            return Err(ViewConfigurationError::InvalidPixelRatio);
        }
        Ok(Self {
            logical_constraints,
            physical_constraints,
            device_pixel_ratio,
        })
    }

    /// Derives logical bounds from a host's physical view constraints.
    /// # Errors
    /// Rejects a malformed pixel ratio, invalid bounds, or loss of any
    /// finite boundary to overflow in the logical layout coordinates.
    pub fn from_view_constraints(
        physical: ViewConstraints,
        device_pixel_ratio: f64,
    ) -> Result<Self, ViewConfigurationError> {
        if !device_pixel_ratio.is_finite() || device_pixel_ratio <= 0. {
            return Err(ViewConfigurationError::InvalidPixelRatio);
        }
        let scale = |value: f32| {
            let scaled = (f64::from(value) / device_pixel_ratio) as f32;
            if value.is_finite() && !scaled.is_finite() {
                Err(ViewConfigurationError::InvalidBounds)
            } else {
                Ok(scaled)
            }
        };
        let physical_constraints =
            Constraints::try_from(physical).map_err(|_| ViewConfigurationError::InvalidBounds)?;
        let logical_constraints = Constraints::try_new(
            scale(physical.min_width)?,
            scale(physical.max_width)?,
            scale(physical.min_height)?,
            scale(physical.max_height)?,
        )
        .map_err(|_| ViewConfigurationError::InvalidBounds)?;
        Self::new(
            logical_constraints,
            physical_constraints,
            device_pixel_ratio,
        )
    }

    #[must_use]
    pub const fn logical_constraints(self) -> Constraints {
        self.logical_constraints
    }

    #[must_use]
    pub const fn physical_constraints(self) -> Constraints {
        self.physical_constraints
    }

    #[must_use]
    pub const fn device_pixel_ratio(self) -> f64 {
        self.device_pixel_ratio
    }

    /// The logical-to-physical transform; depth is not scaled.
    #[must_use]
    pub const fn to_matrix(self) -> Matrix4 {
        Matrix4::scale(self.device_pixel_ratio, self.device_pixel_ratio, 1.)
    }

    /// Bounds alone do not require replacing a scale transform.
    #[must_use]
    pub fn should_update_matrix(self, previous: Self) -> bool {
        self.device_pixel_ratio != previous.device_pixel_ratio
    }

    /// Scales the measured logical size, then clamps it to the physical
    /// constraints. Returns no size if unbounded scaling overflows.
    #[must_use]
    pub fn to_physical_size(self, logical: Size) -> Option<Size> {
        let physical = self.physical_constraints;
        let scaled = |value, minimum, maximum| {
            (f64::from(value) * self.device_pixel_ratio)
                .clamp(f64::from(minimum), f64::from(maximum)) as f32
        };
        Size::maybe_new(
            scaled(logical.width, physical.min_width(), physical.max_width()),
            scaled(logical.height, physical.min_height(), physical.max_height()),
        )
    }
}

impl Default for ViewConfiguration {
    fn default() -> Self {
        Self {
            logical_constraints: Constraints::tight(Size::ZERO),
            physical_constraints: Constraints::tight(Size::ZERO),
            device_pixel_ratio: 1.,
        }
    }
}

impl Hash for ViewConfiguration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.logical_constraints.hash(state);
        self.physical_constraints.hash(state);
        state.write_u64(self.device_pixel_ratio.to_bits());
    }
}
