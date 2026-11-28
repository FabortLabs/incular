use std::{cell::RefCell, rc::Rc};

use incular_core::{RestorationKey, RestorationScope};

use crate::{
    animation_completion::{PendingScrollAnimation, ScrollAnimationCompletion, ScrollAnimationSignal},
    attachment::{AttachmentConflict, MetricAttachment, MetricOwner, StoredAttachment},
    ballistic::FlingSimulation,
    notifications::{ScrollNotificationListener, ScrollNotificationType},
    physics::ScrollPhysics,
    restoration::{ScrollRestoration, persist_scroll_offset, restored_scroll_offset},
    scrollbar::ScrollbarStyle,
};

/// Flutter `PageMetrics` pad that centres pages when `viewportFraction` is
/// above one (`ScrollPosition._initialPageOffset`).
fn initial_page_pixel_offset(viewport: f32, viewport_fraction: f32) -> f32 {
    (viewport * (viewport_fraction - 1.) / 2.).max(0.)
}

/// Flutter `PageController` mapping from a page number to a pixel offset
/// (`_PagePosition.getPixelsFromPage`).
#[must_use]
pub fn pixels_for_page(page: f32, viewport: f32, viewport_fraction: f32) -> f32 {
    page * viewport * viewport_fraction + initial_page_pixel_offset(viewport, viewport_fraction)
}

/// Flutter `PageController` mapping from a pixel offset to a fractional page
/// (`_PagePosition.getPageFromPixels`). Returns zero for a degenerate page.
#[must_use]
pub fn page_for_pixels(pixels: f32, viewport: f32, viewport_fraction: f32) -> f32 {
    page_for_resized_viewport(pixels, viewport, viewport, viewport_fraction)
}

/// Flutter applies its new viewport dimension before decoding the old pixels.
/// The denominator still uses the previous extent, while the centering origin
/// already belongs to the new viewport.
pub(crate) fn page_for_resized_viewport(
    pixels: f32,
    old_viewport: f32,
    new_viewport: f32,
    viewport_fraction: f32,
) -> f32 {
    let extent = old_viewport * viewport_fraction;
    if extent.partial_cmp(&0.) != Some(std::cmp::Ordering::Greater) {
        return 0.;
    }
    (pixels - initial_page_pixel_offset(new_viewport, viewport_fraction)).max(0.) / extent
}

/// A starting position applied on the first extent publication, below any
/// explicit deferred jump and above nothing else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum InitialOffset {
    /// Flutter `ScrollController.initialScrollOffset`.
    Pixels(f32),
    /// Flutter `PageController.initialPage`, resolved against the viewport.
    Page {
        page: f32,
        viewport_fraction: f32,
        priority: PageStartPriority,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PageStartPriority {
    Configured,
    Saved,
    Explicit,
}

impl InitialOffset {
    pub(crate) fn resolve(self, viewport: f32) -> f32 {
        match self {
            Self::Pixels(offset) => offset,
            Self::Page {
                page,
                viewport_fraction,
                ..
            } => pixels_for_page(page, viewport, viewport_fraction),
        }
    }
}

/// A request to move a scroll position along a curve, made by
/// [`ScrollController::animate_to`] and started by the viewport that owns
/// the controller on its next frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollAnimation {
    /// The offset to reach, in logical pixels.
    pub target: f32,
    /// How long the motion takes.
    pub duration: std::time::Duration,
    /// The easing of the motion.
    pub curve: incular_animation::Curve,
}

type ViewportLifecycleCallback = Rc<dyn Fn(&ScrollController)>;

#[derive(Clone, Default)]
struct ViewportLifecycle {
    on_attach: Option<ViewportLifecycleCallback>,
    on_detach: Option<ViewportLifecycleCallback>,
}

