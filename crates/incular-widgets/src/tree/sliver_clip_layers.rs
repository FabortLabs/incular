//! A sliver clip owns one retained layer per consecutive paint scope, not
//! a clip (or offscreen isolation) per materialized row.

use super::*;
use crate::scrolling::sliver_clip::SliverClipPlacement;
use incular_core::reactivity::Subscription;
use incular_rendering::{LayerId, LayerTree};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct SliverClipLayers {
    layers: HashMap<(u64, usize), LayerId>,
    listeners: HashMap<u64, Subscription>,
}

impl SliverClipLayers {
    pub(super) fn remove_all(self, compositor: &mut LayerTree) {
        for layer in self.layers.into_values() {
            compositor.remove(layer);
        }
    }
}

struct PendingClip {
    placement: SliverClipPlacement,
    children: Vec<LayerId>,
    segment: usize,
}

fn finish(
    active: &mut Vec<PendingClip>,
    root: &mut Vec<LayerId>,
    layers: &mut SliverClipLayers,
    previous: &mut SliverClipLayers,
    compositor: &mut LayerTree,
) {
    let Some(PendingClip {
        placement,
        children,
        segment,
    }) = active.pop()
    else {
        return;
    };
    let k = (placement.key, segment);
    let layer = match placement.shape {
        crate::ClipShape::Rect(rect) => {
            if let Some(layer) = previous.layers.remove(&k) {
                compositor.update_clip(layer, rect);
                compositor.update_clip_rect_coverage(layer, placement.coverage);
                layer
            } else {
                compositor.create_clip_rect_with_coverage(rect, placement.coverage)
            }
        }
        crate::ClipShape::RRect(rrect) => {
            let rrect = incular_rendering::RRect::new(rrect.rect, rrect.radii);
            if let Some(layer) = previous.layers.remove(&k) {
                compositor.update_clip_rrect(layer, rrect);
                compositor.update_clip_rrect_coverage(layer, placement.coverage);
                layer
            } else {
                compositor.create_clip_rrect_with_coverage(rrect, placement.coverage)
            }
        }
        crate::ClipShape::Path(_) => unreachable!("sliver clips resolve a rect or rrect"),
    };
    compositor.set_children(layer, children);
    layers.layers.insert(k, layer);
    active
        .last_mut()
        .map_or(root, |p| &mut p.children)
        .push(layer);
}

impl WidgetTree {
    pub(super) fn arrange_sliver_clip_layers(
        &mut self,
        owner: ElementId,
        renders: &[RenderObjectId],
        ordered_layers: Vec<LayerId>,
    ) -> Vec<LayerId> {
        let sources = renders
            .iter()
            .filter_map(|id| {
                let node = self.renders.get(id.0)?;
                let clips = node.parent_sliver_clips.as_ref()?;
                (!node.parked).then(|| (node.object.layers.root, (clips.clone(), node.offset)))
            })
            .collect::<HashMap<_, _>>();
        let mut previous = self
            .element_live_mut(owner, "sliver clip owner")
            .scrolling_state_mut()
            .sliver_clip_layers
            .take()
            .map(|b| *b)
            .unwrap_or_default();
        if sources.is_empty() {
            previous.remove_all(&mut self.compositor);
            return ordered_layers;
        }

        let mut layers = SliverClipLayers::default();
        for (clips, _) in sources.values() {
            for clip in clips.iter() {
                let Some(source) = clip.dependency() else {
                    continue;
                };
                layers.listeners.entry(source.id()).or_insert_with(|| {
                    if let Some(existing) = previous.listeners.remove(&source.id()) {
                        return existing;
                    }
                    let updates =
                        Rc::downgrade(self.sliver_reclips.get_or_insert_with(Default::default));
                    source.subscribe(owner, move || {
                        if let Some(updates) = updates.upgrade()
                            && updates.borrow_mut().insert(owner)
                        {
                            incular_animation::schedule_frame();
                        }
                    })
                });
            }
        }
        let mut seen = HashMap::<u64, usize>::new();
        let mut root = Vec::new();
        let mut active = Vec::<PendingClip>::new();
        for layer in ordered_layers {
            let (clips, origin) = sources
                .get(&layer)
                .map_or((&[][..], Offset::ZERO), |(clips, offset)| {
                    (clips.as_ref(), *offset)
                });
            let shared = active
                .iter()
                .zip(clips)
                .take_while(|(a, b)| a.placement.key == b.key)
                .count();
            while active.len() > shared {
                finish(
                    &mut active,
                    &mut root,
                    &mut layers,
                    &mut previous,
                    &mut self.compositor,
                );
            }
            for clip in &clips[shared..] {
                let next = seen.entry(clip.key).or_default();
                let placement = clip.in_parent_space(origin);
                active.push(PendingClip {
                    placement,
                    children: Vec::new(),
                    segment: *next,
                });
                *next += 1;
            }
            active
                .last_mut()
                .map_or(&mut root, |p| &mut p.children)
                .push(layer);
        }
        while !active.is_empty() {
            finish(
                &mut active,
                &mut root,
                &mut layers,
                &mut previous,
                &mut self.compositor,
            );
        }
        previous.remove_all(&mut self.compositor);
        self.element_live_mut(owner, "sliver clip owner")
            .scrolling_state_mut()
            .sliver_clip_layers = Some(Box::new(layers));
        root
    }

    pub(super) fn has_sliver_clip_layers(&self, owner: ElementId) -> bool {
        self.elements.get(owner.0).is_some_and(|entry| {
            entry
                .scrolling_state
                .as_ref()
                .is_some_and(|s| s.sliver_clip_layers.is_some())
        })
    }
}

impl WidgetTree {
    pub(super) fn flush_sliver_reclips(&mut self) -> bool {
        let Some(updates) = &self.sliver_reclips else {
            return false;
        };
        if updates.borrow().is_empty() {
            return false;
        }
        let pending = std::mem::take(&mut *updates.borrow_mut());
        let mut changed = false;
        for owner in pending {
            if self.has_sliver_clip_layers(owner) {
                self.sync_render_children(owner);
                changed = true;
            }
        }
        if changed {
            self.input.hover_hit_revision = self.input.hover_hit_revision.wrapping_add(1);
        }
        changed
    }
}
