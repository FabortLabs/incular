//! Flutter's interactive ink features for buttons: `InkRipple` (the Material
//! 3 default off Android) and `InkSplash` (the Material 2 default).
//!
//! Each splash runs on its own vsync animation controllers with Flutter's
//! durations, curves and integer alpha tweens, and is clipped to the
//! button's shape. A splash is removed once it has faded out.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

use incular_core::{Color, Lerp, Offset, Size};
use incular_widgets::internal::ActionInkEvent;
use incular_widgets::{
    AnimationController, AnimationStatus, BorderRadius, BoxDecoration, ClipRRect, Container, Curve,
    IgnorePointer, Positioned, SizedBox, Stack, Widget,
};

use crate::styles::SplashFactory;

// `ink_ripple.dart`.
const UNCONFIRMED_RIPPLE: Duration = Duration::from_secs(1);
const RIPPLE_FADE_IN: Duration = Duration::from_millis(75);
const RIPPLE_RADIUS: Duration = Duration::from_millis(225);
const RIPPLE_FADE_OUT: Duration = Duration::from_millis(375);
const RIPPLE_CANCEL: Duration = Duration::from_millis(75);
const RIPPLE_FADE_OUT_INTERVAL_START: f32 = 225.0 / 375.0;

// `ink_splash.dart`.
const UNCONFIRMED_SPLASH: Duration = Duration::from_secs(1);
const SPLASH_FADE: Duration = Duration::from_millis(200);
/// Logical pixels per millisecond.
const SPLASH_CONFIRMED_VELOCITY: f32 = 1.0;

/// Flutter `IntTween.lerp`: `(begin + (end - begin) * t).round()`.
fn int_tween(begin: u8, end: u8, t: f32) -> u8 {
    let (begin, end) = (f32::from(begin), f32::from(end));
    (begin + (end - begin) * t).round().clamp(0.0, 255.0) as u8
}

fn vsync(duration: Duration) -> AnimationController {
    AnimationController::new(duration).with_vsync()
}

enum Kind {
    Ripple {
        fade_in: AnimationController,
        radius: AnimationController,
        fade_out: AnimationController,
    },
    Splash {
        radius: AnimationController,
        alpha: AnimationController,
    },
}

struct Splash {
    position: Offset,
    size: Size,
    color: Color,
    target_radius: f32,
    kind: Kind,
    done: Rc<Cell<bool>>,
}

impl Splash {
    fn new(factory: SplashFactory, position: Offset, size: Size, color: Color) -> Option<Self> {
        let done = Rc::new(Cell::new(false));
        let (kind, target_radius) = match factory {
            SplashFactory::NoSplash => return None,
            // `InkSparkle` is a fragment shader on Android only; elsewhere
            // Material 3 uses `InkRipple`.
            SplashFactory::Ripple | SplashFactory::Sparkle => {
                // `_getTargetRadius`: half the diagonal.
                let target = size.width.hypot(size.height) / 2.0;
                let fade_in = vsync(RIPPLE_FADE_IN);
                fade_in.forward_on_next_tick();
                let radius = vsync(UNCONFIRMED_RIPPLE);
                radius.forward_on_next_tick();
                let fade_out = vsync(RIPPLE_FADE_OUT);
                (
                    Kind::Ripple {
                        fade_in,
                        radius,
                        fade_out,
                    },
                    target,
                )
            }
            SplashFactory::Splash => {
                // `_getSplashRadiusForPositionInSize`: the farthest corner.
                let corners = [
                    Offset::ZERO,
                    Offset::new(size.width, 0.0),
                    Offset::new(0.0, size.height),
                    Offset::new(size.width, size.height),
                ];
                let target = corners
                    .iter()
                    .map(|corner| (position.x - corner.x).hypot(position.y - corner.y))
                    .fold(0.0_f32, f32::max)
                    .ceil();
                let radius = vsync(UNCONFIRMED_SPLASH);
                radius.forward_on_next_tick();
                let alpha = vsync(SPLASH_FADE);
                (Kind::Splash { radius, alpha }, target)
            }
        };
        // The splash is disposed once its fade completes.
        let fade = match &kind {
            Kind::Ripple { fade_out, .. } => fade_out,
            Kind::Splash { alpha, .. } => alpha,
        };
        let flag = Rc::downgrade(&done);
        fade.add_status_listener(move |status| {
            if status == AnimationStatus::Completed
                && let Some(flag) = flag.upgrade()
            {
                flag.set(true);
            }
        });
        Some(Self {
            position,
            size,
            color,
            target_radius,
            kind,
            done,
        })
    }

