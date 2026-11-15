use incular_core::reactivity::DependencySource;
use incular_core::time::Instant;
use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::Color;
use incular_widgets::internal::{OpacityController, RouteHeroes, TranslationController};
use incular_widgets::{
    AnimationController, DoubleAnimation, FadeTransition, ProxyAnimation, SlideTransition, Widget,
    always_dismissed,
};
use typed_builder::TypedBuilder;

use super::{PageKey, RouteId, RouteSettings};

/// A reusable description of a navigable screen.
///
/// Unlike [`Route`], a page has no runtime identity, allowing the same page
/// description to be placed in multiple navigators. The optional key is the
/// page's stable reconciliation identity: keyed pages retain their mounted
/// route across [`set_pages`](super::Navigator::set_pages) builds, while
/// unkeyed pages carry no identity and mount anew on every reconciliation.
/// Presentation and transition ride along declaratively: pushing or
/// reconciling a keyed page with a popup or modal presentation mounts it
/// that way, and same-key updates apply the latest configuration to the
/// retained route (see `set_pages` for the lifetime rule).
#[derive(Clone, TypedBuilder)]
#[builder(builder_type(name = PageDescriptorBuilder))]
pub struct Page {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub child: Widget,
    #[builder(default, setter(strip_option))]
    pub key: Option<PageKey>,
    /// How the route presents: pages, popups, and modals are all
    /// expressible here, so transparent and veiled routes need no
    /// imperative construction.
    #[builder(default)]
    pub presentation: RoutePresentation,
    /// Transition wrapping the presented child.
    #[builder(default = RouteTransition::None)]
    pub transition: RouteTransition,
}

/// Rust-native route presentation composition.
///
/// These variants replace Flutter's `Route`/`ModalRoute`/`PopupRoute` class
/// hierarchy. Core owns the mechanics and Material can supply appearance.
#[derive(Clone)]
pub enum RoutePresentation {
    /// A normal page in the navigator's content area.
    Page {
        opaque: bool,
        maintain_state: bool,
        fullscreen_dialog: bool,
    },
    /// A non-fullscreen overlay presentation.
    Popup {
        barrier: Option<ModalBarrier>,
        maintain_state: bool,
    },
    /// A modal presentation that isolates background input.
    ///
    /// Focus containment was removed: the `focus_trap` flag promised modal
    /// focus trapping that no traversal or dispatch path implemented. Modal routes still block background input
    /// through the barrier; constraining traversal to the modal subtree is
    /// future work with no accepted no-op standing in for it.
    Modal { barrier: ModalBarrier },
    /// A route backed by one or more existing overlay entries.
    Overlay { entries: Vec<OverlayEntry> },
}

impl Default for RoutePresentation {
    fn default() -> Self {
        Self::Page {
            opaque: true,
            maintain_state: true,
            fullscreen_dialog: false,
        }
    }
}

impl RoutePresentation {
    #[must_use]
    pub const fn page() -> Self {
        Self::Page {
            opaque: true,
            maintain_state: true,
            fullscreen_dialog: false,
        }
    }

    #[must_use]
    pub fn popup(barrier: Option<ModalBarrier>) -> Self {
        Self::Popup {
            barrier,
            maintain_state: true,
        }
    }

    #[must_use]
    pub fn modal(barrier: ModalBarrier) -> Self {
        Self::Modal { barrier }
    }

    #[must_use]
    pub fn overlay(entries: impl IntoIterator<Item = OverlayEntry>) -> Self {
        Self::Overlay {
            entries: entries.into_iter().collect(),
        }
    }

    #[must_use]
    pub const fn is_opaque(&self) -> bool {
        matches!(self, Self::Page { opaque: true, .. })
    }

    /// Whether a covered route keeps its mounted content. Pages and popups
    /// follow their flag; modals always retain (a covered dialog keeps its
    /// state); overlay entries are portal-managed, so outlet retention does
    /// not apply to them.
    #[must_use]
    pub const fn maintains_state(&self) -> bool {
        match self {
            Self::Page { maintain_state, .. } | Self::Popup { maintain_state, .. } => {
                *maintain_state
            }
            Self::Modal { .. } => true,
            Self::Overlay { .. } => false,
        }
    }

    /// The veil for content below this route, if it declares one.
    #[must_use]
    pub fn barrier(&self) -> Option<&ModalBarrier> {
        match self {
            Self::Popup { barrier, .. } => barrier.as_ref(),
            Self::Modal { barrier } => Some(barrier),
            Self::Page { .. } | Self::Overlay { .. } => None,
        }
    }

