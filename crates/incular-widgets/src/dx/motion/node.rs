//! The retained motion node: resolves active poses into per-property motion
//! values, ticks them on the tree's animation clock and writes the composed
//! result straight into retained compositor controllers.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use incular_animation::{MotionValue, Transition, schedule_frame};
use incular_config::Alignment;
use incular_core::{
    Code, Color, KeyboardKey, NamedKey, Offset, PointerDeviceKind, Rect, Size,
    reactivity::Subscription, time::Instant,
};
use incular_gestures::{FocusHighlightManager, FocusHighlightMode, FocusHighlightSubscription};
use incular_rendering::{ColorFilter, CornerRadii, RRect};

use super::{
    Motion,
    cond::{GroupScopes, GroupState, Interaction, Interactions},
    config::{MotionConfig, reduce},
    pose::{PROP_COUNT, Pose, PoseValue, Prop, Stage},
};
use crate::tree::{
    ColorFilterController, DropShadowController, FrameDelegate, FrameTick, RetainedState,
    WidgetKind,
};
use crate::{
    AffineController, AffineParts, BlurController, ClipRRect, ClipShape, CustomClipper, Listener,
    MouseRegion, OpacityController, Widget,
};

/// Free-drag settings resolved for the node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DragConfig(pub(crate) crate::dx::drag::Drag);

impl From<crate::dx::drag::Drag> for DragConfig {
    fn from(drag: crate::dx::drag::Drag) -> Self {
        Self(drag)
    }
}

/// Extra per-property contributions composed over the pose channel: free
/// drag, layout correction and `.animate()` timelines.
pub(crate) trait ChannelSource {
    /// Advances to `now`; returns whether its contribution changed.
    fn tick(&self, now: Instant) -> bool;
    fn is_active(&self) -> bool;
    /// Combines this source into `values`.
    fn apply(&self, values: &mut [f32; PROP_COUNT]);
    /// Marks the stages this source writes (so their layers exist).
    fn stages(&self, out: &mut [bool; 6]);
}

/// Touch movement past which a press is treated as a scroll or drag.
const TOUCH_SLOP: f32 = 18.;

pub(crate) struct MotionNode {
    revision: RefCell<Rc<Cell<u64>>>,
    weak: RefCell<Weak<MotionNode>>,
    spec: RefCell<Rc<Motion>>,
    flags: Cell<Interactions>,
    values: RefCell<[Option<MotionValue>; PROP_COUNT]>,
    bound: RefCell<[Option<MotionValue>; PROP_COUNT]>,
    bound_revisions: RefCell<[u64; PROP_COUNT]>,
    origin: Cell<Alignment>,
    tint_color: Cell<Color>,
    shadow_color: Cell<Color>,
    reveal_center: Cell<Alignment>,
    affine: AffineController,
    opacity: OpacityController,
    blur: BlurController,
    color: ColorFilterController,
    shadow: DropShadowController,
    /// The reveal geometry the current widget was built with.
    built_clip: Cell<[f32; 5]>,
    /// The reveal geometry the values currently describe.
    clip: Cell<[f32; 5]>,
    mounted: Cell<bool>,
    /// The hot-reload reassembly this node last built in.
    reassembly: Cell<u64>,
    reduced: Cell<bool>,
    default_transition: RefCell<Transition>,
    groups: RefCell<GroupScopes>,
    own_group: RefCell<Option<Rc<GroupState>>>,
    subscriptions: RefCell<Vec<Subscription>>,
    highlight: RefCell<Option<FocusHighlightSubscription>>,
    pub(crate) extras: RefCell<Vec<Rc<dyn ChannelSource>>>,
    delegate: RefCell<Option<FrameDelegate>>,
    press_origin: Cell<Option<(Offset, bool)>>,
    key_pressed: Cell<bool>,
    focus_inside: Cell<bool>,
    /// Laid-out size, for drag bounds and reveal clips.
    pub(crate) size: Cell<Size>,
    pub(crate) child_scope: RefCell<Option<super::presence::StaggerScope>>,
    pub(crate) drag_channel: RefCell<Option<Rc<super::dragging::DragChannel>>>,
    pub(crate) geometry: Cell<Option<crate::tree::FrameGeometry>>,
    pub(crate) geometry_listeners: RefCell<Vec<Rc<dyn super::geometry::GeometryListener>>>,
    /// Overrides the drag axis (reorder axis lock).
    pub(crate) drag_axis: Cell<Option<crate::dx::drag::DragAxis>>,
    /// Touch drags start with a long press (reorder items).
    pub(crate) long_press_touch: Cell<bool>,
    pub(crate) timeline: RefCell<Option<Rc<super::timeline::Timeline>>>,
    /// Playing the exit pose as a departing element.
    exiting: Cell<bool>,
    exit_done: RefCell<Option<Rc<dyn Fn()>>>,
    /// The presence parent this site was swapped in by, if any.
    presence_scope: RefCell<super::presence::PresenceScope>,
    /// As a presence parent: the keyed child being shown.
    presence: RefCell<super::presence::PresenceState>,
    layout_channel: RefCell<Option<Rc<super::layout::LayoutChannel>>>,
    shared_channel: RefCell<Option<Rc<super::shared::SharedChannel>>>,
    /// The namespace this site publishes with `shared_scope()`.
    own_shared_scope: Cell<Option<u64>>,
    size_channel: RefCell<Option<Rc<super::layout::SizeChannel>>>,
    behaviors: RefCell<Vec<Rc<dyn super::behavior::SiteBehavior>>>,
    /// Per-node state behaviors keep across builds, by type.
    behavior_state: RefCell<Vec<Rc<dyn std::any::Any>>>,
}

