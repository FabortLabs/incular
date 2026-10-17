//! The Incular brand mark, drawn natively at any size.
//!
//! The mark (`docs/public/incular.svg`) is two overlapping circles of
//! radius 130 in a 364 x 361 view box. A thin ring gap (8 units, centred on
//! radius 134) separates the circles wherever one outline crosses the other,
//! and five 45 degree stripes (8.5 wide, 64 apart) are cut out of the front
//! circle. [`covered`] describes that shape analytically; [`mark`] paints it
//! with clip paths and [`rasterize`] samples it for window icons.

use incular::prelude::*;

/// Brand violet of the mark (`#8000FF`).
pub const BRAND_VIOLET: Color = Color::rgba(128, 0, 255, 255);

const VIEW_X: f32 = 120.;
const VIEW_Y: f32 = 66.5;
const VIEW_WIDTH: f32 = 364.;
const VIEW_HEIGHT: f32 = 361.;
const BACK: (f32, f32) = (258., 204.5);
const FRONT: (f32, f32) = (346., 289.5);
const RADIUS: f32 = 130.;
/// Ring gap around each circle outline: radii 130 to 138.
const RING_OUTER: f32 = 138.;
/// Stripes are clipped to a circle slightly larger than the front circle.
const STRIPE_CLIP: f32 = 133.;
const STRIPE_WIDTH: f32 = 8.5;
/// `y = x + c` intercepts of the five stripes.
const STRIPES: [f32; 5] = [-184., -120., -56., 8., 72.];

fn distance(x: f32, y: f32, center: (f32, f32)) -> f32 {
    ((x - center.0).powi(2) + (y - center.1).powi(2)).sqrt()
}

/// Whether the point `(x, y)`, in the SVG view box, is part of the mark.
pub fn covered(x: f32, y: f32) -> bool {
    let to_back = distance(x, y, BACK);
    let to_front = distance(x, y, FRONT);
    if to_back >= RADIUS && to_front >= RADIUS {
        return false;
    }
    // Ring gaps cut through both circles, wherever either outline passes.
    if (RADIUS..=RING_OUTER).contains(&to_back) || (RADIUS..=RING_OUTER).contains(&to_front) {
        return false;
    }
    if to_front <= STRIPE_CLIP {
        let half = STRIPE_WIDTH / 2. * std::f32::consts::SQRT_2;
        if STRIPES.iter().any(|c| (y - x - c).abs() <= half) {
            return false;
        }
    }
    true
}

/// Renders the mark as straight-alpha RGBA, `size` x `size` pixels, using
/// 4 x 4 supersampling. The mark is centred and fits the square.
pub fn rasterize(size: u32, color: [u8; 3]) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    let scale = size as f32 / VIEW_WIDTH.max(VIEW_HEIGHT);
    let top = (size as f32 - VIEW_HEIGHT * scale) / 2.;
    let left = (size as f32 - VIEW_WIDTH * scale) / 2.;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for row in 0..size {
        for column in 0..size {
            let mut hits = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = column as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
                    let py = row as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
                    let x = (px - left) / scale + VIEW_X;
                    let y = (py - top) / scale + VIEW_Y;
                    hits += u32::from(covered(x, y));
                }
            }
            pixels.extend_from_slice(&[
                color[0],
                color[1],
                color[2],
                (hits * 255 / (SAMPLES * SAMPLES)) as u8,
            ]);
        }
    }
    pixels
}

/// The mark as a native window icon (`size` x `size`, brand violet).
pub fn window_icon(size: u32) -> Option<WindowIcon> {
    WindowIcon::from_rgba(rasterize(size, [128, 0, 255]), size, size).ok()
}

type CacheKey = (u32, [u8; 4]);

thread_local! {
    static CACHE: std::cell::RefCell<Vec<(CacheKey, ImageHandle)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The mark rasterised at `pixels` x `pixels` in `color`, cached per size
/// and colour so rebuilding the header does not resample it.
fn image(pixels: u32, color: Color) -> Option<ImageHandle> {
    let key = (pixels, [color.red, color.green, color.blue, color.alpha]);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, image)) = cache.iter().find(|(existing, _)| *existing == key) {
            return Some(image.clone());
        }
        let image = ImageHandle::from_rgba8(
            pixels,
            pixels,
            rasterize(pixels, [color.red, color.green, color.blue]),
        )
        .ok()?;
        if cache.len() >= 8 {
            cache.remove(0);
        }
        cache.push((key, image.clone()));
        Some(image)
    })
}

/// The brand mark `size` logical pixels wide, painted in `color`.
///
/// It is sampled from [`covered`] at three times the logical size (enough
/// for 2x displays) and drawn as an image: the mark's cut-outs would
/// otherwise need nested even-odd clip paths.
pub fn mark(size: f32, color: Color) -> Widget {
    let height = size * VIEW_HEIGHT / VIEW_WIDTH;
    let pixels = (size * 3.).ceil().max(1.) as u32;
    let mut canvas = Canvas::default();
    if let Some(image) = image(pixels, color) {
        // The raster is square with the mark centred in it.
        let side = size.max(height);
        let source = incular::core::Rect::from_origin_size(
            Offset::ZERO,
            Size::new(pixels as f32, pixels as f32),
        );
        let destination = incular::core::Rect::from_origin_size(
            Offset::new(0., (height - side) / 2.),
            Size::new(side, side),
        );
        canvas.image(image, source, destination);
    }
    CustomPaint::new(Size::new(size, height), canvas.finish()).into()
}
