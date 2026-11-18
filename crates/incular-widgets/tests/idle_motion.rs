//! Idle motion can reactivate without losing ticker muting or geometry updates.
use incular_animation::MotionValue;
use incular_config::Constraints;
use incular_core::{Color, Size, reactivity::DependencySource, time::Instant};
use incular_widgets::extensions::AnimateExt;
use incular_widgets::internal::{RenderKind, WidgetTree};
use incular_widgets::motion::{Cond, MotionConfig, MotionExt, Play, ReducedMotion, Transition, ms};
use incular_widgets::{
    Column, Padding, ScrollController, SingleChildScrollView, SizedBox, TickerMode, Visibility,
    Widget,
};
use std::{cell::Cell, rc::Rc};

fn live_motion(value: &MotionValue) -> Widget {
    let mut child = Widget::box_(Size::new(20., 20.), Color::WHITE)
        .motion(|motion| motion.rest(|pose| pose.x(value)));
    for _ in 0..16 {
        child = Padding::all(0., child).into();
    }
    child
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
}

fn motion_offsets(tree: &WidgetTree) -> Vec<f32> {
    let mut offsets = Vec::new();
    let mut work: Vec<_> = tree.root().into_iter().collect();
    while let Some(element) = work.pop() {
        if let Some(render) = tree.render_id(element)
            && matches!(
                tree.render_object_kind(render),
                Some(RenderKind::Affine { .. })
            )
        {
            offsets.push(
                tree.content_transform(render)
                    .expect("motion transform")
                    .translation_offset()
                    .x,
            );
        }
        work.extend(tree.children(element).unwrap_or(&[]).iter().rev().copied());
    }
    offsets
}

fn prepare(widget: Widget) -> (WidgetTree, Instant) {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    layout(&mut tree);
    let now = Instant::now();
    tree.tick_animations(now);
    tree.update_compositor(now).expect("compositor");
    let _ = tree.paint();
    (tree, now)
}

#[test]
fn a_settled_motion_reactivates_beside_idle_siblings_without_rebuilding() {
    let values: Vec<_> = (0..8).map(|_| MotionValue::new(0.)).collect();
    let (mut tree, mut now) = prepare(Column::new(values.iter().map(live_motion)).into());
    assert_eq!(motion_offsets(&tree), vec![0.; values.len()]);
    let before = tree.diagnostics();
    assert!(!tree.tick_animations(now));
    assert_eq!(
        tree.update_compositor(now).expect("idle compositor"),
        (false, false)
    );

    for (index, value) in values.iter().enumerate() {
        now += ms(16);
        value.set((index + 1) as f32);
        assert!(
            tree.tick_animations(now),
            "a newly changed value wakes its delegate"
        );
        let (_, active) = tree.update_compositor(now).expect("reactivated compositor");
        assert!(!active, "a direct value update settles in one frame");
        let offsets = motion_offsets(&tree);
        for (slot, offset) in offsets.iter().enumerate() {
            assert_eq!(*offset, if slot <= index { (slot + 1) as f32 } else { 0. });
        }
        assert!(
            !tree.tick_animations(now),
            "every other delegate remains idle"
        );
    }
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(
        tree.diagnostics().animation_ticks - before.animation_ticks,
        values.len() as u64
    );
}

#[test]
fn reactivated_motion_waits_for_disabled_ticker_mode_or_hidden_visibility() {
    for use_visibility in [false, true] {
        let value = MotionValue::new(0.);
        let busy = MotionValue::new(0.);
        let site = live_motion(&value);
        let busy_site = live_motion(&busy);
        let build = |enabled| {
            let guarded: Widget = if use_visibility {
                Visibility::new(site.clone())
                    .visible(enabled)
                    .maintain_state(true)
                    .into()
            } else {
                TickerMode::new(enabled, site.clone()).into()
            };
            Widget::from(Column::new([guarded, busy_site.clone()]))
        };
        let (mut tree, mut now) = prepare(build(false));
        let root = tree.root().expect("root");
        value.set(30.);
        busy.set(10.);
        now += ms(16);
        assert!(tree.tick_animations(now), "the unmuted sibling changes");
        assert!(!tree.update_compositor(now).expect("muted compositor").1);
        assert_eq!(
            motion_offsets(&tree),
            [0., 10.],
            "the muted delegate keeps its pending value"
        );

        tree.update(root, build(true))
            .expect("enable retained motion");
        layout(&mut tree);
        now += ms(16);
        tree.tick_animations(now);
        tree.update_compositor(now).expect("unmuted compositor");
        assert_eq!(motion_offsets(&tree), [30., 10.]);
    }
}

