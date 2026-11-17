//! `.motion()`: interaction poses, conditions, groups and enters animate on
//! the compositor without rebuilding, relaying out or repainting content.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, Transform as CoreTransform};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{
    Cond, Group, MotionExt, Pose, Spring, Stagger, Transition, hovered, ms,
};
use incular_widgets::{Column, Widget};

fn view() -> Size {
    Size::new(200., 200.)
}

fn pointer(position: Offset, phase: PointerPhase, buttons: u32, at: Instant) -> RawPointerEvent {
    RawPointerEvent::new(0, 1, PointerDeviceKind::Mouse, buttons, position, phase, at)
}

fn square() -> Widget {
    Widget::box_(Size::new(100., 100.), Color::WHITE)
}

struct Harness {
    tree: WidgetTree,
    now: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            now: Instant::now(),
        };
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::loose(view()))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        let _ = self.tree.paint();
    }

    fn run(&mut self, duration: Duration) {
        let end = self.now + duration;
        while self.now < end {
            self.now += ms(16);
            self.frame();
        }
    }

    fn hover(&mut self, position: Offset) {
        let event = pointer(position, PointerPhase::Move, 0, self.now);
        let _ = self.tree.dispatch_raw_pointer_in_window(7, event);
    }

    fn press(&mut self, position: Offset, down: bool) {
        let (phase, buttons) = if down {
            (PointerPhase::Down, 1)
        } else {
            (PointerPhase::Up, 0)
        };
        let event = pointer(position, phase, buttons, self.now);
        let _ = self.tree.dispatch_raw_pointer_in_window(7, event);
    }

    /// The `n`th compositor transform found in tree order.
    fn transform(&self, n: usize) -> CoreTransform {
        let mut found = Vec::new();
        let mut stack = vec![self.tree.root().expect("root")];
        while let Some(id) = stack.pop() {
            if let Some(transform) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.content_transform(render))
            {
                found.push(transform);
            }
            let children: Vec<ElementId> = self.tree.children(id).unwrap_or(&[]).to_vec();
            stack.extend(children.into_iter().rev());
        }
        found.get(n).copied().expect("motion transform")
    }

    fn scale(&self, n: usize) -> f32 {
        let [a, ..] = self.transform(n).coefficients();
        a as f32
    }

    fn translation(&self, n: usize) -> Offset {
        self.transform(n).translation_offset()
    }
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-3
}

#[test]
fn hover_pose_animates_on_the_compositor_only() {
    let mut h = Harness::new(square().motion(|m| {
        m.hovered(|p| p.scale(1.1))
            .transition(Transition::linear(ms(100)))
    }));
    assert!(close(h.scale(0), 1.));
    let before = h.tree.diagnostics();
    h.hover(Offset::new(50., 50.));
    h.run(ms(48));
    let mid = h.scale(0);
    assert!(mid > 1. && mid < 1.1, "mid-flight scale {mid}");
    h.run(ms(100));
    assert!(close(h.scale(0), 1.1));
    let after = h.tree.diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds, "hover never rebuilds");
    assert_eq!(after.layouts, before.layouts, "hover never relayouts");
    assert_eq!(after.paints, before.paints, "hover never repaints");
    assert!(after.compositor_only_updates > before.compositor_only_updates);

    // Leaving returns to rest.
    h.hover(Offset::new(190., 190.));
    h.run(ms(200));
    assert!(close(h.scale(0), 1.));
}

#[test]
fn later_declarations_win_per_property() {
    let mut h = Harness::new(square().motion(|m| {
        m.hovered(|p| p.scale(1.1).y(-4.))
            .pressed(|p| p.scale(0.9))
            .transition(Transition::instant())
    }));
    h.hover(Offset::new(50., 50.));
    h.run(ms(16));
    assert!(close(h.scale(0), 1.1));
    h.press(Offset::new(50., 50.), true);
    h.run(ms(16));
    // Pressed (declared later) wins scale; only hover sets y.
    assert!(close(h.scale(0), 0.9));
    let t = h.transform(0);
    let center = t.transform_point(Offset::new(50., 50.));
    assert!(close(center.y, 46.), "y offset kept from hover: {center:?}");
    h.press(Offset::new(50., 50.), false);
    h.run(ms(16));
    assert!(close(h.scale(0), 1.1));
}