    /// Whether this route's content is portal-managed overlay entries
    /// rather than outlet-mounted stack content.
    #[must_use]
    pub const fn is_overlay(&self) -> bool {
        matches!(self, Self::Overlay { .. })
    }

    #[must_use]
    pub const fn blocks_background_input(&self) -> bool {
        matches!(self, Self::Modal { .. })
    }
}

/// A typed, retained completion channel for a route result.
///
/// The navigator remains synchronous and renderer-independent; applications
/// retain this handle and complete it from their route action. No `Any` or
/// untyped Dart `Object?` is needed.
#[derive(Clone)]
pub struct RouteResult<T> {
    state: Rc<RefCell<Option<T>>>,
}

impl<T> Default for RouteResult<T> {
    fn default() -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
        }
    }
}

impl<T> RouteResult<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Completes the result once. Repeated completion is rejected.
    pub fn complete(&self, value: T) -> Result<(), T> {
        let mut slot = self.state.borrow_mut();
        if slot.is_some() {
            return Err(value);
        }
        *slot = Some(value);
        Ok(())
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state.borrow().is_some()
    }

    #[must_use]
    pub fn take(&self) -> Option<T> {
        self.state.borrow_mut().take()
    }
}
impl Page {
    #[must_use]
    pub fn new(name: impl Into<String>, child: impl Into<Widget>) -> Self {
        Self {
            name: name.into(),
            child: child.into(),
            key: None,
            presentation: RoutePresentation::default(),
            transition: RouteTransition::None,
        }
    }

    /// Assigns the stable reconciliation identity for declarative builds.
    #[must_use]
    pub fn key(mut self, key: PageKey) -> Self {
        self.key = Some(key);
        self
    }

    /// Describes how the route presents. Same-key updates apply the latest
    /// presentation to the retained route; only key removal (or no key)
    /// ends the lifetime.
    #[must_use]
    pub fn presentation(mut self, presentation: RoutePresentation) -> Self {
        self.presentation = presentation;
        self
    }

    /// Wraps the presented child in this transition. Same-key updates
    /// replace the transition on the retained route.
    #[must_use]
    pub fn transition(mut self, transition: RouteTransition) -> Self {
        self.transition = transition;
        self
    }
}

/// A declarative route: an application name and the widget it displays.
#[derive(Clone, TypedBuilder)]
pub struct Route {
    /// Session-local identity assigned when a navigator pushes this presentation.
    #[builder(default = RouteId(0), setter(skip))]
    pub id: RouteId,
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub child: Widget,
    #[builder(default = RouteTransition::None)]
    pub transition: RouteTransition,
    /// Route metadata is initialized from `name` and can be changed through
    /// [`Route::settings`], which keeps the legacy name mirror synchronized.
    #[builder(default = RouteSettings::new((*name).clone()), setter(skip))]
    pub settings: RouteSettings,
    #[builder(default)]
    pub presentation: RoutePresentation,
    /// The route's heroes, shared by its clones.
    #[builder(default, setter(skip))]
    pub(crate) heroes: RouteHeroes,
}

impl Route {
    #[must_use]
    pub fn new(name: impl Into<String>, child: impl Into<Widget>) -> Self {
        let name = name.into();
        Self {
            id: RouteId(0),
            settings: RouteSettings::new(name.clone()),
            name,
            child: child.into(),
            transition: RouteTransition::None,
            presentation: RoutePresentation::default(),
            heroes: RouteHeroes::new(),
        }
    }
    /// Replaces the route metadata and keeps the legacy `name` field in sync.
    #[must_use]
    pub fn settings(mut self, settings: RouteSettings) -> Self {
        self.name = settings.name().to_owned();
        self.settings = settings;
        self
    }

    #[must_use]
    pub fn presentation(mut self, presentation: RoutePresentation) -> Self {
        self.presentation = presentation;
        self
    }

    #[must_use]
    pub fn popup(self, barrier: Option<ModalBarrier>) -> Self {
        self.presentation(RoutePresentation::popup(barrier))
    }

    #[must_use]
    pub fn modal(self, barrier: ModalBarrier) -> Self {
        self.presentation(RoutePresentation::modal(barrier))
    }

