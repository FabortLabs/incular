//! Reuse of backdrop-filter results across frames.
//!
//! A backdrop stage filters the part of its target painted before it. Run
//! naively it costs a capture copy, the filter passes and a split of the
//! target's render pass on every frame the window repaints, even when the
//! repaint was a spinner in another corner. This module names everything a
//! stage's result depends on in one signature; a stage whose signature is
//! the one its retained result was produced from draws that result again
//! and costs nothing.
//!
//! The signature covers, in painter order, each draw before the stage that
//! can touch the pixels the stage reads: its instance data, its clip, and
//! the identity of every texture it samples. Images and gradients are
//! immutable under their ids, glyph pages under their generation, and
//! retained layer textures under the revision stamped when they were
//! rendered. Clip masks are always included, since a mask anywhere changes
//! what later draws cover. A draw entirely outside the pixels the stage
//! reads cannot change them and is left out, which is what lets a small
//! animation elsewhere in the window leave the glass above it alone.

#[cfg(incular_d3d11)]
use crate::d3d11 as wgpu;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use super::*;

/// Device pixels added around a draw's computed bounds: the analytic edge
/// ramp of a rounded rectangle and float rounding both stay inside it.
const BOUNDS_PAD: f32 = 3.;

/// What the destination path does at one `Backdrop` batch.
#[derive(Clone, Copy, Debug)]
pub(super) struct BackdropPlan {
    /// Identity of everything the stage's result depends on.
    pub(super) signature: u64,
    /// The retained result was produced from this signature.
    pub(super) reuse: bool,
    /// The stage copies the target: it takes the capture and some stage
    /// reading that capture has to run.
    pub(super) capture: bool,
}

impl BackdropPlan {
    /// The stage neither reads nor writes anything this frame.
    pub(super) const fn is_idle(self) -> bool {
        self.reuse && !self.capture
    }
}

impl WgpuRenderer {
    /// Plans every `Backdrop` batch of `batches`, in order. `clear` is what
    /// the target holds before the first batch.
    pub(super) fn plan_backdrops(
        &self,
        batches: &[DrawBatch],
        scale: f32,
        width: u32,
        height: u32,
        clear: wgpu::Color,
    ) -> Vec<BackdropPlan> {
        if !batches
            .iter()
            .any(|batch| matches!(batch, DrawBatch::Backdrop { .. }))
        {
            return Vec::new();
        }
        let glyph_generations = self.glyph_page_generations(batches);
        // Signatures of the stages planned so far: a later stage reading
        // an earlier one's composite depends on what that stage shows.
        let mut signatures = HashMap::new();
        let mut capture_ends = HashMap::new();
        let mut capturers = Vec::new();
        let mut plans = Vec::new();
        for (index, batch) in batches.iter().enumerate() {
            let DrawBatch::Backdrop {
                layer,
                sample,
                op,
                region,
            } = batch
            else {
                continue;
            };
            // A shared capture holds the target as it was when the first
            // stage of its key ran, not as it is here.
            let (capturer, end) = match sample {
                incular_painting::BackdropSample::Capture => {
                    capture_ends.insert(*layer, index);
                    (*layer, index)
                }
                incular_painting::BackdropSample::Shared(first) => {
                    (*first, capture_ends.get(first).copied().unwrap_or(index))
                }
            };
            let mut hasher = DefaultHasher::new();
            (width, height, scale.to_bits(), self.device_generation).hash(&mut hasher);
            [clear.r, clear.g, clear.b, clear.a]
                .map(f64::to_bits)
                .hash(&mut hasher);
            // The filter's parameters are small plain values; their debug
            // form distinguishes every one of them.
            format!("{op:?}").hash(&mut hasher);
            hash_pixel_rect(&mut hasher, region.input);
            hash_pixel_rect(&mut hasher, region.output);
            let source = BackdropSource {
                region: region.input,
                scale,
                width,
                height,
                signatures: &signatures,
                glyph_generations: &glyph_generations,
            };
            for painted in &batches[..end] {
                self.hash_backdrop_source(&mut hasher, painted, &source);
            }
            let signature = hasher.finish();
            let reuse = self.effect_cache.get(layer).is_some_and(|entry| {
                entry.backdrop_signature == Some(signature)
                    && entry.device_generation == self.device_generation
            });
            signatures.insert(*layer, signature);
            capturers.push(capturer);
            plans.push(BackdropPlan {
                signature,
                reuse,
                capture: false,
            });
        }
        let needed: Vec<_> = plans
            .iter()
            .zip(&capturers)
            .filter(|(plan, _)| !plan.reuse)
            .map(|(_, capturer)| *capturer)
            .collect();
        let mut stages = batches.iter().filter_map(|batch| match batch {
            DrawBatch::Backdrop { layer, sample, .. } => Some((*layer, *sample)),
            _ => None,
        });
        for plan in &mut plans {
            let Some((layer, sample)) = stages.next() else {
                break;
            };
            plan.capture =
                sample == incular_painting::BackdropSample::Capture && needed.contains(&layer);
        }
        plans
    }

