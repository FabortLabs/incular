//! Spatial defaults for sibling semantics traversal.

use super::{CoreTransform, ElementId, Offset, Size, TextDirection};

#[derive(Clone, Copy)]
pub(super) struct TraversalItem {
    pub(super) edge: (u32, u32, ElementId),
    leading: Offset,
    trailing: Offset,
    top_left: Offset,
    center: Offset,
}

impl TraversalItem {
    pub(super) fn new(
        edge: (u32, u32, ElementId),
        transform: CoreTransform,
        size: Size,
    ) -> Option<Self> {
        // Flutter deflates by 0.1 logical pixels so touching boxes fall
        // into different horizontal or vertical groups.
        let leading = transform.transform_point(Offset::new(0.1, 0.1));
        let trailing = transform.transform_point(Offset::new(size.width - 0.1, size.height - 0.1));
        let top_left = transform.transform_point(Offset::ZERO);
        let center = transform.transform_point(Offset::new(size.width * 0.5, size.height * 0.5));
        let finite = [leading, trailing, top_left, center]
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite());
        (finite && leading.x <= trailing.x && leading.y <= trailing.y).then_some(Self {
            edge,
            leading,
            trailing,
            top_left,
            center,
        })
    }
}

/// Divide siblings into vertically overlapping groups; within a row,
/// partition horizontally overlapping knots and order them by direction.
pub(super) fn order(items: &mut [TraversalItem], direction: TextDirection) {
    items.sort_unstable_by(|a, b| {
        a.leading
            .y
            .total_cmp(&b.leading.y)
            .then_with(|| a.edge.1.cmp(&b.edge.1))
    });
    let mut rest = items;
    while !rest.is_empty() {
        let end = overlap_end(rest, |n| n.leading.y, |n| n.trailing.y);
        let (row, tail) = rest.split_at_mut(end);
        order_row(row, direction);
        rest = tail;
    }
}

fn order_row(row: &mut [TraversalItem], direction: TextDirection) {
    row.sort_unstable_by(|a, b| {
        a.leading.x.total_cmp(&b.leading.x).then_with(|| {
            a.leading
                .y
                .total_cmp(&b.leading.y)
                .then_with(|| a.edge.1.cmp(&b.edge.1))
        })
    });
    let reverse = direction == TextDirection::Rtl;
    let mut rest = &mut *row;
    while !rest.is_empty() {
        let end = overlap_end(rest, |n| n.leading.x, |n| n.trailing.x);
        let (knot, tail) = rest.split_at_mut(end);
        if knot.len() > 1 {
            order_knot(knot, direction);
        }
        // A later whole-row reversal changes the order of the knots but
        // leaves the order within each knot untouched.
        if reverse {
            knot.reverse();
        }
        rest = tail;
    }
    if reverse {
        row.reverse();
    }
}

fn overlap_end(
    entries: &[TraversalItem],
    leading: impl Fn(&TraversalItem) -> f32,
    trailing: impl Fn(&TraversalItem) -> f32,
) -> usize {
    let mut end = trailing(&entries[0]);
    for (index, entry) in entries.iter().enumerate().skip(1) {
        if leading(entry) >= end {
            return index;
        }
        end = end.max(trailing(entry));
    }
    entries.len()
}

/// A knot overlaps in both axes. Flutter determines a forward successor
/// from relative centers, then orders it with a depth-first walk.
fn order_knot(knot: &mut [TraversalItem], direction: TextDirection) {
    use std::f64::consts::FRAC_PI_4;

    let mut next = vec![None; knot.len()];
    for (i, item) in knot.iter().enumerate() {
        for (j, other) in knot.iter().enumerate() {
            if i == j || next[j] == Some(i) {
                continue;
            }
            let delta = other.center - item.center;
            let angle = f64::from(delta.y).atan2(f64::from(delta.x));
            let forward = match direction {
                TextDirection::Ltr => -FRAC_PI_4 < angle && angle < 3. * FRAC_PI_4,
                TextDirection::Rtl => angle.abs() > 3. * FRAC_PI_4,
            };
            if forward {
                next[i] = Some(j);
            }
        }
    }
    let mut starts = (0..knot.len()).collect::<Vec<_>>();
    starts.sort_unstable_by(|&a, &b| {
        knot[b]
            .top_left
            .y
            .total_cmp(&knot[a].top_left.y)
            .then_with(|| {
                knot[b]
                    .top_left
                    .x
                    .total_cmp(&knot[a].top_left.x)
                    .then_with(|| a.cmp(&b))
            })
    });
    let mut visited = vec![false; knot.len()];
    let mut postorder = Vec::with_capacity(knot.len());
    // Each node has at most one successor. Walking paths iteratively
    // avoids a stack proportional to the overlap count.
    for start in starts {
        let first = postorder.len();
        let mut current = Some(start);
        while let Some(index) = current {
            if std::mem::replace(&mut visited[index], true) {
                break;
            }
            postorder.push(index);
            current = next[index];
        }
        postorder[first..].reverse();
    }
    let original = knot.to_vec();
    for (item, index) in knot.iter_mut().zip(postorder.into_iter().rev()) {
        *item = original[index];
    }
}