    #[must_use]
    pub fn dialog(self) -> Self {
        self.modal(ModalBarrier::default())
    }

    #[must_use]
    pub fn overlay(self, entries: impl IntoIterator<Item = OverlayEntry>) -> Self {
        self.presentation(RoutePresentation::overlay(entries))
    }
    #[must_use]
    pub fn transition(mut self, transition: RouteTransition) -> Self {
        self.transition = transition;
        self
    }
    /// Returns the retained presentation with its transition layer applied.
    /// With a hero controller on the navigator, the child is the route's
    /// hero subtree, inside the transition, so hero locations are in the
    /// route's own coordinates.
    #[must_use]
    pub fn presented_child(&self) -> Widget {
        self.present(self.child.clone())
    }

    /// Presents `content` as this route presents its own child: inside the
    /// route's hero subtree and transition layer. Outlets use it for content
    /// composed from the route's presentation, such as an overlay route's
    /// entries.
    #[must_use]
    pub fn present(&self, content: Widget) -> Widget {
        self.transition.apply(self.heroes.scope(content))
    }

    /// The route's heroes.
    #[must_use]
    pub fn heroes(&self) -> &RouteHeroes {
        &self.heroes
    }
}

/// Ergonomic builder for a page route with a custom transition.
#[derive(Clone, TypedBuilder)]
#[builder(builder_type(name = PageRouteDescriptorBuilder))]
pub struct PageRouteBuilder {
    #[builder(setter(into))]
    name: String,
    #[builder(setter(into))]
    child: Widget,
    /// An explicit transition replaces the default route animation.
    #[builder(default, setter(strip_option))]
    transition: Option<RouteTransition>,
    #[builder(default)]
    presentation: RoutePresentation,
    #[builder(default = RouteSettings::new((*name).clone()), setter(skip))]
    settings: RouteSettings,
    #[builder(default = DEFAULT_PAGE_TRANSITION_DURATION, setter(skip))]
    transition_duration: Duration,
    #[builder(default = DEFAULT_PAGE_TRANSITION_DURATION, setter(skip))]
    reverse_transition_duration: Duration,
    #[builder(default, setter(skip))]
    transitions_builder: Option<PageTransitionBuilder>,
}

/// Flutter `PageRouteBuilder.transitionDuration` and
/// `reverseTransitionDuration` default (`widgets/pages.dart:96-97`).
const DEFAULT_PAGE_TRANSITION_DURATION: Duration = Duration::from_millis(300);

impl PageRouteBuilder {
    #[must_use]
    pub fn new(name: impl Into<String>, child: impl Into<Widget>) -> Self {
        let name = name.into();
        Self {
            settings: RouteSettings::new(name.clone()),
            name,
            child: child.into(),
            transition: None,
            presentation: RoutePresentation::default(),
            transition_duration: DEFAULT_PAGE_TRANSITION_DURATION,
            reverse_transition_duration: DEFAULT_PAGE_TRANSITION_DURATION,
            transitions_builder: None,
        }
    }

    /// Flutter `PageRouteBuilder.transitionDuration`: the forward route
    /// animation's duration, used by the default route animation.
    #[must_use]
    pub fn transition_duration(mut self, duration: Duration) -> Self {
        self.transition_duration = duration;
        self
    }

    /// Flutter `PageRouteBuilder.reverseTransitionDuration`: the duration of
    /// the route's reverse animation when popped. The popped route stays
    /// mounted until it completes.
    #[must_use]
    pub fn reverse_transition_duration(mut self, duration: Duration) -> Self {
        self.reverse_transition_duration = duration;
        self
    }

    /// Flutter `PageRouteBuilder.transitionsBuilder`, used by the default
    /// route animation. An explicit [`Self::transition`] takes precedence.
    #[must_use]
    pub fn transitions_builder(
        mut self,
        builder: impl Fn(&DoubleAnimation, &DoubleAnimation, Widget) -> Widget + 'static,
    ) -> Self {
        self.transitions_builder = Some(Rc::new(builder));
        self
    }

    #[must_use]
    pub fn settings(mut self, settings: RouteSettings) -> Self {
        self.name = settings.name().to_owned();
        self.settings = settings;
        self
    }

    #[must_use]
    pub fn presentation(mut self, presentation: RoutePresentation) -> Self {
        self.presentation = presentation;
        self
    }