#[test]
fn idle_motion_still_observes_scroll_geometry_and_reduced_motion() {
    for reduced in [ReducedMotion::Never, ReducedMotion::Always] {
        let controller = ScrollController::new();
        let probe = Widget::box_(Size::new(20., 20.), Color::WHITE).motion(|motion| {
            motion
                .in_view(|pose| pose.x(20.))
                .transition(Transition::linear(ms(100)))
        });
        let content = Column::new([SizedBox::new().width(100.).height(300.).into(), probe]);
        let widget =
            MotionConfig::new(SingleChildScrollView::new(content).controller(controller.clone()))
                .reduced_motion(reduced);
        let (mut tree, mut now) = prepare(widget.into());
        assert_eq!(motion_offsets(&tree), [0.], "below the viewport and idle");
        assert!(!tree.update_compositor(now).expect("idle compositor").1);
        let before = tree.diagnostics();

        controller.jump_to(controller.max_offset());
        now += ms(16);
        let (_, active) = tree.update_compositor(now).expect("scroll into view");
        assert_eq!(
            active,
            reduced == ReducedMotion::Never,
            "geometry can wake an idle delegate"
        );
        if reduced == ReducedMotion::Always {
            assert_eq!(
                motion_offsets(&tree),
                [20.],
                "reduced geometry snaps immediately"
            );
        }
        for _ in 0..16 {
            now += ms(16);
            tree.tick_animations(now);
            tree.update_compositor(now).expect("animate into view");
        }
        assert_eq!(motion_offsets(&tree), [20.]);
        assert!(!tree.update_compositor(now).expect("settled compositor").1);
        assert_eq!(
            tree.diagnostics().layouts,
            before.layouts,
            "geometry uses retained layout"
        );
        assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    }
}

#[test]
fn muted_timeline_keeps_a_live_wake_until_it_can_tick() {
    let source = DependencySource::default();
    let on = Rc::new(Cell::new(false));
    let reads = Rc::new(Cell::new(0));
    let completions = Rc::new(Cell::new(0));
    let condition = Cond::live(source.clone(), {
        let on = on.clone();
        let reads = reads.clone();
        move || {
            reads.set(reads.get() + 1);
            on.get()
        }
    });
    let timeline: Widget = Widget::box_(Size::new(20., 20.), Color::WHITE)
        .animate()
        .ms(64)
        .move_x(0., 24.)
        .play(Play::when(condition))
        .on_complete({
            let completions = completions.clone();
            move || completions.set(completions.get() + 1)
        })
        .into();
    let busy = MotionValue::new(0.);
    let busy_site = live_motion(&busy);
    let build = |enabled| {
        Widget::from(Column::new([
            busy_site.clone(),
            TickerMode::new(enabled, timeline.clone()).into(),
        ]))
    };
    let (mut tree, mut now) = prepare(build(true));
    let root = tree.root().expect("root");
    tree.update(root, build(false))
        .expect("mute settled timeline");
    layout(&mut tree);
    on.set(true);
    source.notify();
    let reads_before = reads.get();

    for step in 1..=4 {
        // A busy sibling puts the muted timeline in both activity passes.
        busy.set(step as f32);
        now += ms(16);
        tree.tick_animations(now);
        assert!(!tree.update_compositor(now).expect("muted compositor").1);
    }
    assert_eq!(motion_offsets(&tree)[1], 0., "muting prevents advancement");
    assert_eq!(
        reads.get(),
        reads_before,
        "repeated activity queries retain the pending wake instead of consuming it"
    );
    assert_eq!(completions.get(), 0);

    tree.update(root, build(true)).expect("unmute timeline");
    layout(&mut tree);
    for _ in 0..12 {
        now += ms(16);
        tree.tick_animations(now);
        tree.update_compositor(now).expect("resumed timeline");
    }
    assert_eq!(motion_offsets(&tree)[1], 24.);
    assert_eq!(completions.get(), 1, "the resumed run completes once");
    for _ in 0..3 {
        now += ms(16);
        assert!(!tree.tick_animations(now));
        assert!(!tree.update_compositor(now).expect("settled timeline").1);
    }
    assert_eq!(completions.get(), 1);
}

#[test]
fn timeline_completion_can_enqueue_another_live_wake_and_then_settle() {
    let source = DependencySource::default();
    let on = Rc::new(Cell::new(false));
    let completions = Rc::new(Cell::new(0));
    let condition = Cond::live(source.clone(), {
        let on = on.clone();
        move || on.get()
    });
    let timeline: Widget = Widget::box_(Size::new(20., 20.), Color::WHITE)
        .animate()
        .ms(64)
        .move_x(0., 24.)
        .play(Play::when(condition))
        .on_complete({
            let on = on.clone();
            let source = source.clone();
            let completions = completions.clone();
            move || {
                completions.set(completions.get() + 1);
                on.set(false);
                source.notify();
            }
        })
        .into();
    let (mut tree, mut now) = prepare(timeline);
    let before = tree.diagnostics();
    on.set(true);
    source.notify();
    let mut saw_completion = false;
    for _ in 0..20 {
        now += ms(16);
        tree.tick_animations(now);
        let (_, active) = tree.update_compositor(now).expect("timeline compositor");
        if !saw_completion && completions.get() == 1 {
            saw_completion = true;
            assert!(
                active,
                "the completion callback queued the reverse transition"
            );
        }
    }
    assert!(saw_completion);
    assert_eq!(
        completions.get(),
        1,
        "reversing does not replay forward completion"
    );
    assert_eq!(motion_offsets(&tree), [0.]);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    for _ in 0..3 {
        now += ms(16);
        assert!(!tree.tick_animations(now));
        assert!(!tree.update_compositor(now).expect("settled timeline").1);
    }
    assert_eq!(completions.get(), 1);
}