impl RetainedState for MotionNode {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

struct NodeTicker(Weak<MotionNode>);

impl FrameTick for NodeTicker {
    fn tick(&self, now: Instant) -> bool {
        self.0.upgrade().is_some_and(|node| node.tick(now))
    }
    fn is_active(&self) -> bool {
        self.0.upgrade().is_some_and(|node| node.is_active())
    }
    fn wants_geometry(&self) -> bool {
        self.0
            .upgrade()
            .is_some_and(|node| super::geometry::wants(&node))
    }
    fn geometry(&self, geometry: crate::tree::FrameGeometry) {
        if let Some(node) = self.0.upgrade() {
            super::geometry::report(&node, geometry);
        }
    }
    fn begin_exit(&self, done: Rc<dyn Fn()>) -> bool {
        self.0.upgrade().is_some_and(|node| node.begin_exit(done))
    }
    fn cancel_exit(&self) {
        if let Some(node) = self.0.upgrade() {
            node.cancel_exit();
        }
    }
    fn debug_properties(&self) -> Vec<(&'static str, String)> {
        self.0
            .upgrade()
            .map(|node| node.debug_properties())
            .unwrap_or_default()
    }
}

/// Clips to the node's current reveal geometry.
struct RevealClipper {
    clip: [f32; 5],
    center: Alignment,
}

impl CustomClipper for RevealClipper {
    fn get_clip(&self, size: Size) -> ClipShape {
        let [top, right, bottom, left, circle] = self.clip;
        let rect = ltrb(
            left * size.width,
            top * size.height,
            size.width - right * size.width,
            size.height - bottom * size.height,
        );
        if circle < 0. {
            return ClipShape::Rect(rect);
        }
        let center = Offset::new(
            (self.center.x + 1.) * 0.5 * size.width,
            (self.center.y + 1.) * 0.5 * size.height,
        );
        let far_x = center.x.max(size.width - center.x);
        let far_y = center.y.max(size.height - center.y);
        let radius = (far_x * far_x + far_y * far_y).sqrt() * circle.max(0.);
        let circle = ltrb(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        );
        ClipShape::RRect(RRect::new(circle, CornerRadii::uniform(radius)))
    }

    fn should_reclip(&self, old: &Self) -> bool {
        self.clip != old.clip || self.center != old.center
    }
}

fn ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new((right - left).max(0.), (bottom - top).max(0.)),
    )
}

fn color_matrix(
    brightness: f32,
    saturate: f32,
    grayscale: f32,
    tint: f32,
    tint_color: Color,
) -> ColorFilter {
    let mut filter = ColorFilter::identity();
    if brightness != 1. {
        filter = filter.then(ColorFilter::brightness(brightness));
    }
    if saturate != 1. {
        filter = filter.then(ColorFilter::saturate(saturate));
    }
    if grayscale != 0. {
        filter = filter.then(ColorFilter::grayscale(grayscale.clamp(0., 1.)));
    }
    if tint != 0. {
        let tinted = filter.then(ColorFilter::tint(tint_color)).to_matrix();
        let base = filter.to_matrix();
        let amount = tint.clamp(0., 1.);
        let mut mixed = [0.; 20];
        for (index, value) in mixed.iter_mut().enumerate() {
            *value = base[index] + (tinted[index] - base[index]) * amount;
        }
        filter = ColorFilter::matrix(mixed);
    }
    filter
}

/// Flutter's `convertRadiusToSigma`.
fn radius_to_sigma(radius: f32) -> f32 {
    if radius > 0. {
        radius * 0.577_35 + 0.5
    } else {
        0.
    }
}

