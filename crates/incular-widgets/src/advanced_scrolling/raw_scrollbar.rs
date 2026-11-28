//! `RawScrollbar` geometry and pointer interaction.
//!
//! Flutter's raw scrollbar is an overlay render object rather than a second
//! scroll position. The implementation here keeps that distinction: the
//! existing [`ScrollController`] remains the source of truth and this module
//! only derives a track/thumb pair and maps pointer motion back to its offset.

use incular_core::time::Instant;
use std::{fmt, rc::Rc, time::Duration};

use incular_animation::Curve;
use incular_config::{Axis, TextDirection};
use incular_core::finite_non_negative;
use incular_core::{Color, Offset, Rect, Size};
use incular_scroll::{ActivityOrigin, OwnedActivity, ScrollController};
use smart_default::SmartDefault;

use crate::MouseCursor;
use crate::gestures::PointerDeviceKind;

/// Flutter `_kScrollbarFadeDuration` (`widgets/scrollbar.dart:43`).
pub(crate) const SCROLLBAR_FADE_DURATION: Duration = Duration::from_millis(300);
/// Flutter `_kScrollbarTimeToFade` (`widgets/scrollbar.dart:44`).
pub(crate) const SCROLLBAR_TIME_TO_FADE: Duration = Duration::from_millis(600);
/// Flutter `_kMinInteractiveSize` (`widgets/scrollbar.dart:41`): the touch and
/// hover target around a thumb is at least this large.
const MIN_INTERACTIVE_SIZE: f32 = 48.0;

/// A request recorded between frames and applied with the next frame's time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ScrollbarFadeCommand {
    /// `_fadeoutAnimationController.forward()`.
    Forward,
    /// `_fadeoutTimer?.cancel()`.
    CancelTimer,
    /// `_maybeStartFadeoutTimer()`: restart the timer that reverses the fade.
    StartTimer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FadeAnimation {
    from: f32,
    to: f32,
    started: Instant,
    duration: Duration,
}

/// The fade of Flutter's `RawScrollbarState`: an `AnimationController` of
/// `fadeDuration` whose value runs through `Curves.fastOutSlowIn` to give the
/// thumb's opacity, and the `timeToFade` timer that reverses it.
///
/// Time only enters through [`Self::advance`], so user input can be recorded
/// without a clock and still replay deterministically on the next frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarFade {
    /// `RawScrollbar.fadeDuration`.
    fade_duration: Duration,
    /// `RawScrollbar.timeToFade`.
    time_to_fade: Duration,
    value: f32,
    animation: Option<FadeAnimation>,
    timer_deadline: Option<Instant>,
    /// Requests since the last frame, oldest first. A frame sees at most a
    /// handful of pointer and scroll events; the newest replaces the last slot
    /// beyond that, which only ever coalesces repeated identical requests.
    commands: [Option<ScrollbarFadeCommand>; 4],
    showing: bool,
}

impl Default for ScrollbarFade {
    fn default() -> Self {
        Self {
            fade_duration: SCROLLBAR_FADE_DURATION,
            time_to_fade: SCROLLBAR_TIME_TO_FADE,
            value: 0.0,
            animation: None,
            timer_deadline: None,
            commands: [None; 4],
            showing: false,
        }
    }
}

impl ScrollbarFade {
    /// Sets `fadeDuration` and `timeToFade`; a running fade keeps its timing.
    pub(crate) fn set_durations(&mut self, fade_duration: Duration, time_to_fade: Duration) {
        self.fade_duration = fade_duration;
        self.time_to_fade = time_to_fade;
    }

    /// `_fadeoutOpacityAnimation.value`.
    pub(crate) fn opacity(&self) -> f32 {
        Curve::FastOutSlowIn.apply(self.value)
    }

