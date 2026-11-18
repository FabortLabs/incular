//! `reading_order_by` must produce exactly the order of Flutter's repeated
//! `_pickNext` loop, which it short-circuits for disjoint rows.

use incular_core::{Offset, Rect, Size};
use incular_gestures::reading_order_by;

/// The unoptimized loop: re-sort the remainder by top, take the band that
/// overlaps the topmost item, emit the band's left-most member.
fn reference(mut unplaced: Vec<(usize, Rect)>) -> Vec<usize> {
    let mut sorted = Vec::with_capacity(unplaced.len());
    while !unplaced.is_empty() {
        unplaced.sort_by(|left, right| {
            left.1
                .origin
                .y
                .partial_cmp(&right.1.origin.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let topmost = unplaced[0].1;
        let band: Vec<usize> = (0..unplaced.len())
            .filter(|&index| overlaps_band(unplaced[index].1, topmost))
            .collect();
        let mut pick = 0;
        if band.len() > 1 {
            let mut band = band;
            band.sort_by(|&left, &right| {
                unplaced[left]
                    .1
                    .origin
                    .x
                    .partial_cmp(&unplaced[right].1.origin.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            pick = band[0];
        }
        sorted.push(unplaced.remove(pick).0);
    }
    sorted
}

fn overlaps_band(candidate: Rect, topmost: Rect) -> bool {
    let top = candidate.origin.y.max(topmost.origin.y);
    let bottom =
        (candidate.origin.y + candidate.size.height).min(topmost.origin.y + topmost.size.height);
    candidate.size.width > 0.0 && candidate.size.height > 0.0 && top < bottom
}

/// Small deterministic generator; coarse value sets force many ties.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn pick(&mut self, values: &[f32]) -> f32 {
        values[(self.next() % values.len() as u64) as usize]
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

/// Disjoint rows (the fast path): equal tops per row, rows separated by at
/// least the tallest member, sometimes touching exactly.
fn form(rng: &mut Lcg) -> Vec<Rect> {
    let mut rects = Vec::new();
    let mut top = rng.pick(&[-20., 0., 3.5]);
    for _ in 0..1 + rng.next() % 6 {
        let mut tallest = 0.0_f32;
        for _ in 0..1 + rng.next() % 5 {
            let height = rng.pick(&[4., 10., 10., 24.]);
            tallest = tallest.max(height);
            rects.push(rect(rng.pick(&[0., 0., 40., 80., 120.]), top, 30., height));
        }
        top += tallest + rng.pick(&[0., 0., 2., 8.]);
    }
    rects
}

/// Arbitrary layouts (the general path): overlaps, ties, empty sizes and
/// occasional NaN or infinite positions. `Size` itself is always finite.
fn arbitrary(rng: &mut Lcg) -> Vec<Rect> {
    let coordinates = [-5., 0., 0., 5., 10., 10., 15., 20., 30.];
    let extents = [0., 5., 10., 10., 20., 40.];
    (0..1 + rng.next() % 9)
        .map(|_| {
            let mut x = rng.pick(&coordinates);
            let mut y = rng.pick(&coordinates);
            match rng.next() % 40 {
                0 => y = f32::NAN,
                1 => x = f32::NAN,
                2 => y = f32::INFINITY,
                _ => {}
            }
            rect(x, y, rng.pick(&extents), rng.pick(&extents))
        })
        .collect()
}

fn assert_matches(rects: Vec<Rect>) {
    let items: Vec<(usize, Rect)> = rects.into_iter().enumerate().collect();
    let expected = reference(items.clone());
    let actual: Vec<usize> = reading_order_by(items.clone(), |item| item.1)
        .into_iter()
        .map(|item| item.0)
        .collect();
    assert_eq!(actual, expected, "layout: {items:?}");
}

#[test]
fn disjoint_rows_match_the_repeated_band_pick() {
    let mut rng = Lcg(0x5eed);
    for _ in 0..20_000 {
        let mut rects = form(&mut rng);
        // Registration order is arbitrary; ties must follow it either way.
        let len = rects.len();
        for index in 0..len {
            rects.swap(index, (rng.next() % len as u64) as usize);
        }
        assert_matches(rects);
    }
}

#[test]
fn arbitrary_layouts_match_the_repeated_band_pick() {
    let mut rng = Lcg(0xf0c5);
    for _ in 0..50_000 {
        assert_matches(arbitrary(&mut rng));
    }
}
