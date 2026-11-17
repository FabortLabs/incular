//! Subtree-wide motion defaults and reduced-motion policy.

use incular_animation::{Channel, Transition, ms};

use crate::{BuildContext, Widget};

/// Whether motion follows the platform's reduced-motion preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReducedMotion {
    /// Follow the platform setting.
    #[default]
    System,
    /// Always reduce.
    Always,
    /// Never reduce.
    Never,
}

/// How one motion site behaves under reduced motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reduced {
    /// Transforms, layout and shared motion snap; opacity and colors keep a
    /// short fade.
    #[default]
    Reduce,
    /// Essential motion (a progress indicator): animate regardless.
    Keep,
}

/// The longest fade reduced motion keeps.
const REDUCED_FADE: std::time::Duration = ms(150);

/// Motion defaults for a subtree: the default transition and the
/// reduced-motion policy.
///
/// ```ignore
/// MotionConfig::new(app)
///     .default_transition(Spring::smooth())
///     .reduced_motion(ReducedMotion::System)
/// ```
#[derive(Clone)]
pub struct MotionConfig {
    transition: Transition,
    reduced_motion: ReducedMotion,
    child: Widget,
}

/// The resolved configuration published to a subtree.
#[derive(Clone)]
pub(crate) struct MotionConfigScope {
    pub(crate) transition: Transition,
    pub(crate) reduced_motion: ReducedMotion,
}

impl Default for MotionConfigScope {
    fn default() -> Self {
        Self {
            transition: Transition::default(),
            reduced_motion: ReducedMotion::System,
        }
    }
}

impl MotionConfig {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            transition: Transition::default(),
            reduced_motion: ReducedMotion::System,
            child: child.into(),
        }
    }

    /// The transition motion uses when a pose names none.
    #[must_use]
    pub fn default_transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = transition.into();
        self
    }

    #[must_use]
    pub fn reduced_motion(mut self, reduced_motion: ReducedMotion) -> Self {
        self.reduced_motion = reduced_motion;
        self
    }

    /// The configuration in effect at `context`.
    pub(crate) fn of(context: &BuildContext<'_>) -> MotionConfigScope {
        context.depend_on::<MotionConfigScope>().unwrap_or_default()
    }

    /// Whether motion at `context` is reduced.
    pub(crate) fn is_reduced(context: &BuildContext<'_>, scope: &MotionConfigScope) -> bool {
        match scope.reduced_motion {
            ReducedMotion::System => context.disable_animations(),
            ReducedMotion::Always => true,
            ReducedMotion::Never => false,
        }
    }
}

impl From<MotionConfig> for Widget {
    fn from(config: MotionConfig) -> Self {
        Widget::environment_scope(
            MotionConfigScope {
                transition: config.transition,
                reduced_motion: config.reduced_motion,
            },
            config.child,
        )
    }
}

/// The transition actually used for a `channel` value under reduced motion:
/// geometry snaps, opacity and colors fade for at most [`REDUCED_FADE`].
pub(crate) fn reduce(transition: &Transition, channel: Channel) -> Transition {
    match channel {
        Channel::Geometry => Transition::instant(),
        Channel::Opacity | Channel::Color => {
            let resolved = transition.for_channel(channel);
            match resolved.kind() {
                incular_animation::TransitionKind::Ease { duration, curve } => {
                    Transition::ease(duration.min(REDUCED_FADE)).curve(curve)
                }
                incular_animation::TransitionKind::Instant => Transition::instant(),
                incular_animation::TransitionKind::Spring(_) => Transition::ease(REDUCED_FADE),
            }
        }
    }
}
