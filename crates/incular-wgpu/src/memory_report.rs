//! Memory diagnostics: what the backend allocator holds, by resource label,
//! and what each renderer cache retains, by owner. Queried on demand (a
//! DevTools snapshot or a memory log), never per frame.

use super::prelude::*;
use super::resources::SharedGpuContext;

/// One named consumer of GPU-side memory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuMemoryEntry {
    pub name: String,
    pub bytes: u64,
    /// Resources (textures, buffers, cache entries) behind `bytes`.
    pub count: u64,
}

impl GpuMemoryEntry {
    pub(crate) fn new(name: impl Into<String>, bytes: u64, count: u64) -> Self {
        Self {
            name: name.into(),
            bytes,
            count,
        }
    }
}

/// The device allocator's view of GPU memory, shared by every window.
///
/// `allocations` is exact for the resources `wgpu` suballocates itself
/// (Direct3D 12 and Vulkan). It excludes swapchain images, driver-internal
/// memory, shader and pipeline state, and committed resources the backend
/// places outside its allocator, so the process can hold more than
/// `reserved_bytes`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuAllocatorSummary {
    /// Bytes in live allocations.
    pub allocated_bytes: u64,
    /// Bytes in the memory blocks those allocations live in, including the
    /// unused and fragmented space between them.
    pub reserved_bytes: u64,
    pub block_count: u64,
    /// Live allocations grouped by resource label, largest first.
    pub allocations: Vec<GpuMemoryEntry>,
}

impl GpuAllocatorSummary {
    /// Groups a backend allocator report by resource label.
    #[must_use]
    pub fn from_report(report: &wgpu::AllocatorReport) -> Self {
        let mut groups: HashMap<&str, (u64, u64)> = HashMap::new();
        for allocation in &report.allocations {
            let name = if allocation.name.is_empty() {
                "(unlabeled)"
            } else {
                allocation.name.as_str()
            };
            let group = groups.entry(name).or_default();
            group.0 = group.0.saturating_add(allocation.size);
            group.1 += 1;
        }
        let mut allocations: Vec<GpuMemoryEntry> = groups
            .into_iter()
            .map(|(name, (bytes, count))| GpuMemoryEntry::new(name, bytes, count))
            .collect();
        sort_largest_first(&mut allocations);
        Self {
            allocated_bytes: report.total_allocated_bytes,
            reserved_bytes: report.total_reserved_bytes,
            block_count: report.blocks.len() as u64,
            allocations,
        }
    }
}

/// Largest first; equal sizes order by name so reports are stable.
pub(crate) fn sort_largest_first(entries: &mut [GpuMemoryEntry]) {
    entries.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.name.cmp(&right.name))
    });
}

impl SharedGpuContext {
    /// The backend allocator's live allocations grouped by resource label,
    /// or `None` on backends that do not report one (Metal, OpenGL, the
    /// browser). Costs one allocator walk; call it on demand.
    #[must_use]
    pub fn allocator_summary(&self) -> Option<GpuAllocatorSummary> {
        self.allocator_report()
            .map(|report| GpuAllocatorSummary::from_report(&report))
    }

    /// What the device-owned caches shared by every window retain:
    /// image textures, gradient lookups and glyph atlas pages. Texel bytes
    /// are nominal (width x height x bytes per texel).
    #[must_use]
    pub fn shared_memory_usage(&self) -> Vec<GpuMemoryEntry> {
        let resources = self.inner.resources.lock().expect("shared resource lock");
        let glyphs = resources.glyph_atlas.memory();
        let mut entries = vec![
            GpuMemoryEntry::new(
                "Image textures",
                resources.image_textures.retained_bytes(),
                resources.image_textures.retained_entries() as u64,
            ),
            GpuMemoryEntry::new(
                "Gradient lookups",
                resources.gradient_textures.retained_bytes(),
                resources.gradient_textures.retained_entries() as u64,
            ),
            GpuMemoryEntry::new(
                "Glyph atlas pages",
                (glyphs.normal_bytes + glyphs.oversize_bytes) as u64,
                u64::from(glyphs.normal_pages) + u64::from(glyphs.oversize_pages),
            ),
        ];
        entries.retain(|entry| entry.count > 0);
        entries
    }
}

/// Decoded image pixels kept alive on the process heap by image handles
/// anywhere in the process (application caches, widgets, upload queues).
/// They stay resident after upload, next to the GPU texture.
#[must_use]
pub fn live_decoded_images() -> GpuMemoryEntry {
    let stats = incular_image::live_image_stats();
    GpuMemoryEntry::new(
        "Decoded image pixels (CPU)",
        stats.pixel_bytes,
        stats.images,
    )
}