#[test]
fn rest_pose_applies_without_animating_on_mount() {
    let h = Harness::new(square().motion(|m| m.rest(|p| p.x(12.))));
    assert!(close(h.translation(0).x, 12.));
}

#[test]
fn enter_plays_from_the_enter_pose_once() {
    let mut h = Harness::new(square().motion(|m| {
        m.enter(|p| p.x(-40.))
            .transition(Transition::linear(ms(100)))
    }));
    assert!(close(h.translation(0).x, -40.));
    h.run(ms(50));
    let x = h.translation(0).x;
    assert!(x > -40. && x < 0., "entering: {x}");
    h.run(ms(100));
    assert!(close(h.translation(0).x, 0.));
}

#[test]
fn pose_transition_is_used_when_entering_it() {
    let mut h = Harness::new(square().motion(|m| {
        m.hovered(|p| p.scale(2.).transition(Transition::instant()))
            .transition(Transition::linear(ms(1000)))
    }));
    h.hover(Offset::new(50., 50.));
    h.run(ms(16));
    assert!(close(h.scale(0), 2.), "entering the pose is instant");
    h.hover(Offset::new(190., 190.));
    h.run(ms(100));
    let s = h.scale(0);
    assert!(
        s < 2. && s > 1.5,
        "leaving uses the destination's slow default: {s}"
    );
}

#[test]
fn bool_conditions_select_poses() {
    let h = Harness::new(square().motion(|m| m.when(true, |p| p.x(5.)).when(false, |p| p.x(99.))));
    assert!(close(h.translation(0).x, 5.));
}

#[test]
fn condition_algebra_combines_states() {
    let cond: Cond = hovered() & !Cond::from(false);
    let mut h = Harness::new(
        square().motion(move |m| m.when(cond, |p| p.x(7.)).transition(Transition::instant())),
    );
    assert!(close(h.translation(0).x, 0.));
    h.hover(Offset::new(50., 50.));
    h.run(ms(16));
    assert!(close(h.translation(0).x, 7.));
}

const CARD: Group = Group::new("card");

#[test]
fn group_hover_reaches_descendants_without_rebuilds() {
    let inner = Widget::box_(Size::new(20., 20.), Color::BLACK).motion(|m| {
        m.rest(|p| p.x(0.))
            .when(CARD.hovered(), |p| p.x(30.))
            .transition(Transition::instant())
    });
    let card = Column::new([inner, square()]).motion(|m| m.group(CARD));
    let mut h = Harness::new(card);
    let before = h.tree.diagnostics();
    assert!(close(h.translation(0).x, 0.));
    h.hover(Offset::new(50., 60.));
    h.run(ms(16));
    assert!(close(h.translation(0).x, 30.));
    assert_eq!(h.tree.diagnostics().rebuilds, before.rebuilds);
}

#[test]
fn nested_same_name_groups_resolve_to_the_nearest() {
    let probe = Widget::box_(Size::new(10., 10.), Color::BLACK).motion(|m| {
        m.when(CARD.hovered(), |p| p.x(30.))
            .transition(Transition::instant())
    });
    // The inner card holds the probe, centered at x 45..55, y 0..10.
    let inner = Column::new([probe]).motion(|m| m.group(CARD));
    let outer = Column::new([inner, square()]).motion(|m| m.group(CARD));
    let mut h = Harness::new(outer);
    // Hover the outer card's square, outside the inner card.
    h.hover(Offset::new(50., 60.));
    h.run(ms(16));
    assert!(
        close(h.translation(0).x, 0.),
        "the inner (nearest) card is not hovered"
    );
    h.hover(Offset::new(50., 5.));
    h.run(ms(16));
    assert!(close(h.translation(0).x, 30.));
}

