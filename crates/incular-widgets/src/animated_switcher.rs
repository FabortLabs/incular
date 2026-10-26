//! Flutter's `AnimatedSwitcher` (`widgets/animated_switcher.dart`): a new
//! child fades in over the old one, which fades out.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::Alignment;

use crate::tree::RetainedState;
use crate::{Opacity, Stack, Widget};

/// Builds a child's transition from its animation value (0 hidden, 1 shown).
pub type AnimatedSwitcherTransitionBuilder = Rc<dyn Fn(Widget, f32) -> Widget>;
/// Lays out the current child above the outgoing ones.
pub type AnimatedSwitcherLayoutBuilder = Rc<dyn Fn(Option<Widget>, Vec<Widget>) -> Widget>;

/// Flutter's `AnimatedSwitcher`: when `child` changes to a widget that
/// cannot update it (another type or key), the new child transitions in
/// over `duration` while the old ones transition out.
#[derive(Clone)]
pub struct AnimatedSwitcher {
    child: Option<Widget>,
    duration: Duration,
    reverse_duration: Option<Duration>,
    switch_in_curve: Curve,
    switch_out_curve: Curve,
    transition_builder: AnimatedSwitcherTransitionBuilder,
    layout_builder: AnimatedSwitcherLayoutBuilder,
}

impl AnimatedSwitcher {
    #[must_use]
    pub fn new(duration: Duration, child: Option<Widget>) -> Self {
        Self {
            child,
            duration,
            reverse_duration: None,
            switch_in_curve: Curve::Linear,
            switch_out_curve: Curve::Linear,
            transition_builder: Rc::new(Self::default_transition_builder),
            layout_builder: Rc::new(Self::default_layout_builder),
        }
    }

    /// The duration of the outgoing transition. Defaults to `duration`.
    #[must_use]
    pub fn reverse_duration(mut self, duration: Duration) -> Self {
        self.reverse_duration = Some(duration);
        self
    }

    #[must_use]
    pub fn switch_in_curve(mut self, curve: Curve) -> Self {
        self.switch_in_curve = curve;
        self
    }

    #[must_use]
    pub fn switch_out_curve(mut self, curve: Curve) -> Self {
        self.switch_out_curve = curve;
        self
    }

    #[must_use]
    pub fn transition_builder(mut self, builder: impl Fn(Widget, f32) -> Widget + 'static) -> Self {
        self.transition_builder = Rc::new(builder);
        self
    }

    #[must_use]
    pub fn layout_builder(
        mut self,
        builder: impl Fn(Option<Widget>, Vec<Widget>) -> Widget + 'static,
    ) -> Self {
        self.layout_builder = Rc::new(builder);
        self
    }

    /// Flutter's `defaultTransitionBuilder`: a fade.
    #[must_use]
    pub fn default_transition_builder(child: Widget, animation: f32) -> Widget {
        Opacity::new(animation, child).into()
    }

    /// Flutter's `defaultLayoutBuilder`: a centered stack, outgoing children
    /// below the current one.
    #[must_use]
    pub fn default_layout_builder(current: Option<Widget>, previous: Vec<Widget>) -> Widget {
        Stack::new(previous.into_iter().chain(current))
            .alignment(Alignment::CENTER)
            .into()
    }
}

struct Entry {
    controller: AnimationController,
    child: RefCell<Widget>,
    key: u64,
    /// `CurvedAnimation._curveDirection`: the direction that started the
    /// running animation, which picks its curve until it settles.
    curve_direction: Cell<Option<AnimationStatus>>,
}

impl Entry {
    fn value(&self, config: &AnimatedSwitcher) -> f32 {
        let t = self.controller.value();
        let curve = match self.curve_direction.get() {
            Some(AnimationStatus::Reverse) => config.switch_out_curve,
            _ => config.switch_in_curve,
        };
        curve.apply(t)
    }

    fn start(&self, direction: AnimationStatus) {
        if self.curve_direction.get().is_none() {
            self.curve_direction.set(Some(direction));
        }
    }
}

