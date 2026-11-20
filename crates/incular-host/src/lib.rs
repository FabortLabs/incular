//! Shell-independent host core for Incular.
//!
//! Every native shell drives the runtime the same way: one [`HostSurface`] per
//! presented window owns the WGPU renderer, the window metrics it was laid out
//! with, and the [`PresentationRetry`] pacing for skipped frames. The Winit
//! shell (`incular-desktop`, used by desktop and the web) and the mobile
//! embedder shell (`incular-mobile`, used by Android and iOS) differ only in
//! where events, surfaces and frame ticks come from.
//!
//! Nothing here touches Winit, raw window handles or OS APIs. Shells hand in
//! an [`incular_wgpu::WindowSurfaceTarget`] and portable runtime events.

mod retry;
mod surface;

pub use retry::PresentationRetry;
pub use surface::{HostSurface, PresentOutcome, ResizeOutcome};

use incular_platform::PhysicalSize;
use incular_runtime::{GpuResourceSummary, RenderFrameMetrics};
use incular_wgpu::{
    RenderStats, RendererError, SharedGpuContext, WgpuRenderer, WindowSurfaceTarget,
};

/// Converts renderer statistics into the runtime's frame metrics.
#[must_use]
pub fn render_metrics(stats: &RenderStats) -> RenderFrameMetrics {
    RenderFrameMetrics {
        draw_calls: stats.draw_calls,
        instances: stats.total_instances(),
        render_passes: stats.render_passes,
        path_triangles: stats.path_triangles,
        upload_bytes: stats.upload_bytes,
        texture_upload_bytes: stats.texture_upload_bytes,
        pipelines_created: stats.pipelines_created,
        queue_submissions: stats.queue_submissions,
        prepare_us: stats.prepare_us,
        encode_us: stats.encode_us,
        submit_us: stats.submit_us,
        ..RenderFrameMetrics::default()
    }
}

/// Creates a renderer for `target`, starting the shared device context on
/// first use. Native shells resolve this inline with a blocking executor;
/// browsers resolve adapters and devices on later page tasks.
///
/// # Errors
///
/// Returns the renderer error when no adapter, device or surface satisfies
/// the requested transparency mode.
pub async fn create_renderer(
    shared: Option<SharedGpuContext>,
    target: WindowSurfaceTarget,
    size: PhysicalSize,
    transparency_mode: incular_config::TransparencyMode,
    background_color: incular_core::Color,
) -> Result<(WgpuRenderer, SharedGpuContext), RendererError> {
    let shared = match shared {
        Some(shared) => shared,
        None => SharedGpuContext::new(target.clone(), transparency_mode).await?,
    };
    let renderer = shared
        .create_renderer(target, size, transparency_mode, background_color)
        .await?;
    Ok((renderer, shared))
}

/// Answers a runtime GPU-resource query from one renderer and the shared
/// device context it renders with.
#[must_use]
pub fn gpu_resource_summary(
    renderer: &WgpuRenderer,
    shared: Option<&SharedGpuContext>,
) -> GpuResourceSummary {
    let counters = renderer.counters();
    let diagnostics = shared.map(SharedGpuContext::diagnostics);
    let allocator = shared.and_then(SharedGpuContext::allocator_report);
    GpuResourceSummary {
        window_stencil_creations: counters.stencil_texture_creations,
        window_stencil_recreations: counters.stencil_texture_recreations,
        allocator_live_bytes: allocator
            .as_ref()
            .map(|report| report.total_allocated_bytes),
        allocator_reserved_bytes: allocator.as_ref().map(|report| report.total_reserved_bytes),
        shared_image_entries: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.shared_image_resources as u64)
            .unwrap_or_default(),
        shared_image_evictions: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.shared_image_evictions)
            .unwrap_or_default(),
        shared_gradient_entries: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.shared_gradient_resources as u64)
            .unwrap_or_default(),
        shared_gradient_evictions: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.shared_gradient_evictions)
            .unwrap_or_default(),
        glyph_live_pages: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.glyph_atlas_pages as u64)
            .unwrap_or_default(),
        glyph_page_evictions: diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.glyph_page_evictions)
            .unwrap_or_default(),
        glyphs_rasterized: counters.glyphs_rasterized,
        backdrop_filters_rendered: counters.backdrop_results_rendered,
        backdrop_filters_reused: counters.backdrop_results_reused,
        local_image_entries: counters.local_image_entries,
        local_gradient_entries: counters.local_gradient_entries,
        local_glyph_pages: counters.local_glyph_page_bindings,
    }
}