/// Cloneable state for a logical vertical scroll position.
#[derive(Clone)]
pub struct ScrollController {
    pub(crate) changes: incular_core::reactivity::DependencySource,
    pub(crate) state: Rc<RefCell<ScrollState>>,
    lifecycle: Option<Rc<ViewportLifecycle>>,
}
impl std::fmt::Debug for ScrollController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollController")
            .field("offset", &self.offset())
            .field("max_offset", &self.max_offset())
            .finish()
    }
}
impl PartialEq for ScrollController {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Default for ScrollController {
    fn default() -> Self {
        // Matches Flutter's `RawScrollbar.thumbVisibility == null` (hidden by
        // default), so the thumb is hover/drag revealed. Callers opt into an
        // always-visible thumb explicitly.
        let state = ScrollState {
            scrollbar_thumb_visibility: false,
            keep_scroll_offset: true,
            ..ScrollState::default()
        };
        Self::from_shared_position(
            Rc::new(RefCell::new(state)),
            incular_core::reactivity::DependencySource::default(),
        )
    }
}

#[derive(Clone)]
pub(crate) struct FixedExtentStart {
    item: usize,
    fallback_extent: Option<f32>,
    /// The outer option is unset until the first publication. The inner
    /// optional ID distinguishes a headless position from a mounted one.
    applied_to: Option<Option<u64>>,
}

impl FixedExtentStart {
    pub(crate) fn take_position(
        &mut self,
        attachment: Option<u64>,
        extent: Option<f32>,
    ) -> Option<f32> {
        let extent = extent.or(self.fallback_extent)?;
        if self.applied_to == Some(attachment) {
            return None;
        }
        self.applied_to = Some(attachment);
        Some(self.item as f32 * extent)
    }
}

#[derive(Default)]
pub(crate) struct ScrollState {
    pub(crate) offset: f32,
    /// The smallest in-range offset: zero, or negative for a viewport with
    /// content before its center (Flutter `minScrollExtent`).
    pub(crate) min_offset: f32,
    pub(crate) max_offset: f32,
    pub(crate) content_extent: f32,
    pub(crate) viewport_extent: f32,
    /// The retained visual contract for an overlay scrollbar attached to this
    /// position. Keeping it with the shared controller means a themed
    /// scrollbar and the viewport always observe the same scroll state.
    pub(crate) scrollbar_style: ScrollbarStyle,
    pub(crate) scrollbar_thumb_visibility: bool,
    pub(crate) revision: u64,
    /// Authoritative metric owner: the live attachment driving geometry,
    /// if any. Plain data by design — widget element types must not leak
    /// into this crate. The attachment identity decides release; the
    /// recorded tree only names the owner in conflict reports. Clones,
    /// scrollbar readers, and coordination handles never touch it. No
    /// global registry: one slot per controller.
    pub(crate) metric_attachment: Option<StoredAttachment>,
    pub(crate) restoration: Option<ScrollRestoration>,
    pub(crate) pending_restored_offset: Option<f32>,
    pub(crate) pending_jump_offset: Option<f32>,
    /// Starting position applied on the first viewport publication.
    pub(crate) pending_initial_offset: Option<InitialOffset>,
    /// A mounted PageView preserves its logical page when no pixel mapping
    /// exists because the viewport extent is zero.
    pub(crate) cached_zero_page: Option<f32>,
    /// A fixed-item controller's default applies to each new position.
    pub(crate) fixed_extent_start: Option<Box<FixedExtentStart>>,