    #[must_use]
    pub fn transition(mut self, transition: RouteTransition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Flutter `PageRouteBuilder.opaque` (default true): the route hides
    /// the routes below it. Applies to page presentations.
    #[must_use]
    pub fn opaque(mut self, value: bool) -> Self {
        if let RoutePresentation::Page { opaque, .. } = &mut self.presentation {
            *opaque = value;
        }
        self
    }

    /// Flutter `PageRouteBuilder.maintainState` (default true): a covered
    /// route keeps its state. Applies to page and popup presentations.
    #[must_use]
    pub fn maintain_state(mut self, value: bool) -> Self {
        if let RoutePresentation::Page { maintain_state, .. }
        | RoutePresentation::Popup { maintain_state, .. } = &mut self.presentation
        {
            *maintain_state = value;
        }
        self
    }

    /// Flutter `PageRoute.fullscreenDialog` (default false). Applies to page
    /// presentations.
    #[must_use]
    pub fn fullscreen_dialog(mut self, value: bool) -> Self {
        if let RoutePresentation::Page {
            fullscreen_dialog, ..
        } = &mut self.presentation
        {
            *fullscreen_dialog = value;
        }
        self
    }

    /// Builds the route. Without an explicit transition the route owns a
    /// [`RouteAnimation`] with this builder's durations, so it animates in
    /// and out as Flutter's page routes do, and a popped route stays mounted
    /// until its reverse animation completes.
    #[must_use]
    pub fn build(self) -> Route {
        let transition = match self.transition {
            Some(transition) => transition,
            None => {
                let builder = self.transitions_builder;
                RouteTransition::Animated(RouteAnimation::new(
                    self.transition_duration,
                    self.reverse_transition_duration,
                    move |animation, secondary, child| match &builder {
                        Some(builder) => builder(animation, secondary, child),
                        None => child,
                    },
                ))
            }
        };
        Route::new(self.name, self.child)
            .settings(self.settings)
            .transition(transition)
            .presentation(self.presentation)
    }
}
impl From<Page> for Route {
    fn from(page: Page) -> Self {
        Self::new(page.name, page.child)
            .presentation(page.presentation)
            .transition(page.transition)
    }
}

/// Flutter `PageTransitionsBuilder.buildTransitions`: wraps a route's child
/// given its animation and its secondary animation (the animation of the
/// route pushed above it).
pub type PageTransitionBuilder = Rc<dyn Fn(&DoubleAnimation, &DoubleAnimation, Widget) -> Widget>;

/// Flutter `TransitionRoute`'s animations. The outlet drives them: the
/// animation runs forward when the route is pushed (an initial route starts
/// complete), backward when it is popped, while the route stays presented
/// until it is dismissed; the secondary animation follows the route above.
/// A page only hides the routes below once its animation completes.
#[derive(Clone)]
pub struct RouteAnimation {
    controller: AnimationController,
    secondary: ProxyAnimation,
    builder: PageTransitionBuilder,
}

impl RouteAnimation {
    /// A route animation over `duration` (and `reverse_duration` when
    /// popped) whose transition `builder` wraps the route's child.
    #[must_use]
    pub fn new(
        duration: Duration,
        reverse_duration: Duration,
        builder: impl Fn(&DoubleAnimation, &DoubleAnimation, Widget) -> Widget + 'static,
    ) -> Self {
        Self {
            controller: AnimationController::new(duration)
                .with_reverse_duration(reverse_duration)
                .with_vsync(),
            secondary: ProxyAnimation::new(Some(always_dismissed())),
            builder: Rc::new(builder),
        }
    }

    /// Flutter `TransitionRoute.animation`.
    #[must_use]
    pub fn animation(&self) -> AnimationController {
        self.controller.clone()
    }

    /// Flutter `TransitionRoute.secondaryAnimation`.
    #[must_use]
    pub fn secondary_animation(&self) -> ProxyAnimation {
        self.secondary.clone()
    }

    fn apply(&self, child: Widget) -> Widget {
        let animation = DoubleAnimation::from(self.controller.clone());
        let secondary = DoubleAnimation::new(self.secondary.clone());
        (self.builder)(&animation, &secondary, child)
    }
}

/// Route-level presentation transitions backed by retained controller layers.
#[derive(Clone)]
pub enum RouteTransition {
    None,
    Fade(OpacityController),
    Slide(TranslationController),
    FadeSlide {
        opacity: OpacityController,
        translation: TranslationController,
    },
    /// A transition driven by the route's own animations, as Flutter's
    /// page routes are.
    Animated(RouteAnimation),
}
impl RouteTransition {
    /// Creates a neutral compositor fade-in without exposing the retained
    /// opacity controller. The route owns the animation and the runtime ticks
    /// it along with the rest of the retained tree.
    #[must_use]
    pub fn fade_in(duration: Duration, now: Instant) -> Self {
        let opacity = OpacityController::new();
        opacity.set_opacity(0.);
        opacity.animate_to(1., duration, now);
        Self::Fade(opacity)
    }

