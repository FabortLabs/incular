//! Renderer-neutral execution plan for backdrop-filter stages: whether a
//! stage runs at all, which stage captures the backdrop and which reuse a
//! capture (Flutter's `BackdropKey`), and where `ImageFilter.matrix`
//! samples the backdrop in device pixels.

use std::collections::HashMap;

use incular_core::Transform;

use crate::{BackdropKey, GaussianBlur, LayerId};

/// Whether a backdrop-filter stage changes anything: it is enabled and has
/// a matrix or a blur with a nonzero sigma. An inactive stage paints its
/// child as if the stage were not there.
#[must_use]
pub fn backdrop_filter_is_active(
    blur: GaussianBlur,
    enabled: bool,
    matrix: Option<Transform>,
) -> bool {
    enabled && (matrix.is_some() || blur.sigma_x > f32::EPSILON || blur.sigma_y > f32::EPSILON)
}

/// Where a backdrop-filter stage gets the backdrop it filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackdropSample {
    /// The stage captures what has been painted into its target so far.
    Capture,
    /// The stage filters the capture another stage with the same
    /// [`BackdropKey`] took earlier in the frame.
    Shared(LayerId),
}

/// The backdrop captures of one frame, in paint order.
///
/// Flutter's `BackdropGroup`: stages that share a [`BackdropKey`] sample one
/// backdrop, captured where the first of them paints, so a later stage does
/// not see an earlier one's output. Sharing is per render target (`scope`):
/// a stage painting into an isolated layer, such as an opacity group, reads
/// that layer, and never a capture of another target.
#[derive(Clone, Debug, Default)]
pub struct BackdropCaptures {
    captured: HashMap<(u64, BackdropKey), LayerId>,
}

impl BackdropCaptures {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets every capture, for the start of a frame.
    pub fn clear(&mut self) {
        self.captured.clear();
    }

    /// How the stage `layer` gets its backdrop when it is reached in paint
    /// order, painting into the target `scope`. An unkeyed stage always
    /// captures; the first stage of a key in a scope captures and the
    /// later ones share it.
    pub fn sample(
        &mut self,
        layer: LayerId,
        key: Option<BackdropKey>,
        scope: u64,
    ) -> BackdropSample {
        let Some(key) = key else {
            return BackdropSample::Capture;
        };
        match self.captured.get(&(scope, key)) {
            Some(&first) => BackdropSample::Shared(first),
            None => {
                self.captured.insert((scope, key), layer);
                BackdropSample::Capture
            }
        }
    }
}

/// `ImageFilter.matrix` applied to a backdrop, in device pixels: the map
/// from an output pixel position to the backdrop position it shows.
///
/// The matrix moves the backdrop in the stage's coordinates: content at a
/// point `p` of the stage moves to `matrix · p`. `stage` maps the stage's
/// coordinates to the target's logical coordinates and `scale` logical to
/// device pixels, so an output pixel `d` shows the backdrop at
/// `S · W · M⁻¹ · W⁻¹ · S⁻¹ · d`. `None` when the matrix or the stage
/// transform cannot be inverted, which leaves nothing to sample.
#[must_use]
pub fn backdrop_matrix_sampling(
    stage: Transform,
    matrix: Transform,
    scale: f32,
) -> Option<Transform> {
    let device = Transform::scale(scale);
    let stage_inverse = stage.inverse()?;
    let matrix_inverse = matrix.inverse()?;
    let device_inverse = device.inverse()?;
    Some(
        device
            .then(stage)
            .then(matrix_inverse)
            .then(stage_inverse)
            .then(device_inverse),
    )
}
