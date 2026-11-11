//! Parse font tables once; decode outlines only for requested atlas glyphs.
//!
//! Glyph masks follow what Flutter's Skia produces on Windows, where
//! DirectWrite rasterizes glyphs:
//!
//! - TrueType outlines are hinted vertically only, at the device size
//!   (DirectWrite's natural symmetric mode; FreeType's v40 smooth
//!   interpreter with linear horizontal metrics behaves the same way).
//! - Coverage is point sampled on a 4x4 grid per pixel, so a pixel takes one
//!   of 17 levels. The text shader applies Skia's mask gamma to them.
use std::sync::Mutex;

use ab_glyph_rasterizer::{Point, Rasterizer, point};
use incular_assets::FontBytes;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, SmoothMode, Target,
};
use skrifa::{FontRef, GlyphId, MetadataProvider};

self_cell::self_cell! {
    struct SharedFont {
        owner: FontBytes,
        #[covariant]
        dependent: FontRef,
    }
}

pub(crate) struct RasterFont {
    font: SharedFont,
    /// Hinting instances by device ppem (bits), most recently used last.
    hinters: Mutex<Vec<(u32, Option<HintingInstance>)>>,
}

/// Point samples per pixel along each axis for grayscale glyph coverage.
const SAMPLES: usize = 4;
/// Retained hinting instances; each holds the font program's state for one
/// size.
const MAX_HINTERS: usize = 16;

#[derive(Default)]
pub(crate) struct RasterMetrics {
    pub width: usize,
    pub height: usize,
    pub xmin: i32,
    pub ymin: i32,
}

/// Collects outline segments in device pixels, y down, offset by the
/// subpixel phase.
struct SegmentPen {
    dx: f32,
    dy: f32,
    segments: Vec<Segment>,
    start: Point,
    current: Point,
}

#[derive(Clone, Copy)]
enum Segment {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

impl SegmentPen {
    fn at(&self, x: f32, y: f32) -> Point {
        point(x + self.dx, self.dy - y)
    }
}

impl OutlinePen for SegmentPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.at(x, y);
        self.current = self.start;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let next = self.at(x, y);
        self.segments.push(Segment::Line(self.current, next));
        self.current = next;
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let next = self.at(x, y);
        self.segments
            .push(Segment::Quad(self.current, self.at(cx0, cy0), next));
        self.current = next;
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let next = self.at(x, y);
        self.segments.push(Segment::Cubic(
            self.current,
            self.at(cx0, cy0),
            self.at(cx1, cy1),
            next,
        ));
        self.current = next;
    }
    fn close(&mut self) {
        if self.current != self.start {
            self.segments.push(Segment::Line(self.current, self.start));
        }
        self.current = self.start;
    }
}

impl RasterFont {
    pub fn new(bytes: FontBytes, face_index: u32) -> Option<Self> {
        // Keep the parsed tables tied to the original immutable allocation.
        // The cell moves safely and drops the parser before its shared owner.
        SharedFont::try_new(bytes, |bytes| FontRef::from_index(bytes, face_index))
            .ok()
            .map(|font| Self {
                font,
                hinters: Mutex::new(Vec::new()),
            })
    }

