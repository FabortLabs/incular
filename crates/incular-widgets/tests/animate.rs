//! `.animate()` on the motion engine: chains sample into compositor layers
//! without rebuilding, with triggers, handles and progress sources.

use std::{
    cell::Cell,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as CoreTransform, reactivity::DependencySource};
use incular_rendering::{LayerKind, Shadow};
use incular_widgets::extensions::{AnimateExt, AnimateRepeat, RevealFrom};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{
    AnimationHandle, Cond, MotionConfig, Play, Progress, Prop, ReducedMotion, ViewRange, ms,
};
use incular_widgets::{Column, ScrollController, SingleChildScrollView, SizedBox, Widget};

struct Harness {
    tree: WidgetTree,
    now: Instant,
}

impl Harness {
    fn new(widget: impl Into<Widget>) -> Self {
        let mut tree = WidgetTree::new();
        tree.mount(widget.into()).expect("mount");
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
            .layout(Constraints::loose(Size::new(200., 200.)))
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

    fn transform(&self) -> CoreTransform {
        let mut stack = vec![self.tree.root().expect("root")];
        while let Some(id) = stack.pop() {
            if let Some(transform) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.content_transform(render))
            {
                return transform;
            }
            let children: Vec<ElementId> = self.tree.children(id).unwrap_or(&[]).to_vec();
            stack.extend(children.into_iter().rev());
        }
        panic!("no motion transform");
    }

    fn translation(&self) -> Offset {
        self.transform().translation_offset()
    }

    fn scale(&self) -> f32 {
        let [a, ..] = self.transform().coefficients();
        a as f32
    }

    fn layers(&self) -> Vec<LayerKind> {
        let layers = self.tree.compositor_layers();
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

    fn opacity(&self) -> f32 {
        self.layers()
            .into_iter()
            .find_map(|kind| match kind {
                LayerKind::Opacity { alpha } => Some(alpha),
                _ => None,
            })
            .unwrap_or(1.)
    }
}

fn square() -> Widget {
    Widget::box_(Size::new(100., 100.), Color::WHITE)
}

fn near(actual: f32, expected: f32, tolerance: f32) -> bool {
    (actual - expected).abs() <= tolerance
}

#[test]
fn chains_play_on_mount_without_rebuilding() {
    let mut h = Harness::new(
        square()
            .animate()
            .ms(200)
            .fade_in()
            .move_from(Offset::new(0., 20.)),
    );
    assert!(h.opacity() < 0.01, "starts at the begin values");
    assert!(near(h.translation().y, 20., 0.01));
    let before = h.tree.diagnostics();
    h.run(ms(112));
    let y = h.translation().y;
    assert!(y > 5. && y < 15., "halfway: {y}");
    h.run(ms(200));
    assert!(near(h.translation().y, 0., 0.01));
    assert!(h.opacity() > 0.99);
    let after = h.tree.diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds, "frames never rebuild");
    assert_eq!(after.layouts, before.layouts, "frames never relayout");
    assert_eq!(after.paints, before.paints, "frames never repaint");
}

#[test]
fn completion_fires_once_and_loops_keep_going() {
    let done = Rc::new(Cell::new(0));
    let counter = done.clone();
    let mut h = Harness::new(
        square()
            .animate()
            .ms(100)
            .scale(1., 2.)
            .on_complete(move || counter.set(counter.get() + 1)),
    );
    h.run(ms(400));
    assert_eq!(done.get(), 1);
    assert!(near(h.scale(), 2., 1e-3));

    let mut h = Harness::new(
        square()
            .animate()
            .ms(100)
            .move_x(0., 100.)
            .repeat(AnimateRepeat::PingPong),
    );
    h.run(ms(96));
    let forward = h.translation().x;
    h.run(ms(80));
    let back = h.translation().x;
    assert!(forward > 60., "went forward: {forward}");
    assert!(back < forward, "ping-pong comes back: {forward} -> {back}");
}

#[test]
fn timing_helpers_place_effects() {
    let chain = square()
        .animate()
        .ms(100)
        .fade_in()
        .then_gap(ms(50))
        .scale_in(0.5)
        .then_overlap(ms(30))
        .rotate_in(0.1);
    let effects = chain.effects();
    assert_eq!(effects[1].delay, ms(150));
    assert_eq!(effects[2].delay, ms(220));
    fn pop_in(a: incular_widgets::extensions::Animate) -> incular_widgets::extensions::Animate {
        a.fade_in().scale_in(0.9)
    }
    assert_eq!(square().animate().apply(pop_in).effects().len(), 2);
}

