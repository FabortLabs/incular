//! Declarative effect chains, after the `flutter_animate` package:
//! `Text::new("Hi").animate().fade_in().slide_y(0.2).then().scale(0.9)`.
//!
//! Each effect has a delay, a duration and a curve; by default an effect
//! starts with the previous one (they run together), and [`Animate::then`]
//! makes the following effects start after everything added so far ends,
//! as `flutter_animate`'s `then` does.
//!
//! A chain runs on the motion engine: its playhead (time, a trigger or a
//! progress source) samples the effects into compositor layer values, so a
//! frame never rebuilds the child. Only [`Animate::shimmer`] and
//! [`Animate::build_each_frame`] rebuild, and say so.

use std::{fmt, rc::Rc, time::Duration};

use incular_animation::{Curve, Keyframes};
use incular_core::{Color, Offset};
use incular_rendering::Shadow;

use crate::Widget;
use crate::dx::motion::{
    AnimationHandle, Motion, Play, Pose, Progress, Prop,
    timeline::{BuildEach, TimelineSpec},
};

/// `flutter_animate`'s `Animate.defaultDuration`.
pub const DEFAULT_EFFECT_DURATION: Duration = Duration::from_millis(300);
/// `Animate.defaultCurve`.
pub const DEFAULT_EFFECT_CURVE: Curve = Curve::Linear;

/// The edge a [`EffectKind::ClipReveal`] uncovers from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevealFrom {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
    /// A circle growing from the center.
    Center,
}

/// A `custom(|t, pose| …)` effect: maps eased progress to a pose.
#[derive(Clone)]
pub struct CustomEffect(pub(crate) Rc<dyn Fn(f32, Pose) -> Pose>);

impl fmt::Debug for CustomEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CustomEffect(..)")
    }
}

impl PartialEq for CustomEffect {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// One animated property of an [`Animate`] chain.
#[derive(Clone, Debug, PartialEq)]
pub enum EffectKind {
    /// Opacity from `begin` to `end`.
    Fade { begin: f32, end: f32 },
    /// Translation as a fraction of the child's size (`SlideEffect`).
    Slide { begin: Offset, end: Offset },
    /// Translation in logical pixels (`MoveEffect`).
    Move { begin: Offset, end: Offset },
    /// Uniform scale about the center (`ScaleEffect`).
    Scale { begin: f32, end: f32 },
    /// Rotation in turns about the center (`RotateEffect`).
    Rotate { begin: f32, end: f32 },
    /// Gaussian blur sigma (`BlurEffect`).
    Blur { begin: f32, end: f32 },
    /// A sideways shake of `hz` oscillations peaking at `offset` pixels
    /// (`ShakeEffect`).
    Shake { hz: f32, offset: f32 },
    /// Tints toward `color` by an amount from `begin` to `end` (0 to 1).
    Tint { color: Color, begin: f32, end: f32 },
    /// Saturation from `begin` to `end` (1 is unchanged, 0 is gray).
    Saturate { begin: f32, end: f32 },
    /// A drop shadow whose strength goes from `begin` to `end` (0 to 1).
    Shadow {
        shadow: Shadow,
        begin: f32,
        end: f32,
    },
    /// Uncovers the child from an edge (or the center) with a clip.
    ClipReveal { from: RevealFrom },
    /// A highlight band of `color` sweeping across the child at `angle`
    /// radians. Paint-tier: it rebuilds a shader mask each frame.
    Shimmer { color: Color, angle: f32 },
    /// A property visiting keyframe values.
    Keys { prop: Prop, keys: Keyframes },
    /// A pose computed from progress; stays compositor-only.
    Custom(CustomEffect),
}

/// An effect with its timing within the chain.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub kind: EffectKind,
    pub delay: Duration,
    pub duration: Duration,
    pub curve: Curve,
}

impl Effect {
    fn end(&self) -> Duration {
        self.delay + self.duration
    }