    /// Set when a jump left the position beyond an extent of an attached
    /// viewport: the viewport springs it back on its next frame (Flutter's
    /// `goBallistic(0)` after `jumpTo`).
    pub(crate) pending_settle: bool,
    /// The idle PageView just enabled snapping; settle with its newly
    /// published viewport-based page geometry.
    pub(crate) pending_page_snap: bool,
    /// Set while a position beyond an extent is springing back, so unchanged
    /// republications keep it until it re-enters the range.
    pub(crate) springing_back: bool,
    /// The physics the attached viewport last published, used to build the
    /// spring back into range.
    pub(crate) last_physics: Option<ScrollPhysics>,
    /// The latest [`ScrollController::animate_to`] not yet taken by the
    /// viewport that drives this controller.
    pub(crate) pending_animation: Option<PendingScrollAnimation>,
    pub(crate) notification_listeners: Rc<RefCell<Vec<(u64, ScrollNotificationListener)>>>,
    pub(crate) position_changes: Option<incular_core::ChangeNotifier>,
    /// The drag driving the current update, for notifications.
    pub(crate) drag_position: Option<incular_core::Offset>,
    pub(crate) next_notification_listener: u64,
    pub(crate) notification_context: Option<(incular_config::Axis, bool)>,
    pub(crate) activity_active: bool,
    pub(crate) wheel_scrolling_depth: usize,
    pub(crate) activity_changes: Option<incular_core::ChangeNotifier>,
    /// Last offset direction emitted by accepted user input, held through a drag
    /// and its
    /// ballistic continuation. Zero means idle.
    pub(crate) activity_user_direction: i8,
    /// Latest activity generation. Bumped on every bracket start and
    /// every takeover; tokens compare against it to prove currency.
    pub(crate) activity_generation: u64,
    /// Flutter `ScrollActivity.velocity` of the activity that set it,
    /// tagged with that activity's generation so it lapses when the
    /// bracket closes or another activity takes over.
    pub(crate) activity_velocity: Option<(u64, f32)>,
    /// Flutter `ScrollPosition._impliedVelocity`: the distance of the last
    /// jump, cleared after the frame that follows it.
    pub(crate) implied_velocity: f32,
    /// Whether a post-frame callback will clear `implied_velocity`.
    pub(crate) implied_velocity_reset_scheduled: bool,
    /// Flutter `ScrollController.keepScrollOffset`.
    pub(crate) keep_scroll_offset: bool,
    /// Takes over user drags and the ballistic after them (Flutter custom
    /// `ScrollPosition`s such as `NestedScrollView` and
    /// `DraggableScrollableSheet` positions).
    pub(crate) drive_delegate: Option<Rc<dyn crate::coordinator::ScrollDriveDelegate>>,
    /// The `NestedScrollView` this controller is the outer position of.
    pub(crate) nested_outer: Option<crate::coordinator::NestedScrollViewCoordinator>,
    /// A ballistic velocity handed to this position by a delegate, started
    /// by the owning viewport on its next frame.
    pub(crate) pending_ballistic: Option<(f32, crate::physics::ScrollPhysics)>,
}

impl ScrollState {
    pub(crate) fn queue_idle_page_snap(&mut self, physics: ScrollPhysics) -> bool {
        if matches!(physics.snap, crate::physics::SnapPhysics::PageViewport)
            && self.metric_attachment.is_some()
            && self.viewport_extent > 0.
            && !self.activity_active
            && self.pending_animation.is_none()
            && !self.pending_settle
            && self.offset > self.min_offset
            && self.offset < self.max_offset
            && physics.snap_target_for_extent(
                self.offset,
                0.,
                self.min_offset,
                self.max_offset,
                self.viewport_extent,
            ) != self.offset
        {
            self.pending_page_snap = true;
            true
        } else {
            false
        }
    }
}

impl ScrollController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates a scroll controller that restores its position, if present.
    ///
    /// The saved logical offset is applied after the first layout establishes
    /// content bounds. The widgets crate's `PageController` alias gains the
    /// same opt-in behavior.
    #[must_use]
    pub fn restored(scope: RestorationScope, key: RestorationKey) -> Self {
        let controller = Self::new();
        controller.bind_restoration(scope, key);
        controller
    }
    /// Flutter `keepScrollOffset` (default true): whether a scroll view with a
    /// `PageStorageKey` saves this position into the nearest `PageStorage`
    /// when scrolling ends and restores it when the view is rebuilt.
    #[must_use]
    pub fn with_keep_scroll_offset(self, keep: bool) -> Self {
        self.state.borrow_mut().keep_scroll_offset = keep;
        self
    }

    #[must_use]
    pub fn keep_scroll_offset(&self) -> bool {
        self.state.borrow().keep_scroll_offset
    }

    /// Flutter `ScrollController.initialScrollOffset`: the position the
    /// controller starts at. It is applied on the first publication of a
    /// viewport's extents and clamped to them, like Flutter's first
    /// `applyContentDimensions`. A saved restoration value takes precedence,
    /// as Flutter's `PageStorage` restoration does.
    #[must_use]
    pub fn with_initial_scroll_offset(self, offset: f32) -> Self {
        if offset.is_finite() {
            self.state.borrow_mut().pending_initial_offset = Some(InitialOffset::Pixels(offset));
        }
        self
    }

    /// Configures a starting item for each newly attached fixed-extent view.
    /// The actual viewport extent per item takes precedence over a fallback.
    #[doc(hidden)]
    #[must_use]
    pub fn with_fixed_initial_item(self, item: usize, fallback_extent: Option<f32>) -> Self {
        self.state.borrow_mut().fixed_extent_start = Some(Box::new(FixedExtentStart {
            item,
            fallback_extent: fallback_extent.filter(|extent| extent.is_finite() && *extent > 0.),
            applied_to: None,
        }));
        self
    }

    /// Initial fixed item used by a wheel's selection notification policy.
    #[doc(hidden)]
    #[must_use]
    pub fn fixed_initial_item(&self) -> Option<usize> {
        self.state
            .borrow()
            .fixed_extent_start
            .as_ref()
            .map(|start| start.item)
    }