    pub(crate) fn queue(&mut self, command: ScrollbarFadeCommand) {
        let slot = self
            .commands
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.commands.len() - 1);
        self.commands[slot] = Some(command);
    }

    /// `AnimationStatus.isForwardOrCompleted`.
    fn is_forward_or_completed(&self) -> bool {
        match self.animation {
            Some(animation) => animation.to > animation.from,
            None => self.value >= 1.0,
        }
    }

    fn animate_to(&mut self, target: f32, now: Instant) {
        if self.value == target {
            self.animation = None;
            return;
        }
        // AnimationController.animateTo scales the duration by the distance
        // left to travel.
        let fraction = (target - self.value).abs();
        self.animation = Some(FadeAnimation {
            from: self.value,
            to: target,
            started: now,
            duration: self.fade_duration.mul_f32(fraction),
        });
    }

    /// Moves the running animation to `at`.
    fn step(&mut self, at: Instant) {
        let Some(animation) = self.animation else {
            return;
        };
        let elapsed = at.saturating_duration_since(animation.started);
        let progress = if animation.duration.is_zero() {
            1.0
        } else {
            (elapsed.as_secs_f32() / animation.duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        self.value = animation.from + (animation.to - animation.from) * progress;
        if progress >= 1.0 {
            self.value = animation.to;
            self.animation = None;
        }
    }

    /// Applies recorded requests and the scrollbar's visibility policy at
    /// `now`, then steps the animation and timer. Returns whether the fade
    /// still needs frames (it is animating or a timer is pending).
    ///
    /// `show_scrollbar` is `RawScrollbar.thumbVisibility ?? false` and
    /// `scrollable` whether the content can scroll at all
    /// (`maxScrollExtent > minScrollExtent`).
    pub(crate) fn advance(&mut self, now: Instant, show_scrollbar: bool, scrollable: bool) -> bool {
        for command in std::mem::take(&mut self.commands).into_iter().flatten() {
            match command {
                ScrollbarFadeCommand::Forward => {
                    if !self.is_forward_or_completed() {
                        self.animate_to(1.0, now);
                    }
                }
                ScrollbarFadeCommand::CancelTimer => self.timer_deadline = None,
                ScrollbarFadeCommand::StartTimer => {
                    // `_maybeStartFadeoutTimer` only arms without
                    // thumbVisibility; a visible thumb never fades out.
                    if !show_scrollbar {
                        self.timer_deadline = Some(now + self.time_to_fade);
                    }
                }
            }
        }
        if show_scrollbar != self.showing {
            // didUpdateWidget: thumbVisibility changed.
            self.showing = show_scrollbar;
            if show_scrollbar {
                self.timer_deadline = None;
                self.animate_to(1.0, now);
            } else {
                self.animate_to(0.0, now);
            }
        } else if show_scrollbar && !self.is_forward_or_completed() {
            // _handleScrollMetricsNotification.
            self.animate_to(1.0, now);
        }
        if !scrollable && self.is_forward_or_completed() {
            // _handleScrollNotification: nothing to scroll hides the bar.
            self.animate_to(0.0, now);
        }
        if let Some(deadline) = self.timer_deadline
            && now >= deadline
        {
            // The timer fires at its deadline, not at the frame that notices
            // it: bring the animation to that instant, then reverse from it.
            self.timer_deadline = None;
            self.step(deadline);
            self.animate_to(0.0, deadline);
        }
        self.step(now);
        self.animation.is_some() || self.timer_deadline.is_some()
    }
}

/// Which edge receives the scrollbar overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RawScrollbarOrientation {
    /// Infer the edge from the scroll axis and ambient text direction.
    #[default]
    Auto,
    /// Place a vertical scrollbar on the right edge.
    Right,
    /// Place a vertical scrollbar on the left edge.
    Left,
    /// Place a horizontal scrollbar on the bottom edge.
    Bottom,
    /// Place a horizontal scrollbar on the top edge.
    Top,
}

impl RawScrollbarOrientation {
    fn resolve(self, axis: Axis, text_direction: TextDirection) -> (Self, Axis) {
        match self {
            Self::Auto => match (axis, text_direction) {
                (Axis::Horizontal, _) => (Self::Bottom, axis),
                (Axis::Vertical, TextDirection::Ltr) => (Self::Right, axis),
                (Axis::Vertical, TextDirection::Rtl) => (Self::Left, axis),
            },
            Self::Left | Self::Right => (self, Axis::Vertical),
            Self::Top | Self::Bottom => (self, Axis::Horizontal),
        }
    }
}

