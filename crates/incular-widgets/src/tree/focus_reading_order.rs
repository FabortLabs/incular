//! Flutter reading-order traversal: the horizontal band uses the nearest
//! common Directionality; adjacent runs then follow their own direction.

use std::cmp::Ordering;

use super::{ElementId, FocusTraversalMember, Rect, TextDirection, WidgetTree};

#[derive(Clone, Copy)]
struct DirectionScope {
    id: ElementId,
    direction: TextDirection,
}

impl WidgetTree {
    fn direction_ancestors(
        &self,
        start: Option<ElementId>,
    ) -> impl Iterator<Item = DirectionScope> + '_ {
        let mut current = start.and_then(|id| self.parent(id));
        std::iter::from_fn(move || {
            while let Some(id) = current {
                let node = self.elements.get(id.0)?;
                current = if node.environment_boundary {
                    None
                } else {
                    node.parent
                };
                if let Some(&direction) = node
                    .environment_override
                    .as_ref()
                    .and_then(|value| value.value.downcast_ref::<TextDirection>())
                {
                    return Some(DirectionScope { id, direction });
                }
            }
            None
        })
    }
}

struct Entry {
    member: FocusTraversalMember,
    bounds: Rect,
    direction: TextDirection,
    ancestors: Vec<DirectionScope>,
}

fn sort_horizontal(a: Rect, b: Rect, direction: TextDirection) -> Ordering {
    match direction {
        TextDirection::Ltr => a.origin.x.total_cmp(&b.origin.x),
        TextDirection::Rtl => right(b).total_cmp(&right(a)),
    }
}

fn right(rect: Rect) -> f32 {
    rect.origin.x + rect.size.width
}

fn overlaps_band(bounds: Rect, top: Rect) -> bool {
    bounds.size.width > 0.
        && bounds.size.height > 0.
        && bounds.origin.y.max(top.origin.y)
            < (bounds.origin.y + bounds.size.height).min(top.origin.y + top.size.height)
}

fn pick_top_band(unplaced: &[Entry]) -> usize {
    let top = unplaced[0].bounds;
    let mut band: Vec<_> = unplaced
        .iter()
        .enumerate()
        .take_while(|(_, entry)| entry.bounds.origin.y < top.origin.y + top.size.height)
        .filter(|(_, entry)| overlaps_band(entry.bounds, top))
        .collect();
    if band.len() <= 1 {
        return 0;
    }

    let (_, first) = band[0];
    let common = first
        .ancestors
        .iter()
        .find(|ancestor| {
            band.iter()
                .all(|(_, other)| other.ancestors.iter().any(|scope| scope.id == ancestor.id))
        })
        .map_or(first.direction, |scope| scope.direction);
    band.sort_by(|(_, a), (_, b)| sort_horizontal(a.bounds, b.bounds, common));

    let mut winning_run: Option<(f32, usize)> = None;
    for run in band.chunk_by(|(_, a), (_, b)| a.direction == b.direction) {
        // Compare the union of the run's bounds in the common direction;
        // choose the first member of that run using its own direction.
        let edge = match common {
            TextDirection::Ltr => run
                .iter()
                .map(|(_, item)| item.bounds.origin.x)
                .fold(f32::INFINITY, f32::min),
            TextDirection::Rtl => -run
                .iter()
                .map(|(_, item)| right(item.bounds))
                .fold(f32::NEG_INFINITY, f32::max),
        };
        if winning_run.is_none_or(|(best, _)| edge < best) {
            let (pick, _) = run
                .iter()
                .min_by(|(_, a), (_, b)| sort_horizontal(a.bounds, b.bounds, a.direction))
                .expect("a directional run is nonempty");
            winning_run = Some((edge, *pick));
        }
    }
    winning_run
        .expect("a band with multiple items has directional runs")
        .1
}

pub(super) fn sort_members(tree: &WidgetTree, members: &mut Vec<FocusTraversalMember>) {
    let fallback = tree.environment.text_direction;
    let direction = |member: &FocusTraversalMember| {
        tree.direction_ancestors(member.direction_owner())
            .next()
            .map_or(fallback, |scope| scope.direction)
    };
    let Some(first) = members.first() else {
        return;
    };
    if members.len() == 1 {
        return;
    }

    // Monodirectional runs retain the disjoint-row fast path, including
    // large focusable lists in virtualized viewports.
    let first_direction = direction(first);
    if members
        .iter()
        .skip(1)
        .all(|member| direction(member) == first_direction)
    {
        *members = incular_gestures::reading_order_by(std::mem::take(members), |member| {
            let mut bounds = member.representative_bounds();
            if first_direction == TextDirection::Rtl {
                bounds.origin.x = -right(bounds);
            }
            bounds
        });
        return;
    }

    let mut unplaced: Vec<_> = std::mem::take(members)
        .into_iter()
        .map(|member| {
            let bounds = member.representative_bounds();
            let ancestors: Vec<_> = tree.direction_ancestors(member.direction_owner()).collect();
            let direction = ancestors.first().map_or(fallback, |scope| scope.direction);
            Entry {
                member,
                bounds,
                direction,
                ancestors,
            }
        })
        .collect();
    unplaced.sort_by(|a, b| a.bounds.origin.y.total_cmp(&b.bounds.origin.y));
    members.reserve(unplaced.len());
    while !unplaced.is_empty() {
        let pick = pick_top_band(&unplaced);
        members.push(unplaced.remove(pick).member);
    }
}
