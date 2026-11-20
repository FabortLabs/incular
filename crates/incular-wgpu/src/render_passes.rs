use super::prelude::*;
use super::{BatchSpan, DrawBatch, create_stencil_attachment};

#[derive(Clone)]
pub(crate) struct OffscreenTarget {
    pub(crate) _color: wgpu::Texture,
    pub(crate) color_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: wgpu::TextureFormat,
}
/// The targets of a destination-reading composition. Backdrop filters copy
/// what they read out of `first`, so a scope with backdrops alone never
/// needs `second`; it is allocated by the first blend that samples the
/// destination and must therefore write into another texture.
pub(crate) struct DestinationTargets {
    pub(crate) first: OffscreenTarget,
    pub(crate) second: Option<OffscreenTarget>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Last frame a destination-reading blend used these targets.
    pub(crate) last_used_frame: u64,
}
impl OffscreenTarget {
    pub(crate) fn bytes(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }
}
impl DestinationTargets {
    pub(crate) fn bytes(&self) -> usize {
        self.first.bytes() + self.second.as_ref().map_or(0, OffscreenTarget::bytes)
    }

    /// The target holding the composition after rendering.
    pub(crate) fn current(&self, current_first: bool) -> &OffscreenTarget {
        if current_first {
            &self.first
        } else {
            self.second
                .as_ref()
                .expect("a flipped composition has its alternate target")
        }
    }
}

/// Creates one sampleable premultiplied scene target using the renderer's
/// target format. Ownership/counter policy stays with the caller so retained
/// effects and native presentation can share the allocation primitive without
/// conflating their diagnostics.
pub(crate) fn create_scene_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    label: &'static str,
) -> OffscreenTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let color_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    OffscreenTarget {
        _color: texture,
        color_view,
        width,
        height,
        format,
    }
}
/// Free offscreen targets waiting for a pass of the same size and format:
/// effect temporaries, backdrop captures and the targets of replaced or
/// evicted layers.
///
/// The pool is least-recently-returned first. When it is over budget the
/// oldest targets go, never the one just returned: a pool that refused new
/// targets once full of sizes nothing requests any more made every frame
/// create and destroy its temporaries. Targets nothing has taken for
/// [`OFFSCREEN_POOL_MAX_UNUSED_FRAMES`] are released as well.
pub(crate) struct OffscreenTargetPool {
    /// Free targets with the frame each was returned in, oldest first.
    free: Vec<(OffscreenTarget, u64)>,
    bytes: usize,
    max_bytes: usize,
    frame: u64,
}
impl Default for OffscreenTargetPool {
    fn default() -> Self {
        Self {
            free: Vec::new(),
            bytes: 0,
            max_bytes: Self::MIN_BUDGET_BYTES,
            frame: 0,
        }
    }
}
impl OffscreenTargetPool {
    const MIN_BUDGET_BYTES: usize = 16 * 1024 * 1024;

    /// Starts `frame` for a surface whose color target is `surface_bytes`.
    /// The budget holds at least two surface-sized targets, so a full-window
    /// layer that re-renders every frame still finds its target here on
    /// large displays.
    pub(crate) fn begin_frame(&mut self, frame: u64, surface_bytes: usize) {
        self.frame = frame;
        self.max_bytes = Self::MIN_BUDGET_BYTES.max(surface_bytes.saturating_mul(2));
        self.trim();
    }