#[test]
fn stagger_offsets_children_enters() {
    let child = || {
        Widget::box_(Size::new(10., 10.), Color::BLACK).motion(|m| {
            m.enter(|p| p.x(-20.))
                .transition(Transition::linear(ms(100)))
        })
    };
    let mut h = Harness::new(
        Column::new([child(), child(), child()]).motion(|m| m.stagger(Stagger::every(ms(50)))),
    );
    h.run(ms(64));
    let first = h.translation(0).x;
    let third = h.translation(2).x;
    assert!(
        first > third,
        "the first child is further along: {first} vs {third}"
    );
    h.run(ms(400));
    for n in 0..3 {
        assert!(close(h.translation(n).x, 0.));
    }
}

#[test]
fn spring_retargets_keep_moving_smoothly() {
    let mut h =
        Harness::new(square().motion(|m| m.hovered(|p| p.x(100.)).transition(Spring::snappy())));
    h.hover(Offset::new(50., 50.));
    h.run(ms(64));
    let mid = h.translation(0).x;
    h.hover(Offset::new(190., 190.));
    h.run(ms(16));
    let after = h.translation(0).x;
    // Momentum carries it on briefly instead of jumping back.
    assert!(after >= mid - 1., "no jump on retarget: {mid} -> {after}");
    h.run(ms(1000));
    assert!(close(h.translation(0).x, 0.));
}

#[test]
fn presets_are_plain_functions() {
    fn nudge(p: Pose) -> Pose {
        p.x(3.)
    }
    let h = Harness::new(square().motion(|m| m.rest(nudge)));
    assert!(close(h.translation(0).x, 3.));
}

// ---- Compositor stages, live values, keyboard, focus, drag, reduced motion.

fn layers(tree: &WidgetTree) -> Vec<incular_rendering::LayerKind> {
    let layers = tree.compositor_layers();
    let mut out = Vec::new();
    let mut stack: Vec<_> = layers.root().into_iter().collect();
    while let Some(id) = stack.pop() {
        if let Some((kind, children)) = layers.layer(id) {
            out.push(kind.clone());
            stack.extend(children.iter().copied());
        }
    }
    out
}

#[test]
fn filter_poses_reach_compositor_layers() {
    use incular_rendering::LayerKind;
    let mut h = Harness::new(square().motion(|m| {
        m.hovered(|p| p.opacity(0.5).blur(4.).brightness(1.2).lift(12.))
            .transition(Transition::instant())
    }));
    let before = h.tree.diagnostics();
    h.hover(Offset::new(50., 50.));
    h.run(ms(16));
    let kinds = layers(&h.tree);
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::Opacity { alpha } if (*alpha - 0.5).abs() < 1e-3)),
        "opacity layer at 0.5"
    );
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::Blur { blur } if (blur.sigma_x - 4.).abs() < 1e-3)),
        "blur layer at sigma 4"
    );
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::ColorFilter { filter } if !filter.is_identity())),
        "color filter applied"
    );
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::DropShadow { shadow } if shadow.color.alpha > 0)),
        "shadow visible"
    );
    let after = h.tree.diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(after.layouts, before.layouts);
    assert_eq!(after.paints, before.paints);
}

#[test]
fn reveal_clip_rebuilds_only_the_wrapper() {
    let mut h = Harness::new(square().motion(|m| {
        m.rest(|p| p.reveal_inset(0., 0.5, 0., 0.))
            .hovered(|p| p.reveal_inset(0., 0., 0., 0.))
            .transition(Transition::linear(ms(100)))
    }));
    h.hover(Offset::new(20., 50.));
    let before = h.tree.diagnostics();
    h.run(ms(200));
    let after = h.tree.diagnostics();
    assert!(
        after.rebuilds > before.rebuilds,
        "the clip wrapper rebuilds"
    );
    // Reveal is paint-tier: the wrapper builder relays out once per frame;
    // the content reuses its measurement.
    let frames = 200 / 16 + 1;
    assert!(
        after.layouts - before.layouts <= frames,
        "at most one wrapper layout per frame: {}",
        after.layouts - before.layouts
    );
}

