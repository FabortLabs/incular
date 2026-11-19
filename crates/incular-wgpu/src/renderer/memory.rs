//! What one window's renderer retains, by owner, for memory diagnostics.

use super::*;
use crate::memory_report::{GpuMemoryEntry, sort_largest_first};

/// Bytes of one `Depth24PlusStencil8` attachment texel.
const STENCIL_TEXEL_BYTES: u64 = 4;

impl WgpuRenderer {
    /// What this window's renderer retains, largest first. Texture bytes are
    /// nominal (width x height x bytes per texel) and exclude the backend's
    /// alignment and block overhead; entries marked `(CPU)` are process
    /// heap, not GPU memory. Shared device caches (images, gradients, glyph
    /// pages) are reported once by
    /// [`SharedGpuContext::shared_memory_usage`], not per window.
    #[must_use]
    pub fn memory_usage(&self) -> Vec<GpuMemoryEntry> {
        let surface_texels = u64::from(self.config.width) * u64::from(self.config.height);
        let mut entries = Vec::new();
        let mut push = |name: &str, bytes: u64, count: u64| {
            if bytes > 0 {
                entries.push(GpuMemoryEntry::new(name, bytes, count));
            }
        };
        if self.window_gpu.presentation.configured {
            // The backend keeps the frame being shown plus each queued one.
            let images = u64::from(self.config.desired_maximum_frame_latency) + 1;
            push(
                "Surface swapchain (estimate)",
                surface_texels * 4 * images,
                images,
            );
        }
        if self.window_gpu.stencil.is_some() {
            push(
                "Window clip stencil",
                surface_texels * STENCIL_TEXEL_BYTES,
                1,
            );
        }
        if let Some(target) = &self.presentation_target {
            push("Presentation target", target.bytes() as u64, 1);
        }
        if let Some(targets) = &self.destination_targets {
            push(
                "Destination blend targets",
                targets.bytes() as u64,
                1 + u64::from(targets.second.is_some()),
            );
        }
        push(
            "Retained layer groups",
            self.offscreen_cache
                .values()
                .map(|entry| entry.bytes as u64)
                .sum(),
            self.offscreen_cache.len() as u64,
        );
        push(
            "Retained effect results",
            self.effect_cache
                .values()
                .map(|entry| entry.bytes as u64)
                .sum(),
            self.effect_cache.len() as u64,
        );
        push(
            "Recycled target pool",
            self.offscreen_target_pool.bytes() as u64,
            self.offscreen_target_pool.len() as u64,
        );
        let (stencil_count, stencil_texels) = self.offscreen_stencils.usage();
        push(
            "Offscreen clip stencils",
            stencil_texels * STENCIL_TEXEL_BYTES,
            stencil_count,
        );
        push(
            "Backdrop captures",
            self.backdrop_capture_targets
                .values()
                .map(|(target, _)| target.bytes() as u64)
                .sum(),
            self.backdrop_capture_targets.len() as u64,
        );
        push(
            "Path meshes",
            self.gpu_path_cache
                .values()
                .map(|mesh| mesh.vertices.size() + mesh.indices.size())
                .sum(),
            self.gpu_path_cache.len() as u64,
        );
        push(
            "Path meshes (CPU)",
            self.cpu_path_cache
                .values()
                .map(|mesh| {
                    (mesh.vertices.capacity() * 8
                        + mesh.extrusions.capacity() * 8
                        + mesh.coverage.capacity() * 4
                        + mesh.indices.capacity() * 4) as u64
                })
                .sum(),
            self.cpu_path_cache.len() as u64,
        );
        push(
            "Instance buffers",
            [
                &self.instances,
                &self.glyph_instances,
                &self.image_instances,
                &self.rounded_rect_instances,
                &self.path_instances,
                &self.composite_instances,
            ]
            .into_iter()
            .map(wgpu::Buffer::size)
            .sum(),
            6,
        );
        push(
            "Instance staging (CPU)",
            self.instance_staging
                .iter()
                .map(|data| data.capacity() as u64)
                .sum(),
            self.instance_staging.len() as u64,
        );
        sort_largest_first(&mut entries);
        entries
    }

