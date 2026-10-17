use incular_devtools_ui::logo::{BRAND_VIOLET, covered, rasterize};

fn alpha(pixels: &[u8], size: u32, x: u32, y: u32) -> u8 {
    pixels[((y * size + x) * 4 + 3) as usize]
}

#[test]
fn raster_is_non_empty_and_uses_the_requested_colour() {
    let pixels = rasterize(64, [128, 0, 255]);
    assert_eq!(pixels.len(), 64 * 64 * 4);
    let opaque = pixels.chunks(4).filter(|px| px[3] == 255).count();
    // Two discs minus stripes and gaps still cover well over a third of the box.
    assert!(opaque > 64 * 64 / 3, "only {opaque} opaque pixels");
    assert!(
        pixels.chunks(4).any(|px| px[3] == 0),
        "corners must be clear"
    );
    assert!(pixels.chunks(4).all(|px| px[..3] == [128, 0, 255]));
    assert_eq!(
        [BRAND_VIOLET.red, BRAND_VIOLET.green, BRAND_VIOLET.blue],
        [128, 0, 255]
    );
}

#[test]
fn mark_bounds_are_symmetric_in_the_frame() {
    let size = 96;
    let pixels = rasterize(size, [0, 0, 0]);
    let (mut left, mut right, mut top, mut bottom) = (size, 0, size, 0);
    for y in 0..size {
        for x in 0..size {
            if alpha(&pixels, size, x, y) > 0 {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    // The SVG view box leaves equal margins on every side of the mark.
    let (margin_left, margin_right) = (left, size - 1 - right);
    let (margin_top, margin_bottom) = (top, size - 1 - bottom);
    assert!(
        margin_left.abs_diff(margin_right) <= 1,
        "{margin_left} vs {margin_right}"
    );
    assert!(
        margin_top.abs_diff(margin_bottom) <= 1,
        "{margin_top} vs {margin_bottom}"
    );
    assert!(
        margin_left.abs_diff(margin_top) <= 2,
        "{margin_left} vs {margin_top}"
    );
}

#[test]
fn geometry_has_the_ring_gaps_and_stripe_cutouts() {
    // Centres of both circles, offset off any stripe, are solid.
    assert!(covered(258., 204.5 - 40.));
    // The back circle's own outline ring is cut away (r = 134 from its centre).
    assert!(!covered(258. + 134., 204.5));
    // Outside both circles is empty.
    assert!(!covered(125., 70.));
    // A stripe passes through y = x + 8: a point on it inside the front circle is cut,
    // a point just beside it is solid.
    assert!(!covered(330., 338.));
    assert!(covered(330., 338. + 12.));
}