#[test]
fn bound_motion_values_link_without_rebuilds() {
    let x = incular_widgets::motion::MotionValue::new(0.);
    let half = x.map(|v| v / 2.);
    let mut h = Harness::new(square().motion(move |m| m.rest(|p| p.x(&half))));
    let before = h.tree.diagnostics();
    x.set(40.);
    h.run(ms(16));
    assert!(close(h.translation(0).x, 20.));
    assert_eq!(h.tree.diagnostics().rebuilds, before.rebuilds);
}

#[test]
fn live_conditions_retarget_without_rebuilds() {
    use std::{cell::Cell, rc::Rc};
    let source = incular_core::reactivity::DependencySource::default();
    let flag = Rc::new(Cell::new(false));
    let read = flag.clone();
    let cond = Cond::live(source.clone(), move || read.get());
    let mut h = Harness::new(
        square().motion(move |m| m.when(cond, |p| p.x(9.)).transition(Transition::instant())),
    );
    let before = h.tree.diagnostics();
    flag.set(true);
    source.notify();
    h.run(ms(16));
    assert!(close(h.translation(0).x, 9.));
    flag.set(false);
    source.notify();
    h.run(ms(16));
    assert!(close(h.translation(0).x, 0.));
    assert_eq!(h.tree.diagnostics().rebuilds, before.rebuilds);
}

fn focusable() -> Widget {
    incular_widgets::FocusableActionDetector::new(square()).into()
}

fn focused_element(tree: &WidgetTree) -> ElementId {
    tree.focusable_elements()[0]
}

#[test]
fn focus_inside_sets_focused_and_focus_visible() {
    use incular_gestures::{FocusHighlightManager, FocusHighlightMode};
    FocusHighlightManager::new().set_mode(FocusHighlightMode::Traditional);
    let mut h = Harness::new(focusable().motion(|m| {
        m.focused(|p| p.x(3.))
            .focus_visible(|p| p.y(4.))
            .transition(Transition::instant())
    }));
    let target = focused_element(&h.tree);
    h.tree.set_keyboard_focus(target, true);
    h.run(ms(16));
    assert!(close(h.translation(0).x, 3.));
    assert!(close(h.translation(0).y, 4.));
    FocusHighlightManager::new().set_mode(FocusHighlightMode::Touch);
    h.run(ms(16));
    assert!(
        close(h.translation(0).y, 0.),
        "touch mode hides focus highlights"
    );
    h.tree.set_keyboard_focus(target, false);
    h.run(ms(16));
    assert!(close(h.translation(0).x, 0.));
}

#[test]
fn enter_and_space_press_while_focused() {
    use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey};
    let mut h = Harness::new(focusable().motion(|m| {
        m.pressed(|p| p.scale(0.9))
            .transition(Transition::instant())
    }));
    let target = focused_element(&h.tree);
    h.tree.set_keyboard_focus(target, true);
    let down = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Enter), Code::Enter);
    let _ = h.tree.dispatch_keyboard(Some(target), down);
    h.run(ms(16));
    assert!(close(h.scale(0), 0.9));
    let up = KeyboardEvent::key_up(KeyboardKey::Named(NamedKey::Enter), Code::Enter);
    let _ = h.tree.dispatch_keyboard(Some(target), up);
    h.run(ms(16));
    assert!(close(h.scale(0), 1.));
}

