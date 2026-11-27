//! Flutter's `AnimatedSize` (`widgets/animated_size.dart`,
//! `rendering/animated_size.dart`): a box that animates to its child's size.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::{Alignment, AlignmentGeometry, Clip, Constraints, TextDirection};
use incular_core::{Offset, Size};

use crate::{BuildContext, Widget};
use crate::custom_layout::{ChildSizedLayout, CustomLayoutDelegate};
use crate::tree::{RetainedState, WidgetKind};

/// Flutter's `AnimatedSize`: when the child's size changes, this widget's
/// size animates from its current size to the child's over `duration`,
/// aligning and clipping the child meanwhile.
#[derive(Clone)]
pub struct AnimatedSize {
    child: Widget,
    duration: Duration,
    reverse_duration: Option<Duration>,
    curve: Curve,
    alignment: AlignmentGeometry,
    clip_behavior: Clip,
    on_end: Option<Rc<dyn Fn()>>,
}

impl AnimatedSize {
    #[must_use]
    pub fn new(duration: Duration, child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            duration,
            reverse_duration: None,
            curve: Curve::Linear,
            alignment: AlignmentGeometry::CENTER,
            clip_behavior: Clip::HardEdge,
            on_end: None,
        }
    }

    /// Flutter's `reverseDuration`: the duration of a reverse run of the
    /// size controller, `duration` when unset. Flutter 3.50 only runs the
    /// size animation forward, so this is stored on the controller for
    /// source compatibility and has no effect on today's size changes.
    #[must_use]
    pub fn reverse_duration(mut self, duration: Duration) -> Self {
        self.reverse_duration = Some(duration);
        self
    }

    /// The configured reverse duration, if any.
    #[must_use]
    pub fn reverse_duration_value(&self) -> Option<Duration> {
        self.reverse_duration
    }

    /// Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Where the child sits while the size differs. A directional alignment
    /// follows the current text direction. Defaults to center.
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<AlignmentGeometry>) -> Self {
        self.alignment = alignment.into();
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
    const DISPOSE_ON_UNMOUNT: bool = true;

    fn dispose(&self) {
        // An inspector can keep a controller handle alive after unmount.
        // Dispose the shared controller rather than only dropping our handle.
        let _ = self.controller.try_dispose();
    }

    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl Drop for SizeState {
    fn drop(&mut self) {
        // Also release the controller when the entire tree is dropped
        // without reconciling its elements out one at a time.
        self.dispose();
    }
}

impl SizeState {
    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// `_animatedSize`: the size tween at the curved animation value.
    fn animated(&self, curve: &Curve) -> Size {
        let t = curve.apply(self.controller.value());
        let (begin, end) = (self.begin.get(), self.end.get());
        // An overshooting curve can carry the tween below zero. Flutter does
        // not assert here: the caller constrains the animated size, which
        // clamps it to the box, so clamping to zero first gives the same size.
        Size::new(
            (begin.width + (end.width - begin.width) * t).max(0.),
            (begin.height + (end.height - begin.height) * t).max(0.),
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
    }

    /// `RenderAnimatedSize.performLayout`, after the child is laid out under
    /// `constraints`: the box size and the child's aligned offset.
    fn perform_layout(
        &self,
        constraints: Constraints,
        child: Size,
        alignment: Alignment,
    ) -> (Size, Offset) {
        let Some(config) = self.config.borrow().clone() else {
            return (constraints.constrain(child), Offset::ZERO);
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
            return (smallest, Offset::ZERO);
        }
        self.child_laid_out(child);
        let size = constraints.constrain(self.animated(&config.curve));
        self.current.set(size);
        // `alignChild`: the child sits at the alignment within the box.
        let offset = Offset::new(
            (size.width - child.width) / 2. * (1. + alignment.x),
            (size.height - child.height) / 2. * (1. + alignment.y),
        );
        (size, offset)
    }

    fn build(self: &Rc<Self>, context: &BuildContext<'_>) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the size updates before it builds");
        let alignment = match config.alignment {
            AlignmentGeometry::Physical(value) => value,
            logical => logical.resolve(context.text_direction().unwrap_or(TextDirection::Ltr)),
        };
        // Flutter sizes the box in the same layout as the child, after it.
        let layout = {
            let weak = Rc::downgrade(self);
            ChildSizedLayout::new(move |constraints, child| match weak.upgrade() {
                Some(state) => state.perform_layout(constraints, child, alignment),
                None => (constraints.constrain(child), Offset::ZERO),
            })
            .clip_overflow(config.clip_behavior)
        };
        let sized = Widget::from_kind(WidgetKind::CustomLayout {
            delegate: CustomLayoutDelegate::ChildSized(layout),
            children: vec![config.child.clone()],
        });
        Widget::animation_ticker_with_revision(
            self.controller.clone(),
            false,
            self.revision.borrow().clone(),
            sized,
        )
    }
}

impl From<AnimatedSize> for Widget {
    fn from(value: AnimatedSize) -> Self {
        let config = Rc::new(value);
        Widget::mount_scoped_layout_builder(
            {
                let config = config.clone();
                move |revision| {
                    let controller = AnimationController::new(config.duration);
                    controller.set_reverse_duration(config.reverse_duration);
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
            move |state, context, _| {
                if state.this.borrow().upgrade().is_none() {
                    *state.this.borrow_mut() = Rc::downgrade(state);
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        state.bump();
                        let on_end = state
                            .config
                            .borrow()
                            .as_ref()
                            .and_then(|config| config.on_end.clone());
                        if state.controller.status() == AnimationStatus::Completed
                            && let Some(on_end) = on_end
                        {
                            on_end();
                        }
                    });
                }
                // `didUpdateWidget`: the new durations apply to later runs.
                state.controller.set_duration(config.duration);
                state
                    .controller
                    .set_reverse_duration(config.reverse_duration);
                *state.config.borrow_mut() = Some(config.clone());
                state.build(context)
            },
            true,
        )
    }
}