impl MotionNode {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            revision: RefCell::new(revision),
            weak: RefCell::new(Weak::new()),
            spec: RefCell::new(Rc::new(Motion::new())),
            flags: Cell::new(Interactions::NONE),
            values: RefCell::new(std::array::from_fn(|_| None)),
            bound: RefCell::new(std::array::from_fn(|_| None)),
            bound_revisions: RefCell::new([0; PROP_COUNT]),
            origin: Cell::new(Alignment::CENTER),
            tint_color: Cell::new(Color::TRANSPARENT),
            shadow_color: Cell::new(Color::TRANSPARENT),
            reveal_center: Cell::new(Alignment::CENTER),
            affine: AffineController::new(),
            opacity: OpacityController::new(),
            blur: BlurController::new(0.),
            color: ColorFilterController::new(ColorFilter::identity()),
            shadow: DropShadowController::new(Offset::ZERO, 0., Color::TRANSPARENT),
            built_clip: Cell::new([0., 0., 0., 0., -1.]),
            clip: Cell::new([0., 0., 0., 0., -1.]),
            mounted: Cell::new(false),
            reassembly: Cell::new(super::reassembly()),
            reduced: Cell::new(false),
            default_transition: RefCell::new(Transition::default()),
            groups: RefCell::new(GroupScopes::default()),
            own_group: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            highlight: RefCell::new(None),
            extras: RefCell::new(Vec::new()),
            delegate: RefCell::new(None),
            press_origin: Cell::new(None),
            key_pressed: Cell::new(false),
            focus_inside: Cell::new(false),
            size: Cell::new(Size::ZERO),
            child_scope: RefCell::new(None),
            drag_channel: RefCell::new(None),
            geometry: Cell::new(None),
            geometry_listeners: RefCell::new(Vec::new()),
            drag_axis: Cell::new(None),
            long_press_touch: Cell::new(false),
            timeline: RefCell::new(None),
            exiting: Cell::new(false),
            exit_done: RefCell::new(None),
            presence_scope: RefCell::new(super::presence::PresenceScope::none()),
            presence: RefCell::new(super::presence::PresenceState::default()),
            layout_channel: RefCell::new(None),
            shared_channel: RefCell::new(None),
            own_shared_scope: Cell::new(None),
            size_channel: RefCell::new(None),
            behaviors: RefCell::new(Vec::new()),
            behavior_state: RefCell::new(Vec::new()),
        }
    }

    /// Starts the exit pose as a departing element. `false` when the site has
    /// no exit, so the tree removes it at once.
    pub(crate) fn begin_exit(&self, done: Rc<dyn Fn()>) -> bool {
        if self.spec.borrow().exit.is_none() {
            return false;
        }
        if !self.exiting.replace(true) {
            self.presence_scope.borrow().departing();
        }
        *self.exit_done.borrow_mut() = Some(done);
        self.resolve(None);
        self.request_rebuild();
        if !self.is_active() {
            self.finish_exit();
        }
        true
    }

    /// Re-inserted mid-exit: return to the resolved pose.
    pub(crate) fn cancel_exit(&self) {
        if !self.exiting.replace(false) {
            return;
        }
        self.exit_done.borrow_mut().take();
        self.presence_scope.borrow().departed();
        self.resolve(None);
        self.request_rebuild();
    }

    fn finish_exit(&self) {
        let done = self.exit_done.borrow_mut().take();
        if let Some(done) = done {
            self.presence_scope.borrow().departed();
            done();
        }
    }

    /// The behaviors of the current build.
    pub(crate) fn behaviors(&self) -> Vec<Rc<dyn super::behavior::SiteBehavior>> {
        self.behaviors.borrow().clone()
    }

    /// The `T` state this node keeps for a behavior, created by `init`.
    pub(crate) fn behavior_state<T: 'static>(&self, init: impl FnOnce() -> T) -> Rc<T> {
        let mut states = self.behavior_state.borrow_mut();
        for state in states.iter() {
            if let Ok(state) = state.clone().downcast::<T>() {
                return state;
            }
        }
        let state = Rc::new(init());
        states.push(state.clone());
        state
    }

    /// Offsets behaviors hand over to a layout change (FLIP absorption).
    pub(crate) fn absorb_offsets(&self, moved: Offset) -> Offset {
        let behaviors = self.behaviors.borrow().clone();
        behaviors.iter().fold(Offset::ZERO, |sum, behavior| {
            sum + behavior.take_offset(self, moved)
        })
    }

    /// The transition layout changes move with.
    pub(crate) fn layout_transition(&self) -> Transition {
        self.transition_for(Prop::X, None)
    }

    /// Rebuilds this node (and its wrappers) on the next frame.
    pub(crate) fn request_rebuild(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
        schedule_frame();
    }

    /// The group scopes the node sits in.
    pub(crate) fn groups(&self) -> GroupScopes {
        self.groups.borrow().clone()
    }

    /// The current interaction states.
    pub(crate) fn interactions(&self) -> Interactions {
        self.flags.get()
    }

    /// Sets one interaction state and re-resolves when it changed.
    pub(crate) fn set_flag(&self, state: Interaction, on: bool) {
        let before = self.flags.get();
        let after = before.with(state, on);
        if before == after {
            return;
        }
        self.flags.set(after);
        self.publish_group(before, after);
        self.resolve(None);
    }

    /// Mirrors this node's interaction state into its own group and keeps
    /// the hovered-descendant counts of enclosing groups.
    fn publish_group(&self, before: Interactions, after: Interactions) {
        let Some(group) = self.own_group.borrow().clone() else {
            return;
        };
        group.hovered.set(after.contains(Interaction::Hovered));
        group.pressed.set(after.contains(Interaction::Pressed));
        group.focus_within.set(after.contains(Interaction::Focused));
        group
            .focus_visible
            .set(after.contains(Interaction::FocusVisible));
        let was = before.contains(Interaction::Hovered);
        let is = after.contains(Interaction::Hovered);
        if was != is {
            for outer in self.groups.borrow().0.iter() {
                let count = outer.hovered_descendants.get();
                outer.hovered_descendants.set(if is {
                    count + 1
                } else {
                    count.saturating_sub(1)
                });
                outer.changes.notify();
            }
        }
        group.changes.notify();
    }

    fn value_for(&self, prop: Prop) -> MotionValue {
        let mut values = self.values.borrow_mut();
        values[prop.index()]
            .get_or_insert_with(|| MotionValue::new(prop.identity()))
            .clone()
    }

    /// The transition a `prop` moves with, given the pose's own transition.
    fn transition_for(&self, prop: Prop, pose: Option<&Transition>) -> Transition {
        let spec = self.spec.borrow();
        // A `with_transition` update overrides every local transition.
        let base = incular_animation::transaction().unwrap_or_else(|| {
            pose.or(spec.transition.as_ref())
                .cloned()
                .unwrap_or_else(|| self.default_transition.borrow().clone())
        });
        if self.reduced.get() && spec.reduced == super::Reduced::Reduce {
            reduce(&base, prop.channel())
        } else {
            base.for_channel(prop.channel())
        }
    }

    /// The pose every active condition produces over the rest pose, with the
    /// transition of the last active pose setting each property.
    fn resolved(&self) -> (Pose, [Option<Transition>; PROP_COUNT]) {
        let spec = self.spec.borrow().clone();
        let own = self.flags.get();
        let groups = self.groups.borrow().clone();
        let mut pose = spec.rest.clone();
        let mut transitions: [Option<Transition>; PROP_COUNT] =
            std::array::from_fn(|_| spec.rest.transition.clone());
        for (cond, state) in &spec.states {
            if cond.eval(own, &groups) {
                for prop in state.props() {
                    transitions[prop.index()].clone_from(&state.transition);
                }
                pose = pose.merge(state);
            }
        }
        if self.exiting.get()
            && let Some(exit) = &spec.exit
        {
            // The exit is the destination: its transition moves every prop.
            for prop in exit.props() {
                transitions[prop.index()].clone_from(&exit.transition);
            }
            pose = pose.merge(exit);
        }
        (pose, transitions)
    }

    /// Retargets every property to the active poses. `forced` replaces the
    /// per-pose transitions (used for the enter animation).
    pub(crate) fn resolve(&self, forced: Option<&Transition>) {
        let (pose, transitions) = self.resolved();
        self.origin.set(pose.origin.unwrap_or(Alignment::CENTER));
        if let Some(color) = pose.tint_color {
            self.tint_color.set(color);
        }
        if let Some(color) = pose.shadow_color {
            self.shadow_color.set(color);
        }
        if let Some(center) = pose.reveal_center {
            self.reveal_center.set(center);
        }
        let mut moved = false;
        for prop in Prop::ALL {
            let used = self.spec.borrow().poses().any(|p| p.sets(prop));
            if !used {
                continue;
            }
            let transition =
                self.transition_for(prop, forced.or(transitions[prop.index()].as_ref()));
            match pose.get(prop) {
                Some(PoseValue::Bound(value)) => {
                    let mut bound = self.bound.borrow_mut();
                    if bound[prop.index()].as_ref() != Some(value) {
                        bound[prop.index()] = Some(value.clone());
                        moved = true;
                    }
                }
                other => {
                    let target = other.map_or(prop.identity(), PoseValue::get);
                    let previous = self.bound.borrow_mut()[prop.index()].take();
                    let value = self.value_for(prop);
                    if let Some(previous) = previous {
                        // Continue from where the live value left off.
                        value.set(previous.get());
                    }
                    moved |= value.animate_to(target, &transition);
                }
            }
        }
        if moved {
            schedule_frame();
            if self.mounted.get() {
                self.push();
            }
        }
    }

    /// Jumps every used property to its resolved target.
    fn settle(&self) {
        let (pose, _) = self.resolved();
        for prop in Prop::ALL {
            if !self.spec.borrow().poses().any(|p| p.sets(prop)) {
                continue;
            }
            match pose.get(prop) {
                Some(PoseValue::Bound(value)) => {
                    self.bound.borrow_mut()[prop.index()] = Some(value.clone());
                }
                other => {
                    self.value_for(prop)
                        .set(other.map_or(prop.identity(), PoseValue::get));
                }
            }
        }
    }

    /// Starts the enter animation from the enter pose.
    fn enter(&self, enter: &Pose, delay: std::time::Duration) {
        self.settle();
        for prop in enter.props() {
            if let Some(value) = enter.get(prop) {
                self.value_for(prop).set(value.get());
            }
        }
        let (pose, transitions) = self.resolved();
        let enter_transition = enter.transition.clone();
        for prop in enter.props() {
            let target = pose.get(prop).map_or(prop.identity(), PoseValue::get);
            let transition = self.transition_for(
                prop,
                enter_transition
                    .as_ref()
                    .or(transitions[prop.index()].as_ref()),
            );
            let transition = if delay.is_zero() {
                transition
            } else {
                let base = transition.delay_duration();
                transition.delay(base + delay)
            };
            self.value_for(prop).animate_to(target, &transition);
        }
        schedule_frame();
    }

    /// The composed value of every property.
    pub(crate) fn composed(&self) -> [f32; PROP_COUNT] {
        let mut out: [f32; PROP_COUNT] = std::array::from_fn(|index| Prop::ALL[index].identity());
        {
            let values = self.values.borrow();
            let bound = self.bound.borrow();
            for prop in Prop::ALL {
                let index = prop.index();
                if let Some(value) = &bound[index] {
                    out[index] = value.get();
                } else if let Some(value) = &values[index] {
                    out[index] = value.get();
                }
            }
        }
        for extra in self.extras.borrow().iter() {
            extra.apply(&mut out);
        }
        out
    }

    /// Writes the composed values into the compositor controllers. A changed
    /// reveal clip rebuilds this node's wrappers (not its child).
    pub(crate) fn push(&self) -> bool {
        let v = self.composed();
        let get = |prop: Prop| v[prop.index()];
        let mut changed = self.affine.set_parts(AffineParts {
            translation: Offset::new(get(Prop::X), get(Prop::Y)),
            fraction: Offset::new(get(Prop::SlideX), get(Prop::SlideY)),
            scale_x: get(Prop::Scale) * get(Prop::ScaleX),
            scale_y: get(Prop::Scale) * get(Prop::ScaleY),
            rotation: get(Prop::Rotate) * std::f32::consts::TAU,
            skew_x: get(Prop::SkewX),
            skew_y: get(Prop::SkewY),
            origin: self.origin.get(),
        });
        changed |= self.opacity.set_opacity(get(Prop::Opacity).clamp(0., 1.));
        changed |= self.blur.set_sigma(get(Prop::Blur).max(0.));
        let filter = color_matrix(
            get(Prop::Brightness),
            get(Prop::Saturate),
            get(Prop::Grayscale),
            get(Prop::Tint),
            self.tint_color.get(),
        );
        changed |= self.color.set_filter(filter);
        let shadow_color = self.shadow_color.get();
        let amount = get(Prop::Shadow).clamp(0., 1.);
        changed |= self
            .shadow
            .set_offset(Offset::new(get(Prop::ShadowX), get(Prop::ShadowY)));
        changed |= self
            .shadow
            .set_sigma(radius_to_sigma(get(Prop::ShadowBlur).max(0.)));
        changed |= self.shadow.set_color(Color {
            alpha: (f32::from(shadow_color.alpha) * amount).round() as u8,
            ..shadow_color
        });
        let clip = [
            get(Prop::RevealTop),
            get(Prop::RevealRight),
            get(Prop::RevealBottom),
            get(Prop::RevealLeft),
            get(Prop::RevealCircle),
        ];
        self.clip.set(clip);
        if clip != self.built_clip.get() {
            let revision = self.revision.borrow().clone();
            revision.set(revision.get().wrapping_add(1));
            changed = true;
        }
        changed
    }

    fn tick(&self, now: Instant) -> bool {
        let mut changed = false;
        let values: Vec<MotionValue> = self.values.borrow().iter().flatten().cloned().collect();
        for value in values {
            changed |= value.tick(now);
        }
        {
            let bound = self.bound.borrow();
            let mut revisions = self.bound_revisions.borrow_mut();
            for (index, value) in bound.iter().enumerate() {
                if let Some(value) = value {
                    let revision = value.revision();
                    if revisions[index] != revision {
                        revisions[index] = revision;
                        changed = true;
                    }
                }
            }
        }
        let extras: Vec<_> = self.extras.borrow().clone();
        for extra in extras {
            changed |= extra.tick(now);
        }
        if changed {
            self.push();
        }
        if self.exiting.get() && !self.is_active() {
            self.finish_exit();
        }
        changed
    }

    fn is_active(&self) -> bool {
        if self
            .values
            .borrow()
            .iter()
            .flatten()
            .any(MotionValue::is_animating)
        {
            return true;
        }
        {
            let bound = self.bound.borrow();
            let revisions = self.bound_revisions.borrow();
            if bound.iter().enumerate().any(|(index, value)| {
                value
                    .as_ref()
                    .is_some_and(|v| v.is_animating() || v.revision() != revisions[index])
            }) {
                return true;
            }
        }
        self.extras.borrow().iter().any(|extra| extra.is_active())
    }

    /// The compositor stages the configuration writes.
    fn stages(&self) -> [bool; 6] {
        let mut stages = [false; 6];
        let spec = self.spec.borrow();
        for pose in spec.poses() {
            for prop in pose.props() {
                stages[prop.stage() as usize] = true;
            }
        }
        if spec.drag.is_some() || spec.layout.is_some() || spec.shared.is_some() {
            stages[Stage::Transform as usize] = true;
        }
        for extra in self.extras.borrow().iter() {
            extra.stages(&mut stages);
        }
        stages
    }

    fn update_press(&self) {
        let pressed = self.press_origin.get().is_some() || self.key_pressed.get();
        self.set_flag(Interaction::Pressed, pressed);
    }

    fn update_focus(&self) {
        let inside = self.focus_inside.get();
        self.set_flag(Interaction::Focused, inside);
        let visible =
            inside && FocusHighlightManager::new().mode() == FocusHighlightMode::Traditional;
        self.set_flag(Interaction::FocusVisible, visible);
    }

    /// Wraps `child` in the controlled layers and interaction detectors.
    fn assemble(self: &Rc<Self>, child: Widget) -> Widget {
        let raw_child = child.clone();
        let spec = self.spec.borrow().clone();
        let stages = self.stages();
        let used = spec.interactions();
        let child = match self.timeline.borrow().clone() {
            Some(timeline) => timeline.decorate(child),
            None => child,
        };
        let child = match self.size_channel.borrow().as_ref() {
            Some(channel) => super::layout::sized(channel, child),
            None => child,
        };
        let child = match spec.presence {
            Some(mode) => {
                let mut state = self.presence.borrow_mut();
                super::presence::presence_child(self, &mut state, mode, child)
            }
            // A swapped-in site keeps deeper sites out of its parent's count.
            None if self.presence_scope.borrow().is_some() => {
                Widget::environment_scope(super::presence::PresenceScope::none(), child)
            }
            None => child,
        };
        let mut widget = super::presence::scope_child(self, child);
        if spec.shared_scope {
            let id = match self.own_shared_scope.get() {
                Some(id) => id,
                None => {
                    let id = super::shared::next_scope();
                    self.own_shared_scope.set(Some(id));
                    id
                }
            };
            widget = Widget::environment_scope(super::shared::SharedScope(id), widget);
        }
        if let Some(group) = self.own_group.borrow().clone() {
            let scopes = self.groups.borrow().pushed(group);
            widget = Widget::environment_scope(scopes, widget);
        }
        if stages[Stage::Clip as usize] {
            let clip = self.clip.get();
            self.built_clip.set(clip);
            widget = ClipRRect::new(0., widget)
                .custom_clipper(RevealClipper {
                    clip,
                    center: self.reveal_center.get(),
                })
                .into();
        }
        if stages[Stage::Blur as usize] {
            widget = crate::Blur::controlled(self.blur.clone(), widget).into();
        }
        if stages[Stage::Color as usize] {
            widget = crate::ColorFiltered::controlled(self.color.clone(), widget).into();
        }
        if stages[Stage::Shadow as usize] {
            widget = crate::DropShadow::controlled(self.shadow.clone(), widget).into();
        }
        if stages[Stage::Opacity as usize] {
            widget = crate::Opacity::new(1., widget)
                .controller(self.opacity.clone())
                .into();
        }
        if stages[Stage::Transform as usize] {
            widget = Widget::controlled_affine(self.affine.clone(), widget);
        }
        widget = super::geometry::observe_size(self, widget);
        if let Some(drag) = &spec.drag {
            widget = super::dragging::attach(self, &drag.0, widget);
        }
        if used.contains(Interaction::Pressed) {
            let weak = Rc::downgrade(self);
            let down = weak.clone();
            let up = weak.clone();
            let cancel = weak.clone();
            widget = Listener::new(widget)
                .on_pointer_down(move |event| {
                    let Some(node) = down.upgrade() else { return };
                    if event.buttons & incular_core::PRIMARY_POINTER_BUTTON == 0
                        && event.kind == PointerDeviceKind::Mouse
                    {
                        return;
                    }
                    let touch = matches!(
                        event.kind,
                        PointerDeviceKind::Touch
                            | PointerDeviceKind::Stylus
                            | PointerDeviceKind::InvertedStylus
                    );
                    node.press_origin.set(Some((event.position, touch)));
                    node.update_press();
                })
                .on_pointer_move(move |event| {
                    let Some(node) = weak.upgrade() else { return };
                    if let Some((origin, true)) = node.press_origin.get() {
                        let delta = event.position - origin;
                        if (delta.x * delta.x + delta.y * delta.y).sqrt() > TOUCH_SLOP {
                            node.press_origin.set(None);
                            node.update_press();
                        }
                    }
                })
                .on_pointer_up(move |_| {
                    let Some(node) = up.upgrade() else { return };
                    node.press_origin.set(None);
                    node.update_press();
                })
                .on_pointer_cancel(move |_| {
                    let Some(node) = cancel.upgrade() else { return };
                    node.press_origin.set(None);
                    node.update_press();
                })
                .into();
        }
        if used.contains(Interaction::Hovered) {
            let enter = Rc::downgrade(self);
            let exit = Rc::downgrade(self);
            widget = MouseRegion::new(widget)
                .opaque(false)
                .on_enter(move |event| {
                    if event.kind == PointerDeviceKind::Touch {
                        return;
                    }
                    if let Some(node) = enter.upgrade() {
                        node.set_flag(Interaction::Hovered, true);
                    }
                })
                .on_exit(move |_| {
                    if let Some(node) = exit.upgrade() {
                        node.set_flag(Interaction::Hovered, false);
                    }
                })
                .into();
        }
        let wants_focus = used.contains(Interaction::Focused)
            || used.contains(Interaction::FocusVisible)
            || used.contains(Interaction::Pressed);
        if wants_focus {
            let focus = Rc::downgrade(self);
            let keys = Rc::downgrade(self);
            let callbacks = incular_gestures::GestureCallbacks {
                on_focus_within: Some(Rc::new(move |inside| {
                    if let Some(node) = focus.upgrade() {
                        node.focus_inside.set(inside);
                        node.update_focus();
                    }
                })),
                on_key_observe: Some(Rc::new(move |event| {
                    let Some(node) = keys.upgrade() else { return };
                    let activates = event.code == Code::Space
                        || event.key == KeyboardKey::Named(NamedKey::Enter);
                    if !activates {
                        return;
                    }
                    node.key_pressed.set(event.state.is_down());
                    node.update_press();
                })),
                ..incular_gestures::GestureCallbacks::default()
            };
            widget = Widget::from_kind(WidgetKind::Gesture {
                behavior: crate::gestures::HitTestBehavior::DeferToChild,
                callbacks: Box::new(callbacks),
                child: widget,
            });
            if self.highlight.borrow().is_none() {
                let weak = Rc::downgrade(self);
                *self.highlight.borrow_mut() =
                    Some(FocusHighlightManager::new().observe(move |_| {
                        if let Some(node) = weak.upgrade() {
                            node.update_focus();
                        }
                    }));
            }
        }
        for behavior in &spec.behaviors {
            widget = behavior.wrap(self, widget, &raw_child);
        }
        if spec.exit.is_some() {
            let exiting = self.exiting.get();
            if spec.exit_layout == super::ExitLayout::Pop {
                // Pop: leave the layout at once, painting from the same corner.
                let factor = if exiting { 0. } else { 1. };
                widget = crate::Align::new(Alignment::TOP_LEFT, widget)
                    .width_factor(factor)
                    .height_factor(factor)
                    .into();
            }
            // A departing element is inert at once: no hits, focus or semantics.
            let mut inert: Widget = crate::IgnorePointer::new(widget).ignoring(exiting).into();
            if exiting {
                inert = inert.exclude_semantics();
            }
            widget = inert.with_excluded_focus(exiting);
        }
        let delegate = self
            .delegate
            .borrow_mut()
            .get_or_insert_with(|| FrameDelegate::new(Rc::new(NodeTicker(Rc::downgrade(self)))))
            .clone();
        Widget::frame_ticker(delegate, widget)
    }

    fn build(
        self: &Rc<Self>,
        spec: &Rc<Motion>,
        context: &crate::BuildContext<'_>,
        child: Widget,
    ) -> Widget {
        *self.weak.borrow_mut() = Rc::downgrade(self);
        *self.spec.borrow_mut() = spec.clone();
        let config = MotionConfig::of(context);
        self.reduced.set(MotionConfig::is_reduced(context, &config));
        *self.default_transition.borrow_mut() = config.transition;
        let groups = context.find::<GroupScopes>().unwrap_or_default();
        *self.groups.borrow_mut() = groups.clone();
        {
            let mut own = self.own_group.borrow_mut();
            match spec.group {
                Some(group) if own.as_ref().is_none_or(|state| state.group != group) => {
                    *own = Some(GroupState::new(group));
                }
                None => *own = None,
                _ => {}
            }
        }
        // Live conditions and group states retarget without rebuilding.
        let mut subscriptions = Vec::new();
        let weak = Rc::downgrade(self);
        let invalidate: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(node) = weak.upgrade() {
                node.resolve(None);
            }
        });
        for (cond, _) in &spec.states {
            cond.subscribe(&groups, &invalidate, &mut subscriptions);
        }
        *self.subscriptions.borrow_mut() = subscriptions;
        *self.presence_scope.borrow_mut() = context
            .find::<super::presence::PresenceScope>()
            .unwrap_or_else(super::presence::PresenceScope::none);
        super::presence::before_build(self, spec, context, &child);
        let mounted_now = !self.mounted.replace(true);
        // A hot reload replays enter animations and mount timelines.
        let epoch = super::reassembly();
        let replay = self.reassembly.replace(epoch) != epoch && !mounted_now;
        let mounting = mounted_now || replay;
        let stagger = if mounting && (spec.enter.is_some() || spec.timeline.is_some()) {
            super::presence::stagger_delay(self, context)
        } else {
            std::time::Duration::ZERO
        };
        self.bind_timeline(spec, mounting.then_some(stagger), &groups);
        self.bind_layout(spec);
        self.bind_shared(spec, context);
        *self.behaviors.borrow_mut() = spec.behaviors.clone();
        for behavior in &spec.behaviors {
            behavior.bind(self, context);
        }
        if mounting {
            match &spec.enter {
                Some(enter) if !self.reduced.get() || spec.reduced == super::Reduced::Keep => {
                    self.enter(enter, stagger);
                }
                _ => self.settle(),
            }
        } else {
            self.resolve(None);
        }
        self.push();
        self.assemble(child)
    }

    /// Installs the layout channels the first time `layout()` is declared.
    fn bind_layout(self: &Rc<Self>, spec: &Motion) {
        let Some(mode) = spec.layout else {
            return;
        };
        if self.layout_channel.borrow().is_none() {
            let channel = super::layout::LayoutChannel::new(Rc::downgrade(self));
            self.add_extra(channel.clone());
            self.geometry_listeners.borrow_mut().push(channel.clone());
            *self.layout_channel.borrow_mut() = Some(channel);
        }
        if super::layout::animates_size(mode) && self.size_channel.borrow().is_none() {
            let channel = super::layout::SizeChannel::new(Rc::downgrade(self));
            self.add_extra(channel.clone());
            *self.size_channel.borrow_mut() = Some(channel);
        }
    }

    /// Installs or updates the `shared(id)` channel.
    fn bind_shared(self: &Rc<Self>, spec: &Motion, context: &crate::BuildContext<'_>) {
        let Some((id, mode)) = spec.shared.clone() else {
            return;
        };
        let scope = context
            .find::<super::shared::SharedScope>()
            .map_or(0, |scope| scope.0);
        let channel = self.shared_channel.borrow().clone();
        let channel = channel.unwrap_or_else(|| {
            let channel = super::shared::SharedChannel::new(Rc::downgrade(self));
            self.add_extra(channel.clone());
            self.geometry_listeners
                .borrow_mut()
                .push(Rc::new(super::shared::SharedListener(channel.clone())));
            *self.shared_channel.borrow_mut() = Some(channel.clone());
            channel
        });
        channel.bind(scope, id, mode);
    }

    /// The transform origin poses scale and rotate around.
    pub(crate) fn transform_origin(&self) -> Alignment {
        self.origin.get()
    }

    /// Installs or updates the `.animate()` timeline channel.
    fn bind_timeline(
        self: &Rc<Self>,
        spec: &Motion,
        mount_delay: Option<std::time::Duration>,
        groups: &GroupScopes,
    ) {
        let Some(timeline_spec) = spec.timeline.clone() else {
            return;
        };
        let existing = self.timeline.borrow().clone();
        let timeline = existing.unwrap_or_else(|| {
            let timeline = super::timeline::Timeline::new(Rc::downgrade(self));
            self.add_extra(timeline.clone());
            *self.timeline.borrow_mut() = Some(timeline.clone());
            timeline
        });
        timeline.bind(timeline_spec.clone(), mount_delay, |cond, wake, out| {
            cond.subscribe(groups, wake, out);
        });
        if timeline_spec.uses_geometry() {
            let listening = self
                .geometry_listeners
                .borrow()
                .iter()
                .any(|listener| std::ptr::addr_eq(Rc::as_ptr(listener), Rc::as_ptr(&timeline)));
            if !listening {
                self.geometry_listeners.borrow_mut().push(timeline.clone());
            }
        }
        let (tint, shadow) = timeline.colors();
        if let Some(tint) = tint {
            self.tint_color.set(tint);
        }
        if let Some(shadow) = shadow {
            self.shadow_color.set(shadow);
        }
    }

    /// The interaction states the configuration reads.
    pub(crate) fn interactions_used(&self) -> Interactions {
        self.spec.borrow().interactions()
    }

    pub(crate) fn spec(&self) -> Rc<Motion> {
        self.spec.borrow().clone()
    }

    pub(crate) fn is_reduced(&self) -> bool {
        self.reduced.get() && self.spec.borrow().reduced == super::Reduced::Reduce
    }

    pub(crate) fn add_extra(&self, extra: Rc<dyn ChannelSource>) {
        self.extras.borrow_mut().push(extra);
    }
}

