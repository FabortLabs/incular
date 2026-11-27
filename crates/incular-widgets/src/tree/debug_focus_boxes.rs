//! Opt-in focus diagnostics. Draw in the focus wrapper's retained foreground.

use super::*;

#[derive(Default)]
pub(super) struct DebugFocusBoxes {
    pub(super) enabled: bool,
    bindings: FastHashMap<RenderObjectId, BorderBinding>,
}

struct BorderBinding {
    node: incular_gestures::FocusNode,
    color: Color,
    _subscription: incular_gestures::FocusNodeSubscription,
}

fn debug_focus_node(kind: &WidgetKind) -> Option<&incular_gestures::FocusNode> {
    let WidgetKind::Gesture { callbacks, .. } = kind else {
        return None;
    };
    // FocusScope builds a semantics boundary, not a second debug decoration.
    (!callbacks.owns_focus_scope && callbacks.focus_scope.is_none())
        .then(|| super::focus::focus_boundary_node(kind))
        .flatten()
}

fn outline_color(node: &incular_gestures::FocusNode) -> Color {
    Color::from_argb32(if node.has_primary_focus() {
        0xF000FF00
    } else if node.has_focus() {
        0xF00000FF
    } else if !node.can_request_focus() {
        0xF0FF0000
    } else if node.skip_traversal() {
        0xF0FFFF00
    } else {
        0xF000FFFF
    })
}

impl WidgetTree {
    pub(super) fn sync_debug_focus_boxes(&mut self) {
        let Some(mut state) = self.debug_focus_boxes.take() else {
            return;
        };
        let mut dirty = Vec::new();
        if state.enabled {
            for (_, element) in self.elements.iter() {
                let Some(node) = debug_focus_node(element.widget.kind()) else {
                    continue;
                };
                let color = outline_color(node);
                if let Some(binding) = state.bindings.get_mut(&element.render)
                    && &binding.node == node
                {
                    if binding.color != color {
                        binding.color = color;
                        dirty.push(element.render);
                    }
                    continue;
                }
                state.bindings.insert(
                    element.render,
                    BorderBinding {
                        node: node.clone(),
                        color,
                        _subscription: node.observe(incular_animation::scheduler::schedule_frame),
                    },
                );
                dirty.push(element.render);
            }
            state.bindings.retain(|&render, binding| {
                let current = self
                    .element_for_render(render)
                    .and_then(|id| self.elements.get(id.0))
                    .and_then(|element| debug_focus_node(element.widget.kind()));
                if current == Some(&binding.node) {
                    true
                } else {
                    dirty.push(render);
                    false
                }
            });
            self.debug_focus_boxes = Some(state);
        } else {
            dirty.extend(state.bindings.keys().copied());
        }
        for render in dirty {
            if let Some(node) = self.renders.get_mut(render.0) {
                node.dirty.insert(DirtyFlags::PAINT);
                self.propagate_subtree_paint(render);
            }
        }
    }

    pub(super) fn debug_focus_color(&self, id: RenderObjectId) -> Option<Color> {
        self.debug_focus_boxes
            .as_ref()?
            .bindings
            .get(&id)
            .map(|binding| binding.color)
    }
}