/// The renderer-independent style inputs used by [`RawScrollbar`].
#[derive(Clone, Copy, Debug, PartialEq, SmartDefault)]
pub struct RawScrollbarStyle {
    /// Thickness of the track and thumb in the cross axis.
    #[default(6.0)]
    pub thickness: f32,
    /// Minimum thumb extent when the content is larger than the viewport.
    #[default(18.0)]
    pub min_thumb_extent: f32,
    /// Thumb floor used while overscrolling (Flutter's `minOverscrollLength`).
    /// The thumb minimum shrinks toward this floor over the first 20% of
    /// overscroll.
    #[default(18.0)]
    pub min_overscroll_thumb_extent: f32,
    /// Empty space inset from both ends of the main axis.
    pub main_axis_margin: f32,
    /// Empty space inset from the overlay edge in the cross axis.
    pub cross_axis_margin: f32,
    /// Track paint color. A transparent track is the default overlay style.
    pub track_color: Color,
    /// Thumb paint color.
    #[default(Color::rgba(188, 188, 188, 102))]
    pub thumb_color: Color,
    /// Overlay edge.
    pub orientation: RawScrollbarOrientation,
    /// Whether pointer interaction is enabled.
    #[default(true)]
    pub interactive: bool,
    /// Flutter's `RawScrollbar.thumbVisibility`. `Some(true)` keeps the thumb
    /// visible; `None` and `Some(false)` hide it at rest, revealing it while
    /// the content scrolls or the pointer hovers it and fading it out
    /// `time_to_fade` after the last activity.
    pub thumb_visibility: Option<bool>,
    /// Flutter `fadeDuration`: how long the thumb takes to fade in or out.
    #[default(SCROLLBAR_FADE_DURATION)]
    pub fade_duration: Duration,
    /// Flutter `timeToFade`: how long the thumb stays after the last
    /// activity before fading out.
    #[default(SCROLLBAR_TIME_TO_FADE)]
    pub time_to_fade: Duration,
}

impl RawScrollbarStyle {
    fn normalized(self) -> Self {
        Self {
            thickness: finite_positive(self.thickness, 1.0),
            min_thumb_extent: finite_positive(self.min_thumb_extent, 1.0),
            min_overscroll_thumb_extent: finite_positive(self.min_overscroll_thumb_extent, 1.0),
            main_axis_margin: finite_non_negative(self.main_axis_margin),
            cross_axis_margin: finite_non_negative(self.cross_axis_margin),
            ..self
        }
    }
}

/// The computed scrollbar geometry for one viewport size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawScrollbarGeometry {
    /// Whether the track/thumb are painted: the content can scroll and the
    /// fade has not hidden them (Flutter paints nothing at opacity 0).
    pub visible: bool,
    /// The thumb's opacity from the fade, `Curves.fastOutSlowIn` applied.
    pub opacity: f32,
    /// Track rectangle in viewport coordinates.
    pub track: Rect,
    /// Thumb rectangle in viewport coordinates.
    pub thumb: Rect,
    /// Current minimum logical scroll offset: zero unless the viewport has
    /// content before its center.
    pub min_scroll_extent: f32,
    /// Current maximum logical scroll offset.
    pub max_scroll_extent: f32,
    /// Distance available to move the thumb's leading edge.
    pub thumb_travel: f32,
    /// Content extent reported by the controller.
    pub content_extent: f32,
    /// Viewport extent reported by the controller.
    pub viewport_extent: f32,
}

impl RawScrollbarGeometry {
    /// The scroll range the thumb travels across.
    #[must_use]
    pub fn scroll_range(&self) -> f32 {
        self.max_scroll_extent - self.min_scroll_extent
    }

    /// The scroll offset at `fraction` of the thumb's travel.
    #[must_use]
    pub fn offset_at(&self, fraction: f32) -> f32 {
        self.min_scroll_extent + self.scroll_range() * fraction
    }

    /// Returns whether a point is inside the painted thumb.
    #[must_use]
    pub fn thumb_contains(self, point: Offset) -> bool {
        self.visible && self.thumb.contains(point)
    }

