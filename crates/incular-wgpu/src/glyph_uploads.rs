//! Batch sparse atlas writes into one mapped upload buffer, rather than one
//! driver allocation per glyph. Glyphs allocated side by side on an atlas
//! shelf share one copy rectangle, so small glyphs do not each pad their rows
//! to the copy alignment. Shared-context ownership lets either window flush
//! masks before drawing, including masks resolved by a failed frame.
use crate::{AtlasEntry, GLYPH_ATLAS_PADDING};
use std::ops::Range;

const MAX_PENDING_BYTES: usize = 4 * 1024 * 1024;
/// Row pitch required for buffer-to-texture copies.
const ROW_ALIGNMENT: usize = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
/// D3D12 texture-data placement alignment; aligned offsets avoid a per-copy
/// staging detour in the DX12 backend and are harmless elsewhere.
const OFFSET_ALIGNMENT: usize = 512;

/// Consecutively allocated glyphs on one atlas shelf, uploaded as a single
/// padded rectangle. Its only other pixels are allocation borders and shelf
/// space below shorter glyphs, which the shelf allocator never hands out, so
/// zeroing them cannot clobber a neighbor.
struct Run {
    texture: wgpu::Texture,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    glyphs: Range<usize>,
}

impl Run {
    fn stride(&self) -> usize {
        (self.width as usize).div_ceil(ROW_ALIGNMENT) * ROW_ALIGNMENT
    }

    fn staged_bytes(&self) -> usize {
        (self.stride() * self.height as usize).next_multiple_of(OFFSET_ALIGNMENT)
    }
}

/// A tight coverage bitmap and its content origin within its run.
struct PendingGlyph {
    x: usize,
    width: usize,
    height: usize,
    bitmap: usize,
}

#[derive(Default)]
pub(crate) struct GlyphUploads {
    bitmaps: Vec<u8>,
    glyphs: Vec<PendingGlyph>,
    runs: Vec<Run>,
    staged_bytes: usize,
}

impl GlyphUploads {
    pub(crate) fn push(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: wgpu::Texture,
        entry: AtlasEntry,
        bitmap: &[u8],
    ) {
        if entry.width == 0 || entry.height == 0 {
            return;
        }
        let padding = u32::from(GLYPH_ATLAS_PADDING);
        let left = u32::from(entry.x) - padding;
        let top = u32::from(entry.y) - padding;
        let width = u32::from(entry.width) + padding * 2;
        let height = u32::from(entry.height) + padding * 2;
        let extends = self
            .runs
            .last()
            .is_some_and(|run| run.texture == texture && run.y == top && run.x + run.width == left);
        if !extends {
            self.runs.push(Run {
                texture,
                x: left,
                y: top,
                width: 0,
                height: 0,
                glyphs: self.glyphs.len()..self.glyphs.len(),
            });
        }
        let run = self.runs.last_mut().expect("current run");
        self.staged_bytes -= run.staged_bytes();
        self.glyphs.push(PendingGlyph {
            x: (left - run.x + padding) as usize,
            width: usize::from(entry.width),
            height: usize::from(entry.height),
            bitmap: self.bitmaps.len(),
        });
        run.width = left + width - run.x;
        run.height = run.height.max(height);
        run.glyphs.end += 1;
        self.staged_bytes += run.staged_bytes();
        self.bitmaps
            .extend_from_slice(&bitmap[..usize::from(entry.width) * usize::from(entry.height)]);
        if self.staged_bytes >= MAX_PENDING_BYTES {
            self.flush(device, queue);
        }
    }

    pub(crate) fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        if self.runs.is_empty() {
            return false;
        }
        // Mapped-at-creation buffers start zeroed, which supplies every
        // border and unused shelf pixel; only coverage rows are written.
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("incular batched glyph upload"),
            size: self.staged_bytes as u64,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        let padding = usize::from(GLYPH_ATLAS_PADDING);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("incular batched glyph copies"),
        });
        {
            let mut staging = buffer
                .slice(..)
                .get_mapped_range_mut()
                .expect("new upload buffer is mapped");
            let mut offset = 0;
            for run in &self.runs {
                let stride = run.stride();
                for glyph in &self.glyphs[run.glyphs.clone()] {
                    for row in 0..glyph.height {
                        let source = glyph.bitmap + row * glyph.width;
                        let destination = offset + (row + padding) * stride + glyph.x;
                        staging
                            .slice(destination..destination + glyph.width)
                            .copy_from_slice(&self.bitmaps[source..source + glyph.width]);
                    }
                }
                encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: offset as u64,
                            bytes_per_row: Some(stride as u32),
                            rows_per_image: Some(run.height),
                        },
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &run.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: run.x,
                            y: run.y,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: run.width,
                        height: run.height,
                        depth_or_array_layers: 1,
                    },
                );
                offset += run.staged_bytes();
            }
        }
        buffer.unmap();
        queue.submit(Some(encoder.finish()));
        // No CPU staging capacity or mapped buffer remains retained at idle.
        // Submitted copies retain their GPU resources until completion.
        *self = Self::default();
        true
    }
}