#[test]
fn keys_and_custom_effects_stay_on_the_compositor() {
    let mut h = Harness::new(
        square()
            .animate()
            .ms(300)
            .keys(Prop::Scale, [1., 2., 1.])
            .custom(|t, pose| pose.x(t * 40.)),
    );
    h.run(ms(160));
    let scale = h.scale();
    assert!(scale > 1.5, "keyframe peak: {scale}");
    h.run(ms(300));
    assert!(near(h.scale(), 1., 1e-3));
    assert!(near(h.translation().x, 40., 0.01));
}

#[test]
fn filter_effects_reach_layers() {
    let mut h = Harness::new(
        square()
            .animate()
            .ms(50)
            .tint(Color::rgba(255, 0, 0, 255), 0.5)
            .saturate(1., 0.2)
            .shadow(Shadow::new(Color::BLACK, Offset::new(0., 4.), 8.))
            .blur_in(3.),
    );
    h.run(ms(200));
    let kinds = h.layers();
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::ColorFilter { .. })),
        "color layer: {kinds:?}"
    );
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, LayerKind::DropShadow { .. })),
        "shadow layer: {kinds:?}"
    );
}

#[test]
fn clip_reveal_and_shimmer_build_as_paint_tier() {
    let mut h = Harness::new(square().animate().ms(100).clip_reveal(RevealFrom::Left));
    h.run(ms(300));
    let built = Rc::new(Cell::new(0));
    let seen = built.clone();
    let mut h2 = Harness::new(square().animate().ms(100).shimmer().build_each_frame(
        move |t, child| {
            seen.set(seen.get() + 1);
            assert!((0. ..=1.).contains(&t));
            child
        },
    ));
    let before = h2.tree.diagnostics().rebuilds;
    h2.run(ms(64));
    assert!(h2.tree.diagnostics().rebuilds > before, "shimmer rebuilds");
    assert!(built.get() > 1, "build_each_frame runs per frame");
    h.run(ms(16));
}

#[test]
fn when_plays_forward_and_back() {
    let on = Rc::new(Cell::new(false));
    let source = DependencySource::default();
    let read = on.clone();
    let cond = Cond::live(source.clone(), move || read.get());
    let mut h = Harness::new(
        square()
            .animate()
            .ms(100)
            .move_x(0., 50.)
            .play(Play::when(cond)),
    );
    h.run(ms(48));
    assert!(
        near(h.translation().x, 0., 1e-3),
        "starts at the matching end"
    );
    on.set(true);
    source.notify();
    h.run(ms(200));
    assert!(near(h.translation().x, 50., 1e-3), "forward while true");
    on.set(false);
    source.notify();
    h.run(ms(48));
    let mid = h.translation().x;
    assert!(mid > 0. && mid < 50., "reversing: {mid}");
    h.run(ms(200));
    assert!(near(h.translation().x, 0., 1e-3));
}

#[test]
fn handles_play_seek_reverse_and_finish() {
    let handle = AnimationHandle::new();
    let mut h = Harness::new(square().animate().ms(100).move_x(0., 100.).handle(&handle));
    h.run(ms(64));
    assert!(near(h.translation().x, 0., 1e-3), "waits for the handle");
    handle.seek(0.5);
    h.run(ms(16));
    assert!(near(h.translation().x, 50., 1e-3));
    handle.play();
    let mut finished = pin!(handle.finished());
    let mut context = Context::from_waker(Waker::noop());
    assert!(finished.as_mut().poll(&mut context).is_pending());
    h.run(ms(200));
    assert!(near(h.translation().x, 100., 1e-3));
    assert_eq!(finished.as_mut().poll(&mut context), Poll::Ready(()));
    handle.reverse();
    h.run(ms(200));
    assert!(near(h.translation().x, 0., 1e-3));
    assert!(!handle.is_playing());
}

