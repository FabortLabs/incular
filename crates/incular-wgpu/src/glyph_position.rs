//! Physical positioning shared by mask selection and quad placement.
use incular_core::{Offset, Transform};

pub(crate) fn device_origin(origin: Offset, transform: Transform, scale: f32) -> Option<Offset> {
    if !transform.is_translation() {
        return None;
    }
    let translation = transform.translation_offset();
    let x = (origin.x + translation.x) * scale;
    let y = (origin.y + translation.y) * scale;
    // Skia positions horizontal, axis-aligned text with subpixel precision
    // along x only: floor(x + 1/8) at quarter-pixel resolution (carrying
    // negative half-way positions toward +infinity), and y rounded to a whole
    // pixel (SkScalerContext::computeAxisAlignmentForHText).
    Some(Offset::new((x * 4. + 0.5).floor() / 4., (y + 0.5).floor()))
}

pub(crate) fn phase(origin: Option<Offset>) -> [u8; 2] {
    origin.map_or([0, 0], |p| {
        [
            ((p.x - p.x.floor()) * 4.) as u8,
            ((p.y - p.y.floor()) * 4.) as u8,
        ]
    })
}