    /// Returns whether a point is inside the painted track.
    #[must_use]
    pub fn track_contains(self, point: Offset) -> bool {
        self.visible && self.track.contains(point)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawScrollbarPart {
    Thumb,
    Track,
}

#[derive(Debug, PartialEq)]
struct ThumbDrag {
    grab_offset: f32,
    /// The bracket this drag owns. Release closes exactly this activity;
    /// a newer takeover makes the token stale so cleanup stays silent.
    /// Non-cloneable with the token: clones never inherit gestures.
    activity: OwnedActivity,
}

/// Interaction state for resolving the cursor of an explicit scrollbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollbarCursorState {
    pub hovered: bool,
    pub dragged: bool,
    pub disabled: bool,
}

#[derive(Clone)]
pub(crate) enum ScrollbarCursor {
    Fixed(MouseCursor),
    Resolved(Rc<dyn Fn(ScrollbarCursorState) -> Option<MouseCursor>>),
}

impl fmt::Debug for ScrollbarCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(cursor) => f.debug_tuple("Fixed").field(cursor).finish(),
            Self::Resolved(_) => f.write_str("Resolved(..)"),
        }
    }
}

impl PartialEq for ScrollbarCursor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fixed(a), Self::Fixed(b)) => a == b,
            (Self::Resolved(a), Self::Resolved(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl ScrollbarCursor {
    pub(crate) fn resolve(&self, state: ScrollbarCursorState) -> Option<MouseCursor> {
        match self {
            Self::Fixed(cursor) => Some(*cursor),
            Self::Resolved(resolve) => resolve(state),
        }
        .filter(|cursor| *cursor != MouseCursor::Defer)
    }
}

/// A stateful raw scrollbar overlay attached to an existing scroll controller.
///
/// Pointer methods use viewport coordinates and preserve Flutter's thumb
/// mapping: the pointer's grab point stays under the pointer and logical
/// offset is reversed when the controller's physical axis direction is
/// reversed.
///
/// Dropping a mid-drag scrollbar needs no manual cleanup: the drag
/// state's activity token aborts silently when still current.
///
/// Cloneable configuration, never cloneable ownership: clones share the
/// controller and style but start idle — an in-flight gesture is not
/// inherited and cannot be cancelled through a copy.
#[derive(Debug)]
pub struct RawScrollbar {
    controller: ScrollController,
    cursor: Option<ScrollbarCursor>,
    style: RawScrollbarStyle,
    text_direction: TextDirection,
    drag: Option<ThumbDrag>,
    fade: ScrollbarFade,
    /// `_hoverIsActive`.
    hover_active: bool,
    /// The offset the last [`Self::tick`] saw, to notice scrolling.
    last_offset: Option<f32>,
    /// Whether the controller had an activity at the last tick.
    was_active: bool,
}

impl Clone for RawScrollbar {
    fn clone(&self) -> Self {
        let mut fade = ScrollbarFade::default();
        fade.set_durations(self.style.fade_duration, self.style.time_to_fade);
        Self {
            controller: self.controller.clone(),
            cursor: self.cursor.clone(),
            style: self.style,
            text_direction: self.text_direction,
            drag: None,
            fade,
            hover_active: false,
            last_offset: None,
            was_active: false,
        }
    }
}

impl RawScrollbar {
    /// Creates a raw overlay for `controller` using the default style.
    #[must_use]
    pub fn new(controller: ScrollController) -> Self {
        Self {
            controller,
            cursor: None,
            style: RawScrollbarStyle::default(),
            text_direction: TextDirection::Ltr,
            drag: None,
            fade: ScrollbarFade::default(),
            hover_active: false,
            last_offset: None,
            was_active: false,
        }
    }

    /// Overrides the mouse cursor over this bar or during its thumb drag.
    #[must_use]
    pub fn mouse_cursor(mut self, cursor: MouseCursor) -> Self {
        self.cursor = Some(ScrollbarCursor::Fixed(cursor));
        self
    }

    /// Resolves an optional cursor from current scrollbar interaction.
    /// Returning None or Defer leaves ordinary pointer cursor routing intact.
    #[must_use]
    pub fn mouse_cursor_with_states(
        mut self,
        resolve: impl Fn(ScrollbarCursorState) -> Option<MouseCursor> + 'static,
    ) -> Self {
        self.cursor = Some(ScrollbarCursor::Resolved(Rc::new(resolve)));
        self
    }

    pub(crate) fn cursor_resolver(&self) -> Option<ScrollbarCursor> {
        self.cursor.clone()
    }

    pub(crate) fn captures_cursor(&self, size: Size) -> bool {
        self.drag.is_some() && self.cursor_enabled(size)
    }

    pub(crate) fn cursor_enabled(&self, size: Size) -> bool {
        self.style.interactive && self.scrollable_geometry(size).is_some()
    }

    /// `RawScrollbar.thumbVisibility ?? false`.
    fn show_scrollbar(&self) -> bool {
        self.style.thumb_visibility.unwrap_or(false)
    }

    /// The thumb's current opacity (`fadeoutOpacityAnimation.value`).
    #[must_use]
    pub fn opacity(&self) -> f32 {
        self.fade.opacity()
    }

    /// Advances the fade to `now` and reports whether it needs another frame.
    ///
    /// Scrolling since the previous tick reveals the scrollbar and holds it
    /// (Flutter's `ScrollUpdateNotification`); when the controller goes idle,
    /// or the thumb drag or hover ends, the fade-out timer starts.
    pub fn tick(&mut self, now: Instant) -> bool {
        let offset = self.controller.offset();
        let active = self.controller.current_activity_id().is_some();
        let scrolled = self.last_offset.is_some_and(|last| last != offset);
        self.last_offset = Some(offset);
        if scrolled {
            self.fade.queue(ScrollbarFadeCommand::Forward);
            self.fade.queue(ScrollbarFadeCommand::CancelTimer);
        }
        if (scrolled || self.was_active) && !active && self.drag.is_none() {
            // ScrollEndNotification.
            self.fade.queue(ScrollbarFadeCommand::StartTimer);
        }
        self.was_active = active;
        let scrollable = self.controller.max_offset() > 0.0;
        let show = self.show_scrollbar();
        self.fade.advance(now, show, scrollable)
    }

    /// `RawScrollbarState.handleHover`: a mouse over the (padded) scrollbar
    /// brings it into view and holds it; leaving restarts the fade-out timer.
    pub fn pointer_hover(&mut self, viewport_size: Size, point: Offset, kind: PointerDeviceKind) {
        if self.over_scrollbar(viewport_size, point, kind, true) {
            self.hover_active = true;
            self.fade.queue(ScrollbarFadeCommand::Forward);
            self.fade.queue(ScrollbarFadeCommand::CancelTimer);
        } else if self.hover_active {
            self.hover_active = false;
            self.fade.queue(ScrollbarFadeCommand::StartTimer);
        }
    }

    /// `RawScrollbarState.handleHoverExit`.
    pub fn pointer_exit(&mut self) {
        self.hover_active = false;
        self.fade.queue(ScrollbarFadeCommand::StartTimer);
    }

    /// `ScrollbarPainter.hitTestInteractive`: the track, padded to the minimum
    /// interactive size around the thumb for touch and hover. A transparent
    /// scrollbar can only be hit by a mouse hovering it, which brings it in.
    pub(crate) fn over_scrollbar(
        &self,
        viewport_size: Size,
        point: Offset,
        kind: PointerDeviceKind,
        for_hover: bool,
    ) -> bool {
        if !self.style.normalized().interactive {
            return false;
        }
        let Some(geometry) = self.scrollable_geometry(viewport_size) else {
            return false;
        };
        let padded = padded_interactive_rect(geometry.track, geometry.thumb);
        if self.fade.opacity() == 0.0 {
            return for_hover && kind == PointerDeviceKind::Mouse && padded.contains(point);
        }
        match kind {
            PointerDeviceKind::Touch | PointerDeviceKind::Trackpad => padded.contains(point),
            _ => geometry.track.contains(point),
        }
    }

    /// Closes the live drag's bracket, if any. Only the still-current
    /// token emits `End`: track clicks, stray releases, and takeovers
    /// stay silent. Shared with release, cancellation, and style drops.
    fn end_drag_activity(&mut self) {
        if let Some(drag) = self.drag.take() {
            drag.activity.finish();
            // handleThumbPressEnd: the fade-out timer restarts.
            self.fade.queue(ScrollbarFadeCommand::StartTimer);
        }
    }

    /// Returns the attached controller.
    #[must_use]
    pub fn controller(&self) -> ScrollController {
        self.controller.clone()
    }

    /// Returns the current style.
    #[must_use]
    pub fn style(&self) -> RawScrollbarStyle {
        self.style
    }

    /// Replaces the style, closing a live drag's bracket first: only the
    /// still-current token emits `End`, so a taken-over bracket stays
    /// silent here.
    pub fn set_style(&mut self, style: RawScrollbarStyle) {
        self.end_drag_activity();
        self.style = style.normalized();
        self.fade
            .set_durations(self.style.fade_duration, self.style.time_to_fade);
    }

    pub(crate) fn set_text_direction(&mut self, direction: TextDirection) {
        self.text_direction = direction;
    }

    fn resolved_axis(&self) -> Axis {
        self.style
            .orientation
            .resolve(
                self.controller.metrics().axis_direction.axis(),
                self.text_direction,
            )
            .1
    }

    /// Computes track and thumb rectangles for `viewport_size`. The
    /// geometry is `visible` while the content scrolls and the fade has not
    /// hidden the scrollbar.
    #[must_use]
    pub fn geometry(&self, viewport_size: Size) -> RawScrollbarGeometry {
        let mut geometry = self.layout_geometry(viewport_size);
        let opacity = self.fade.opacity();
        geometry.opacity = opacity;
        geometry.visible = geometry.visible && opacity > 0.0;
        geometry
    }

    /// The geometry when the content can scroll, regardless of the fade.
    fn scrollable_geometry(&self, viewport_size: Size) -> Option<RawScrollbarGeometry> {
        let geometry = self.layout_geometry(viewport_size);
        geometry.visible.then_some(geometry)
    }

    /// Track and thumb for the current scroll metrics; `visible` here only
    /// says the content is scrollable.
    fn layout_geometry(&self, viewport_size: Size) -> RawScrollbarGeometry {
        let style = self.style.normalized();
        let metrics = self.controller.metrics();
        let (orientation, axis) = style
            .orientation
            .resolve(metrics.axis_direction.axis(), self.text_direction);
        let content_extent = self.controller.content_extent().max(0.0);
        let viewport_extent = self.controller.viewport_extent().max(0.0);
        let min_scroll_extent = self.controller.min_offset().min(0.0);
        let max_scroll_extent = self.controller.max_offset().max(0.0);
        let scroll_range = max_scroll_extent - min_scroll_extent;
        let cross_extent = axis.cross_extent(viewport_size);
        let travel_extent = (viewport_extent - style.main_axis_margin * 2.0).max(0.0);
        let track_cross = style.thickness + style.cross_axis_margin * 2.0;
        let cross_start = match orientation {
            RawScrollbarOrientation::Left | RawScrollbarOrientation::Top => style.cross_axis_margin,
            RawScrollbarOrientation::Auto
            | RawScrollbarOrientation::Right
            | RawScrollbarOrientation::Bottom => {
                cross_extent - style.cross_axis_margin - style.thickness
            }
        };
        // As in Flutter, the track has the scroll metrics' viewport length,
        // not the canvas length. Main-axis margins inset the thumb only.
        let track_origin = axis.offset(0., cross_start - style.cross_axis_margin);
        let track = Rect::from_origin_size(track_origin, axis.size(viewport_extent, track_cross));

        if cross_extent <= 0.0
            || content_extent <= viewport_extent
            || viewport_extent <= 0.0
            || travel_extent <= 0.0
        {
            return RawScrollbarGeometry {
                visible: false,
                opacity: 0.0,
                track,
                thumb: Rect::from_origin_size(track_origin, Size::ZERO),
                min_scroll_extent,
                max_scroll_extent,
                thumb_travel: 0.0,
                content_extent,
                viewport_extent,
            };
        }

        // Flutter's ScrollbarPainter: the fraction of the viewport that is
        // overscrolled is `1 - extentInside / viewportDimension`, and the
        // thumb minimum shrinks linearly over its first 20%.
        let extent_inside = viewport_extent
            - (metrics.min_scroll_extent - metrics.pixels).clamp(0.0, viewport_extent)
            - (metrics.pixels - metrics.max_scroll_extent).clamp(0.0, viewport_extent);
        let fraction_overscrolled = 1.0 - extent_inside / viewport_extent;
        let fraction = (extent_inside / content_extent).clamp(0.0, 1.0);
        let min_extent = style.min_thumb_extent.min(travel_extent)
            * (1.0 - fraction_overscrolled.clamp(0.0, 0.2) / 0.2);
        let thumb_main = (travel_extent * fraction)
            .max(style.min_overscroll_thumb_extent.min(travel_extent))
            .max(min_extent);
        let thumb_travel = (travel_extent - thumb_main).max(0.0);
        let offset_fraction = if scroll_range > 0.0 {
            ((self.controller.offset() - min_scroll_extent) / scroll_range).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let offset_fraction = if metrics.axis_direction.is_reversed() {
            1.0 - offset_fraction
        } else {
            offset_fraction
        };
        let thumb_origin = axis.offset(
            style.main_axis_margin + thumb_travel * offset_fraction,
            cross_start,
        );
        RawScrollbarGeometry {
            visible: true,
            opacity: 1.0,
            track,
            thumb: Rect::from_origin_size(thumb_origin, axis.size(thumb_main, style.thickness)),
            min_scroll_extent,
            max_scroll_extent,
            thumb_travel,
            content_extent,
            viewport_extent,
        }
    }

    /// Starts a mouse thumb drag or performs a one-shot page step.
    /// Mounted taps instead defer their page step until pointer release.
    pub fn pointer_down(&mut self, viewport_size: Size, point: Offset) -> bool {
        match self.press_part(viewport_size, point, PointerDeviceKind::Mouse) {
            Some(RawScrollbarPart::Thumb) => {
                self.begin_thumb_drag(viewport_size, point);
                true
            }
            Some(RawScrollbarPart::Track) => {
                self.page_track(viewport_size, point);
                true
            }
            None => false,
        }
    }

    pub(crate) fn press_part(
        &self,
        viewport_size: Size,
        point: Offset,
        kind: PointerDeviceKind,
    ) -> Option<RawScrollbarPart> {
        let geometry = self.geometry(viewport_size);
        if !self.style.interactive || !geometry.visible {
            return None;
        }
        let (thumb, track) = match kind {
            PointerDeviceKind::Touch | PointerDeviceKind::Trackpad => (
                padded_interactive_rect(geometry.thumb, geometry.thumb),
                padded_interactive_rect(geometry.track, geometry.thumb),
            ),
            _ => (geometry.thumb, geometry.track),
        };
        if thumb.contains(point) {
            Some(RawScrollbarPart::Thumb)
        } else if track.contains(point) {
            Some(RawScrollbarPart::Track)
        } else {
            None
        }
    }

    pub(crate) fn begin_thumb_drag(&mut self, viewport_size: Size, point: Offset) {
        let thumb = self.geometry(viewport_size).thumb;
        let axis = self.resolved_axis();
        let grab = axis.main_extent_of(point) - axis.main_extent_of(thumb.origin);
        self.drag = Some(ThumbDrag {
            grab_offset: grab.clamp(0., axis.main_extent(thumb.size)),
            activity: self
                .controller
                .start_owned_activity(ActivityOrigin::Scrollbar),
        });
        // An owned thumb press holds the scrollbar in view until it ends.
        self.fade.queue(ScrollbarFadeCommand::CancelTimer);
        self.fade.queue(ScrollbarFadeCommand::Forward);
    }

    fn track_page_direction(&self, viewport_size: Size, point: Offset) -> Option<f32> {
        let geometry = self.geometry(viewport_size);
        let axis = self.resolved_axis();
        let along = axis.main_extent_of(point);
        let start = axis.main_extent_of(geometry.thumb.origin);
        let direction = if along < start {
            -1.0
        } else if along > start + axis.main_extent(geometry.thumb.size) {
            1.0
        } else {
            return None;
        };
        let reversed = self.controller.metrics().axis_direction.is_reversed();
        Some(if reversed { -direction } else { direction })
    }

    fn page_track(&self, viewport_size: Size, point: Offset) {
        if let Some(direction) = self.track_page_direction(viewport_size, point) {
            self.controller
                .scroll_by(direction * self.geometry(viewport_size).viewport_extent);
        }
    }

    pub(crate) fn animate_track_page(&self, viewport_size: Size, point: Offset) -> bool {
        let Some(direction) = self.track_page_direction(viewport_size, point) else {
            return false;
        };
        let metrics = self.controller.metrics();
        // Flutter ScrollAction pages by 80% of the viewport. RawScrollbar
        // uses ScrollPosition.moveTo, which clamps the target before driving
        // the 100ms ease-in-out motion instead of overshooting and bouncing.
        let target = (metrics.pixels + direction * 0.8 * metrics.viewport_dimension)
            .clamp(metrics.min_scroll_extent, metrics.max_scroll_extent);
        target != metrics.pixels
            && self
                .controller
                .animate_to(target, Duration::from_millis(100), Curve::EaseInOut)
    }

    /// Updates a thumb drag and returns whether it moved the controller.
    pub fn pointer_move(&mut self, viewport_size: Size, point: Offset) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let geometry = self.geometry(viewport_size);
        if !drag.activity.is_current()
            || geometry.thumb_travel <= 0.0
            || geometry.scroll_range() <= 0.0
        {
            return false;
        }
        let axis = self.resolved_axis();
        let track_start = self.style.main_axis_margin;
        let point_main = axis.main_extent_of(point);
        let desired_thumb =
            (point_main - drag.grab_offset - track_start).clamp(0.0, geometry.thumb_travel);
        let mut fraction = desired_thumb / geometry.thumb_travel;
        if self.controller.metrics().axis_direction.is_reversed() {
            fraction = 1.0 - fraction;
        }
        self.controller.jump_to(geometry.offset_at(fraction))
    }

    /// Ends the active thumb drag, closing its activity with `End`. A
    /// release with no live drag — track clicks, stray releases — emits
    /// nothing.
    pub fn pointer_up(&mut self) {
        self.end_drag_activity();
    }

    /// Cancels the active thumb drag. Cancellation ends the genuine user
    /// interaction, so it closes the bracket exactly like a release.
    pub fn pointer_cancel(&mut self) {
        self.end_drag_activity();
    }

    /// Returns whether a thumb drag is active.
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Maps a track coordinate directly into a logical offset.
    ///
    /// One-shot programmatic move like a track click: unbracketed by
    /// design, composable inside an activity the caller owns.
    pub fn scroll_to_thumb_position(&self, viewport_size: Size, main_position: f32) -> bool {
        let geometry = self.geometry(viewport_size);
        if !geometry.visible || geometry.thumb_travel <= 0.0 {
            return false;
        }
        let axis = self.resolved_axis();
        let track_start = self.style.main_axis_margin;
        let mut fraction =
            (main_position - track_start - axis.main_extent_of(geometry.thumb.size) / 2.0)
                .clamp(0.0, geometry.thumb_travel)
                / geometry.thumb_travel;
        if self.controller.metrics().axis_direction.is_reversed() {
            fraction = 1.0 - fraction;
        }
        self.controller.jump_to(geometry.offset_at(fraction))
    }

    /// Returns whether the given pointer is over the interactive thumb.
    #[must_use]
    pub fn hit_test(&self, viewport_size: Size, point: Offset) -> bool {
        self.geometry(viewport_size).thumb_contains(point)
    }
}

trait AxisGeometryExt {
    fn main_extent_of(self, value: impl MainExtentValue) -> f32;
}

trait MainExtentValue {
    fn main_extent(self, axis: Axis) -> f32;
}

impl MainExtentValue for Offset {
    fn main_extent(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }
}

impl MainExtentValue for Size {
    fn main_extent(self, axis: Axis) -> f32 {
        axis.main_extent(self)
    }
}

impl AxisGeometryExt for Axis {
    fn main_extent_of(self, value: impl MainExtentValue) -> f32 {
        value.main_extent(self)
    }
}

/// `ScrollbarPainter.hitTestInteractive`'s padded rectangle: the track
/// expanded to include a `_kMinInteractiveSize` square around the thumb.
fn padded_interactive_rect(track: Rect, thumb: Rect) -> Rect {
    let center = Offset::new(
        thumb.origin.x + thumb.size.width / 2.0,
        thumb.origin.y + thumb.size.height / 2.0,
    );
    let half = MIN_INTERACTIVE_SIZE / 2.0;
    let left = track.origin.x.min(center.x - half);
    let top = track.origin.y.min(center.y - half);
    let right = (track.origin.x + track.size.width).max(center.x + half);
    let bottom = (track.origin.y + track.size.height).max(center.y + half);
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

fn finite_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}