    pub(crate) fn take(
        &mut self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Option<OffscreenTarget> {
        // The most recently returned match: older ones age out.
        let index = self.free.iter().rposition(|(target, _)| {
            target.width == width && target.height == height && target.format == format
        })?;
        let (target, _) = self.free.remove(index);
        self.bytes = self.bytes.saturating_sub(target.bytes());
        Some(target)
    }

    pub(crate) fn recycle(&mut self, target: OffscreenTarget) {
        let bytes = target.bytes();
        if bytes > self.max_bytes {
            return;
        }
        self.bytes = self.bytes.saturating_add(bytes);
        self.free.push((target, self.frame));
        self.trim();
    }

    /// Releases the oldest targets until the pool fits its budget.
    fn trim(&mut self) {
        let mut released = 0;
        while self.bytes > self.max_bytes && released < self.free.len() {
            self.bytes = self.bytes.saturating_sub(self.free[released].0.bytes());
            released += 1;
        }
        self.free.drain(..released);
    }

    /// Releases targets returned more than `max_unused_frames` ago.
    pub(crate) fn evict_unused(&mut self, frame: u64, max_unused_frames: u64) {
        let stale = self
            .free
            .iter()
            .take_while(|(_, returned)| frame.saturating_sub(*returned) > max_unused_frames)
            .count();
        for (target, _) in self.free.drain(..stale) {
            self.bytes = self.bytes.saturating_sub(target.bytes());
        }
    }

    pub(crate) fn clear(&mut self) {
        self.free.clear();
        self.bytes = 0;
    }

    pub(crate) fn len(&self) -> usize {
        self.free.len()
    }

    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }
}
pub(crate) struct OffscreenCacheEntry {
    pub(crate) target: OffscreenTarget,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) scale_factor_bits: u32,
    pub(crate) generation: u64,
    pub(crate) device_generation: u64,
    pub(crate) last_used_frame: u64,
    pub(crate) bytes: usize,
    /// Stamped when the texture was rendered: equal revisions are the same
    /// pixels.
    pub(crate) revision: u64,
}
pub(crate) struct EffectCacheEntry {
    pub(crate) target: OffscreenTarget,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) scale_factor_bits: u32,
    pub(crate) source_generation: u64,
    pub(crate) sigma_x_bits: u32,
    pub(crate) sigma_y_bits: u32,
    pub(crate) matrix_bits: [u32; 20],
    /// A color filter stage (`ColorFilterStage::encoded`) as bits.
    pub(crate) stage_bits: [u32; 6],
    pub(crate) device_generation: u64,
    pub(crate) last_used_frame: u64,
    pub(crate) bytes: usize,
    pub(crate) downsample_factor: u32,
    /// Stamped when the texture was rendered: equal revisions are the same
    /// pixels.
    pub(crate) revision: u64,
    /// For a backdrop stage's result, the signature of what it was filtered
    /// from (`plan_backdrops`).
    pub(crate) backdrop_signature: Option<u64>,
}

/// A revision no other retained texture carries.
pub(crate) fn next_texture_revision() -> u64 {
    static NEXT: incular_core::IdGenerator = incular_core::IdGenerator::new();
    NEXT.next()
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CachedSource {
    pub(crate) origin: Offset,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CachedEffect {
    pub(crate) origin: Offset,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Clip stencils for offscreen passes, shared by size. Every pass clears its
/// stencil before drawing (`LoadOp::Clear(0)`) and re-emits the masks that
/// are open at its start, so no stencil content outlives one pass. One
/// attachment per pass size therefore serves every target of that size, and
/// targets whose content has no clip masks get none at all. Without this,
/// each retained group, effect temp and pooled target carried its own
/// full-size `Depth24PlusStencil8` texture, doubling its memory.
#[derive(Default)]
pub(crate) struct StencilAttachments {
    entries: HashMap<(u32, u32), StencilAttachment>,
}
struct StencilAttachment {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    last_used_frame: u64,
}
impl StencilAttachments {
    /// A cleared-per-pass stencil of exactly `width` x `height` (render pass
    /// attachments must match the color target), and whether it was created.
    pub(crate) fn view(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        frame: u64,
    ) -> (wgpu::TextureView, bool) {
        let mut created = false;
        let entry = self
            .entries
            .entry((width.max(1), height.max(1)))
            .or_insert_with(|| {
                created = true;
                let (texture, view) = create_stencil_attachment(device, width, height);
                StencilAttachment {
                    _texture: texture,
                    view,
                    last_used_frame: frame,
                }
            });
        entry.last_used_frame = frame;
        (entry.view.clone(), created)
    }

    /// Drops attachments no pass has used for more than `max_unused_frames`.
    pub(crate) fn evict_unused(&mut self, frame: u64, max_unused_frames: u64) {
        self.entries
            .retain(|_, entry| frame.saturating_sub(entry.last_used_frame) <= max_unused_frames);
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Attachments held and their total texels, for memory diagnostics.
    pub(crate) fn usage(&self) -> (u64, u64) {
        (
            self.entries.len() as u64,
            self.entries
                .keys()
                .map(|(width, height)| u64::from(*width) * u64::from(*height))
                .sum(),
        )
    }
}

/// Whether a pass over `batches` draws clip masks, and so needs a stencil.
pub(crate) fn span_needs_stencil(batches: BatchSpan<'_>) -> bool {
    batches.iter().any(|batch| {
        matches!(
            batch,
            DrawBatch::StencilRRect { .. } | DrawBatch::StencilPath { .. }
        )
    })
}