    /// The retained layer textures themselves (isolated groups and effect
    /// results), largest first, at most `limit`: which layers hold the
    /// offscreen memory, their pixel size and how long they have been idle.
    #[must_use]
    pub fn retained_layer_textures(&self, limit: usize) -> Vec<GpuMemoryEntry> {
        let frame = self.counters.frames;
        let groups = self.offscreen_cache.iter().map(|(layer, entry)| {
            (
                "Layer group",
                layer,
                entry.width,
                entry.height,
                entry.bytes,
                entry.last_used_frame,
            )
        });
        let effects = self.effect_cache.iter().map(|(layer, entry)| {
            (
                "Effect result",
                layer,
                entry.width,
                entry.height,
                entry.bytes,
                entry.last_used_frame,
            )
        });
        let mut entries: Vec<GpuMemoryEntry> = groups
            .chain(effects)
            .map(|(kind, layer, width, height, bytes, last_used)| {
                let idle = frame.saturating_sub(last_used);
                GpuMemoryEntry::new(
                    format!("{kind} {layer:?} {width}x{height}, idle {idle} frames"),
                    bytes as u64,
                    1,
                )
            })
            .collect();
        sort_largest_first(&mut entries);
        entries.truncate(limit);
        entries
    }
}

impl WgpuRenderer {
    /// When [`Self::trim_idle`] is due: two seconds after the last
    /// presented frame, or `None` when no frame was presented since the
    /// last trim. Hosts wake at this deadline instead of polling.
    #[must_use]
    pub fn idle_trim_deadline(&self) -> Option<Instant> {
        self.idle_trim_at
    }

    /// Releases what only an immediately following frame would have reused,
    /// for a window that has stopped rendering: free pool targets, clip
    /// stencils, destination-blend and presentation targets, backdrop
    /// captures and results (recomputed every frame), and layer textures
    /// the last frame did not draw. Layer textures the last frame drew stay,
    /// so the next frame of an unchanged scene still composites them
    /// without re-rendering; everything released here is recreated on
    /// demand. Returns the nominal bytes released.
    pub fn trim_idle(&mut self) -> usize {
        self.idle_trim_at = None;
        let last_frame = self.counters.frames.saturating_sub(1);
        let mut released = self.offscreen_target_pool.bytes();
        self.offscreen_target_pool.clear();
        let (_, stencil_texels) = self.offscreen_stencils.usage();
        released += (stencil_texels * STENCIL_TEXEL_BYTES) as usize;
        self.offscreen_stencils.clear();
        if self.window_gpu.stencil.take().is_some() {
            released += self.config.width as usize
                * self.config.height as usize
                * STENCIL_TEXEL_BYTES as usize;
        }
        if let Some(targets) = self.destination_targets.take() {
            released += targets.bytes();
            self.counters.blend_intermediate_cached_bytes = 0;
        }
        if let Some(target) = self.presentation_target.take() {
            released += target.bytes();
            self.counters.surface_present_cached_bytes = 0;
        }
        for (_, (target, _)) in self.backdrop_capture_targets.drain() {
            released += target.bytes();
        }
        let stale_groups: Vec<_> = self
            .offscreen_cache
            .iter()
            .filter(|(_, entry)| entry.last_used_frame < last_frame)
            .map(|(layer, _)| *layer)
            .collect();
        for layer in stale_groups {
            if let Some(entry) = self.offscreen_cache.remove(&layer) {
                released += entry.bytes;
                self.counters.offscreen_cached_bytes = self
                    .counters
                    .offscreen_cached_bytes
                    .saturating_sub(entry.bytes);
                self.counters.offscreen_texture_evictions += 1;
            }
        }
        let stale_effects: Vec<_> = self
            .effect_cache
            .iter()
            .filter(|(_, entry)| entry.last_used_frame < last_frame)
            .map(|(layer, _)| *layer)
            .collect();
        for layer in stale_effects {
            if let Some(entry) = self.effect_cache.remove(&layer) {
                released += entry.bytes;
                self.remove_effect_cache_bytes(entry.bytes);
                self.counters.offscreen_texture_evictions += 1;
                self.counters.effect_texture_evictions += 1;
            }
        }
        released
    }
}
