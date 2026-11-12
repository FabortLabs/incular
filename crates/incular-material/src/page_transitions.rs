//! Flutter's page transitions (`material/page_transitions_theme.dart`,
//! `widgets/page_transitions_builder.dart`): the transitions a page route
//! plays as it is pushed and popped, and as a route is pushed over it.
//!
//! The Zoom transition follows the snapshot painters Flutter uses off the
//! web: the entering page fades and scales over a scrim of the surface
//! color, the exiting page scales and fades away, and both draw the page
//! plainly once the animation rests.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::TargetPlatform;
use incular_core::{Color, Offset};
use incular_navigation::{RouteAnimation, RouteTransition};
use incular_widgets::{
    AnimatedBuilder, AnimationStatus, ColoredBox, Curve, DoubleAnimation, DualTransitionBuilder,
    FractionalTranslation, IgnorePointer, Opacity, ReverseAnimation, Transform, Widget,
};

/// A page transition: Flutter's `PageTransitionsBuilder` subclasses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageTransitionsBuilder {
    /// `ZoomPageTransitionsBuilder`, the default on Windows, Linux and
    /// Fuchsia. `background_color` is the scrim behind the entering page
    /// (default: the color scheme's surface).
    Zoom { background_color: Option<Color> },
    /// `FadeForwardsPageTransitionsBuilder`, the default on Android (outside
    /// a predictive back gesture).
    FadeForwards { background_color: Option<Color> },
    /// `FadeUpwardsPageTransitionsBuilder`.
    FadeUpwards,
    /// `CupertinoPageTransitionsBuilder`, the default on iOS and macOS: the
    /// page slides in from the end edge while the page below slides a third
    /// of the way out.
    Cupertino,
}

impl PageTransitionsBuilder {
    /// Flutter `transitionDuration`.
    #[must_use]
    pub const fn transition_duration(self) -> Duration {
        match self {
            Self::Zoom { .. } | Self::FadeUpwards => Duration::from_millis(300),
            Self::FadeForwards { .. } => Duration::from_millis(450),
            Self::Cupertino => Duration::from_millis(500),
        }
    }

    /// Flutter `reverseTransitionDuration`.
    #[must_use]
    pub const fn reverse_transition_duration(self) -> Duration {
        self.transition_duration()
    }

    /// A route transition that plays this builder, with `surface` as the
    /// default background color.
    #[must_use]
    pub fn route_transition(self, surface: Color) -> RouteTransition {
        // Per-route state: the effective statuses of the dual transitions.
        let primary: Rc<Cell<Option<AnimationStatus>>> = Rc::default();
        let secondary: Rc<Cell<Option<AnimationStatus>>> = Rc::default();
        RouteTransition::Animated(RouteAnimation::new(
            self.transition_duration(),
            self.reverse_transition_duration(),
            move |animation, secondary_animation, child| {
                self.build_transitions(
                    animation,
                    secondary_animation,
                    child,
                    surface,
                    &primary,
                    &secondary,
                )
            },
        ))
    }

    fn build_transitions(
        self,
        animation: &DoubleAnimation,
        secondary: &DoubleAnimation,
        child: Widget,
        surface: Color,
        primary_state: &Rc<Cell<Option<AnimationStatus>>>,
        secondary_state: &Rc<Cell<Option<AnimationStatus>>>,
    ) -> Widget {
        match self {
            Self::Zoom { background_color } => zoom(
                animation,
                secondary,
                background_color.unwrap_or(surface),
                child,
                primary_state,
                secondary_state,
            ),
            Self::FadeForwards { background_color } => fade_forwards(
                animation,
                secondary,
                background_color.unwrap_or(surface),
                child,
                primary_state,
                secondary_state,
            ),
            Self::FadeUpwards => fade_upwards(animation, child),
            Self::Cupertino => cupertino(animation, secondary, child),
        }
    }
}

/// The builders by platform: Flutter's `PageTransitionsTheme.builders`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageTransitionBuilders {
    pub android: PageTransitionsBuilder,
    pub ios: PageTransitionsBuilder,
    pub macos: PageTransitionsBuilder,
    pub windows: PageTransitionsBuilder,
    pub linux: PageTransitionsBuilder,
    pub fuchsia: PageTransitionsBuilder,
}

impl Default for PageTransitionBuilders {
    /// Flutter's `_defaultBuilders`.
    fn default() -> Self {
        let zoom = PageTransitionsBuilder::Zoom {
            background_color: None,
        };
        Self {
            android: PageTransitionsBuilder::FadeForwards {
                background_color: None,
            },
            ios: PageTransitionsBuilder::Cupertino,
            macos: PageTransitionsBuilder::Cupertino,
            windows: zoom,
            linux: zoom,
            fuchsia: zoom,
        }
    }
}