    fn controllers(&self) -> Vec<&AnimationController> {
        match &self.kind {
            Kind::Ripple {
                fade_in,
                radius,
                fade_out,
            } => vec![fade_in, radius, fade_out],
            Kind::Splash { radius, alpha } => vec![radius, alpha],
        }
    }

    fn confirm(&self) {
        match &self.kind {
            Kind::Ripple {
                fade_in,
                radius,
                fade_out,
            } => {
                radius.set_duration(RIPPLE_RADIUS);
                radius.forward_on_next_tick();
                fade_in.forward_on_next_tick();
                fade_out.animate_to_on_next_tick_curved(1.0, Some(RIPPLE_FADE_OUT), Curve::Linear);
            }
            Kind::Splash { radius, alpha } => {
                let millis = (self.target_radius / SPLASH_CONFIRMED_VELOCITY).floor();
                radius.set_duration(Duration::from_millis(millis.max(0.0) as u64));
                radius.forward_on_next_tick();
                alpha.forward_on_next_tick();
            }
        }
    }

    fn cancel(&self) {
        match &self.kind {
            Kind::Ripple {
                fade_in, fade_out, ..
            } => {
                fade_in.stop();
                let value = 1.0 - fade_in.value();
                fade_out.set_value(value);
                if value < 1.0 {
                    fade_out.animate_to_on_next_tick_curved(
                        1.0,
                        Some(RIPPLE_CANCEL),
                        Curve::Linear,
                    );
                } else {
                    self.done.set(true);
                }
            }
            Kind::Splash { alpha, .. } => {
                alpha.forward_on_next_tick();
            }
        }
    }

    /// The circle to paint: center, radius and color (`paintFeature`).
    fn circle(&self) -> (Offset, f32, Color) {
        let alpha = self.color.alpha;
        match &self.kind {
            Kind::Ripple {
                fade_in,
                radius,
                fade_out,
            } => {
                let painted = if fade_in.is_animating() {
                    int_tween(0, alpha, fade_in.value())
                } else {
                    let t = Curve::interval(RIPPLE_FADE_OUT_INTERVAL_START, 1.0, &Curve::Linear)
                        .apply(fade_out.value());
                    int_tween(alpha, 0, t)
                };
                let eased = Curve::Ease.apply(radius.value());
                let center = self.position.lerp(
                    &Offset::new(self.size.width / 2.0, self.size.height / 2.0),
                    eased,
                );
                let r = self.target_radius * 0.30
                    + (self.target_radius + 5.0 - self.target_radius * 0.30) * eased;
                (
                    center,
                    r,
                    Color::rgba(self.color.red, self.color.green, self.color.blue, painted),
                )
            }
            Kind::Splash {
                radius,
                alpha: fade,
            } => {
                let painted = int_tween(alpha, 0, fade.value());
                (
                    self.position,
                    self.target_radius * radius.value(),
                    Color::rgba(self.color.red, self.color.green, self.color.blue, painted),
                )
            }
        }
    }
}