    /// Flutter's `TransitionRoute.animation`: the animation of the
    /// transition's controller, or `None` without a transition, which
    /// Flutter's routes model as an animation already completed.
    #[must_use]
    pub fn animation(&self) -> Option<AnimationController> {
        match self {
            Self::None => None,
            Self::Fade(opacity) | Self::FadeSlide { opacity, .. } => Some(opacity.animation()),
            Self::Slide(translation) => Some(translation.animation()),
            Self::Animated(route) => Some(route.animation()),
        }
    }

    #[must_use]
    pub fn apply(&self, child: Widget) -> Widget {
        match self {
            Self::None => child,
            Self::Fade(controller) => FadeTransition::new(controller.clone(), child).into(),
            Self::Slide(controller) => SlideTransition::new(controller.clone(), child).into(),
            Self::FadeSlide {
                opacity,
                translation,
            } => FadeTransition::new(
                opacity.clone(),
                SlideTransition::new(translation.clone(), child),
            )
            .into(),
            Self::Animated(route) => route.apply(child),
        }
    }

    /// The route animation, for transitions the outlet drives.
    #[must_use]
    pub fn route_animation(&self) -> Option<&RouteAnimation> {
        match self {
            Self::Animated(route) => Some(route),
            _ => None,
        }
    }
}

/// Modal overlay configuration. The barrier is input-blocking by default.
#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder)]
pub struct ModalBarrier {
    #[builder(default = Color::rgba(0, 0, 0, 128))]
    pub color: Color,
    #[builder(default = true)]
    pub dismissible: bool,
    #[builder(default, setter(strip_option, into))]
    pub label: Option<String>,
}
impl Default for ModalBarrier {
    fn default() -> Self {
        Self {
            color: Color::rgba(0, 0, 0, 128),
            dismissible: true,
            label: None,
        }
    }
}

/// Stable identity of one [`OverlayEntry`] within the process. Clones of an
/// entry share its identity, so an entry read back from [`Overlay::entries`]
/// can still be removed or positioned against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OverlayEntryId(u64);

static NEXT_OVERLAY_ENTRY_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl OverlayEntryId {
    fn next() -> Self {
        Self(NEXT_OVERLAY_ENTRY_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// An entry in painter-ordered overlay state.
#[derive(Clone, TypedBuilder)]
pub struct OverlayEntry {
    #[builder(setter(into))]
    pub child: Widget,
    #[builder(default, setter(strip_option))]
    pub barrier: Option<ModalBarrier>,
    #[builder(default = OverlayEntryId::next(), setter(skip))]
    id: OverlayEntryId,
}
impl OverlayEntry {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            barrier: None,
            id: OverlayEntryId::next(),
        }
    }

    /// The entry's stable identity, for [`Overlay::remove`] and relative
    /// insertion.
    #[must_use]
    pub fn id(&self) -> OverlayEntryId {
        self.id
    }
    #[must_use]
    pub fn modal(mut self, barrier: ModalBarrier) -> Self {
        self.barrier = Some(barrier);
        self
    }

    #[must_use]
    pub fn blocks_background_input(&self) -> bool {
        self.barrier.is_some()
    }

    #[must_use]
    pub fn dismissible(&self) -> bool {
        self.barrier
            .as_ref()
            .is_none_or(|barrier| barrier.dismissible)
    }
}

/// A modal dialog presentation owned by application overlay state.
#[derive(Clone, TypedBuilder)]
pub struct Dialog {
    #[builder(setter(into))]
    pub child: Widget,
    #[builder(default)]
    pub barrier: ModalBarrier,
}
impl Dialog {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            barrier: ModalBarrier::default(),
        }
    }
    #[must_use]
    pub fn barrier(mut self, barrier: ModalBarrier) -> Self {
        self.barrier = barrier;
        self
    }
    #[must_use]
    pub fn into_entry(self) -> OverlayEntry {
        OverlayEntry::new(self.child).modal(self.barrier)
    }
}

