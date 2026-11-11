use super::super::*;

impl WidgetTree {
    pub(super) fn paint_scrolling_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        _size: Size,
        cache: &mut DisplayList,
    ) {
        match *kind {
            RenderKind::Scroll { .. } | RenderKind::SliverViewport { .. } => {
                self.paint_scrollbar(id, cache);
            }
            _ => unreachable!("scroll paint received a non-scrolling render kind"),
        }
    }
}