#[test]
fn progress_values_animate_and_frames_hold() {
    let chain = square().animate().ms(100).move_x(0., 100.);
    let mut h = Harness::new(chain.frame(ms(25)));
    h.run(ms(48));
    assert!(near(h.translation().x, 25., 0.01), "frame holds");
    #[allow(deprecated)]
    let legacy = square().animate().ms(100).move_x(0., 100.).target(0.75);
    let mut h = Harness::new(legacy);
    h.run(ms(16));
    assert!(
        near(h.translation().x, 75., 0.01),
        "target is a progress alias"
    );
}

fn scroller(probe: impl Into<Widget>, controller: &ScrollController) -> Widget {
    let content = Column::new([
        SizedBox::new().width(100.).height(400.).into(),
        probe.into(),
    ]);
    SingleChildScrollView::new(content)
        .controller(controller.clone())
        .into()
}

#[test]
fn scroll_and_view_progress_follow_the_viewport() {
    let controller = ScrollController::new();
    let probe = square()
        .animate()
        .move_x(0., 100.)
        .progress(Progress::scroll().range(0., 100.));
    let mut h = Harness::new(scroller(probe, &controller));
    controller.jump_to(50.);
    h.run(ms(32));
    assert!(
        near(h.translation().x, 50., 0.5),
        "scroll 50 of 100: {:?}",
        h.translation()
    );

    let controller = ScrollController::new();
    let probe = square()
        .animate()
        .move_x(0., 100.)
        .progress(Progress::view(ViewRange::Entry));
    let mut h = Harness::new(scroller(probe, &controller));
    h.run(ms(16));
    assert!(near(h.translation().x, 0., 0.5), "not entered");
    // Viewport 200 tall; the probe's top is at 400 - offset.
    controller.jump_to(250.);
    h.run(ms(32));
    assert!(
        near(h.translation().x, 50., 0.5),
        "half entered: {:?}",
        h.translation()
    );
}

#[test]
fn in_view_plays_on_entry() {
    let controller = ScrollController::new();
    let probe = square()
        .animate()
        .ms(100)
        .move_x(0., 40.)
        .play(Play::in_view().once());
    let mut h = Harness::new(scroller(probe, &controller));
    h.run(ms(300));
    assert!(near(h.translation().x, 0., 1e-3), "below the fold");
    controller.jump_to(300.);
    h.run(ms(300));
    assert!(near(h.translation().x, 40., 1e-3), "played on entry");
}

#[test]
fn loops_pause_off_screen() {
    let controller = ScrollController::new();
    let probe = square()
        .animate()
        .ms(1000)
        .move_x(0., 100.)
        .repeat(AnimateRepeat::Loop);
    let mut h = Harness::new(scroller(probe, &controller));
    h.run(ms(400));
    assert!(
        near(h.translation().x, 0., 2.),
        "parked off screen: {:?}",
        h.translation()
    );
    controller.jump_to(300.);
    h.run(ms(400));
    assert!(
        h.translation().x > 20.,
        "resumes in view: {:?}",
        h.translation()
    );
}

#[test]
fn stagger_offsets_animate_chains() {
    use incular_widgets::motion::{MotionExt, Stagger};
    let item = || {
        Widget::box_(Size::new(20., 20.), Color::WHITE)
            .animate()
            .ms(100)
            .move_x(0., 50.)
    };
    let list = Column::new([Widget::from(item()), item().into(), item().into()])
        .motion(|m| m.stagger(Stagger::every(ms(100))));
    let mut h = Harness::new(list);
    h.run(ms(150));
    let mut xs = Vec::new();
    let mut stack = vec![h.tree.root().expect("root")];
    while let Some(id) = stack.pop() {
        if let Some(transform) = h
            .tree
            .render_id(id)
            .and_then(|render| h.tree.content_transform(render))
        {
            xs.push(transform.translation_offset().x);
        }
        let children: Vec<ElementId> = h.tree.children(id).unwrap_or(&[]).to_vec();
        stack.extend(children.into_iter().rev());
    }
    assert_eq!(xs.len(), 3, "three chains: {xs:?}");
    assert!(
        xs[0] > xs[1] && xs[1] >= xs[2],
        "later items start later: {xs:?}"
    );
}

#[test]
fn reduced_motion_jumps_to_the_end() {
    let mut h = Harness::new(
        MotionConfig::new(square().animate().ms(500).move_x(0., 30.))
            .reduced_motion(ReducedMotion::Always),
    );
    h.run(ms(32));
    assert!(near(h.translation().x, 30., 1e-3));
}
