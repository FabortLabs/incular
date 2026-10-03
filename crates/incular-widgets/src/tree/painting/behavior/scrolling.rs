use super::super::*;

impl WidgetTree {
    pub(super) fn paint_scrolling_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        size: Size,
        cache: &mut DisplayList,
    ) {
        match *kind {
            RenderKind::Scroll { ref controller, .. } => {
                self.paint_scrollbar(id, size, controller, cache);
            }
            RenderKind::SliverViewport { ref config } => {
                self.paint_scrollbar(id, size, &config.controller, cache);
            }
            _ => unreachable!("scroll paint received a non-scrolling render kind"),
        }
    }
}