/// A modal bottom-sheet presentation. Positioning remains a widget concern.
#[derive(Clone, TypedBuilder)]
pub struct BottomSheet {
    #[builder(setter(into))]
    pub child: Widget,
    #[builder(default)]
    pub barrier: ModalBarrier,
}
impl BottomSheet {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            barrier: ModalBarrier::default(),
        }
    }
    #[must_use]
    pub fn barrier(mut self, barrier: ModalBarrier) -> Self {
        self.barrier = barrier;
        self
    }
    #[must_use]
    pub fn into_entry(self) -> OverlayEntry {
        OverlayEntry::new(self.child).modal(self.barrier)
    }
}

/// Cloneable painter-ordered overlay state. Reading [`Overlay::entries`] from a
/// build subscribes that build, and every change notifies it, so the host
/// rebuilds when entries are added or removed (Flutter `OverlayState` marks
/// its overlay for a rebuild).
#[derive(Clone, Default)]
pub struct Overlay {
    entries: Rc<RefCell<Vec<OverlayEntry>>>,
    changes: DependencySource,
}
impl Overlay {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends `entry` on top and returns its identity.
    pub fn insert(&self, entry: OverlayEntry) -> OverlayEntryId {
        let id = entry.id;
        self.entries.borrow_mut().push(entry);
        self.changes.notify();
        id
    }
    /// Flutter `OverlayState.insert(entry, above:)`: places `entry` directly
    /// above `anchor`. Returns `None`, inserting nothing, when `anchor` is
    /// absent.
    pub fn insert_above(
        &self,
        anchor: OverlayEntryId,
        entry: OverlayEntry,
    ) -> Option<OverlayEntryId> {
        self.insert_relative(anchor, entry, 1)
    }
    /// Flutter `OverlayState.insert(entry, below:)`: places `entry` directly
    /// below `anchor`. Returns `None`, inserting nothing, when `anchor` is
    /// absent.
    pub fn insert_below(
        &self,
        anchor: OverlayEntryId,
        entry: OverlayEntry,
    ) -> Option<OverlayEntryId> {
        self.insert_relative(anchor, entry, 0)
    }
    fn insert_relative(
        &self,
        anchor: OverlayEntryId,
        entry: OverlayEntry,
        offset: usize,
    ) -> Option<OverlayEntryId> {
        let mut entries = self.entries.borrow_mut();
        let index = entries
            .iter()
            .position(|candidate| candidate.id == anchor)?
            + offset;
        let id = entry.id;
        entries.insert(index, entry);
        drop(entries);
        self.changes.notify();
        Some(id)
    }
    /// Flutter `OverlayEntry.remove`: removes the entry with `id`, wherever
    /// it sits, and leaves the others in place. Returns the removed entry.
    pub fn remove(&self, id: OverlayEntryId) -> Option<OverlayEntry> {
        let mut entries = self.entries.borrow_mut();
        let index = entries.iter().position(|entry| entry.id == id)?;
        let removed = entries.remove(index);
        drop(entries);
        self.changes.notify();
        Some(removed)
    }
    pub fn show_dialog(&self, dialog: Dialog) {
        self.insert(dialog.into_entry());
    }
    pub fn show_bottom_sheet(&self, sheet: BottomSheet) {
        self.insert(sheet.into_entry());
    }
    pub fn remove_top(&self) -> Option<OverlayEntry> {
        let removed = self.entries.borrow_mut().pop();
        if removed.is_some() {
            self.changes.notify();
        }
        removed
    }

    /// Removes the top entry only when it has a dismissible barrier.
    pub fn dismiss_top(&self) -> Option<OverlayEntry> {
        let dismissible = self
            .entries
            .borrow()
            .last()
            .is_some_and(OverlayEntry::dismissible);
        dismissible.then(|| self.remove_top()).flatten()
    }

    /// Whether the current overlay stack must consume pointer/keyboard input
    /// before it reaches the route underneath it.
    #[must_use]
    pub fn blocks_background_input(&self) -> bool {
        self.changes.track();
        self.entries
            .borrow()
            .iter()
            .rev()
            .any(OverlayEntry::blocks_background_input)
    }
    #[must_use]
    pub fn entries(&self) -> Vec<OverlayEntry> {
        self.changes.track();
        self.entries.borrow().clone()
    }
}
