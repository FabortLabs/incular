//! CustomPainter's optional repaint source invalidates only its picture.

use super::*;
use incular_core::reactivity::Subscription;

#[derive(Default)]
pub(super) struct CustomPainterRepaints {
    pending: Rc<RefCell<FastHashSet<RenderObjectId>>>,
    listeners: FastHashMap<RenderObjectId, (u64, Subscription)>,
}

impl WidgetTree {
    pub(super) fn sync_custom_painter_repaint(&mut self, render: RenderObjectId) {
        let source = self.renders.get(render.0).and_then(|render| {
            if let RenderKind::CustomPaint {
                painter: Some(painter),
                ..
            } = render.object.kind()
            {
                painter.repaint_source()
            } else {
                None
            }
        });
        let Some(source) = source else {
            if let Some(state) = &mut self.custom_painter_repaints {
                state.listeners.remove(&render);
            }
            return;
        };
        let state = self
            .custom_painter_repaints
            .get_or_insert_with(Default::default);
        let source_id = source.id();
        if state
            .listeners
            .get(&render)
            .is_some_and(|(old, _)| *old == source_id)
        {
            return;
        }
        let pending = Rc::downgrade(&state.pending);
        let subscription = source.subscribe(render, move || {
            if let Some(pending) = pending.upgrade()
                && pending.borrow_mut().insert(render)
            {
                incular_animation::schedule_frame();
            }
        });
        state.listeners.insert(render, (source_id, subscription));
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
                && matches!(node.object.kind(), RenderKind::CustomPaint { .. })
            {
                // Flutter's Listenable drives markNeedsPaint only. Do not
                // invalidate layout, semantic regions, or the declaration.
                node.dirty.insert(DirtyFlags::PAINT);
                self.propagate_subtree_paint(render);
                any = true;
            }
        }
        any
    }
}
