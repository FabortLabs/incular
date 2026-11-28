//! Optional painter notifications invalidate only the mounted picture.

use super::*;
use incular_core::reactivity::{DependencySource, Subscription};

struct PainterSubscriptions {
    sources: [Option<u64>; 2],
    _handles: [Option<Subscription>; 2],
}

#[derive(Default)]
pub(super) struct CustomPainterRepaints {
    pending: Rc<RefCell<FastHashSet<RenderObjectId>>>,
    listeners: FastHashMap<RenderObjectId, PainterSubscriptions>,
}

fn repaint_sources(kind: &RenderKind) -> [Option<DependencySource>; 2] {
    match kind {
        RenderKind::CustomPaint {
            painter: Some(painter),
            ..
        } => [painter.repaint_source(), None],
        RenderKind::TextField(field) => field.painters.as_ref().map_or([None, None], |painters| {
            [
                painters.background.as_ref().and_then(|p| p.source()),
                painters.foreground.as_ref().and_then(|p| p.source()),
            ]
        }),
        _ => [None, None],
    }
}

impl WidgetTree {
    pub(super) fn sync_custom_painter_repaint(&mut self, render: RenderObjectId) {
        let mut sources = self
            .renders
            .get(render.0)
            .map_or([None, None], |render| repaint_sources(render.object.kind()));
        let ids = sources
            .each_ref()
            .map(|source| source.as_ref().map(DependencySource::id));
        if ids == [None, None] {
            if let Some(state) = &mut self.custom_painter_repaints {
                state.listeners.remove(&render);
                state.pending.borrow_mut().remove(&render);
            }
            return;
        }
        let state = self
            .custom_painter_repaints
            .get_or_insert_with(Default::default);
        if state
            .listeners
            .get(&render)
            .is_some_and(|old| old.sources == ids)
        {
            return;
        }
        if ids[0] == ids[1] {
            // A source shared by both delegates only needs one frame request.
            sources[1] = None;
        }
        let subscriptions = sources.map(|source| {
            source.map(|source| {
                let pending = Rc::downgrade(&state.pending);
                source.subscribe(render, move || {
                    if let Some(pending) = pending.upgrade()
                        && pending.borrow_mut().insert(render)
                    {
                        incular_animation::schedule_frame();
                    }
                })
            })
        });
        state.listeners.insert(
            render,
            PainterSubscriptions {
                sources: ids,
                _handles: subscriptions,
            },
        );
    }

    pub(super) fn detach_custom_painter_repaint(&mut self, render: RenderObjectId) {
        if let Some(state) = &mut self.custom_painter_repaints {
            state.listeners.remove(&render);
            state.pending.borrow_mut().remove(&render);
        }
    }

    pub(super) fn flush_custom_painter_repaints(&mut self) -> bool {
        let Some(state) = &self.custom_painter_repaints else {
            return false;
        };
        if state.pending.borrow().is_empty() {
            return false;
        }
        let pending = std::mem::take(&mut *state.pending.borrow_mut());
        let mut any = false;
        for render in pending {
            if let Some(node) = self.renders.get_mut(render.0)
                && matches!(
                    node.object.kind(),
                    RenderKind::CustomPaint { .. } | RenderKind::TextField(_)
                )
            {
                // The notification requests paint, not a layout or semantic change.
                node.dirty.insert(DirtyFlags::PAINT);
                self.propagate_subtree_paint(render);
                any = true;
            }
        }
        any
    }
}
