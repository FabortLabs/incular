//! Flutter's `Hero` and `HeroMode` (`widgets/heroes.dart`), and the flight
//! machinery a navigator's hero controller drives: per-route hero
//! registries and the overlay where heroes fly between routes.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

use incular_core::{Lerp, Offset, Rect, Size};
use incular_rendering::LayerLink;

use crate::animation_transitions::AnimationListenerState;
use crate::tree::RetainedState;
use crate::{
    AnimationController, AnimationStatus, BuildContext, CompositedTransformTarget, Curve,
    IgnorePointer, Offstage, Opacity, Positioned, SizedBox, Stack, TickerMode, Widget,
};

/// A hero's identity: heroes with the same tag in the routes of a
/// transition fly from one to the other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HeroTag(Rc<str>);

impl HeroTag {
    #[must_use]
    pub fn new(tag: impl AsRef<str>) -> Self {
        Self(Rc::from(tag.as_ref()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for HeroTag {
    fn from(tag: &str) -> Self {
        Self::new(tag)
    }
}

impl From<String> for HeroTag {
    fn from(tag: String) -> Self {
        Self::new(tag)
    }
}

impl From<u64> for HeroTag {
    fn from(tag: u64) -> Self {
        Self::new(tag.to_string())
    }
}

/// Flutter's `HeroFlightDirection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroFlightDirection {
    /// The flight towards a pushed route.
    Push,
    /// The flight back to the route a pop reveals.
    Pop,
}

/// A hero's rectangle over a flight: Flutter's `Tween<Rect?>`. The default
/// is `RectTween`, which interpolates the rectangles linearly.
#[derive(Clone)]
pub struct HeroRectTween {
    begin: Rect,
    end: Rect,
    lerp: Rc<dyn Fn(Rect, Rect, f32) -> Rect>,
}

impl HeroRectTween {
    /// Flutter's `RectTween`.
    #[must_use]
    pub fn linear(begin: Rect, end: Rect) -> Self {
        Self::custom(begin, end, |begin, end, t| begin.lerp(&end, t))
    }

    /// A tween whose `lerp` gives the rectangle at `t` from `begin` to `end`.
    #[must_use]
    pub fn custom(
        begin: Rect,
        end: Rect,
        lerp: impl Fn(Rect, Rect, f32) -> Rect + 'static,
    ) -> Self {
        Self {
            begin,
            end,
            lerp: Rc::new(lerp),
        }
    }

    #[must_use]
    pub fn begin(&self) -> Rect {
        self.begin
    }

    #[must_use]
    pub fn end(&self) -> Rect {
        self.end
    }

    /// The rectangle at `t`: exactly `begin` at 0 and `end` at 1, as
    /// Flutter's `Tween.transform`.
    #[must_use]
    pub fn transform(&self, t: f32) -> Rect {
        if t == 0. {
            return self.begin;
        }
        if t == 1. {
            return self.end;
        }
        (self.lerp)(self.begin, self.end, t)
    }
}

impl fmt::Debug for HeroRectTween {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HeroRectTween")
            .field("begin", &self.begin)
            .field("end", &self.end)
            .finish_non_exhaustive()
    }
}

/// Flutter's `CreateRectTween`.
pub type CreateRectTween = Rc<dyn Fn(Rect, Rect) -> HeroRectTween>;

/// What a [`HeroFlightShuttleBuilder`] builds the flying widget from.
#[derive(Clone)]
pub struct HeroShuttleContext {
    /// The flight's progress: the route animation through
    /// `Curves.fastOutSlowIn`.
    pub animation: f32,
    pub direction: HeroFlightDirection,
    /// The child of the hero the flight leaves.
    pub from_child: Widget,
    /// The child of the hero the flight lands on.
    pub to_child: Widget,
}

/// Flutter's `HeroFlightShuttleBuilder`: the widget shown in flight. It is
/// built on every frame of the flight. The default shows the destination
/// hero's child.
pub type HeroFlightShuttleBuilder = Rc<dyn Fn(&BuildContext<'_>, &HeroShuttleContext) -> Widget>;

/// Flutter's `HeroPlaceholderBuilder`: what a hero shows in its place while
/// it is in flight, given its size and child.
pub type HeroPlaceholderBuilder = Rc<dyn Fn(&BuildContext<'_>, Size, Widget) -> Widget>;

/// Flutter's `Hero`: a widget that flies to the hero with the same tag in
/// the next route when a navigator's hero controller sees a page route
/// transition.
#[derive(Clone)]
pub struct Hero {
    tag: HeroTag,
    child: Widget,
    create_rect_tween: Option<CreateRectTween>,
    flight_shuttle_builder: Option<HeroFlightShuttleBuilder>,
    placeholder_builder: Option<HeroPlaceholderBuilder>,
    transition_on_user_gestures: bool,
}

impl Hero {
    #[must_use]
    pub fn new(tag: impl Into<HeroTag>, child: impl Into<Widget>) -> Self {
        Self {
            tag: tag.into(),
            child: child.into(),
            create_rect_tween: None,
            flight_shuttle_builder: None,
            placeholder_builder: None,
            transition_on_user_gestures: false,
        }
    }

    /// The flight's rectangle tween when this is the destination hero; it
    /// takes precedence over the controller's.
    #[must_use]
    pub fn create_rect_tween(
        mut self,
        create: impl Fn(Rect, Rect) -> HeroRectTween + 'static,
    ) -> Self {
        self.create_rect_tween = Some(Rc::new(create));
        self
    }

    /// The flying widget. The destination hero's builder takes precedence
    /// over the departing hero's.
    #[must_use]
    pub fn flight_shuttle_builder(
        mut self,
        builder: impl Fn(&BuildContext<'_>, &HeroShuttleContext) -> Widget + 'static,
    ) -> Self {
        self.flight_shuttle_builder = Some(Rc::new(builder));
        self
    }

    /// What this hero shows in its place during a flight. Without it the
    /// hero keeps its size and hides its child.
    #[must_use]
    pub fn placeholder_builder(
        mut self,
        builder: impl Fn(&BuildContext<'_>, Size, Widget) -> Widget + 'static,
    ) -> Self {
        self.placeholder_builder = Some(Rc::new(builder));
        self
    }

    /// Whether the hero flies during a user-gesture route transition, such
    /// as the iOS back swipe. Defaults to false.
    #[must_use]
    pub fn transition_on_user_gestures(mut self, enabled: bool) -> Self {
        self.transition_on_user_gestures = enabled;
        self
    }
}

/// Flutter's `HeroMode`: with `enabled` false, the heroes below it do not
/// fly.
#[derive(Clone)]
pub struct HeroMode {
    enabled: bool,
    child: Widget,
}

impl HeroMode {
    #[must_use]
    pub fn new(enabled: bool, child: impl Into<Widget>) -> Self {
        Self {
            enabled,
            child: child.into(),
        }
    }
}

#[derive(Clone, Copy)]
struct HeroModeScope(bool);

impl From<HeroMode> for Widget {
    fn from(value: HeroMode) -> Self {
        Widget::environment_scope(HeroModeScope(value.enabled), value.child)
    }
}

/// One mounted hero.
struct HeroEntry {
    config: RefCell<Hero>,
    link: LayerLink,
    route: RefCell<Weak<RouteHeroesInner>>,
    enabled: Cell<bool>,
    /// `_placeholderSize`.
    placeholder: Cell<Option<Size>>,
    /// `_shouldIncludeChild`.
    include_child: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl HeroEntry {
    fn rebuild(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// The hero's box in its route's coordinates, once laid out:
    /// `_boundingBoxFor(hero, route.subtreeContext)`.
    fn location(&self) -> Option<Rect> {
        let route = self.route.borrow().upgrade()?;
        let size = self.link.leader_size()?;
        let hero = self.link.leader_transform()?;
        let content = route.content.leader_transform()?.inverse()?;
        let rect = content
            .then(hero)
            .transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
        let finite = [
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ]
        .iter()
        .all(|value| value.is_finite());
        finite.then_some(rect)
    }

    /// `_HeroState.startFlight`.
    fn start_flight(&self, include_child: bool) {
        self.include_child.set(include_child);
        if let Some(size) = self.link.leader_size() {
            self.placeholder.set(Some(size));
        }
        self.rebuild();
    }

    /// `_HeroState.endFlight`.
    fn end_flight(&self, keep_placeholder: bool) {
        if keep_placeholder || self.placeholder.get().is_none() {
            return;
        }
        self.placeholder.set(None);
        self.rebuild();
    }
}

struct HeroState(Rc<HeroEntry>);

impl RetainedState for HeroState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.0.revision.borrow_mut() = revision;
    }
}

impl From<Hero> for Widget {
    fn from(value: Hero) -> Self {
        Widget::retained_builder(
            {
                let value = value.clone();
                move |revision| {
                    HeroState(Rc::new(HeroEntry {
                        config: RefCell::new(value.clone()),
                        link: LayerLink::new(),
                        route: RefCell::new(Weak::new()),
                        enabled: Cell::new(true),
                        placeholder: Cell::new(None),
                        include_child: Cell::new(true),
                        revision: RefCell::new(revision),
                    }))
                }
            },
            move |state, context, _| {
                let entry = &state.0;
                *entry.config.borrow_mut() = value.clone();
                entry
                    .enabled
                    .set(context.find::<HeroModeScope>().is_none_or(|mode| mode.0));
                if let Some(route) = context.find_shared::<RouteHeroesScope>() {
                    route.0.register(entry);
                }
                CompositedTransformTarget::new(entry.link.clone(), hero_body(entry, context)).into()
            },
            false,
        )
    }
}

/// `_HeroState.build`.
fn hero_body(entry: &HeroEntry, context: &BuildContext<'_>) -> Widget {
    let config = entry.config.borrow();
    let placeholder = entry.placeholder.get();
    if let (Some(size), Some(builder)) = (placeholder, &config.placeholder_builder) {
        return builder(context, size, config.child.clone());
    }
    if let Some(size) = placeholder
        && !entry.include_child.get()
    {
        return SizedBox::from_dimensions(Some(size.width), Some(size.height), None::<Widget>)
            .into();
    }
    let shown = placeholder.is_none();
    SizedBox::from_dimensions(
        placeholder.map(|size| size.width),
        placeholder.map(|size| size.height),
        Some(Offstage::new(TickerMode::new(shown, config.child.clone())).offstage(!shown)),
    )
    .into()
}

/// The heroes of one route, and where its content sits: Flutter finds them
/// below `ModalRoute.subtreeContext`.
struct RouteHeroesInner {
    heroes: RefCell<Vec<Weak<HeroEntry>>>,
    content: LayerLink,
    active: Cell<bool>,
}

impl RouteHeroesInner {
    fn register(self: &Rc<Self>, entry: &Rc<HeroEntry>) {
        *entry.route.borrow_mut() = Rc::downgrade(self);
        let mut heroes = self.heroes.borrow_mut();
        heroes.retain(|hero| hero.strong_count() > 0);
        if !heroes.iter().any(|hero| hero.as_ptr() == Rc::as_ptr(entry)) {
            heroes.push(Rc::downgrade(entry));
        }
    }

    /// `Hero._allHeroesFor`: the enabled heroes by tag.
    fn heroes(self: &Rc<Self>) -> HashMap<HeroTag, Rc<HeroEntry>> {
        let mut heroes = self.heroes.borrow_mut();
        heroes.retain(|hero| {
            hero.upgrade()
                .is_some_and(|hero| std::ptr::eq(hero.route.borrow().as_ptr(), Rc::as_ptr(self)))
        });
        heroes
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|hero| hero.enabled.get())
            .map(|hero| {
                let tag = hero.config.borrow().tag.clone();
                (tag, hero)
            })
            .collect()
    }
}

struct RouteHeroesScope(Rc<RouteHeroesInner>);

/// The heroes of one route. A navigator wraps each route's content in
/// [`Self::scope`], inside its transition, so hero locations are in the
/// route's own coordinates as in Flutter.
#[derive(Clone)]
pub struct RouteHeroes(Rc<RouteHeroesInner>);

impl Default for RouteHeroes {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteHeroes {
    #[must_use]
    pub fn new() -> Self {
        Self(Rc::new(RouteHeroesInner {
            heroes: RefCell::new(Vec::new()),
            content: LayerLink::new(),
            active: Cell::new(false),
        }))
    }

    /// Makes [`Self::scope`] register the route's heroes, once a hero
    /// controller observes its navigator.
    pub fn activate(&self) {
        self.0.active.set(true);
    }

    /// The route's content, with its heroes registered here once active.
    /// An inactive route's content is returned as it is.
    #[must_use]
    pub fn scope(&self, child: Widget) -> Widget {
        if !self.0.active.get() {
            return child;
        }
        Widget::shared_environment_scope(
            Rc::new(RouteHeroesScope(self.0.clone())),
            CompositedTransformTarget::new(self.0.content.clone(), child).into(),
        )
    }

    /// The tags of the route's enabled heroes.
    #[must_use]
    pub fn tags(&self) -> Vec<HeroTag> {
        self.0.heroes().into_keys().collect()
    }

    /// The location of the route's hero with `tag`, once laid out.
    #[must_use]
    pub fn location(&self, tag: &HeroTag) -> Option<Rect> {
        self.0.heroes().get(tag)?.location()
    }

    /// Whether the hero with `tag` currently shows a placeholder.
    #[must_use]
    pub fn is_in_flight(&self, tag: &HeroTag) -> bool {
        self.0
            .heroes()
            .get(tag)
            .is_some_and(|hero| hero.placeholder.get().is_some())
    }

    fn generation(&self) -> u64 {
        self.0.content.leader_generation()
    }
}

/// `_HeroFlightManifest`.
#[derive(Clone)]
struct Manifest {
    direction: HeroFlightDirection,
    animation: AnimationController,
    from_hero: Rc<HeroEntry>,
    to_hero: Rc<HeroEntry>,
    to_route: RouteHeroes,
    create_rect_tween: Option<CreateRectTween>,
}

impl Manifest {
    /// `animation`: the route animation through `Curves.fastOutSlowIn`.
    fn value(&self) -> f32 {
        Curve::FastOutSlowIn.apply(self.animation.value())
    }

    /// `createHeroRectTween`: the destination hero's, the controller's, or
    /// a linear tween.
    fn tween(&self, begin: Rect, end: Rect) -> HeroRectTween {
        let create = self
            .to_hero
            .config
            .borrow()
            .create_rect_tween
            .clone()
            .or_else(|| self.create_rect_tween.clone());
        create.map_or_else(
            || HeroRectTween::linear(begin, end),
            |create| create(begin, end),
        )
    }

    fn shuttle(&self, context: &BuildContext<'_>) -> Widget {
        let to = self.to_hero.config.borrow().clone();
        let from = self.from_hero.config.borrow().clone();
        let builder = to
            .flight_shuttle_builder
            .clone()
            .or(from.flight_shuttle_builder.clone());
        match builder {
            Some(builder) => builder(
                context,
                &HeroShuttleContext {
                    animation: self.value(),
                    direction: self.direction,
                    from_child: from.child,
                    to_child: to.child,
                },
            ),
            None => to.child,
        }
    }
}

/// `_HeroFlight`.
struct Flight {
    manifest: Manifest,
    tween: HeroRectTween,
    /// The animation value where the destination hero went away, after
    /// which the shuttle fades out.
    fade_from: Option<f32>,
    aborted: bool,
}

impl Flight {
    /// `start`.
    fn start(manifest: Manifest, from: Rect, to: Rect) -> Self {
        let tween = manifest.tween(from, to);
        manifest
            .from_hero
            .start_flight(manifest.direction == HeroFlightDirection::Push);
        manifest.to_hero.start_flight(false);
        Self {
            manifest,
            tween,
            fade_from: None,
            aborted: false,
        }
    }

    /// `divert` for a flight redirected by a transition in the same
    /// direction: it continues from where it is to the new destination.
    fn divert(&mut self, manifest: Manifest, to: Rect) {
        let here = self.tween.transform(self.manifest.value());
        self.tween = self.manifest.tween(here, to);
        self.manifest.from_hero.end_flight(true);
        self.manifest.to_hero.end_flight(true);
        manifest
            .from_hero
            .start_flight(manifest.direction == HeroFlightDirection::Push);
        manifest.to_hero.start_flight(false);
        self.manifest = manifest;
    }

    /// `onTick`: follows the destination hero if it moves, and fades the
    /// shuttle out from the moment it goes away.
    fn tick(&mut self) {
        let origin = if self.aborted {
            None
        } else {
            self.manifest.to_route.0.heroes().values().find_map(|hero| {
                Rc::ptr_eq(hero, &self.manifest.to_hero)
                    .then(|| hero.location())
                    .flatten()
                    .map(|rect| rect.origin)
            })
        };
        match origin {
            Some(origin) => {
                if origin != self.tween.end().origin {
                    let end = Rect::from_origin_size(origin, self.tween.end().size);
                    self.tween = self.manifest.tween(self.tween.begin(), end);
                }
            }
            None if self.fade_from.is_none() => {
                self.fade_from = Some(self.manifest.value());
            }
            None => {}
        }
        self.aborted = origin.is_none();
    }

    /// `_heroOpacity`: 1, or `1 - Interval(fade_from, 1)` once fading.
    fn opacity(&self) -> f32 {
        let Some(start) = self.fade_from else {
            return 1.;
        };
        let value = self.manifest.value();
        if start >= 1. {
            return 0.;
        }
        1. - ((value - start) / (1. - start)).clamp(0., 1.)
    }

    fn is_animating(&self) -> bool {
        matches!(
            self.manifest.animation.status(),
            AnimationStatus::Forward | AnimationStatus::Reverse
        )
    }

    /// `_performAnimationUpdate` at a stop.
    fn end(&self) {
        let status = self.manifest.animation.status();
        self.manifest
            .from_hero
            .end_flight(status == AnimationStatus::Completed);
        self.manifest
            .to_hero
            .end_flight(status == AnimationStatus::Dismissed);
    }
}

/// A route transition waiting for its routes to be laid out: Flutter's
/// post-frame `_startHeroTransition`.
struct PendingTransition {
    from: RouteHeroes,
    to: RouteHeroes,
    direction: Option<HeroFlightDirection>,
    animation: AnimationController,
    generation: u64,
    waited: u32,
}

struct FlightsInner {
    create_rect_tween: Option<CreateRectTween>,
    flights: RefCell<HashMap<HeroTag, Flight>>,
    pending: RefCell<Vec<PendingTransition>>,
    driver: RefCell<Option<AnimationController>>,
    revision: RefCell<Rc<Cell<u64>>>,
}

/// The flights of one navigator: Flutter's `HeroController` without the
/// route bookkeeping, which belongs to the navigator. [`Self::overlay`]
/// goes above the navigator's routes, where the heroes fly.
#[derive(Clone)]
pub struct HeroFlights(Rc<FlightsInner>);

impl HeroFlights {
    /// Flights use `create_rect_tween` unless the destination hero has its
    /// own.
    #[must_use]
    pub fn new(create_rect_tween: Option<CreateRectTween>) -> Self {
        Self(Rc::new(FlightsInner {
            create_rect_tween,
            flights: RefCell::new(HashMap::new()),
            pending: RefCell::new(Vec::new()),
            driver: RefCell::new(None),
            revision: RefCell::new(Rc::new(Cell::new(0))),
        }))
    }

    /// `_maybeStartHeroTransition` once the flight type is known: the
    /// transition from `from` to `to` starts when both are laid out, driven
    /// by `animation`. Without a direction, existing flights of the tags
    /// abort and the destination heroes leave their placeholders.
    pub fn transition(
        &self,
        from: &RouteHeroes,
        to: &RouteHeroes,
        direction: Option<HeroFlightDirection>,
        animation: AnimationController,
    ) {
        let pending = PendingTransition {
            from: from.clone(),
            to: to.clone(),
            direction,
            generation: to.generation(),
            animation: animation.clone(),
            waited: 0,
        };
        if direction.is_none() {
            // Nothing flies, so nothing waits for geometry: the revealed
            // route shows its heroes in its first frame.
            self.start(&pending);
        } else {
            *self.0.driver.borrow_mut() = Some(animation);
            self.0.pending.borrow_mut().push(pending);
        }
        let revision = self.0.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// The tags in flight.
    #[must_use]
    pub fn tags(&self) -> Vec<HeroTag> {
        self.0.flights.borrow().keys().cloned().collect()
    }

    /// `_startHeroTransition`.
    fn start(&self, pending: &PendingTransition) {
        let from_heroes = pending.from.0.heroes();
        let mut to_heroes = pending.to.0.heroes();
        let mut flights = self.0.flights.borrow_mut();
        for (tag, from_hero) in from_heroes {
            let existing = flights.contains_key(&tag);
            let manifest = match (to_heroes.get(&tag), pending.direction) {
                (Some(to_hero), Some(direction)) => Some(Manifest {
                    direction,
                    animation: pending.animation.clone(),
                    from_hero: from_hero.clone(),
                    to_hero: to_hero.clone(),
                    to_route: pending.to.clone(),
                    create_rect_tween: self.0.create_rect_tween.clone(),
                }),
                _ => None,
            };
            // `isValid`: the destination is laid out, and so is the origin
            // unless the flight is diverted.
            let locations = manifest.as_ref().and_then(|manifest| {
                let to = manifest.to_hero.location()?;
                let from = manifest.from_hero.location();
                (existing || from.is_some()).then_some((from, to))
            });
            match (manifest, locations) {
                (Some(manifest), Some((from, to))) => {
                    to_heroes.remove(&tag);
                    match flights.get_mut(&tag) {
                        Some(flight) => flight.divert(manifest, to),
                        None => {
                            let from = from.expect("an undiverted flight has an origin");
                            flights.insert(tag, Flight::start(manifest, from, to));
                        }
                    }
                }
                _ => {
                    if let Some(flight) = flights.get_mut(&tag) {
                        flight.aborted = true;
                    }
                }
            }
        }
        for hero in to_heroes.values() {
            hero.end_flight(false);
        }
    }

    /// Starts the pending transitions whose destination has been laid out
    /// since it was requested, or after two ticks regardless.
    fn start_ready(&self) {
        let ready: Vec<PendingTransition> = {
            let mut pending = self.0.pending.borrow_mut();
            let mut ready = Vec::new();
            let mut index = 0;
            while index < pending.len() {
                let transition = &mut pending[index];
                transition.waited += 1;
                if transition.to.generation() != transition.generation || transition.waited > 2 {
                    ready.push(pending.remove(index));
                } else {
                    index += 1;
                }
            }
            ready
        };
        for transition in &ready {
            self.start(transition);
        }
    }

    /// On every tick of the route animation, before the frame builds:
    /// starts the transitions that are ready and ends the flights whose
    /// animation stopped, so the heroes change in the same frame as the
    /// shuttles.
    fn advance(&self) {
        self.start_ready();
        self.0.flights.borrow_mut().retain(|_, flight| {
            if flight.is_animating() {
                return true;
            }
            flight.end();
            false
        });
    }

    /// Follows the destinations and builds the shuttles.
    fn build(&self, context: &BuildContext<'_>) -> Widget {
        let mut flights = self.0.flights.borrow_mut();
        let mut shuttles = Vec::new();
        for flight in flights.values_mut() {
            flight.tick();
            let rect = flight.tween.transform(flight.manifest.value());
            shuttles.push(Widget::from(
                Positioned::new(IgnorePointer::new(Opacity::new(
                    flight.opacity(),
                    flight.manifest.shuttle(context),
                )))
                .left(rect.origin.x)
                .top(rect.origin.y)
                .width(rect.size.width)
                .height(rect.size.height),
            ));
        }
        Stack::new(shuttles).into()
    }

    /// The layer the heroes fly in, to be placed above the navigator's
    /// routes and sized like them.
    #[must_use]
    pub fn overlay(&self) -> Widget {
        let flights = self.clone();
        Widget::retained_builder(
            |revision| OverlayState(AnimationListenerState::new(revision)),
            move |state, context, _| {
                *flights.0.revision.borrow_mut() = state.0.revision();
                let content = flights.build(context);
                let driver = flights.0.driver.borrow().clone();
                match driver {
                    Some(driver) => {
                        let weak = Rc::downgrade(&flights.0);
                        state.0.listen(&driver, move || {
                            if let Some(flights) = weak.upgrade() {
                                HeroFlights(flights).advance();
                            }
                        });
                        Widget::animation_ticker_with_revision(
                            driver,
                            false,
                            state.0.revision(),
                            content,
                        )
                    }
                    None => content,
                }
            },
            false,
        )
    }
}

struct OverlayState(AnimationListenerState);

impl RetainedState for OverlayState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        self.0.rebind_revision(revision);
    }
}
