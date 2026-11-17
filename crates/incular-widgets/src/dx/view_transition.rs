//! `view_transition(|| ..)`: whole-scene changes (§8.3).
//!
//! The old scene is not re-rendered or rasterized: the display list the tree
//! painted last frame (shared buffers, so keeping it is a reference count)
//! stays alive and is painted above the new scene while it fades out or is
//! clipped away by a growing circle. The new scene is live and interactive
//! from the first frame.
//!
//! ```ignore
//! view_transition(|| page.set(Page::Settings));                   // crossfade
//! view_transition(|| dark.update(|d| *d = !*d))
//!     .reveal(Reveal::circle_from(pointer))
//!     .duration(ms(450));
//! view_transition(|| tab.set(2)).within(&panel_scope);            // one subtree
//! ```

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use incular_animation::{MotionValue, Transition};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_rendering::{
    CornerRadii, DisplayList, FillRule, LayerId, LayerTree, PaintCommand, Path,
};

use crate::Widget;
use crate::tree::{FrameDelegate, FrameGeometry, FrameTick};

/// How the new scene replaces the old one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reveal {
    /// The old scene fades out over the new one.
    Crossfade,
    /// The new scene grows out of a circle centered on a point (in window
    /// coordinates).
    Circle { center: Offset },
}

impl Reveal {
    /// A circular reveal growing from `point`.
    #[must_use]
    pub const fn circle_from(point: Offset) -> Self {
        Self::Circle { center: point }
    }
}

struct Request {
    reveal: Reveal,
    transition: Transition,
    scope: Option<ViewScope>,
}

thread_local! {
    static PENDING: RefCell<Vec<Rc<RefCell<Request>>>> = const { RefCell::new(Vec::new()) };
}

/// A running view transition request; configure it right after the call.
#[must_use = "configure the transition with reveal / duration / within"]
pub struct ViewTransition(Rc<RefCell<Request>>);

impl ViewTransition {
    /// How the new scene appears (a crossfade by default).
    pub fn reveal(self, reveal: Reveal) -> Self {
        self.0.borrow_mut().reveal = reveal;
        self
    }

    /// An ease over `duration` (300 ms by default).
    pub fn duration(self, duration: Duration) -> Self {
        self.0.borrow_mut().transition = Transition::ease(duration);
        self
    }

    /// Any transition (a spring, a curve).
    pub fn transition(self, transition: impl Into<Transition>) -> Self {
        self.0.borrow_mut().transition = transition.into();
        self
    }

    /// Limits the transition to the subtree marked with `scope`.
    pub fn within(self, scope: &ViewScope) -> Self {
        self.0.borrow_mut().scope = Some(scope.clone());
        self
    }
}

/// Runs `update`; the next frame shows its result through a transition from
/// the scene on screen now.
pub fn view_transition(update: impl FnOnce()) -> ViewTransition {
    let request = Rc::new(RefCell::new(Request {
        reveal: Reveal::Crossfade,
        transition: Transition::ease(Duration::from_millis(300)),
        scope: None,
    }));
    PENDING.with(|pending| pending.borrow_mut().push(request.clone()));
    update();
    incular_animation::schedule_frame();
    ViewTransition(request)
}

/// Marks a subtree that [`ViewTransition::within`] limits a transition to.
#[derive(Clone, Default)]
pub struct ViewScope(Rc<Cell<Option<Rect>>>);