struct SwitcherState {
    config: RefCell<Option<Rc<AnimatedSwitcher>>>,
    current: RefCell<Option<Rc<Entry>>>,
    outgoing: RefCell<Vec<Rc<Entry>>>,
    child_number: Cell<u64>,
    revision: RefCell<Rc<Cell<u64>>>,
    this: RefCell<Weak<SwitcherState>>,
}

impl RetainedState for SwitcherState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl SwitcherState {
    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// `initState` / `didUpdateWidget` for a new descriptor.
    fn update(self: &Rc<Self>, config: &Rc<AnimatedSwitcher>) {
        let previous = self.config.replace(Some(config.clone()));
        let Some(previous) = previous else {
            self.add_entry(config, false);
            return;
        };
        if Rc::ptr_eq(&previous, config) {
            return;
        }
        let current = self.current.borrow().clone();
        match (&config.child, current) {
            (Some(child), Some(entry)) if child.can_update(&entry.child.borrow()) => {
                *entry.child.borrow_mut() = child.clone();
            }
            (None, None) => {}
            _ => {
                self.child_number.set(self.child_number.get() + 1);
                self.add_entry(config, true);
            }
        }
    }

    /// `_addEntryForNewChild`.
    fn add_entry(self: &Rc<Self>, config: &AnimatedSwitcher, animate: bool) {
        if let Some(current) = self.current.take() {
            current
                .controller
                .set_duration(config.reverse_duration.unwrap_or(config.duration));
            current.start(AnimationStatus::Reverse);
            current.controller.reverse_on_next_tick();
            self.outgoing.borrow_mut().push(current);
        }
        let Some(child) = config.child.clone() else {
            return;
        };
        let controller = AnimationController::new(config.duration);
        let entry = Rc::new(Entry {
            controller: controller.clone(),
            child: RefCell::new(child),
            key: self.child_number.get(),
            curve_direction: Cell::new(None),
        });
        let weak_state = self.this.borrow().clone();
        let weak_entry = Rc::downgrade(&entry);
        controller.add_listener(move |_| {
            let (Some(state), Some(entry)) = (weak_state.upgrade(), weak_entry.upgrade()) else {
                return;
            };
            let status = entry.controller.status();
            if matches!(
                status,
                AnimationStatus::Completed | AnimationStatus::Dismissed
            ) {
                entry.curve_direction.set(None);
            }
            // A dismissed outgoing child leaves the switcher.
            if status == AnimationStatus::Dismissed {
                state
                    .outgoing
                    .borrow_mut()
                    .retain(|e| !Rc::ptr_eq(e, &entry));
            }
            state.bump();
        });
        if animate {
            entry.start(AnimationStatus::Forward);
            controller.forward_on_next_tick();
        } else {
            controller.set_value(1.0);
        }
        *self.current.borrow_mut() = Some(entry);
    }

    fn build(&self) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the switcher updates before it builds");
        let revision = self.revision.borrow().clone();
        let transition = |entry: &Rc<Entry>| {
            let child =
                (config.transition_builder)(entry.child.borrow().clone(), entry.value(&config));
            Widget::animation_ticker_with_revision(
                entry.controller.clone(),
                false,
                revision.clone(),
                child,
            )
            .with_key(entry.key)
        };
        let current = self.current.borrow().clone();
        let current_key = current.as_ref().map(|entry| entry.key);
        let previous = self
            .outgoing
            .borrow()
            .iter()
            .filter(|entry| Some(entry.key) != current_key)
            .map(&transition)
            .collect();
        (config.layout_builder)(current.as_ref().map(transition), previous)
    }
}

impl From<AnimatedSwitcher> for Widget {
    fn from(value: AnimatedSwitcher) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |revision| SwitcherState {
                config: RefCell::new(None),
                current: RefCell::new(None),
                outgoing: RefCell::new(Vec::new()),
                child_number: Cell::new(0),
                revision: RefCell::new(revision),
                this: RefCell::new(Weak::new()),
            },
            move |state, _, _| {
                if state.this.borrow().upgrade().is_none() {
                    *state.this.borrow_mut() = Rc::downgrade(state);
                }
                state.update(&config);
                state.build()
            },
            false,
        )
    }
}