/// The `.motion()` widget.
pub(crate) fn motion_widget(child: Widget, spec: Motion) -> Widget {
    // A presence parent swaps keyed children, so it must keep its own identity.
    let key = if spec.presence.is_some() {
        None
    } else {
        child.key().cloned()
    };
    super::lint::report(&super::lint::lints(&spec, key.is_some()));
    let spec = Rc::new(spec);
    let outer = spec.clone();
    let mut widget = super::presence::wrap(
        &outer,
        Widget::retained_builder(
            MotionNode::new,
            move |node, context, _| node.build(&spec, context, child.clone()),
            false,
        ),
    );
    // The parent diffs the motion wrapper, so it carries the child's key.
    if let Some(key) = key {
        widget.set_key(Some(key));
    }
    widget
}

impl MotionNode {
    /// What DevTools shows for a motion site: its cost tier, active states
    /// and the values that differ from rest or are animating.
    pub(crate) fn debug_properties(&self) -> Vec<(&'static str, String)> {
        let stages = self.stages();
        let tier = if self.size_channel.borrow().is_some() {
            "layout (size relayouts this element)"
        } else if self
            .timeline
            .borrow()
            .as_ref()
            .is_some_and(|timeline| timeline.rebuilds())
        {
            "build (rebuilds each frame)"
        } else if stages[Stage::Clip as usize] {
            "paint (reveal clip rebuilds the wrappers)"
        } else {
            "composite"
        };
        let layers: Vec<&str> = [
            (Stage::Transform, "transform"),
            (Stage::Opacity, "opacity"),
            (Stage::Shadow, "shadow"),
            (Stage::Color, "color"),
            (Stage::Blur, "blur"),
            (Stage::Clip, "clip"),
        ]
        .into_iter()
        .filter(|(stage, _)| stages[*stage as usize])
        .map(|(_, name)| name)
        .collect();
        let flags = self.flags.get();
        let states: Vec<&str> = [
            Interaction::Hovered,
            Interaction::Pressed,
            Interaction::Focused,
            Interaction::FocusVisible,
            Interaction::Dragging,
            Interaction::InView,
            Interaction::DropOver,
            Interaction::DropActive,
        ]
        .into_iter()
        .filter(|state| flags.contains(*state))
        .map(super::lint::name)
        .collect();
        let composed = self.composed();
        let animating = self.is_active();
        let values: Vec<String> = Prop::ALL
            .iter()
            .filter(|prop| (composed[prop.index()] - prop.identity()).abs() > 1e-4)
            .map(|prop| format!("{prop:?}={:.3}", composed[prop.index()]))
            .collect();
        vec![
            ("motion.tier", tier.to_owned()),
            ("motion.layers", layers.join(", ")),
            ("motion.states", states.join(", ")),
            ("motion.values", values.join(", ")),
            ("motion.animating", animating.to_string()),
        ]
    }
}