    /// Defers an explicit page request until a non-zero viewport can
    /// translate it to pixels. It takes precedence over the initial page
    /// and a saved PageStorage position.
    pub fn defer_page(&self, page: f32, viewport_fraction: f32) -> bool {
        self.set_starting_page(page, viewport_fraction, PageStartPriority::Explicit)
    }

    #[doc(hidden)]
    pub fn configure_initial_page(&self, page: f32, viewport_fraction: f32) -> bool {
        self.set_starting_page(page, viewport_fraction, PageStartPriority::Configured)
    }

    #[doc(hidden)]
    pub fn restore_page_from_storage(&self, page: f32, viewport_fraction: f32) -> bool {
        self.set_starting_page(page, viewport_fraction, PageStartPriority::Saved)
    }

    fn set_starting_page(
        &self,
        page: f32,
        viewport_fraction: f32,
        priority: PageStartPriority,
    ) -> bool {
        if !(page.is_finite() && viewport_fraction.is_finite() && viewport_fraction > 0.) {
            return false;
        }
        {
            let mut state = self.state.borrow_mut();
            if matches!(state.pending_initial_offset, Some(InitialOffset::Page { priority: current, .. }) if current > priority)
            {
                return false;
            }
            if state.cached_zero_page.is_some() {
                state.cached_zero_page = Some(page);
            }
            state.pending_initial_offset = Some(InitialOffset::Page {
                page,
                viewport_fraction,
                priority,
            });
        }
        self.changes.notify();
        true
    }

    /// Binds this controller's logical offset to a stable restoration value.
    ///
    /// Binding a second scope replaces the previous binding. No value is
    /// written until a position actually changes, so defaults remain defaults
    /// when an application never scrolls the viewport.
    pub fn bind_restoration(&self, scope: RestorationScope, key: RestorationKey) {
        let stored = scope.get_json(&key);
        let pending_restored_offset = match stored {
            None => None,
            Some(value) => match restored_scroll_offset(value) {
                Some(offset) => Some(offset),
                None => {
                    scope.note_restoration_outcome(0, 1);
                    None
                }
            },
        };
        let mut state = self.state.borrow_mut();
        state.restoration = Some(ScrollRestoration { scope, key });
        state.pending_restored_offset = pending_restored_offset;
    }
    /// Stops persisting subsequent position changes without removing the
    /// already-stored restoration value.
    pub fn unbind_restoration(&self) {
        let mut state = self.state.borrow_mut();
        state.restoration = None;
        state.pending_restored_offset = None;
    }
    #[must_use]
    pub fn offset(&self) -> f32 {
        self.changes.track();
        self.state.borrow().offset
    }
    /// The smallest in-range offset (Flutter `minScrollExtent`): zero unless
    /// the viewport lays out content before its center sliver.
    #[must_use]
    pub fn min_offset(&self) -> f32 {
        self.changes.track();
        self.state.borrow().min_offset
    }
    #[must_use]
    pub fn max_offset(&self) -> f32 {
        self.changes.track();
        self.state.borrow().max_offset
    }
    #[must_use]
    pub fn content_extent(&self) -> f32 {
        self.changes.track();
        self.state.borrow().content_extent
    }
    #[must_use]
    pub fn viewport_extent(&self) -> f32 {
        self.changes.track();
        self.state.borrow().viewport_extent
    }

    /// Reads the actual mounted page geometry. A detached PageView has no
    /// position, even though its generic ScrollController keeps raw metrics.
    /// Headless metric publications still use the caller's configured
    /// fraction.
    #[doc(hidden)]
    #[must_use]
    pub fn page_for_controller(&self, fallback_fraction: f32) -> Option<f32> {
        self.changes.track();
        let state = self.state.borrow();
        if state
            .metric_attachment
            .is_some_and(|live| live.pending_page_dimensions)
        {
            return None;
        }
        let fraction = match state
            .last_physics
            .and_then(ScrollPhysics::page_geometry_fraction)
        {
            Some(fraction) => {
                state.metric_attachment?;
                fraction
            }
            None => fallback_fraction,
        };
        logical_page(&state, fraction)
    }

    /// The current logical page of a mounted PageView position, even if its
    /// snapping physics are disabled.
    #[doc(hidden)]
    #[must_use]
    pub fn page_for_handoff(&self) -> Option<f32> {
        let state = self.state.borrow();
        if state.metric_attachment?.pending_page_dimensions {
            return None;
        }
        let fraction = state.last_physics?.page_geometry_fraction()?;
        logical_page(&state, fraction)
    }
}

