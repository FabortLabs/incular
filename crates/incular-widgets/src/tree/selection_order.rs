//! Spatial order for selectable siblings, independent of their widget order.

use std::cmp::Ordering;

use incular_core::Rect;

use super::ElementId;

#[derive(Clone, Copy)]
pub(super) struct ScreenSelectable {
    pub id: ElementId,
    pub bounds: Rect,
}

fn compare(a: Rect, b: Rect) -> Ordering {
    // Flutter 3.50 MultiSelectableSelectionContainerDelegate.
    const VERTICAL_SLOP: f32 = 3.;
    const EPSILON: f32 = 1e-10;
    let (top_a, top_b) = (a.origin.y, b.origin.y);
    let (bottom_a, bottom_b) = (top_a + a.size.height, top_b + b.size.height);
    let overlaps = (top_a - top_b < VERTICAL_SLOP && bottom_a - bottom_b > -VERTICAL_SLOP)
        || (top_b - top_a < VERTICAL_SLOP && bottom_b - bottom_a > -VERTICAL_SLOP);
    if !overlaps {
        return if (top_a - top_b).abs() > VERTICAL_SLOP {
            top_a.total_cmp(&top_b)
        } else {
            bottom_a.total_cmp(&bottom_b)
        };
    }
    let (left_a, left_b) = (a.origin.x, b.origin.x);
    let (right_a, right_b) = (left_a + a.size.width, left_b + b.size.width);
    if left_a == left_b && right_a == right_b {
        return Ordering::Equal;
    }
    if left_a - left_b < EPSILON && right_a - right_b > -EPSILON {
        Ordering::Less
    } else if left_b - left_a < EPSILON && right_b - right_a > -EPSILON {
        Ordering::Greater
    } else if (left_a - left_b).abs() > EPSILON {
        left_a.total_cmp(&left_b)
    } else {
        right_a.total_cmp(&right_b)
    }
}

pub(super) fn sort_in_screen_order(items: &mut [ScreenSelectable]) {
    // Most retained Column and Row lists already have the right ordering.
    if items
        .windows(2)
        .all(|pair| compare(pair[0].bounds, pair[1].bounds) != Ordering::Greater)
    {
        return;
    }

    // Overlapping boxes can be pairwise incomparable as rows (one tall
    // block can overlap two disjoint rows). A stable merge keeps geometry
    // ties in declaration order without relying on a total-order sort.
    let mut merged = items.to_vec();
    let mut width = 1usize;
    while width < items.len() {
        let step = width.saturating_mul(2);
        for start in (0..items.len()).step_by(step) {
            let middle = start.saturating_add(width).min(items.len());
            let end = start.saturating_add(step).min(items.len());
            let (mut a, mut b) = (start, middle);
            for slot in &mut merged[start..end] {
                if b == end
                    || (a < middle
                        && compare(items[a].bounds, items[b].bounds) != Ordering::Greater)
                {
                    *slot = items[a];
                    a += 1;
                } else {
                    *slot = items[b];
                    b += 1;
                }
            }
        }
        items.copy_from_slice(&merged);
        width = step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incular_core::{Offset, Size};

    fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
        Rect::from_origin_size(Offset::new(left, top), Size::new(width, height))
    }

    #[test]
    fn screen_order_uses_strict_three_pixel_vertical_threshold() {
        let right_high = rect(35., 0., 20., 10.);
        assert_eq!(
            compare(rect(0., 2.99, 20., 10.), right_high),
            Ordering::Less
        );
        assert_eq!(
            compare(rect(0., 3., 20., 10.), right_high),
            Ordering::Greater
        );
        assert_eq!(
            compare(rect(0., 3.01, 20., 10.), right_high),
            Ordering::Greater
        );
    }

    #[test]
    fn screen_order_prefers_horizontal_enclosure_and_has_stable_equal_boxes() {
        let outer = rect(10., 15., 60., 12.);
        let inner = rect(25., 15., 20., 12.);
        assert_eq!(compare(outer, inner), Ordering::Less);
        assert_eq!(compare(inner, outer), Ordering::Greater);
        assert_eq!(compare(outer, outer), Ordering::Equal);
    }
}