#[test]
fn free_drag_follows_the_pointer_and_snaps_back() {
    use incular_widgets::motion::Drag;
    let mut h = Harness::new(square().motion(|m| {
        m.drag(Drag::x())
            .dragging(|p| p.scale(1.1))
            .transition(Transition::instant())
    }));
    let gesture = |h: &mut Harness, x: f32, phase: PointerPhase| {
        h.tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(x, 50.),
            phase,
            time: h.now,
        });
    };
    h.press(Offset::new(50., 50.), true);
    gesture(&mut h, 50., PointerPhase::Down);
    for x in [55., 60., 70., 80., 90.] {
        h.now += ms(16);
        let event = pointer(Offset::new(x, 50.), PointerPhase::Move, 1, h.now);
        let _ = h.tree.dispatch_raw_pointer_in_window(7, event);
        gesture(&mut h, x, PointerPhase::Move);
        h.frame();
    }
    let dragged = h.translation(0);
    assert!(dragged.x > 10., "follows the pointer: {dragged:?}");
    assert!(dragged.y.abs() < 6., "x axis only");
    assert!(h.scale(0) > 1.05, "dragging pose");
    h.press(Offset::new(90., 50.), false);
    gesture(&mut h, 90., PointerPhase::Up);
    h.run(ms(1500));
    assert!(h.translation(0).x.abs() < 0.5, "snaps back");
}

#[test]
fn reduced_motion_snaps_geometry_and_shortens_fades() {
    use incular_widgets::motion::{MotionConfig, ReducedMotion};
    let site = square().motion(|m| {
        m.hovered(|p| p.x(50.).opacity(0.))
            .transition(Transition::linear(ms(1000)))
    });
    let mut h = Harness::new(
        MotionConfig::new(site)
            .reduced_motion(ReducedMotion::Always)
            .into(),
    );
    h.hover(Offset::new(50., 50.));
    h.run(ms(16));
    assert!(close(h.translation(0).x, 50.), "geometry snaps");
    h.run(ms(200));
    let kinds = layers(&h.tree);
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, incular_rendering::LayerKind::Opacity { alpha } if *alpha < 0.01)),
        "fade finished within 150 ms"
    );
}

#[test]
fn reduced_keep_still_animates() {
    use incular_widgets::motion::{MotionConfig, Reduced, ReducedMotion};
    let site = square().motion(|m| {
        m.hovered(|p| p.x(50.))
            .transition(Transition::linear(ms(1000)))
            .reduced(Reduced::Keep)
    });
    let mut h = Harness::new(
        MotionConfig::new(site)
            .reduced_motion(ReducedMotion::Always)
            .into(),
    );
    h.hover(Offset::new(50., 50.));
    h.run(ms(100));
    let x = h.translation(0).x;
    assert!(x > 0. && x < 50., "essential motion keeps animating: {x}");
}

#[test]
fn in_view_follows_scrolling() {
    use incular_widgets::{ScrollController, SingleChildScrollView, SizedBox};
    let controller = ScrollController::new();
    let probe = Widget::box_(Size::new(50., 50.), Color::BLACK)
        .motion(|m| m.in_view(|p| p.x(10.)).transition(Transition::instant()));
    let content = Column::new([SizedBox::new().width(50.).height(400.).into(), probe]);
    let mut h = Harness::new(
        SingleChildScrollView::new(content)
            .controller(controller.clone())
            .into(),
    );
    h.run(ms(16));
    assert!(close(h.translation(0).x, 0.), "below the fold");
    controller.jump_to(300.);
    h.run(ms(32));
    assert!(close(h.translation(0).x, 10.), "scrolled into view");
    controller.jump_to(0.);
    h.run(ms(32));
    assert!(close(h.translation(0).x, 0.), "scrolled out again");
}

#[test]
fn in_view_once_latches() {
    use incular_widgets::motion::InView;
    use incular_widgets::{ScrollController, SingleChildScrollView, SizedBox};
    let controller = ScrollController::new();
    let probe = Widget::box_(Size::new(50., 50.), Color::BLACK).motion(|m| {
        m.in_view(|p| p.x(10.))
            .in_view_options(InView::new().amount(0.5).once())
            .transition(Transition::instant())
    });
    let content = Column::new([SizedBox::new().width(50.).height(400.).into(), probe]);
    let mut h = Harness::new(
        SingleChildScrollView::new(content)
            .controller(controller.clone())
            .into(),
    );
    // 400 - 200 + 10 = 210: only 10 of 50 px visible, below the amount.
    controller.jump_to(210.);
    h.run(ms(32));
    assert!(close(h.translation(0).x, 0.), "20% visible is not enough");
    controller.jump_to(300.);
    h.run(ms(32));
    assert!(close(h.translation(0).x, 10.));
    controller.jump_to(0.);
    h.run(ms(32));
    assert!(close(h.translation(0).x, 10.), "once keeps it in view");
}

