//! Pointer routing for explicitly mounted RawScrollbar overlays.
use super::*;
use crate::advanced_scrolling::RawScrollbarPart;
use incular_core::{PointerDeviceKind, PointerPhase};

#[derive(Clone, Copy, Debug)]
pub(super) enum ExplicitScrollbarPress {
    Thumb(RenderObjectId),
    Track {
        render: RenderObjectId,
        start: Offset,
        slop_squared: f32,
        canceled: bool,
    },
}

impl ExplicitScrollbarPress {
    pub(super) const fn render(self) -> RenderObjectId {
        match self {
            Self::Thumb(render) | Self::Track { render, .. } => render,
        }
    }
}

fn movement_squared(start: Offset, point: Offset) -> f32 {
    let delta = point - start;
    delta.x * delta.x + delta.y * delta.y
}

impl WidgetTree {
    /// The retained foreground of an ancestor paints on top of every
    /// scrollbar on its child path. Sibling overlays outside the hit path
    /// must not steal the event.
    fn explicit_scrollbar_on_path<T>(
        &self,
        point: Offset,
        hit: &PointerHitTest,
        mut accepts: impl FnMut(&RawScrollbar, Size, Offset) -> Option<T>,
    ) -> Option<(RenderObjectId, Offset, T)> {
        if self.tracked.raw_scrollbars.is_empty() {
            return None;
        }
        let mut render = hit.target();
        let mut found = None;
        while let Some(id) = render {
            let Some(node) = self.renders.get(id.0) else {
                break;
            };
            if let Some(scrollbar) = node
                .raw_scrollbar_state()
                .and_then(|state| state.scrollbar.as_ref())
                && !node.parked
                && let Some(local) = self.scrollbar_local_point(id, point)
                && let Some(target) = accepts(scrollbar, node.size, local)
            {
                found = Some((id, local, target));
            }
            render = node.parent;
        }
        found
    }

    fn explicit_scrollbar_mut(
        &mut self,
        render: RenderObjectId,
    ) -> Option<(&mut RawScrollbar, Size)> {
        let node = self.renders.get_mut(render.0)?;
        let size = node.size;
        Some((node.raw_scrollbar_state_mut()?.scrollbar.as_mut()?, size))
    }

    /// Handles a mounted RawScrollbar contact before its viewport can
    /// enroll the same pointer in the ordinary scroll gesture arena.
    #[doc(hidden)]
    pub fn explicit_scrollbar_pointer_with_hit(
        &mut self,
        phase: PointerPhase,
        point: Offset,
        kind: PointerDeviceKind,
        hit: &PointerHitTest,
        hover: bool,
    ) -> bool {
        if let Some(press) = self.input.explicit_scrollbar_press {
            return match phase {
                PointerPhase::Move => {
                    match press {
                        ExplicitScrollbarPress::Thumb(render) => {
                            if let Some(local) = self.scrollbar_local_point(render, point)
                                && let Some((scrollbar, size)) = self.explicit_scrollbar_mut(render)
                                && scrollbar.pointer_move(size, local)
                            {
                                self.diagnostics.scroll_events += 1;
                            }
                        }
                        ExplicitScrollbarPress::Track {
                            render,
                            start,
                            slop_squared,
                            canceled,
                        } => {
                            self.input.explicit_scrollbar_press =
                                Some(ExplicitScrollbarPress::Track {
                                    render,
                                    start,
                                    slop_squared,
                                    canceled: canceled
                                        || movement_squared(start, point) > slop_squared,
                                });
                        }
                    }
                    true
                }
                PointerPhase::Up | PointerPhase::Cancel => {
                    self.input.explicit_scrollbar_press = None;
                    match press {
                        ExplicitScrollbarPress::Thumb(render) => {
                            if let Some((scrollbar, _)) = self.explicit_scrollbar_mut(render) {
                                scrollbar.pointer_up();
                            }
                        }
                        ExplicitScrollbarPress::Track {
                            render,
                            start,
                            slop_squared,
                            canceled,
                        } => {
                            if phase == PointerPhase::Up
                                && !canceled
                                && movement_squared(start, point) <= slop_squared
                                && let Some(local) = self.scrollbar_local_point(render, point)
                                && let Some((scrollbar, size)) = self.explicit_scrollbar_mut(render)
                                && scrollbar.press_part(size, local, kind)
                                    == Some(RawScrollbarPart::Track)
                                && scrollbar.animate_track_page(size, local)
                            {
                                self.diagnostics.scroll_events += 1;
                            }
                        }
                    }
                    true
                }
                PointerPhase::Down => true,
                PointerPhase::Exit => self.exit_explicit_scrollbar_hover(),
                PointerPhase::Enter => false,
            };
        }
        match phase {
            PointerPhase::Down => {
                let Some((render, local, part)) =
                    self.explicit_scrollbar_on_path(point, hit, |bar, size, local| {
                        bar.press_part(size, local, kind)
                    })
                else {
                    return false;
                };
                let press = match part {
                    RawScrollbarPart::Thumb => {
                        let Some((scrollbar, size)) = self.explicit_scrollbar_mut(render) else {
                            return false;
                        };
                        scrollbar.begin_thumb_drag(size, local);
                        ExplicitScrollbarPress::Thumb(render)
                    }
                    RawScrollbarPart::Track => {
                        let settings = self
                            .element_for_render(render)
                            .map_or(self.environment.gesture_settings, |element| {
                                self.gesture_settings_at(element)
                            });
                        let slop = incular_gestures::compute_hit_slop(kind, settings);
                        ExplicitScrollbarPress::Track {
                            render,
                            start: point,
                            slop_squared: slop * slop,
                            canceled: false,
                        }
                    }
                };
                self.input.explicit_scrollbar_press = Some(press);
                true
            }
            PointerPhase::Move if hover => self.explicit_scrollbar_hover_with_hit(point, kind, hit),
            PointerPhase::Exit => self.exit_explicit_scrollbar_hover(),
            _ => false,
        }
    }

