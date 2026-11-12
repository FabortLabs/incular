pub(crate) const ATLAS_PAGE_SIZE: u16 = 1024;
pub(crate) const ATLAS_PADDING: u16 = 1;
pub(crate) const OVERSIZE_PAGE_AREA_DIVISOR: u32 = 4;
/// Default device-owned budget for glyph atlas pages (8 MiB nominal at one
/// byte per texel). Constrains retained pages, not raster work: eviction
/// retires the least-recently-used page and its entries, and later uses
/// re-rasterize on demand.
pub(crate) const DEFAULT_MAX_GLYPH_ATLAS_PAGES: usize = 8;
/// Default device-owned retention for parsed rasterizer fonts. Counts
/// entries, not bytes. Each object retains font bytes and parsed tables;
/// glyph outlines are decoded transiently on atlas misses. The limit
/// bounds object count only. Eviction drops the parsed object
/// without invalidating glyph placements, atlas pages, source handles,
/// or submitted work; the next miss re-parses from retained source bytes.
pub(crate) const DEFAULT_MAX_PARSED_FONTS: usize = 8;
/// Prevent an untrusted font size from causing a multi-gigabyte CPU bitmap
/// allocation before the renderer can reject it.
pub(crate) const MAX_GLYPH_BITMAP_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_GLYPH_RASTER_PPEM: f32 = 1024.;
pub(crate) const GLYPH_ATLAS_FILTER: wgpu::FilterMode = wgpu::FilterMode::Linear;
pub(crate) const IMAGE_CACHE_MAX_UNUSED_FRAMES: u64 = 600;
pub(crate) const PATH_CACHE_MAX_UNUSED_FRAMES: u64 = 600;
pub(crate) const GRADIENT_CACHE_MAX_UNUSED_FRAMES: u64 = 600;
pub(crate) const OFFSCREEN_CACHE_MAX_UNUSED_FRAMES: u64 = 600;
/// Each normalized gradient is resampled into this compact one-dimensional
/// lookup texture. This deterministic representation supports any stop count:
/// every stop participates in the premultiplied working-space samples.
pub(crate) const GRADIENT_LUT_SAMPLES: u32 = 256;
pub(crate) const MAX_STENCIL_CLIP_DEPTH: u8 = u8::MAX;
/// Direct Gaussian kernels are capped at this radius (in the pixels the pass
/// runs at). Downsampling keeps the effective sigma near
/// [`BLUR_TARGET_SIGMA`], so only sigmas past 16 x 32 physical pixels reach
/// the cap.
pub(crate) const MAX_BLUR_RADIUS: usize = 96;
/// Bilinear tap pairs of a kernel at the radius cap, plus the centre slot.
pub(crate) const BLUR_TAP_SLOTS: usize = MAX_BLUR_RADIUS / 2 + 4;
/// Uniform floats per effect pass: one `vec4` per tap slot.
pub(crate) const BLUR_WEIGHT_SLOTS: usize = BLUR_TAP_SLOTS * 4;
/// Impeller.s `GaussianBlurFilterContents::CalculateScale`: a blur runs at
/// the power-of-two scale that brings its sigma closest to this many pixels.
pub(crate) const BLUR_TARGET_SIGMA: f32 = 4.;
/// The coarsest downsample, 1/16.
pub(crate) const MAX_BLUR_DOWNSAMPLE: u32 = 16;
/// Impeller keeps a 1/8 downsample over 1/16 while the 1/8 kernel is at most
/// this many taps wide (`kEighthDownsampleKernalWidthMax`).
pub(crate) const EIGHTH_DOWNSAMPLE_KERNEL_WIDTH_MAX: i32 = 41;
/// Parameter slots in the effect uniform ring: one per pass of a blur
/// (downsample, horizontal, vertical, upsample) with room to spare. Every
/// blur records its passes into one encoder and submits once, so the ring
/// restarts with each blur.
pub(crate) const EFFECT_PARAM_SLOTS: u64 = 8;
/// Kernel coefficients are stable across tiny animation/DPI floating-point
/// differences. Quantizing only the CPU resource key does not change the
/// effect's bounds or filtered-result key.
pub(crate) const BLUR_KERNEL_QUANTUM: f32 = 1. / 64.;
