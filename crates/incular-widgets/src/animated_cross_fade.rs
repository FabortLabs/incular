//! Flutter's `AnimatedCrossFade` (`widgets/animated_cross_fade.dart`): two
//! children cross-fade while the size animates between theirs.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::{Alignment, Clip};

use crate::tree::RetainedState;
use crate::{
    AnimatedSize, ClipRect, ExcludeFocus, ExcludeSemantics, IgnorePointer, Opacity, Positioned,
    Stack, TickerMode, Widget,
};

/// Flutter's `CrossFadeState`: which child is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossFadeState {
    ShowFirst,
    ShowSecond,
}

impl CrossFadeState {
    /// The key Flutter gives the child's layout slot (`ValueKey(state)`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::ShowFirst => "CrossFadeState.showFirst",
            Self::ShowSecond => "CrossFadeState.showSecond",
        }
    }
}

/// Lays out the top (fading in) and bottom (fading out) children, each with
/// the state whose key its slot must carry.
pub type AnimatedCrossFadeBuilder =
    Rc<dyn Fn(Widget, CrossFadeState, Widget, CrossFadeState) -> Widget>;

/// Flutter's `AnimatedCrossFade`.
#[derive(Clone)]
pub struct AnimatedCrossFade {
    first_child: Widget,
    second_child: Widget,
    cross_fade_state: CrossFadeState,
    duration: Duration,
    reverse_duration: Option<Duration>,
    first_curve: Curve,
    second_curve: Curve,
    size_curve: Curve,
    alignment: Alignment,
    layout_builder: AnimatedCrossFadeBuilder,
    exclude_bottom_focus: bool,
}

impl AnimatedCrossFade {
    #[must_use]
    pub fn new(
        first_child: impl Into<Widget>,
        second_child: impl Into<Widget>,
        cross_fade_state: CrossFadeState,
        duration: Duration,
    ) -> Self {
        Self {
            first_child: first_child.into(),
            second_child: second_child.into(),
            cross_fade_state,
            duration,
            reverse_duration: None,
            first_curve: Curve::Linear,
            second_curve: Curve::Linear,
            size_curve: Curve::Linear,
            alignment: Alignment::TOP_CENTER,
            layout_builder: Rc::new(Self::default_layout_builder),
            exclude_bottom_focus: true,
        }
    }

    /// The duration back to the first child. Defaults to `duration`.
    #[must_use]
    pub fn reverse_duration(mut self, duration: Duration) -> Self {
        self.reverse_duration = Some(duration);
        self
    }

    #[must_use]
    pub fn first_curve(mut self, curve: Curve) -> Self {
        self.first_curve = curve;
        self
    }

    #[must_use]
    pub fn second_curve(mut self, curve: Curve) -> Self {
        self.second_curve = curve;
        self
    }

    #[must_use]
    pub fn size_curve(mut self, curve: Curve) -> Self {
        self.size_curve = curve;
        self
    }

    /// Where the children sit while the size animates. Defaults to top
    /// center.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub fn layout_builder(
        mut self,
        builder: impl Fn(Widget, CrossFadeState, Widget, CrossFadeState) -> Widget + 'static,
    ) -> Self {
        self.layout_builder = Rc::new(builder);
        self
    }

    /// Whether the fading-out child is excluded from focus. Defaults to true.
    #[must_use]
    pub fn exclude_bottom_focus(mut self, exclude: bool) -> Self {
        self.exclude_bottom_focus = exclude;
        self
    }

    /// Flutter's `defaultLayoutBuilder`: the bottom child positioned over the
    /// top child's width, the top child sizing the stack.
    #[must_use]
    pub fn default_layout_builder(
        top: Widget,
        top_key: CrossFadeState,
        bottom: Widget,
        bottom_key: CrossFadeState,
    ) -> Widget {
        Stack::new([
            Widget::from(Positioned::new(bottom).left(0.).top(0.).right(0.))
                .with_key(bottom_key.key()),
            // Flutter's edge-less `Positioned` is a non-positioned child,
            // which sizes the stack.
            top.with_key(top_key.key()),
        ])
        .clip_behavior(Clip::None)
        .into()
    }
}

struct CrossFadeStateData {
    config: RefCell<Option<Rc<AnimatedCrossFade>>>,
    controller: AnimationController,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for CrossFadeStateData {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl CrossFadeStateData {
    fn update(&self, config: &Rc<AnimatedCrossFade>) {
        let previous = self.config.replace(Some(config.clone()));
        let Some(previous) = previous else {
            return;
        };
        self.controller.set_duration(config.duration);
        if previous.cross_fade_state != config.cross_fade_state {
            match config.cross_fade_state {
                CrossFadeState::ShowFirst => {
                    self.controller
                        .set_duration(config.reverse_duration.unwrap_or(config.duration));
                    self.controller.reverse_on_next_tick();
                }
                CrossFadeState::ShowSecond => self.controller.forward_on_next_tick(),
            }
        }
    }

    fn build(&self) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the cross-fade updates before it builds");
        let t = self.controller.value();
        // `_initAnimation`: the first child's opacity is inverted.
        let first = 1. - config.first_curve.apply(t);
        let second = config.second_curve.apply(t);
        let animating = self.controller.is_active();
        let forward = matches!(
            self.controller.status(),
            AnimationStatus::Forward | AnimationStatus::Completed
        );
        let (top_key, top, top_opacity, bottom_key, bottom, bottom_opacity) = if forward {
            (
                CrossFadeState::ShowSecond,
                config.second_child.clone(),
                second,
                CrossFadeState::ShowFirst,
                config.first_child.clone(),
                first,
            )
        } else {
            (
                CrossFadeState::ShowFirst,
                config.first_child.clone(),
                first,
                CrossFadeState::ShowSecond,
                config.second_child.clone(),
                second,
            )
        };
        let bottom = TickerMode::new(
            animating,
            IgnorePointer::new(ExcludeSemantics::new(
                ExcludeFocus::new(Opacity::new(bottom_opacity, bottom))
                    .excluding(config.exclude_bottom_focus),
            )),
        );
        let top = TickerMode::new(true, Opacity::new(top_opacity, top));
        let laid_out = (config.layout_builder)(top.into(), top_key, bottom.into(), bottom_key);
        let sized = AnimatedSize::new(config.duration, laid_out)
            .alignment(config.alignment)
            .curve(config.size_curve);
        let clipped = ClipRect::new(sized);
        Widget::animation_ticker_with_revision(
            self.controller.clone(),
            false,
            self.revision.borrow().clone(),
            clipped,
        )
    }
}

impl From<AnimatedCrossFade> for Widget {
    fn from(value: AnimatedCrossFade) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            {
                let config = config.clone();
                move |revision| {
                    let controller = AnimationController::new(config.duration);
                    if config.cross_fade_state == CrossFadeState::ShowSecond {
                        controller.set_value(1.0);
                    }
                    CrossFadeStateData {
                        config: RefCell::new(None),
                        controller,
                        listening: Cell::new(false),
                        revision: RefCell::new(revision),
                    }
                }
            },
            move |state, _, _| {
                if !state.listening.replace(true) {
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        if let Some(state) = weak.upgrade() {
                            let revision = state.revision.borrow().clone();
                            revision.set(revision.get().wrapping_add(1));
                        }
                    });
                }
                state.update(&config);
                state.build()
            },
            false,
        )
    }
}