fn logical_page(state: &ScrollState, fraction: f32) -> Option<f32> {
    if state.viewport_extent == 0. {
        return state.cached_zero_page;
    }
    (state.viewport_extent > 0.).then(|| {
        page_for_pixels(
            state.offset.clamp(state.min_offset, state.max_offset),
            state.viewport_extent,
            fraction,
        )
    })
}

impl ScrollController {
    /// Monotonic state revision for retained viewports. Layout owners use this
    /// to refresh sliver cache windows and pinned placements without requiring
    /// the application to rebuild its widget description.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state.borrow().revision
    }
}

impl ScrollController {
    /// Silently clears an open activity without notifying, for tree
    /// disposal: listeners belong to torn-down context by then, and
    /// notifying from `Drop` would risk panics during unwinding. The next
    /// `begin_activity` starts fresh instead of bricking on a stuck flag.
    /// Framework-internal.
    #[doc(hidden)]
    pub fn abort_activity(&self) -> bool {
        let mut state = self.state.borrow_mut();
        if state.activity_active {
            state.activity_active = false;
            state.activity_user_direction = 0;
            true
        } else {
            false
        }
    }

    /// Creates a position-only coordination handle, without mount callbacks.
    pub(crate) fn from_shared_position(
        state: Rc<RefCell<ScrollState>>,
        changes: incular_core::reactivity::DependencySource,
    ) -> Self {
        Self {
            state,
            changes,
            lifecycle: None,
        }
    }

    /// Registers the callback invoked when a viewport attaches to this handle.
    /// Configure before mounting the view. Content extents for a fresh
    /// position may not be available yet.
    #[must_use]
    pub fn with_on_attach(mut self, callback: impl Fn(&Self) + 'static) -> Self {
        Rc::make_mut(self.lifecycle.get_or_insert_with(Rc::default)).on_attach =
            Some(Rc::new(callback));
        self
    }

    /// Registers the callback invoked just before a mounted viewport
    /// detaches, while its controller still exposes the position.
    /// Configure before mounting the view. Silent destruction of a widget
    /// tree does not invoke application callbacks.
    #[must_use]
    pub fn with_on_detach(mut self, callback: impl Fn(&Self) + 'static) -> Self {
        Rc::make_mut(self.lifecycle.get_or_insert_with(Rc::default)).on_detach =
            Some(Rc::new(callback));
        self
    }

    /// Whether a live viewport currently uses this controller.
    #[must_use]
    pub fn has_clients(&self) -> bool {
        self.state.borrow().metric_attachment.is_some()
    }

    /// Whether the active metric writer is a PageView rather than a
    /// different scrollable reusing this underlying controller.
    #[doc(hidden)]
    #[must_use]
    pub fn has_page_client(&self) -> bool {
        self.state
            .borrow()
            .metric_attachment
            .is_some_and(|attachment| attachment.page_view)
    }

    /// The live PageView's usable viewport extent. Fresh and zero-sized
    /// positions defer page commands instead of using an older viewport.
    #[doc(hidden)]
    #[must_use]
    pub fn live_page_viewport_extent(&self) -> Option<f32> {
        let state = self.state.borrow();
        state
            .metric_attachment
            .filter(|attachment| attachment.page_view && attachment.viewport_published)
            .map(|_| state.viewport_extent)
            .filter(|extent| *extent > 0.0)
    }

    pub(crate) fn notify_viewport_lifecycle(&self, attached: bool) {
        if let Some(lifecycle) = &self.lifecycle {
            let callback = if attached {
                &lifecycle.on_attach
            } else {
                &lifecycle.on_detach
            };
            if let Some(callback) = callback {
                callback(self);
            }
        }
    }

    /// Attaches `owner` as the controller's metric driver, failing when
    /// another attachment is live. An occupied controller rejects every
    /// newcomer — there is no re-acquire and no overwrite — so geometry
    /// always has exactly one writer. The returned handle is the only
    /// key that releases; keep it for the attachment's lifetime.
    /// Framework-internal: viewport layouts call this through their
    /// tree's lease map, never directly per frame.
    #[doc(hidden)]
    pub fn try_attach(&self, owner: MetricOwner) -> Result<MetricAttachment, AttachmentConflict> {
        self.try_attach_viewport(owner, false)
    }