    fn outline(&self, glyph: u16, ppem: f32, pen: &mut SegmentPen) -> Option<()> {
        let font = self.font.borrow_dependent();
        let outlines = font.outline_glyphs();
        let outline = outlines.get(GlyphId::new(u32::from(glyph)))?;
        let mut hinters = self
            .hinters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let key = ppem.to_bits();
        let index = match hinters.iter().position(|(size, _)| *size == key) {
            Some(index) => index,
            None => {
                if hinters.len() >= MAX_HINTERS {
                    hinters.remove(0);
                }
                let options = HintingOptions {
                    engine: Engine::Interpreter,
                    target: Target::Smooth {
                        mode: SmoothMode::Normal,
                        symmetric_rendering: true,
                        preserve_linear_metrics: true,
                    },
                };
                let instance = HintingInstance::new(
                    &outlines,
                    Size::new(ppem),
                    LocationRef::default(),
                    options,
                )
                .ok();
                hinters.push((key, instance));
                hinters.len() - 1
            }
        };
        let entry = hinters.remove(index);
        let drawn = match &entry.1 {
            Some(instance) => outline
                .draw(DrawSettings::hinted(instance, false), pen)
                .is_ok(),
            None => false,
        };
        hinters.push(entry);
        drop(hinters);
        if !drawn {
            pen.segments.clear();
            outline
                .draw(
                    DrawSettings::unhinted(Size::new(ppem), LocationRef::default()),
                    pen,
                )
                .ok()?;
        }
        Some(())
    }

    pub fn rasterize_indexed(
        &self,
        glyph: u16,
        ppem: f32,
        phase: [u8; 2],
    ) -> Option<(RasterMetrics, Vec<u8>)> {
        let mut pen = SegmentPen {
            dx: f32::from(phase[0]) / 4.,
            dy: f32::from(phase[1]) / 4.,
            segments: Vec::new(),
            start: point(0., 0.),
            current: point(0., 0.),
        };
        self.outline(glyph, ppem, &mut pen)?;
        if pen.segments.is_empty() {
            return Some((RasterMetrics::default(), Vec::new()));
        }
        // Rasterize at SAMPLES times the size; a sub-pixel counts as inside
        // when its centre is, which an area coverage of one half matches.
        let oversample = SAMPLES as f32;
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut grow = |p: Point| {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        };
        for segment in &pen.segments {
            match *segment {
                Segment::Line(a, b) => [a, b].into_iter().for_each(&mut grow),
                Segment::Quad(a, b, c) => [a, b, c].into_iter().for_each(&mut grow),
                Segment::Cubic(a, b, c, d) => [a, b, c, d].into_iter().for_each(&mut grow),
            }
        }
        if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
            return Some((RasterMetrics::default(), Vec::new()));
        }
        let left = min_x.floor() as i32;
        let top = min_y.floor() as i32;
        let right = max_x.ceil() as i32;
        let bottom = max_y.ceil() as i32;
        let metrics = RasterMetrics {
            width: (right - left).max(0) as usize,
            height: (bottom - top).max(0) as usize,
            xmin: left,
            // Atlas placement uses a bottom bearing in upward-positive font
            // coordinates; outline bounds use downward-positive screen space.
            ymin: -bottom,
        };
        let len = metrics.width.checked_mul(metrics.height)?;
        if len == 0 {
            return Some((RasterMetrics::default(), Vec::new()));
        }
        if len > crate::MAX_GLYPH_BITMAP_BYTES {
            return None;
        }
        let fine = |p: Point| {
            point(
                (p.x - left as f32) * oversample,
                (p.y - top as f32) * oversample,
            )
        };
        let mut raster = Rasterizer::new(metrics.width * SAMPLES, metrics.height * SAMPLES);
        for segment in &pen.segments {
            match *segment {
                Segment::Line(a, b) => raster.draw_line(fine(a), fine(b)),
                Segment::Quad(a, b, c) => raster.draw_quad(fine(a), fine(b), fine(c)),
                Segment::Cubic(a, b, c, d) => {
                    raster.draw_cubic(fine(a), fine(b), fine(c), fine(d));
                }
            }
        }
        let mut counts = vec![0u8; len];
        raster.for_each_pixel_2d(|x, y, coverage| {
            if coverage >= 0.5 {
                let index = (y as usize / SAMPLES) * metrics.width + x as usize / SAMPLES;
                counts[index] += 1;
            }
        });
        let levels = (SAMPLES * SAMPLES) as f32;
        let bitmap = counts
            .into_iter()
            .map(|count| (f32::from(count) * 255. / levels).round() as u8)
            .collect();
        Some((metrics, bitmap))
    }
}
