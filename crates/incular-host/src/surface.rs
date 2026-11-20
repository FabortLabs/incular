//! One presented window: renderer, layout metrics and retry pacing.

use crate::{PresentationRetry, render_metrics};
use incular_core::time::Instant;
use incular_platform::{PhysicalSize, WindowId, WindowMetrics};
use incular_rendering::DisplayList;
use incular_runtime::{Application, GpuSample, Screenshot};
use incular_wgpu::{FrameOutcome, RenderStats, RendererError, WgpuRenderer};

/// The renderer of one runtime window together with the metrics its content
/// was laid out for and the pacing of skipped presentations.
///
/// Keeping the three together lets [`Self::resize`] keep the surface size and
/// the reported metrics consistent, and lets [`Self::present`] settle pacing
/// and runtime bookkeeping in one place for every shell.
pub struct HostSurface {
    id: WindowId,
    renderer: WgpuRenderer,
    retry: PresentationRetry,
    metrics: WindowMetrics,
}

/// What [`HostSurface::resize`] changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResizeOutcome {
    /// The logical metrics changed; the shell must publish them to the runtime.
    pub metrics_changed: bool,
    /// The surface has a usable size and pacing was reset, so the shell
    /// should request a frame now.
    pub redraw: bool,
}

/// The settled result of one [`HostSurface::present`] call.
#[derive(Debug)]
pub enum PresentOutcome {
    /// The frame reached the screen.
    Presented(RenderStats),
    /// The renderer skipped the frame; pacing decides when to retry.
    Skipped,
    /// The renderer failed. The runtime simulation waiters were failed with
    /// the error and no automatic retry is armed.
    Failed(RendererError),
}

impl HostSurface {
    /// Wraps a renderer created for `id` at `metrics`.
    #[must_use]
    pub fn new(id: WindowId, renderer: WgpuRenderer, metrics: WindowMetrics) -> Self {
        Self {
            id,
            renderer,
            retry: PresentationRetry::new(),
            metrics,
        }
    }

    /// The runtime window this surface presents.
    #[must_use]
    pub const fn id(&self) -> WindowId {
        self.id
    }

    /// The metrics the runtime lays this window out with.
    #[must_use]
    pub const fn metrics(&self) -> WindowMetrics {
        self.metrics
    }

    /// The renderer, for capability and diagnostics queries.
    #[must_use]
    pub const fn renderer(&self) -> &WgpuRenderer {
        &self.renderer
    }

    /// The renderer, for captures, idle trimming and texture reclamation.
    pub const fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.renderer
    }

    /// The presentation retry pacing.
    #[must_use]
    pub const fn retry(&self) -> &PresentationRetry {
        &self.retry
    }

    /// The presentation retry pacing, for shell maintenance passes.
    pub const fn retry_mut(&mut self) -> &mut PresentationRetry {
        &mut self.retry
    }

    /// Applies a new native surface size and scale factor.
    pub fn resize(&mut self, size: PhysicalSize, scale_factor: f64) -> ResizeOutcome {
        let metrics = WindowMetrics::new(size, scale_factor);
        let metrics_changed = self.metrics != metrics;
        let surface_size_changed = self.renderer.physical_size() != size;
        if !metrics_changed && !surface_size_changed {
            return ResizeOutcome::default();
        }
        self.metrics = metrics;
        if surface_size_changed {
            self.renderer.resize(size);
        }
        let redraw = !size.is_zero();
        if redraw {
            // A usable size makes attempts useful again: clear any backoff
            // or dormancy so recovery is prompt.
            self.retry.note_recovered();
        }
        ResizeOutcome {
            metrics_changed,
            redraw,
        }
    }

    /// Renders `list` and settles pacing, presentation, render metrics and
    /// simulation captures with the runtime.
    pub fn present(&mut self, application: &mut Application, list: &DisplayList) -> PresentOutcome {
        let id = self.id;
        match self.renderer.render(list, self.metrics.scale_factor) {
            Ok(outcome) => {
                // Settle pacing first: skips arm a deadline (or park dormant)
                // instead of requesting an immediate redraw, so no path can
                // spin an unrestricted cycle.
                self.retry.note_outcome(&outcome, Instant::now());
                let stats = match outcome {
                    FrameOutcome::Presented(stats) => stats,
                    FrameOutcome::Skipped(_) => {
                        // Skipped attempts record no presentation and an empty
                        // metrics sample; simulator waiters stay pending for
                        // the armed attempt.
                        application.note_presented(id, false);
                        application.note_render_metrics(id, Default::default(), None);
                        application.complete_simulation_frame(id, false, None);
                        return PresentOutcome::Skipped;
                    }
                };
                application.note_presented(id, true);
                let gpu = self.renderer.gpu_frame_timings().map(|timing| GpuSample {
                    supported: true,
                    frame: timing.frame,
                    main_pass_us: timing.main_pass_us,
                });
                application.note_render_metrics(id, render_metrics(&stats), gpu);
                let capture = self.renderer.take_capture().map(|result| {
                    result.and_then(|frame| {
                        Screenshot::from_rgba8(frame.width, frame.height, frame.rgba8)
                            .map_err(|error| error.to_string())
                    })
                });
                application.complete_simulation_frame(id, true, capture);
                // Presented frames never re-request: the next frame comes from
                // normal application demand.
                PresentOutcome::Presented(stats)
            }
            Err(error) => {
                if matches!(error, RendererError::OutOfMemory) {
                    application.fail_simulation_frame(id, "renderer stopped: out of GPU memory");
                } else {
                    application.fail_simulation_frame(id, error.to_string());
                    // Terminal failure, not a skippable attempt: drop any armed
                    // retry so the same content is not re-attempted
                    // automatically. Later demand re-renders from idle.
                    self.retry.note_failed();
                }
                PresentOutcome::Failed(error)
            }
        }
    }
}