/// The ink splashes of one button: Flutter's `InkWell._splashes` and
/// `_currentSplash`.
pub(crate) struct InkSplashes {
    splashes: RefCell<Vec<Splash>>,
    /// The splash of the press in progress, by index into `splashes`.
    current: Cell<Option<usize>>,
    revision: Weak<Cell<u64>>,
    /// Whether the button has shown ink. The ink layer is built from then
    /// on, so a button that was never hovered, focused or pressed mounts
    /// without it.
    inked: Cell<bool>,
}

impl InkSplashes {
    pub(crate) fn new(revision: &Rc<Cell<u64>>) -> Self {
        Self {
            splashes: RefCell::new(Vec::new()),
            current: Cell::new(None),
            revision: Rc::downgrade(revision),
            inked: Cell::new(false),
        }
    }

    /// Whether the ink layer is built: once a highlight or a splash has
    /// shown, it stays so the layer is not remounted as ink comes and goes.
    pub(crate) fn needs_layer(&self, highlight_visible: bool) -> bool {
        if highlight_visible || !self.splashes.borrow().is_empty() {
            self.inked.set(true);
        }
        self.inked.get()
    }

    /// Applies the press events since the last build. `color` is the
    /// overlay resolved for the pressed state.
    pub(crate) fn handle(&self, events: Vec<ActionInkEvent>, factory: SplashFactory, color: Color) {
        for event in events {
            match event {
                ActionInkEvent::Down { position, size } => {
                    // `_startNewSplash`: the previous current splash cancels.
                    self.cancel_current();
                    if let Some(splash) = Splash::new(factory, position, size, color) {
                        for controller in splash.controllers() {
                            let revision = self.revision.clone();
                            controller.add_listener(move |_| {
                                if let Some(revision) = revision.upgrade() {
                                    revision.set(revision.get().wrapping_add(1));
                                }
                            });
                        }
                        let mut splashes = self.splashes.borrow_mut();
                        splashes.push(splash);
                        self.current.set(Some(splashes.len() - 1));
                    }
                }
                ActionInkEvent::Confirm => {
                    if let Some(index) = self.current.take()
                        && let Some(splash) = self.splashes.borrow().get(index)
                    {
                        splash.confirm();
                    }
                }
                ActionInkEvent::Cancel => self.cancel_current(),
            }
        }
    }

    fn cancel_current(&self) {
        if let Some(index) = self.current.take()
            && let Some(splash) = self.splashes.borrow().get(index)
        {
            splash.cancel();
        }
    }

    /// The splash layer, clipped to the button's shape, or `None` while no
    /// splash shows. Faded splashes are dropped here.
    pub(crate) fn layer(&self, radius: BorderRadius) -> Option<Widget> {
        {
            let mut splashes = self.splashes.borrow_mut();
            if splashes.iter().any(|splash| splash.done.get()) {
                let current = self.current.get();
                let mut kept = 0;
                let mut remapped = None;
                let old = std::mem::take(&mut *splashes);
                for (index, splash) in old.into_iter().enumerate() {
                    if splash.done.get() {
                        continue;
                    }
                    if current == Some(index) {
                        remapped = Some(kept);
                    }
                    splashes.push(splash);
                    kept += 1;
                }
                self.current.set(remapped);
            }
        }
        let splashes = self.splashes.borrow();
        if splashes.is_empty() {
            return None;
        }
        let circles = splashes.iter().map(|splash| {
            let (center, r, color) = splash.circle();
            let r = r.max(0.0);
            Widget::from(
                Positioned::new(
                    Container::new().decoration(
                        BoxDecoration::new()
                            .color(color)
                            .border_radius(BorderRadius::circular(r)),
                    ),
                )
                .left(center.x - r)
                .top(center.y - r)
                .width(r * 2.0)
                .height(r * 2.0),
            )
        });
        let sized: Widget = SizedBox::expand().into();
        let stack = Stack::new(std::iter::once(sized).chain(circles));
        Some(IgnorePointer::new(ClipRRect::new(radius, stack)).into())
    }
}
