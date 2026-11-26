//! Contacts that have won separate arenas but drive the same viewport.
use super::*;
use crate::MultitouchDragStrategy;
use incular_gestures::DragBoundaryFrame;

pub(super) struct ScrollContactGroup {
    members: Vec<GestureArenaKey>,
    active: Option<GestureArenaKey>,
    strategy: MultitouchDragStrategy,
    boundary: Option<Box<DragBoundaryFrame<GestureArenaKey>>>,
}

impl Default for ScrollContactGroup {
    fn default() -> Self {
        Self {
            members: Vec::new(),
            active: None,
            strategy: MultitouchDragStrategy::LatestPointer,
            boundary: None,
        }
    }
}

impl ScrollContactGroup {
    pub(super) fn join(&mut self, key: GestureArenaKey, strategy: MultitouchDragStrategy) {
        if self.strategy != strategy {
            self.strategy = strategy;
            self.boundary =
                (strategy == MultitouchDragStrategy::AverageBoundaryPointers).then(Box::default);
        }
        if !self.members.contains(&key) {
            self.members.push(key);
            self.active = Some(key);
        }
    }

    fn leave(&mut self, key: GestureArenaKey) -> Option<GestureArenaKey> {
        self.members.retain(|member| *member != key);
        if let Some(frame) = &mut self.boundary {
            frame.remove(key);
        }
        if self.active == Some(key) {
            self.active = self.members.first().copied();
        }
        self.active
    }

    fn update(
        &mut self,
        key: GestureArenaKey,
        current_frame: Option<Instant>,
        physical_delta: f32,
    ) -> Option<f32> {
        if self.strategy == MultitouchDragStrategy::LatestPointer && self.active != Some(key) {
            return None;
        }
        if let Some(frame) = &mut self.boundary {
            if let Some(stamp) = current_frame {
                frame.begin(stamp);
            }
            Some(frame.update_scroll_axis(key, physical_delta))
        } else {
            Some(physical_delta)
        }
    }
}

impl WidgetTree {
    /// Changes the system-frame grouping for touch scrolling, independently
    /// of individual input event timestamps.
    #[doc(hidden)]
    pub fn set_scroll_drag_frame_time(&mut self, stamp: Instant) {
        self.input.scroll_drag_frame_time = Some(stamp);
    }

    pub(super) fn join_scroll_contact(
        &mut self,
        key: GestureArenaKey,
        attachment: u64,
        strategy: MultitouchDragStrategy,
    ) {
        self.input
            .scroll_contact_groups
            .entry((key.window, attachment))
            .or_default()
            .join(key, strategy);
    }

    pub(super) fn scroll_group_has_live_activity(&self, window: u64, attachment: u64) -> bool {
        self.input
            .scroll_contact_groups
            .get(&(window, attachment))
            .is_some_and(|group| {
                group
                    .members
                    .iter()
                    .any(|member| self.scroll_contact_has_activity(*member))
            })
    }

    pub(super) fn scroll_contact_is_driving(
        &self,
        key: GestureArenaKey,
        bracket: &ScrollBracket,
    ) -> bool {
        self.input
            .scroll_contact_groups
            .get(&(key.window, bracket.attachment))
            .is_some_and(|group| {
                group.members.contains(&key)
                    && group
                        .members
                        .iter()
                        .any(|member| self.scroll_contact_has_activity(*member))
            })
    }

    fn scroll_contact_has_activity(&self, key: GestureArenaKey) -> bool {
        self.input
            .scroll_brackets
            .get(&key)
            .and_then(|held| held.activity.as_ref())
            .is_some_and(|activity| activity.is_current())
    }

    pub(super) fn resolve_scroll_contact_motion(
        &mut self,
        key: GestureArenaKey,
        member: GestureArenaMember,
        action: GestureAction,
    ) -> Option<GestureAction> {
        let (mut details, horizontal) = match action {
            GestureAction::HorizontalDrag(details) => (details, true),
            GestureAction::VerticalDrag(details) => (details, false),
            _ => return Some(action),
        };
        let Some(bracket) = self
            .input
            .scroll_brackets
            .get(&key)
            .filter(|bracket| bracket.member == member)
        else {
            return Some(action);
        };
        let Some(group) = self
            .input
            .scroll_contact_groups
            .get_mut(&(key.window, bracket.attachment))
        else {
            return Some(action);
        };
        let physical = if horizontal {
            details.delta.x
        } else {
            details.delta.y
        };
        let resolved = group.update(key, self.input.scroll_drag_frame_time, physical)?;
        details.primary_delta = Some(resolved);
        if horizontal {
            details.delta.x = resolved;
            Some(GestureAction::HorizontalDrag(details))
        } else {
            details.delta.y = resolved;
            Some(GestureAction::VerticalDrag(details))
        }
    }

    /// Removes one stream and hands its live activity to a survivor. The last
    /// contact alone can close the shared activity or start a ballistic.
    pub(super) fn take_scroll_contact(
        &mut self,
        key: GestureArenaKey,
    ) -> Option<(ScrollBracket, bool)> {
        let mut bracket = self.input.scroll_brackets.remove(&key)?;
        let group_id = (key.window, bracket.attachment);
        let Some(group) = self
            .input
            .scroll_contact_groups
            .get_mut(&group_id)
            .filter(|group| group.members.contains(&key))
        else {
            return Some((bracket, false));
        };
        let Some(next) = group.leave(key) else {
            self.input.scroll_contact_groups.remove(&group_id);
            return Some((bracket, false));
        };
        if let Some(survivor) = self.input.scroll_brackets.get_mut(&next) {
            transfer_activity(&mut bracket.activity, &mut survivor.activity);
            for link in &mut bracket.outer {
                if let Some(same_ancestor) = survivor
                    .outer
                    .iter_mut()
                    .find(|candidate| candidate.attachment == link.attachment)
                {
                    transfer_activity(&mut link.activity, &mut same_ancestor.activity);
                }
            }
        }
        Some((bracket, true))
    }
}

fn transfer_activity(source: &mut Option<OwnedActivity>, target: &mut Option<OwnedActivity>) {
    if source
        .as_ref()
        .is_some_and(|activity| activity.is_current())
        && !target
            .as_ref()
            .is_some_and(|activity| activity.is_current())
    {
        *target = source.take();
    }
}