    /// Claims a viewport. For a fresh page position, metrics used to pick the
    /// first children are provisional until those children have been laid out.
    #[doc(hidden)]
    pub fn try_attach_viewport(
        &self,
        owner: MetricOwner,
        pending_page_dimensions: bool,
    ) -> Result<MetricAttachment, AttachmentConflict> {
        self.try_attach_position(owner, pending_page_dimensions.then_some(true))
    }

    /// Claims a regular viewport, or a PageView with the given pending
    /// content-dimensions flag. A page handoff can already know its page
    /// without having published the replacement viewport's geometry.
    #[doc(hidden)]
    pub fn try_attach_position(
        &self,
        owner: MetricOwner,
        page_position: Option<bool>,
    ) -> Result<MetricAttachment, AttachmentConflict> {
        let mut state = self.state.borrow_mut();
        if let Some(live) = state.metric_attachment {
            return Err(AttachmentConflict::for_owner(live));
        }
        let stored = StoredAttachment::mint(owner.tree(), page_position);
        state.metric_attachment = Some(stored);
        Ok(MetricAttachment::new(self.clone(), stored))
    }

    /// Returns the widget-tree id currently owning metric publication,
    /// if any. Diagnostic only: naming the owner grants no power to
    /// release it. Framework-internal.
    #[doc(hidden)]
    #[must_use]
    pub fn metric_owner(&self) -> Option<u64> {
        self.state.borrow().metric_attachment.map(|live| live.tree)
    }

    /// Returns the live attachment's identity, if any. Diagnostic only:
    /// trees match this against their leases to report which viewport
    /// owns the controller. Framework-internal.
    #[doc(hidden)]
    #[must_use]
    pub fn attachment_id(&self) -> Option<u64> {
        self.state.borrow().metric_attachment.map(|live| live.id)
    }
}

impl ScrollController {
    /// Applies a sliver viewport's in-layout correction before its new
    /// content extents are known. This is Flutter's `ViewportOffset.correctBy`:
    /// it neither clamps against stale metrics nor sends a scroll event.
    #[doc(hidden)]
    pub fn apply_layout_correction_by(&self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let mut state = self.state.borrow_mut();
        let offset = state.offset + delta;
        if !offset.is_finite() || offset == state.offset {
            return false;
        }
        state.offset = offset;
        state.pending_restored_offset = None;
        state.revision += 1;
        true
    }

    /// Flutter `ScrollPosition.jumpTo`: moves the offset without animation.
    ///
    /// When the position changes outside an open activity bracket, one
    /// `Start`, `Update` and `End` follow, as Flutter's `jumpTo` dispatches
    /// them. Inside an open bracket a jump reports a bare `Update` and leaves
    /// the bracket to its owner, so gesture and fling takeover keep their
    /// ordering. A position attached to a viewport is not clamped: beyond an
    /// extent it is sprung back on the viewport's next frame, as Flutter's
    /// `goBallistic(0)` does. Without an attached viewport nothing would
    /// spring it back, so it is clamped. Returns whether the offset changed.
    pub fn jump_to(&self, offset: f32) -> bool {
        if !offset.is_finite() {
            return false;
        }
        // A jump replaces animations and springs that have not started yet.
        let (bracket_open, replaced) = {
            let mut state = self.state.borrow_mut();
            let replaced = state.pending_animation.take();
            state.pending_settle = false;
            state.pending_page_snap = false;
            state.pending_ballistic = None;
            (state.activity_active, replaced)
        };
        let (moved, value, delta, persistence, spring_back) = {
            let mut state = self.state.borrow_mut();
            let attached = state.metric_attachment.is_some();
            let value = if attached {
                offset
            } else {
                offset.clamp(state.min_offset, state.max_offset)
            };
            let previous = state.offset;
            let moved = value != previous;
            if moved {
                state.offset = value;
                state.pending_restored_offset = None;
                state.revision += 1;
            }
            let in_range = (state.min_offset..=state.max_offset).contains(&state.offset);
            let persistence = (moved && in_range)
                .then(|| state.restoration.clone())
                .flatten();
            let spring_back = attached && !in_range;
            state.pending_settle = spring_back;
            state.springing_back = spring_back;
            (moved, value, value - previous, persistence, spring_back)
        };
        if moved {
            self.imply_velocity(delta);
            persist_scroll_offset(persistence, value);
            self.notify_position_changed();
            if !bracket_open {
                self.dispatch_notification(ScrollNotificationType::Start, 0., 0.);
            }
            self.dispatch_scroll_notification(ScrollNotificationType::Update, delta, 0.);
            if !bracket_open {
                self.dispatch_notification(ScrollNotificationType::End, 0., 0.);
            }
        }
        if spring_back {
            // Wake the runtime so the attached viewport starts the spring.
            self.changes.notify();
        }
        // A completion can wake an executor. Publish the new offset and
        // release all controller borrows before alerting it.
        drop(replaced);
        moved
    }