    /// Updates explicit scrollbar hover without creating or owning a contact.
    #[doc(hidden)]
    pub fn explicit_scrollbar_hover_with_hit(
        &mut self,
        point: Offset,
        kind: PointerDeviceKind,
        hit: &PointerHitTest,
    ) -> bool {
        if kind != PointerDeviceKind::Mouse {
            return false;
        }
        let found = self.explicit_scrollbar_on_path(point, hit, |bar, size, local| {
            bar.over_scrollbar(size, local, kind, true).then_some(())
        });
        let next = found.map(|(id, _, ())| id);
        if self.input.explicit_scrollbar_hover == next {
            return next.is_some();
        }
        let changed = self.exit_explicit_scrollbar_hover();
        if let Some((render, local, ())) = found
            && let Some((scrollbar, size)) = self.explicit_scrollbar_mut(render)
        {
            scrollbar.pointer_hover(size, local, kind);
            self.input.explicit_scrollbar_hover = Some(render);
            return true;
        }
        changed
    }

    fn exit_explicit_scrollbar_hover(&mut self) -> bool {
        if let Some(render) = self.input.explicit_scrollbar_hover.take()
            && let Some((scrollbar, _)) = self.explicit_scrollbar_mut(render)
        {
            scrollbar.pointer_exit();
            return true;
        }
        false
    }

    pub(super) fn clear_explicit_scrollbar_input(&mut self) {
        self.exit_explicit_scrollbar_hover();
        if let Some(ExplicitScrollbarPress::Thumb(render)) =
            self.input.explicit_scrollbar_press.take()
            && let Some((scrollbar, _)) = self.explicit_scrollbar_mut(render)
        {
            scrollbar.pointer_cancel();
        }
    }

    pub(super) fn forget_explicit_scrollbar(&mut self, render: RenderObjectId) {
        if self.input.explicit_scrollbar_hover == Some(render) {
            self.input.explicit_scrollbar_hover = None;
        }
        if self
            .input
            .explicit_scrollbar_press
            .is_some_and(|press| press.render() == render)
            && let Some(ExplicitScrollbarPress::Thumb(_)) =
                self.input.explicit_scrollbar_press.take()
            && let Some((scrollbar, _)) = self.explicit_scrollbar_mut(render)
        {
            scrollbar.pointer_cancel();
        }
    }
}
