//! Named accessibility traversal links leave physical hit-test ancestry intact.

use super::*;

type Edges = Vec<(u32, u32, ElementId)>;
type HitOrders = FastHashMap<Option<ElementId>, Vec<(u32, ElementId)>>;
const DETACHED: u32 = u32::MAX - 2;

impl WidgetTree {
    pub(super) fn graft_semantic_traversal(
        &self,
        builds: &mut Vec<Collected>,
        edges: &mut Edges,
        detached: &mut Vec<SemanticNodeId>,
    ) -> HitOrders {
        detached.clear();
        let mut hit_orders = HitOrders::default();
        if self.tracked.semantic_traversal_links.is_empty() {
            return hit_orders;
        }
        let by_element = builds
            .iter()
            .enumerate()
            .map(|(index, build)| (build.element(), index as u32))
            .collect::<FastHashMap<_, _>>();
        let mut targets = FastHashMap::<Arc<str>, Option<u32>>::default();
        for element in &self.tracked.semantic_traversal_links {
            let Some(&index) = by_element.get(element) else {
                continue;
            };
            let properties = self
                .element_live(*element, "semantic link remains live")
                .widget
                .semantic_properties();
            if let Some(target) = &properties.traversal_parent {
                // A duplicate name cannot select a deterministic parent.
                let slot = targets.entry(target.clone()).or_insert(Some(index));
                if *slot != Some(index) {
                    *slot = None;
                }
            }
        }
        let mut traversal_parent = vec![DETACHED; builds.len()];
        let mut changed_parents = FastHashSet::default();
        for &(parent, _, child) in edges.iter() {
            let properties = self
                .element_live(child, "semantic child remains live")
                .widget
                .semantic_properties();
            let new_parent = properties.traversal_child.as_ref().map_or(parent, |name| {
                targets.get(name).copied().flatten().unwrap_or(DETACHED)
            });
            traversal_parent[by_element[&child] as usize] = new_parent;
            if parent != new_parent {
                changed_parents.insert(parent);
                changed_parents.insert(new_parent);
            }
        }
        if changed_parents.is_empty() {
            return hit_orders;
        }
        let attached = attached_nodes(&traversal_parent);
        // A removed parent can already be gone from the retained arena by
        // this pass. Seed removal from each pruned member, not only from its
        // previously published parent's children.
        detached.extend(
            builds
                .iter()
                .zip(&attached)
                .filter_map(|(build, attached)| {
                    (!attached)
                        .then(|| self.semantic_ids.get(&build.element()).copied())
                        .flatten()
                }),
        );
        let element_for_parent = |index: u32| {
            if index == TOP_LEVEL {
                None
            } else {
                Some(builds[index as usize].element())
            }
        };
        for &parent in &changed_parents {
            if parent == TOP_LEVEL || (parent != DETACHED && attached[parent as usize]) {
                hit_orders.entry(element_for_parent(parent)).or_default();
            }
        }
        for &(physical_parent, sequence, child) in edges.iter() {
            if (physical_parent == TOP_LEVEL || attached[physical_parent as usize])
                && attached[by_element[&child] as usize]
                && let Some(order) = hit_orders.get_mut(&element_for_parent(physical_parent))
            {
                order.push((sequence, child));
            }
        }

        let mut relocated = vec![DETACHED; builds.len()];
        let mut next = 0;
        for (index, attached) in attached.iter().copied().enumerate() {
            if attached {
                relocated[index] = next;
                next += 1;
            }
        }
        edges.retain_mut(|(parent, _, child)| {
            let index = by_element[child] as usize;
            if !attached[index] {
                return false;
            }
            let target = traversal_parent[index];
            *parent = if target == TOP_LEVEL {
                target
            } else {
                relocated[target as usize]
            };
            true
        });
        let mut index = 0;
        builds.retain(|_| {
            let keep = attached[index];
            index += 1;
            keep
        });
        hit_orders
    }

    /// Named traversal does not rewrite the physical hit ancestry. An
    /// unresolved or cyclic branch is absent from both.
    pub(super) fn apply_grafted_hit_orders(&mut self, hit_orders: HitOrders) {
        for (parent, order) in hit_orders {
            let parent = match parent {
                Some(element) => {
                    let Some(&id) = self.semantic_ids.get(&element) else {
                        continue;
                    };
                    Some(id)
                }
                None => None,
            };
            let mapped = self.semantic_point_order.entry(parent).or_default();
            mapped.clear();
            mapped.extend(order.into_iter().filter_map(|(sequence, child)| {
                self.semantic_ids
                    .get(&child)
                    .copied()
                    .map(|id| (sequence, id))
            }));
            mapped.sort_unstable_by_key(|&(sequence, _)| sequence);
        }
    }
}

/// Each member has at most one traversal parent. Walk links iteratively,
/// promoting chains ending at a root and dropping missing or cyclic chains.
fn attached_nodes(parents: &[u32]) -> Vec<bool> {
    const VISITING: u8 = 1;
    const LINKED: u8 = 2;
    const PRUNED: u8 = 3;
    let mut state = vec![0u8; parents.len()];
    let mut path = Vec::new();
    for index in 0..parents.len() {
        if state[index] != 0 {
            continue;
        }
        let mut current = index as u32;
        while let Some(status) = state.get_mut(current as usize) {
            if *status != 0 {
                break;
            }
            *status = VISITING;
            path.push(current as usize);
            current = parents[current as usize];
        }
        let linked = current == TOP_LEVEL || state.get(current as usize) == Some(&LINKED);
        for slot in path.drain(..) {
            state[slot] = if linked { LINKED } else { PRUNED };
        }
    }
    state.into_iter().map(|status| status == LINKED).collect()
}