impl ViewScope {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

struct ScopeTicker(ViewScope);

impl FrameTick for ScopeTicker {
    fn tick(&self, _now: Instant) -> bool {
        false
    }
    fn is_active(&self) -> bool {
        false
    }
    fn wants_geometry(&self) -> bool {
        true
    }
    fn geometry(&self, geometry: FrameGeometry) {
        self.0.0.set(Some(geometry.layout_rect));
    }
}

/// `.view_transition_scope(&scope)` for anything that becomes a widget.
pub trait ViewTransitionExt: Into<Widget> + Sized {
    /// Marks this subtree as `scope` for [`ViewTransition::within`].
    #[must_use]
    fn view_transition_scope(self, scope: &ViewScope) -> Widget {
        Widget::frame_ticker(
            FrameDelegate::new(Rc::new(ScopeTicker(scope.clone()))),
            self.into(),
        )
    }
}

impl<T: Into<Widget>> ViewTransitionExt for T {}

/// A transition the tree is painting.
pub(crate) struct Active {
    old: DisplayList,
    reveal: Reveal,
    transition: Transition,
    clip: Option<Rect>,
    progress: MotionValue,
    started: bool,
    /// The compositor group the old scene fades in (created on first paint).
    layer: Option<LayerId>,
}

/// Whether a request waits for the next frame.
pub(crate) fn has_pending() -> bool {
    PENDING
        .try_with(|pending| !pending.borrow().is_empty())
        .unwrap_or(false)
}

/// Starts the requests made since the last frame from `old`, the scene the
/// tree painted last.
pub(crate) fn start_pending(old: Option<&DisplayList>, active: &mut Vec<Active>) {
    let requests = PENDING
        .try_with(|pending| std::mem::take(&mut *pending.borrow_mut()))
        .unwrap_or_default();
    let Some(old) = old else { return };
    for request in requests {
        let request = request.borrow();
        if request.transition.is_instant() {
            continue;
        }
        let clip = match &request.scope {
            Some(scope) => match scope.0.get() {
                Some(rect) => Some(rect),
                // The scope is not on screen: nothing to transition.
                None => continue,
            },
            None => None,
        };
        active.push(Active {
            old: old.clone(),
            reveal: request.reveal,
            transition: request.transition.clone(),
            clip,
            progress: MotionValue::new(0.),
            started: false,
            layer: None,
        });
    }
}

/// Advances the transitions, releasing finished ones' layers; true while
/// any is running.
pub(crate) fn tick(active: &mut Vec<Active>, compositor: &mut LayerTree, now: Instant) -> bool {
    for transition in active.iter_mut() {
        if !transition.started {
            transition.started = true;
            transition.progress.animate_to(1., &transition.transition);
        }
        transition.progress.tick(now);
    }
    active.retain(|transition| {
        let running = transition.progress.is_animating();
        if !running && let Some(layer) = transition.layer {
            compositor.remove(layer);
        }
        running
    });
    if active.is_empty() {
        false
    } else {
        incular_animation::schedule_frame();
        true
    }
}

/// Paints the old scenes above `output` (the new scene of `size`).
pub(crate) fn paint(
    active: &mut [Active],
    compositor: &mut LayerTree,
    output: &mut DisplayList,
    size: Size,
) {
    for transition in active {
        let t = transition.progress.get().clamp(0., 1.);
        let bounds = transition
            .clip
            .unwrap_or_else(|| Rect::from_origin_size(Offset::ZERO, size));
        output.push(PaintCommand::PushClip { rect: bounds });
        match transition.reveal {
            Reveal::Crossfade => {
                let layer = *transition
                    .layer
                    .get_or_insert_with(|| compositor.create_opacity(1.));
                output.push(PaintCommand::PushOpacity {
                    layer,
                    alpha: incular_rendering::normalize_opacity(1. - t),
                    // The old scene never changes.
                    generation: 0,
                    bounds,
                });
                output.extend_from(&transition.old);
                output.push(PaintCommand::PopOpacity);
            }
            Reveal::Circle { center } => {
                // The circle grows until it covers the farthest corner.
                let corners = [
                    bounds.origin,
                    Offset::new(bounds.origin.x + bounds.size.width, bounds.origin.y),
                    Offset::new(bounds.origin.x, bounds.origin.y + bounds.size.height),
                    Offset::new(
                        bounds.origin.x + bounds.size.width,
                        bounds.origin.y + bounds.size.height,
                    ),
                ];
                let reach = corners
                    .iter()
                    .map(|corner| (corner.x - center.x).hypot(corner.y - center.y))
                    .fold(0., f32::max);
                let radius = reach * t;
                let mut circle = Path::builder();
                circle.rounded_rect(
                    Rect::from_origin_size(
                        Offset::new(center.x - radius, center.y - radius),
                        Size::new(radius * 2., radius * 2.),
                    ),
                    CornerRadii::uniform(radius),
                );
                let hole = Path::rect_excluding(bounds, &circle.build());
                output.push(PaintCommand::PushClipPath {
                    path: Arc::new(hole),
                    fill_rule: FillRule::NonZero,
                });
                output.extend_from(&transition.old);
                output.push(PaintCommand::PopClip);
            }
        }
        output.push(PaintCommand::PopClip);
    }
}