#[test]
fn stagger_delay_orders() {
    use incular_widgets::motion::StaggerFrom;
    let s = Stagger::every(ms(10));
    assert_eq!(s.delay(3, 5), ms(30));
    assert_eq!(s.from(StaggerFrom::End).delay(0, 5), ms(40));
    assert_eq!(s.from(StaggerFrom::Center).delay(2, 5), ms(0));
    assert_eq!(s.from(StaggerFrom::Center).delay(0, 5), ms(20));
    assert_eq!(s.from(StaggerFrom::Index(1)).delay(4, 5), ms(30));
}

#[test]
fn drag_within_parent_stops_at_the_parent_edges() {
    use incular_widgets::SizedBox;
    use incular_widgets::motion::{Bounds, Drag, Release};
    let site = square().motion(|m| {
        m.drag(Drag::x().within(Bounds::Parent).release(Release::Stay))
            .transition(Transition::instant())
    });
    // A 150-wide column centres the 100-wide square: 25 px of room each side.
    let mut h =
        Harness::new(Column::new([SizedBox::new().width(150.).height(0.).into(), site]).into());
    h.frame();
    let gesture = |h: &mut Harness, x: f32, phase: PointerPhase| {
        h.tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(x, 50.),
            phase,
            time: h.now,
        });
    };
    h.press(Offset::new(75., 50.), true);
    gesture(&mut h, 75., PointerPhase::Down);
    for x in [80., 100., 130., 160., 175.] {
        h.now += ms(16);
        let event = pointer(Offset::new(x, 50.), PointerPhase::Move, 1, h.now);
        let _ = h.tree.dispatch_raw_pointer_in_window(7, event);
        gesture(&mut h, x, PointerPhase::Move);
        h.frame();
    }
    let dragged = h.translation(0);
    assert!(close(dragged.x, 25.), "clamped to the parent: {dragged:?}");
    h.press(Offset::new(175., 50.), false);
    gesture(&mut h, 175., PointerPhase::Up);
    h.run(ms(500));
    assert!(close(h.translation(0).x, 25.), "stays at the edge");
}

#[test]
fn lints_flag_shadowed_states_and_unkeyed_layout() {
    use incular_widgets::motion::{Interaction, Motion, MotionLint, Prop};
    let shadowed = Motion::new()
        .pressed(|p| p.scale(0.95).opacity(0.8))
        .hovered(|p| p.scale(1.05));
    assert_eq!(
        shadowed.lints(true),
        vec![MotionLint::Shadowed {
            narrow: Interaction::Pressed,
            wide: Interaction::Hovered,
            props: vec![Prop::Scale],
        }]
    );
    let ordered = Motion::new()
        .hovered(|p| p.scale(1.05))
        .pressed(|p| p.scale(0.95));
    assert!(
        ordered.lints(true).is_empty(),
        "hovered then pressed is fine"
    );
    assert_eq!(
        Motion::new().layout().lints(false),
        vec![MotionLint::UnkeyedLayout]
    );
    assert!(Motion::new().layout().lints(true).is_empty());
}

#[test]
fn motion_wrapper_carries_the_child_key() {
    let keyed = square()
        .with_key("card")
        .motion(|m| m.hovered(|p| p.scale(1.1)));
    assert!(keyed.key().is_some());
    assert!(
        square()
            .motion(|m| m.hovered(|p| p.scale(1.1)))
            .key()
            .is_none()
    );
}