impl PageTransitionBuilders {
    /// The builder for `platform`.
    #[must_use]
    pub const fn for_platform(&self, platform: TargetPlatform) -> PageTransitionsBuilder {
        match platform {
            TargetPlatform::Android => self.android,
            TargetPlatform::Ios => self.ios,
            TargetPlatform::MacOs => self.macos,
            TargetPlatform::Windows => self.windows,
            TargetPlatform::Linux => self.linux,
            TargetPlatform::Fuchsia => self.fuchsia,
        }
    }
}

/// Flutter `DualTransitionBuilder`: the animations its forward and reverse
/// builders see, following the effective status (a reversal mid-run keeps
/// the direction it was going until the animation rests).
fn dual(
    state: &Cell<Option<AnimationStatus>>,
    animation: &DoubleAnimation,
) -> (DoubleAnimation, DoubleAnimation) {
    let current = animation.status();
    let effective = match state.get() {
        Some(last) => DualTransitionBuilder::effective_status(last, current),
        None => current,
    };
    state.set(Some(effective));
    match effective {
        AnimationStatus::Dismissed | AnimationStatus::Forward => {
            (animation.clone(), incular_widgets::always_dismissed())
        }
        AnimationStatus::Reverse | AnimationStatus::Completed => (
            incular_widgets::always_complete(),
            DoubleAnimation::new(ReverseAnimation::new(animation.clone())),
        ),
    }
}

/// `FadeTransition`'s layer alpha: `(opacity * 255).round()`.
fn faded(opacity: f32, child: Widget) -> Widget {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() / 255.0;
    Opacity::new(alpha, child).into()
}

fn interval(begin: f32, end: f32, t: f32) -> f32 {
    Curve::interval(begin, end, &Curve::Linear).apply(t)
}

fn lerp(begin: f32, end: f32, t: f32) -> f32 {
    begin + (end - begin) * t
}

/// `_ZoomPageTransition._scaleCurveSequence`: a two-step sequence close to
/// the native `fastOutExtraSlowIn`.
fn zoom_scale_curve(t: f32) -> f32 {
    const SPLIT: f32 = 0.166_666;
    let first = Curve::CubicBezier {
        x1: 0.05,
        y1: 0.0,
        x2: 0.133_333,
        y2: 0.06,
    };
    let second = Curve::CubicBezier {
        x1: 0.208_333,
        y1: 0.82,
        x2: 0.25,
        y2: 1.0,
    };
    // `TweenSequence`: the weights sum to 1.
    if t < SPLIT {
        lerp(0.0, 0.4, first.apply(t / SPLIT))
    } else {
        lerp(0.4, 1.0, second.apply((t - SPLIT) / (1.0 - SPLIT)))
    }
}

/// `_ZoomEnterTransition` (`_ZoomEnterTransitionPainter`).
fn zoom_enter(
    animation: &DoubleAnimation,
    reverse: bool,
    background: Color,
    child: Widget,
) -> Widget {
    let value = animation.value();
    let animating = animation.is_animating();
    let scrim = if animating && !reverse && !animation.is_completed() {
        lerp(0.0, 0.60, interval(0.2075, 0.4175, value))
    } else {
        0.0
    };
    let fade = if !animating || reverse {
        1.0
    } else {
        interval(0.125, 0.250, value)
    };
    let scale = if !animating {
        1.0
    } else if reverse {
        lerp(1.10, 1.00, zoom_scale_curve(value))
    } else {
        lerp(0.85, 1.00, zoom_scale_curve(value))
    };
    let scrim_color = Color::rgba(
        background.red,
        background.green,
        background.blue,
        (f32::from(background.alpha) * scrim).round() as u8,
    );
    ColoredBox::new(scrim_color, Transform::scale(scale, faded(fade, child))).into()
}

/// `_ZoomExitTransition` (`_ZoomExitTransitionPainter`).
fn zoom_exit(animation: &DoubleAnimation, reverse: bool, child: Widget) -> Widget {
    let value = animation.value();
    let animating = animation.is_animating();
    let fade = if animating && reverse {
        lerp(1.0, 0.0, interval(0.0825, 0.2075, value))
    } else {
        1.0
    };
    let scale = if !animating {
        1.0
    } else if reverse {
        lerp(1.00, 0.90, zoom_scale_curve(value))
    } else {
        lerp(1.00, 1.05, zoom_scale_curve(value))
    };
    Transform::scale(scale, faded(fade, child)).into()
}

fn zoom(
    animation: &DoubleAnimation,
    secondary: &DoubleAnimation,
    background: Color,
    child: Widget,
    primary_state: &Rc<Cell<Option<AnimationStatus>>>,
    secondary_state: &Rc<Cell<Option<AnimationStatus>>>,
) -> Widget {
    // The route below zooms out of the way as a route is pushed over it
    // (`_snapshotAwareDelegatedTransition`).
    let reversed = DoubleAnimation::new(ReverseAnimation::new(secondary.clone()));
    let below = {
        let (state, reversed) = (secondary_state.clone(), reversed.clone());
        AnimatedBuilder::new(reversed.clone(), move |_, child| {
            let child = child.expect("the page");
            let (forward, reverse) = dual(&state, &reversed);
            zoom_enter(
                &forward,
                true,
                background,
                zoom_exit(&reverse, false, child),
            )
        })
        .child(child)
    };
    let (state, animation) = (primary_state.clone(), animation.clone());
    AnimatedBuilder::new(animation.clone(), move |_, child| {
        let child = child.expect("the page");
        let (forward, reverse) = dual(&state, &animation);
        zoom_enter(
            &forward,
            false,
            background,
            zoom_exit(&reverse, true, child),
        )
    })
    .child(below)
    .into()
}