    /// This effect's eased progress when the effects are `elapsed` in.
    pub(crate) fn progress(&self, elapsed: Duration) -> f32 {
        if elapsed <= self.delay {
            return 0.;
        }
        if self.duration.is_zero() {
            return 1.;
        }
        let t = (elapsed - self.delay).as_secs_f32() / self.duration.as_secs_f32();
        self.curve.apply(t.clamp(0., 1.))
    }
}

/// How an [`Animate`] repeats once it reaches the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimateRepeat {
    /// Play once and hold the final frame.
    #[default]
    Once,
    /// Restart from the beginning forever.
    Loop,
    /// Play forward then backward forever.
    PingPong,
}

/// A chain of effects applied to a child.
#[derive(Clone)]
pub struct Animate {
    child: Widget,
    effects: Vec<Effect>,
    /// Where the next effect starts.
    cursor: Duration,
    /// The timing the next effect inherits.
    duration: Duration,
    curve: Curve,
    delay: Duration,
    repeat: AnimateRepeat,
    on_complete: Option<Rc<dyn Fn()>>,
    play: Play,
    progress: Option<Progress>,
    build_each: Option<BuildEach>,
}

impl Animate {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            effects: Vec::new(),
            cursor: Duration::ZERO,
            duration: DEFAULT_EFFECT_DURATION,
            curve: DEFAULT_EFFECT_CURVE,
            delay: Duration::ZERO,
            repeat: AnimateRepeat::Once,
            on_complete: None,
            play: Play::OnMount,
            progress: None,
            build_each: None,
        }
    }

    /// The duration of effects added after this call.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// The duration in milliseconds of effects added after this call.
    #[must_use]
    pub fn ms(self, millis: u64) -> Self {
        self.duration(Duration::from_millis(millis))
    }

    /// The curve of effects added after this call.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// The whole chain waits `delay` before starting (`Animate.delay`).
    #[must_use]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Adds an effect with the current timing, starting with the previous
    /// effect.
    #[must_use]
    pub fn effect(mut self, kind: EffectKind) -> Self {
        // An effect starts with the previous one, unless `then` moved the
        // cursor past it.
        let start = self
            .effects
            .last()
            .map_or(self.cursor, |last| last.delay.max(self.cursor));
        self.effects.push(Effect {
            kind,
            delay: start,
            duration: self.duration,
            curve: self.curve,
        });
        self
    }

    /// Adds an effect with explicit timing relative to the chain start.
    #[must_use]
    pub fn effect_at(
        mut self,
        kind: EffectKind,
        delay: Duration,
        duration: Duration,
        curve: Curve,
    ) -> Self {
        self.effects.push(Effect {
            kind,
            delay,
            duration,
            curve,
        });
        self
    }

    /// `flutter_animate`'s `then`: effects added next start after every
    /// effect so far has ended.
    #[must_use]
    pub fn then(mut self) -> Self {
        self.cursor = self.total_duration();
        self
    }

    /// Like [`Animate::then`], with a pause of `gap` first.
    #[must_use]
    pub fn then_gap(mut self, gap: Duration) -> Self {
        self.cursor = self.total_duration() + gap;
        self
    }

    /// Like [`Animate::then`], starting `overlap` before the previous
    /// effects end (motion.dev's `at: "-0.1"`).
    #[must_use]
    pub fn then_overlap(mut self, overlap: Duration) -> Self {
        self.cursor = self.total_duration().saturating_sub(overlap);
        self
    }

    /// Applies a reusable preset: `fn pop_in(a: Animate) -> Animate`.
    #[must_use]
    pub fn apply(self, preset: impl FnOnce(Self) -> Self) -> Self {
        preset(self)
    }

    #[must_use]
    pub fn fade_in(self) -> Self {
        self.effect(EffectKind::Fade { begin: 0., end: 1. })
    }

    #[must_use]
    pub fn fade_out(self) -> Self {
        self.effect(EffectKind::Fade { begin: 1., end: 0. })
    }

    #[must_use]
    pub fn fade(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Fade { begin, end })
    }

    /// Slides in horizontally from `begin` times the child's width.
    #[must_use]
    pub fn slide_x(self, begin: f32) -> Self {
        self.effect(EffectKind::Slide {
            begin: Offset::new(begin, 0.),
            end: Offset::ZERO,
        })
    }

    /// Slides in vertically from `begin` times the child's height.
    #[must_use]
    pub fn slide_y(self, begin: f32) -> Self {
        self.effect(EffectKind::Slide {
            begin: Offset::new(0., begin),
            end: Offset::ZERO,
        })
    }

    #[must_use]
    pub fn slide(self, begin: Offset, end: Offset) -> Self {
        self.effect(EffectKind::Slide { begin, end })
    }

    /// Moves from `begin` pixels to rest.
    #[must_use]
    pub fn move_from(self, begin: Offset) -> Self {
        self.effect(EffectKind::Move {
            begin,
            end: Offset::ZERO,
        })
    }

    /// Moves vertically from `begin` to `end` pixels.
    #[must_use]
    pub fn move_y(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Move {
            begin: Offset::new(0., begin),
            end: Offset::new(0., end),
        })
    }

    /// Moves horizontally from `begin` to `end` pixels.
    #[must_use]
    pub fn move_x(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Move {
            begin: Offset::new(begin, 0.),
            end: Offset::new(end, 0.),
        })
    }

    /// Scales from `begin` to full size.
    #[must_use]
    pub fn scale_in(self, begin: f32) -> Self {
        self.effect(EffectKind::Scale { begin, end: 1. })
    }

    #[must_use]
    pub fn scale(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Scale { begin, end })
    }

    /// Rotates from `begin` turns to rest.
    #[must_use]
    pub fn rotate_in(self, begin: f32) -> Self {
        self.effect(EffectKind::Rotate { begin, end: 0. })
    }

    #[must_use]
    pub fn rotate(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Rotate { begin, end })
    }

    /// Unblurs from `sigma` to sharp.
    #[must_use]
    pub fn blur_in(self, sigma: f32) -> Self {
        self.effect(EffectKind::Blur {
            begin: sigma,
            end: 0.,
        })
    }

    /// `ShakeEffect` with `flutter_animate`'s defaults: 8 Hz, 5 pixels.
    #[must_use]
    pub fn shake(self) -> Self {
        self.effect(EffectKind::Shake { hz: 8., offset: 5. })
    }

    /// Tints toward `color` up to `amount` (0 to 1).
    #[must_use]
    pub fn tint(self, color: Color, amount: f32) -> Self {
        self.effect(EffectKind::Tint {
            color,
            begin: 0.,
            end: amount,
        })
    }

    /// Saturation from `begin` to `end` (1 is unchanged, 0 is gray).
    #[must_use]
    pub fn saturate(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Saturate { begin, end })
    }

    /// Fades a drop shadow in.
    #[must_use]
    pub fn shadow(self, shadow: Shadow) -> Self {
        self.effect(EffectKind::Shadow {
            shadow,
            begin: 0.,
            end: 1.,
        })
    }

    /// Uncovers the child from an edge or the center.
    #[must_use]
    pub fn clip_reveal(self, from: RevealFrom) -> Self {
        self.effect(EffectKind::ClipReveal { from })
    }

    /// A soft white highlight sweeping left to right. Paint-tier.
    #[must_use]
    pub fn shimmer(self) -> Self {
        self.shimmer_with(Color::rgba(0xFF, 0xFF, 0xFF, 0x80), 0.)
    }

    /// A highlight band of `color` sweeping at `angle` radians. Paint-tier.
    #[must_use]
    pub fn shimmer_with(self, color: Color, angle: f32) -> Self {
        self.effect(EffectKind::Shimmer { color, angle })
    }

    /// A property visiting `values`, evenly spaced over the effect.
    #[must_use]
    pub fn keys(self, prop: Prop, values: impl IntoIterator<Item = f32>) -> Self {
        let duration = self.duration;
        self.effect(EffectKind::Keys {
            prop,
            keys: Keyframes::new(values, duration),
        })
    }

    /// A pose computed from the effect's eased progress (compositor-only):
    /// `.custom(|t, p| p.y((t * TAU).sin() * 6.))`.
    #[must_use]
    pub fn custom(self, effect: impl Fn(f32, Pose) -> Pose + 'static) -> Self {
        self.effect(EffectKind::Custom(CustomEffect(Rc::new(effect))))
    }

    /// Rebuilds the child each frame with the playhead position (0 to 1).
    /// The escape hatch for effects that are not layer values: it costs a
    /// rebuild of the child per frame.
    #[must_use]
    pub fn build_each_frame(mut self, build: impl Fn(f32, Widget) -> Widget + 'static) -> Self {
        self.build_each = Some(Rc::new(build));
        self
    }

    #[must_use]
    pub fn repeat(mut self, repeat: AnimateRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Runs when a non-repeating chain reaches its end.
    #[must_use]
    pub fn on_complete(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_complete = Some(Rc::new(callback));
        self
    }

    /// What starts and steers the chain (default [`Play::OnMount`]).
    #[must_use]
    pub fn play(mut self, play: Play) -> Self {
        self.play = play;
        self
    }

    /// Drives the playhead from a progress source instead of time.
    #[must_use]
    pub fn progress(mut self, progress: impl Into<Progress>) -> Self {
        self.progress = Some(progress.into());
        self
    }

    /// `Animate.target`: drives the chain to this progress (0 to 1) instead
    /// of playing it, animating whenever the target changes.
    #[deprecated(note = "use `.progress(Progress::value(target))`")]
    #[must_use]
    pub fn target(self, target: f32) -> Self {
        self.progress(Progress::value(target))
    }

    /// Drives the chain with `handle`: shorthand for
    /// `.play(Play::handle(handle))`.
    #[must_use]
    pub fn handle(self, handle: &AnimationHandle) -> Self {
        self.play(Play::handle(handle))
    }

    /// The effects with their final timing.
    #[must_use]
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// When the last effect ends.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.effects
            .iter()
            .map(Effect::end)
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// The child as it looks with the effects `elapsed` in, held there.
    #[must_use]
    pub fn frame(&self, elapsed: Duration) -> Widget {
        let total = self.delay + self.total_duration();
        let position = if total.is_zero() {
            1.
        } else {
            ((elapsed + self.delay).as_secs_f32() / total.as_secs_f32()).clamp(0., 1.)
        };
        self.clone().progress(Progress::value(position)).into()
    }

    fn into_parts(self) -> (Widget, Motion) {
        let spec = TimelineSpec {
            effects: self.effects,
            delay: self.delay,
            repeat: self.repeat,
            on_complete: self.on_complete,
            play: self.play,
            progress: self.progress,
            build_each: self.build_each,
        };
        let mut motion = Motion::new();
        if let Some(options) = spec.in_view_options() {
            motion.in_view = options;
        }
        motion.timeline = Some(Rc::new(spec));
        (self.child, motion)
    }
}

impl From<Animate> for Widget {
    fn from(animate: Animate) -> Self {
        let (child, motion) = animate.into_parts();
        crate::dx::motion::node::motion_widget(child, motion)
    }
}

/// `.animate()` for anything that becomes a [`Widget`].
pub trait AnimateExt: Into<Widget> + Sized {
    /// Starts an effect chain on this widget.
    #[must_use]
    fn animate(self) -> Animate {
        Animate::new(self)
    }
}

impl<T: Into<Widget>> AnimateExt for T {}