    /// Records a jump of `delta` as Flutter's `forcePixels` does: until the
    /// frame after it ends, the jump counts as velocity for
    /// [`recommend_deferred_loading`](Self::recommend_deferred_loading).
    fn imply_velocity(&self, delta: f32) {
        let schedule = {
            let mut state = self.state.borrow_mut();
            state.implied_velocity = delta;
            !std::mem::replace(&mut state.implied_velocity_reset_scheduled, true)
        };
        if schedule {
            let state = Rc::downgrade(&self.state);
            incular_animation::scheduler::add_post_frame_callback(move |_| {
                if let Some(state) = state.upgrade() {
                    let mut state = state.borrow_mut();
                    state.implied_velocity = 0.;
                    state.implied_velocity_reset_scheduled = false;
                }
            });
        }
    }

    /// Flutter `ScrollActivity.velocity`: the velocity of the activity
    /// driving this position, in logical pixels per second. Zero while idle
    /// and during drags, which Flutter reports as having no velocity.
    #[must_use]
    pub fn activity_velocity(&self) -> f32 {
        let state = self.state.borrow();
        match state.activity_velocity {
            Some((generation, velocity))
                if state.activity_active && generation == state.activity_generation =>
            {
                velocity
            }
            _ => 0.,
        }
    }

    /// Flutter `ScrollPosition.recommendDeferredLoading`: asks the physics
    /// the attached viewport last published whether the activity velocity,
    /// plus the distance of a jump in the current frame, is fast enough to
    /// defer expensive work. `physical_longest_side` is the longest side of
    /// the view in physical pixels (Flutter's `View.of(context)`).
    #[must_use]
    pub fn recommend_deferred_loading(&self, physical_longest_side: f32) -> bool {
        let (physics, implied) = {
            let state = self.state.borrow();
            (
                state.last_physics.unwrap_or_default(),
                state.implied_velocity,
            )
        };
        physics
            .recommend_deferred_loading(self.activity_velocity() + implied, physical_longest_side)
    }

    /// Flutter `ScrollPosition.correctPixels`: a layout-time correction of the
    /// offset. It clamps to the content bounds, reports a bare `Update`, and
    /// never starts a spring, because the viewport that asks for it is the
    /// one laying out the content. Returns whether the offset changed.
    pub fn correct_to(&self, offset: f32) -> bool {
        if !offset.is_finite() {
            return false;
        }
        let (moved, delta, persistence) = {
            let mut state = self.state.borrow_mut();
            let value = offset.clamp(state.min_offset, state.max_offset);
            let previous = state.offset;
            let moved = value != previous;
            if moved {
                state.offset = value;
                state.pending_restored_offset = None;
                state.pending_settle = false;
                state.pending_page_snap = false;
                state.springing_back = false;
                state.revision += 1;
            }
            let persistence = moved.then(|| state.restoration.clone()).flatten();
            (moved, value - previous, persistence)
        };
        if moved {
            persist_scroll_offset(persistence, self.state.borrow().offset);
            self.dispatch_scroll_notification(ScrollNotificationType::Update, delta, 0.);
        }
        moved
    }

    /// Flutter's implicit reveal jumps to the target, then starts the
    /// paged position's zero-velocity ballistic if the target is between
    /// pages. It does not change the contract of an explicit Rust jump.
    #[doc(hidden)]
    pub fn reveal_at(&self, offset: f32) {
        if !self.jump_to(offset) {
            return;
        }
        let scheduled = {
            let mut state = self.state.borrow_mut();
            state
                .last_physics
                .is_some_and(|physics| state.queue_idle_page_snap(physics))
        };
        if scheduled {
            self.changes.notify();
        }
    }