fn slide(offset: Offset, child: Widget) -> Widget {
    FractionalTranslation::new(offset, child).into()
}

fn fade_forwards(
    animation: &DoubleAnimation,
    secondary: &DoubleAnimation,
    background: Color,
    child: Widget,
    primary_state: &Rc<Cell<Option<AnimationStatus>>>,
    secondary_state: &Rc<Cell<Option<AnimationStatus>>>,
) -> Widget {
    let curve = Curve::EaseInOutCubicEmphasized;
    let fade_in = |t: f32| interval(0.0, 0.75, t);
    let fade_out = |t: f32| 1.0 - interval(0.0, 0.25, t);
    // `_delegatedTransition`: the page below slides a quarter out while
    // fading, over the background while the page above animates.
    let reversed = DoubleAnimation::new(ReverseAnimation::new(secondary.clone()));
    let below = {
        let (state, reversed, secondary) =
            (secondary_state.clone(), reversed.clone(), secondary.clone());
        AnimatedBuilder::new(secondary.clone(), move |_, child| {
            let child = child.expect("the page");
            let (forward, reverse) = dual(&state, &reversed);
            let reverse_value = reverse.value();
            let inner = faded(
                fade_out(reverse_value),
                slide(
                    Offset::new(lerp(0.0, -0.25, curve.apply(reverse_value)), 0.0),
                    child,
                ),
            );
            let forward_value = forward.value();
            let outer = faded(
                fade_in(forward_value),
                slide(
                    Offset::new(lerp(-0.25, 0.0, curve.apply(forward_value)), 0.0),
                    inner,
                ),
            );
            let color = if secondary.is_animating() {
                background
            } else {
                Color::TRANSPARENT
            };
            ColoredBox::new(color, outer).into()
        })
        .child(child)
    };
    let (state, animation) = (primary_state.clone(), animation.clone());
    AnimatedBuilder::new(animation.clone(), move |_, child| {
        let child = child.expect("the page");
        let (forward, reverse) = dual(&state, &animation);
        let reverse_value = reverse.value();
        let leaving = IgnorePointer::new(faded(
            fade_out(reverse_value),
            slide(
                Offset::new(lerp(0.0, 0.25, curve.apply(reverse_value)), 0.0),
                child,
            ),
        ))
        .ignoring(reverse.status() == AnimationStatus::Forward);
        let forward_value = forward.value();
        faded(
            fade_in(forward_value),
            slide(
                Offset::new(lerp(0.25, 0.0, curve.apply(forward_value)), 0.0),
                leaving.into(),
            ),
        )
    })
    .child(below)
    .into()
}

/// `_FadeUpwardsPageTransition`.
fn fade_upwards(animation: &DoubleAnimation, child: Widget) -> Widget {
    let animation = animation.clone();
    AnimatedBuilder::new(animation.clone(), move |_, child| {
        let child = child.expect("the page");
        let value = animation.value();
        slide(
            Offset::new(0.0, lerp(0.25, 0.0, Curve::FastOutSlowIn.apply(value))),
            faded(Curve::EaseIn.apply(value), child),
        )
    })
    .child(child)
    .into()
}

/// `CupertinoPageTransition`'s slides: the page enters from the end edge
/// (`linearToEaseOut`, `easeInToLinear` back) while the page below moves a
/// third of its width the other way.
fn cupertino(animation: &DoubleAnimation, secondary: &DoubleAnimation, child: Widget) -> Widget {
    let below = {
        let secondary = secondary.clone();
        AnimatedBuilder::new(secondary.clone(), move |_, child| {
            let child = child.expect("the page");
            let curve = if secondary.status() == AnimationStatus::Reverse {
                Curve::EaseInToLinear
            } else {
                Curve::LinearToEaseOut
            };
            slide(
                Offset::new(lerp(0.0, -1.0 / 3.0, curve.apply(secondary.value())), 0.0),
                child,
            )
        })
        .child(child)
    };
    let animation = animation.clone();
    AnimatedBuilder::new(animation.clone(), move |_, child| {
        let child = child.expect("the page");
        let curve = if animation.status() == AnimationStatus::Reverse {
            Curve::EaseInToLinear
        } else {
            Curve::LinearToEaseOut
        };
        slide(
            Offset::new(lerp(1.0, 0.0, curve.apply(animation.value())), 0.0),
            child,
        )
    })
    .child(below)
    .into()
}
