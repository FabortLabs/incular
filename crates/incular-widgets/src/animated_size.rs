//! Flutter's `AnimatedSize` (`widgets/animated_size.dart`,
//! `rendering/animated_size.dart`): a box that animates to its child's size.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::{Alignment, Clip, Constraints};
use incular_core::{Offset, Size};

use crate::tree::RetainedState;
use crate::{ClipRect, OverflowBox, SizedBox, Transform, Widget};

/// Flutter's `AnimatedSize`: when the child's size changes, this widget's
/// size animates from its current size to the child's over `duration`,
/// aligning and clipping the child meanwhile.
#[derive(Clone)]
pub struct AnimatedSize {
    child: Widget,
    duration: Duration,
    curve: Curve,
    alignment: Alignment,
    clip_behavior: Clip,
    on_end: Option<Rc<dyn Fn()>>,
}

impl AnimatedSize {
    #[must_use]
    pub fn new(duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            duration,
            curve: Curve::Linear,
            alignment: Alignment::CENTER,
            clip_behavior: Clip::HardEdge,
            on_end: None,
        }
    }

    /// Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Where the child sits while the size differs. Defaults to center.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Defaults to [`Clip::HardEdge`].
    #[must_use]
    pub fn clip_behavior(mut self, clip: Clip) -> Self {
        self.clip_behavior = clip;
        self
    }

    /// Called when a size animation completes.
    #[must_use]
    pub fn on_end(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_end = Some(Rc::new(callback));
        self
    }
}

/// Flutter's `RenderAnimatedSizeState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// No child size measured yet.
    Start,
    /// The child's size has not changed since the last layout.
    Stable,
    /// The child's size changed once and an animation runs toward it.
    Changed,
    /// The child's size keeps changing; the box tracks it without
    /// animating.
    Unstable,
}

struct SizeState {
    config: RefCell<Option<Rc<AnimatedSize>>>,
    controller: AnimationController,
    phase: Cell<Phase>,
    begin: Cell<Size>,
    end: Cell<Size>,
    /// The size laid out last, Flutter's `_currentSize`.
    current: Cell<Size>,
    revision: RefCell<Rc<Cell<u64>>>,
    this: RefCell<Weak<SizeState>>,
}

impl RetainedState for SizeState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl SizeState {
    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// `_animatedSize`: the size tween at the curved animation value.
    fn animated(&self, curve: Curve) -> Size {
        let t = curve.apply(self.controller.value());
        let (begin, end) = (self.begin.get(), self.end.get());
        Size::new(
            begin.width + (end.width - begin.width) * t,
            begin.height + (end.height - begin.height) * t,
        )
    }

    /// `_restartAnimation`: `forward(from: 0.0)`.
    fn restart(&self) {
        self.controller.set_value(0.0);
        self.controller.forward_on_next_tick();
    }

    /// The layout-time state machine of `RenderAnimatedSize.performLayout`,
    /// run each time the child is laid out.
    fn child_laid_out(&self, child: Size) {
        let end = self.end.get();
        match self.phase.get() {
            Phase::Start => {
                self.begin.set(child);
                self.end.set(child);
                self.phase.set(Phase::Stable);
            }
            Phase::Stable => {
                if end != child {
                    self.begin.set(self.current.get());
                    self.end.set(child);
                    self.restart();
                    self.phase.set(Phase::Changed);
                } else if self.controller.value() >= self.controller.bounds().1 {
                    // The animation finished: reset the target sizes.
                    self.begin.set(child);
                    self.end.set(child);
                } else if !self.controller.is_active() {
                    self.controller.forward_on_next_tick();
                }
            }
            Phase::Changed => {
                if end != child {
                    self.begin.set(child);
                    self.end.set(child);
                    self.restart();
                    self.phase.set(Phase::Unstable);
                } else {
                    self.phase.set(Phase::Stable);
                    if !self.controller.is_active() {
                        self.controller.forward_on_next_tick();
                    }
                }
            }
            Phase::Unstable => {
                if end != child {
                    self.begin.set(child);
                    self.end.set(child);
                    self.restart();
                } else {
                    self.controller.stop();
                    self.phase.set(Phase::Stable);
                }
            }
        }
        if end != self.end.get() {
            self.bump();
        }
    }

    fn build(self: &Rc<Self>, constraints: Constraints) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the size updates before it builds");
        let observer = {
            let weak = Rc::downgrade(self);
            move |size: Size| {
                if let Some(state) = weak.upgrade() {
                    state.child_laid_out(size);
                }
            }
        };
        let child = Widget::size_observer(observer, config.child.clone());
        let ticked = |child: Widget| {
            Widget::animation_ticker_with_revision(
                self.controller.clone(),
                false,
                self.revision.borrow().clone(),
                child,
            )
        };
        // Tight constraints decide the size: Flutter resets to `start`.
        if constraints.min_width() >= constraints.max_width()
            && constraints.min_height() >= constraints.max_height()
        {
            self.controller.stop();
            let smallest = constraints.smallest();
            self.begin.set(smallest);
            self.end.set(smallest);
            self.current.set(smallest);
            self.phase.set(Phase::Start);
            return ticked(child);
        }
        if self.phase.get() == Phase::Start {
            // Nothing measured yet: the child decides this first size.
            return ticked(child);
        }
        let size = constraints.constrain(self.animated(config.curve));
        self.current.set(size);
        // `alignChild`: the child sits at the alignment within the box.
        let end = self.end.get();
        let offset = Offset::new(
            (size.width - end.width) / 2. * (1. + config.alignment.x),
            (size.height - end.height) / 2. * (1. + config.alignment.y),
        );
        let placed = Transform::translation(offset, child);
        let unclamped = OverflowBox::new(placed)
            .min_width(constraints.min_width())
            .max_width(constraints.max_width())
            .min_height(constraints.min_height())
            .max_height(constraints.max_height());
        let clipped = ClipRect::new(unclamped).clip_behavior(config.clip_behavior);
        ticked(SizedBox::from_dimensions(Some(size.width), Some(size.height), Some(clipped)).into())
    }
}

impl From<AnimatedSize> for Widget {
    fn from(value: AnimatedSize) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            {
                let config = config.clone();
                move |revision| {
                    let controller = AnimationController::new(config.duration);
                    SizeState {
                        config: RefCell::new(None),
                        controller,
                        phase: Cell::new(Phase::Start),
                        begin: Cell::new(Size::ZERO),
                        end: Cell::new(Size::ZERO),
                        current: Cell::new(Size::ZERO),
                        revision: RefCell::new(revision),
                        this: RefCell::new(Weak::new()),
                    }
                }
            },
            move |state, _, constraints| {
                if state.this.borrow().upgrade().is_none() {
                    *state.this.borrow_mut() = Rc::downgrade(state);
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        state.bump();
                        if state.controller.status() == AnimationStatus::Completed
                            && let Some(on_end) = state
                                .config
                                .borrow()
                                .as_ref()
                                .and_then(|config| config.on_end.clone())
                        {
                            on_end();
                        }
                    });
                }
                // `didUpdateWidget`: the new durations apply to later runs.
                state.controller.set_duration(config.duration);
                *state.config.borrow_mut() = Some(config.clone());
                state.build(constraints)
            },
            true,
        )
    }
}