    /// Whether the attached viewport has a pending settling simulation.
    #[must_use]
    pub fn has_pending_settle(&self) -> bool {
        let state = self.state.borrow();
        state.pending_settle || state.pending_page_snap || state.pending_ballistic.is_some()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn has_pending_page_snap(&self) -> bool {
        let state = self.state.borrow();
        state.pending_page_snap && !state.pending_settle && state.pending_ballistic.is_none()
    }

    /// Takes a pending ballistic simulation from the current position,
    /// whether to recover from overscroll, settle a paged position, or
    /// run a delegate's handoff. The owning viewport starts the simulation.
    #[doc(hidden)]
    pub fn take_pending_settle(&self) -> Option<FlingSimulation> {
        let mut state = self.state.borrow_mut();
        let snap = std::mem::take(&mut state.pending_page_snap);
        if let Some((velocity, physics)) = state.pending_ballistic.take() {
            state.pending_settle = false;
            return crate::coordinator::clamped_ballistic(
                physics,
                state.offset,
                velocity,
                state.min_offset,
                state.max_offset,
            );
        }
        let physics = state.last_physics.unwrap_or_default();
        if std::mem::take(&mut state.pending_settle) {
            state.springing_back = true;
            physics.ballistic_simulation(state.offset, 0., state.min_offset, state.max_offset)
        } else if snap && !state.activity_active && state.pending_animation.is_none() {
            physics.release_simulation(
                state.offset,
                0.,
                state.min_offset,
                state.max_offset,
                state.viewport_extent,
            )
        } else {
            None
        }
    }

    /// Requests a programmatic position before the viewport has established
    /// its extents.  This is useful for retained page/tab controllers: the
    /// request is applied on the first layout pass instead of being clamped
    /// away when `max_offset` is still zero.
    pub fn deferred_jump_to(&self, offset: f32) -> bool {
        if !offset.is_finite() {
            return false;
        }
        let mut state = self.state.borrow_mut();
        state.pending_jump_offset = Some(offset);
        state.revision = state.revision.checked_add(1).expect("revision exhausted");
        true
    }
    pub fn scroll_by(&self, delta: f32) -> bool {
        self.jump_to(self.offset() + delta)
    }

    /// Flutter `ScrollController.animateTo`: moves the offset to `offset`
    /// over `duration` along `curve`. The viewport that owns this controller
    /// starts the motion on its next frame, replacing any fling, drag-end
    /// ballistic or earlier animation; a pointer going down on the viewport,
    /// a drag, or a [`jump_to`](Self::jump_to) stops it. Like Flutter's
    /// `DrivenScrollActivity`, the motion stops at a content edge it reaches.
    ///
    /// A zero duration jumps immediately. Returns `false` for a non-finite
    /// offset.
    pub fn animate_to(
        &self,
        offset: f32,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
    ) -> bool {
        self.queue_animation(offset, duration, curve, None)
    }

    /// Starts a driven scroll like [`animate_to`](Self::animate_to), with a
    /// future that ends when this particular request finishes or is replaced.
    pub fn animate_to_completion(
        &self,
        offset: f32,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
    ) -> ScrollAnimationCompletion {
        if !offset.is_finite() || duration.is_zero() {
            self.animate_to(offset, duration, curve);
            return ScrollAnimationCompletion::ready();
        }
        let (completion, signal) = ScrollAnimationCompletion::pending();
        self.queue_animation(offset, duration, curve, Some(signal));
        completion
    }

    fn queue_animation(
        &self,
        offset: f32,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
        completion: Option<ScrollAnimationSignal>,
    ) -> bool {
        if !offset.is_finite() {
            return false;
        }
        if duration.is_zero() {
            return self.jump_to(offset);
        }
        let replaced = {
            let mut state = self.state.borrow_mut();
            state.pending_page_snap = false;
            state.pending_animation.replace(PendingScrollAnimation {
                motion: ScrollAnimation {
                    target: offset,
                    duration,
                    curve,
                },
                completion,
            })
        };
        self.changes.notify();
        drop(replaced);
        true
    }

    /// Whether an [`animate_to`](Self::animate_to) request is waiting for
    /// the owning viewport's next frame.
    #[must_use]
    pub fn has_pending_animation(&self) -> bool {
        self.state.borrow().pending_animation.is_some()
    }

    /// Takes the waiting [`animate_to`](Self::animate_to) request.
    /// Framework-internal: the viewport that owns the controller starts it.
    #[doc(hidden)]
    pub fn take_pending_animation(&self) -> Option<ScrollAnimation> {
        self.take_tracked_animation().map(|(motion, _)| motion)
    }

    /// Transfers the request's completion to the owning viewport's driver.
    #[doc(hidden)]
    pub fn take_tracked_animation(
        &self,
    ) -> Option<(ScrollAnimation, Option<ScrollAnimationSignal>)> {
        let pending = self.state.borrow_mut().pending_animation.take()?;
        Some((pending.motion, pending.completion))
    }
}