    /// The atlas generation of every glyph page `batches` draw from. A page
    /// keeps its texels until it is retired, which advances its generation.
    fn glyph_page_generations(&self, batches: &[DrawBatch]) -> HashMap<u16, Option<u64>> {
        let mut pages: HashMap<u16, Option<u64>> = batches
            .iter()
            .filter_map(|batch| match batch {
                DrawBatch::Glyphs { page, .. } => Some((*page, None)),
                _ => None,
            })
            .collect();
        if pages.is_empty() {
            return pages;
        }
        let shared = self
            .shared
            .inner
            .resources
            .lock()
            .expect("shared resource lock");
        for (page, generation) in &mut pages {
            *generation = shared.glyph_atlas.page_generation(*page);
        }
        pages
    }

    /// Adds to `hasher` what `batch` contributes to the pixels of
    /// `source.region`: nothing when it cannot touch them.
    fn hash_backdrop_source(
        &self,
        hasher: &mut DefaultHasher,
        batch: &DrawBatch,
        source: &BackdropSource<'_>,
    ) {
        let region = source.region;
        // A draw is confined to its clip's scissor.
        let visible = |clip: ClipState| {
            clip_device_region(clip, source.scale, source.width, source.height)
                .is_some_and(|scissor| pixel_rects_touch(scissor, region))
        };
        match batch {
            DrawBatch::Rectangles { clip, instances } if visible(*clip) => {
                let mut header = Header::new(0, *clip);
                for instance in instances {
                    let rect = instance.rect;
                    let bounds = [
                        rect.origin.x * source.scale,
                        rect.origin.y * source.scale,
                        (rect.origin.x + rect.size.width) * source.scale,
                        (rect.origin.y + rect.size.height) * source.scale,
                    ];
                    if bounds_touch(bounds, region) {
                        header.write(hasher);
                        hasher.write(bytemuck::bytes_of(&logical_instance(
                            *instance,
                            source.width as f32,
                            source.height as f32,
                            source.scale,
                        )));
                    }
                }
            }
            DrawBatch::Glyphs {
                page,
                clip,
                instances,
            } if visible(*clip) => {
                let mut header = Header::new(1, *clip);
                for instance in instances {
                    let bounds = quad_bounds(instance.rect, instance.affine, instance.translation);
                    if bounds_touch(bounds, region) {
                        if header.write(hasher) {
                            page.hash(hasher);
                            source.glyph_generations.get(page).hash(hasher);
                        }
                        hasher.write(bytemuck::bytes_of(instance));
                    }
                }
            }
            DrawBatch::Images {
                image,
                sampling,
                clip,
                instances,
            } if visible(*clip) => {
                let mut header = Header::new(2, *clip);
                for instance in instances {
                    let bounds = quad_bounds(instance.rect, instance.affine, instance.translation);
                    if bounds_touch(bounds, region) {
                        if header.write(hasher) {
                            image.hash(hasher);
                            sampling.hash(hasher);
                        }
                        hasher.write(bytemuck::bytes_of(instance));
                    }
                }
            }
            DrawBatch::RoundedRects {
                clip,
                gradient,
                instances,
            } if visible(*clip) => {
                let mut header = Header::new(3, *clip);
                for instance in instances {
                    let bounds = quad_bounds(instance.rect, instance.affine, instance.translation);
                    if bounds_touch(bounds, region) {
                        if header.write(hasher) {
                            gradient.hash(hasher);
                        }
                        hasher.write(bytemuck::bytes_of(instance));
                    }
                }
            }
            // A path's extent is not known here: it always counts.
            DrawBatch::Path {
                clip,
                key,
                gradient,
                instance,
            } if visible(*clip) => {
                Header::new(4, *clip).write(hasher);
                key.hash(hasher);
                gradient.hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            // A mask changes what every draw up to its pop covers.
            DrawBatch::StencilRRect {
                clip,
                instance,
                increment,
            } => {
                Header::new(5, *clip).write(hasher);
                increment.hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            DrawBatch::StencilPath {
                clip,
                key,
                instance,
                increment,
            } => {
                Header::new(6, *clip).write(hasher);
                increment.hash(hasher);
                key.hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            DrawBatch::Offscreen {
                clip,
                layer,
                instance,
            } if visible(*clip) && composite_touches(instance, source) => {
                Header::new(7, *clip).write(hasher);
                layer.hash(hasher);
                self.offscreen_cache
                    .get(layer)
                    .map(|entry| entry.revision)
                    .hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            DrawBatch::Filtered {
                clip,
                layer,
                instance,
            } if visible(*clip) && composite_touches(instance, source) => {
                Header::new(8, *clip).write(hasher);
                layer.hash(hasher);
                // A stage of this target shows what its signature names,
                // whether or not it runs this frame.
                if let Some(signature) = source.signatures.get(layer) {
                    (true, signature).hash(hasher);
                } else {
                    let revision = self.effect_cache.get(layer).map(|entry| entry.revision);
                    (false, revision).hash(hasher);
                }
                hasher.write(bytemuck::bytes_of(instance));
            }
            DrawBatch::Shadow {
                clip,
                layer,
                instance,
            } if visible(*clip) && composite_touches(instance, source) => {
                Header::new(9, *clip).write(hasher);
                layer.hash(hasher);
                self.effect_cache
                    .get(layer)
                    .map(|entry| entry.revision)
                    .or_else(|| self.offscreen_cache.get(layer).map(|entry| entry.revision))
                    .hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            DrawBatch::Blend {
                clip,
                layer,
                source: blend_source,
                mode,
                instance,
            } if visible(*clip) && composite_touches(instance, source) => {
                Header::new(10, *clip).write(hasher);
                layer.hash(hasher);
                mode.code().hash(hasher);
                match blend_source {
                    BlendSource::Group => (
                        0_u8,
                        self.offscreen_cache.get(layer).map(|entry| entry.revision),
                    ),
                    BlendSource::Effect => (
                        1_u8,
                        self.effect_cache.get(layer).map(|entry| entry.revision),
                    ),
                }
                .hash(hasher);
                hasher.write(bytemuck::bytes_of(instance));
            }
            // Clipped away, outside the region, or a stage (which paints
            // through its `Filtered` composite).
            _ => {}
        }
    }
}

/// The pixels a stage reads and what identifies the textures drawn there.
struct BackdropSource<'a> {
    region: PixelRect,
    scale: f32,
    width: u32,
    height: u32,
    signatures: &'a HashMap<incular_painting::LayerId, u64>,
    glyph_generations: &'a HashMap<u16, Option<u64>>,
}

/// A batch's kind and clip, written ahead of its first contributing draw so
/// a batch that contributes nothing leaves no trace.
struct Header {
    kind: u8,
    clip: ClipState,
    written: bool,
}

impl Header {
    const fn new(kind: u8, clip: ClipState) -> Self {
        Self {
            kind,
            clip,
            written: false,
        }
    }

    /// Writes the header once; true the time it is written.
    fn write(&mut self, hasher: &mut DefaultHasher) -> bool {
        if self.written {
            return false;
        }
        self.written = true;
        self.kind.hash(hasher);
        hash_clip(hasher, self.clip);
        true
    }
}

fn hash_clip(hasher: &mut DefaultHasher, clip: ClipState) {
    let rect_bits = |clip: ClipRect| {
        [
            clip.rect.origin.x,
            clip.rect.origin.y,
            clip.rect.size.width,
            clip.rect.size.height,
        ]
        .map(f32::to_bits)
    };
    match clip {
        ClipState::Unbounded => 0_u8.hash(hasher),
        ClipState::Rect(clip) => (1_u8, rect_bits(clip)).hash(hasher),
        ClipState::Stencil { rect, depth } => (2_u8, depth, rect.map(rect_bits)).hash(hasher),
        ClipState::Empty => 3_u8.hash(hasher),
    }
}

fn hash_pixel_rect(hasher: &mut DefaultHasher, rect: PixelRect) {
    (rect.x, rect.y, rect.width, rect.height).hash(hasher);
}

fn pixel_rects_touch(a: PixelRect, b: PixelRect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Whether device bounds `[left, top, right, bottom]`, padded, reach into
/// `region`. Bounds that are not numbers count as reaching it.
pub(crate) fn bounds_touch(bounds: [f32; 4], region: PixelRect) -> bool {
    let [left, top, right, bottom] = bounds;
    !(right + BOUNDS_PAD < region.x as f32
        || left - BOUNDS_PAD > region.right() as f32
        || bottom + BOUNDS_PAD < region.y as f32
        || top - BOUNDS_PAD > region.bottom() as f32)
}

/// Device bounds of the quad the rounded-rectangle, image and text shaders
/// place: local `rect` through the physical `affine` and `translation`.
pub(crate) fn quad_bounds(rect: [f32; 4], affine: [f32; 4], translation: [f32; 4]) -> [f32; 4] {
    let [x, y, width, height] = rect;
    let [a, b, c, d] = affine;
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut unordered = false;
    for (local_x, local_y) in [
        (x, y),
        (x + width, y),
        (x, y + height),
        (x + width, y + height),
    ] {
        let device_x = a * local_x + c * local_y + translation[0];
        let device_y = b * local_x + d * local_y + translation[1];
        unordered |= device_x.is_nan() || device_y.is_nan();
        bounds[0] = bounds[0].min(device_x);
        bounds[1] = bounds[1].min(device_y);
        bounds[2] = bounds[2].max(device_x);
        bounds[3] = bounds[3].max(device_y);
    }
    if unordered { [f32::NAN; 4] } else { bounds }
}

/// Whether a composite's quad, placed in clip space, reaches the region.
fn composite_touches(instance: &GpuCompositeInstance, source: &BackdropSource<'_>) -> bool {
    let [x, y, width, height] = instance.rect;
    let device_x = |ndc: f32| (ndc + 1.) * 0.5 * source.width as f32;
    let device_y = |ndc: f32| (1. - ndc) * 0.5 * source.height as f32;
    let (x0, x1) = (device_x(x), device_x(x + width));
    let (y0, y1) = (device_y(y), device_y(y + height));
    if x0.is_nan() || x1.is_nan() || y0.is_nan() || y1.is_nan() {
        return true;
    }
    bounds_touch(
        [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)],
        source.region,
    )
}
