//! Retained accessibility and real hit tests for Flutter proxy boxes.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_rendering::LayerLink;
use incular_runtime::Runtime;
use incular_widgets::{
    CompositedTransformFollower, CompositedTransformTarget, FractionalTranslation, GestureDetector,
    HitTestBehavior, Positioned, Semantics, SizedBox, Stack, Widget,
};

fn viewport() -> Constraints {
    Constraints::tight(Size::new(200., 200.))
}

#[test]
fn fractional_translation_updates_retained_semantic_bounds() {
    let build = |fraction| {
        FractionalTranslation::new(
            Offset::new(fraction, fraction),
            Semantics::new(Widget::box_(Size::new(200., 200.), Color::WHITE)).label("translated"),
        )
        .into()
    };
    let mut runtime = Runtime::new(build(0.5)).unwrap();
    runtime.run_frame(viewport()).unwrap();
    let id = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(id, node)| (node.label.as_deref() == Some("translated")).then_some(id))
        .unwrap();
    let assert_bounds = |runtime: &Runtime, from| {
        let bounds = runtime.tree().semantics().node(id).unwrap().bounds;
        let near = |actual: f32, reference: f32| (actual - reference).abs() <= 0.00001;
        assert!(
            near(bounds.origin.x, from) && near(bounds.origin.y, from),
            "{bounds:?}"
        );
        assert_eq!(bounds.size, Size::new(200., 200.));
    };
    assert_bounds(&runtime, 100.);
    let root = runtime.tree().root().unwrap();
    for (fraction, from) in [(0.4, 80.), (0.3, 60.)] {
        runtime.schedule_update(root, build(fraction)).unwrap();
        runtime.run_frame(viewport()).unwrap();
        assert_bounds(&runtime, from);
    }
}

#[test]
fn follower_hitability_depends_on_its_actual_link_only_when_configured() {
    for linked in [false, true] {
        for show_when_unlinked in [false, true] {
            let link = LayerLink::new();
            let calls = Rc::new(Cell::new(0));
            let child = GestureDetector::new(SizedBox::expand())
                .behavior(HitTestBehavior::Opaque)
                .on_tap({
                    let calls = calls.clone();
                    move || calls.set(calls.get() + 1)
                });
            let follower = CompositedTransformFollower::new(link.clone(), child)
                .show_when_unlinked(show_when_unlinked);
            let mut children = Vec::new();
            if linked {
                children.push(Widget::from(Positioned::fill(
                    CompositedTransformTarget::new(link, SizedBox::expand()),
                )));
            }
            children.push(Positioned::fill(follower).into());
            let mut runtime = Runtime::new(Stack::new(children).into()).unwrap();
            runtime.run_frame(viewport()).unwrap();
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let _ = runtime.handle_input(InputEvent::Pointer {
                    phase,
                    position: Offset::ZERO,
                });
            }
            assert_eq!(
                calls.get(),
                u32::from(linked || show_when_unlinked),
                "linked={linked} show_when_unlinked={show_when_unlinked}"
            );
        }
    }
}

#[test]
fn follower_hit_routing_uses_the_current_leader_or_fallback_slot() {
    use incular_config::Alignment;
    use incular_widgets::Align;

    let link = LayerLink::new();
    let calls = Rc::new(Cell::new(0));
    let scene = |leader: Option<Offset>, show_when_unlinked| {
        let mut children = Vec::new();
        if let Some(leader) = leader {
            children.push(
                Positioned::new(CompositedTransformTarget::new(
                    link.clone(),
                    SizedBox::new().width(20.).height(20.),
                ))
                .left(leader.x)
                .top(leader.y)
                .into(),
            );
        }
        let content = GestureDetector::new(SizedBox::new().width(20.).height(20.))
            .behavior(HitTestBehavior::Opaque)
            .on_tap({
                let calls = calls.clone();
                move || calls.set(calls.get() + 1)
            });
        let follower = CompositedTransformFollower::new(link.clone(), content)
            .offset(Offset::new(5., 10.))
            .show_when_unlinked(show_when_unlinked);
        children.push(
            Widget::from(Positioned::fill(Align::new(Alignment::TOP_LEFT, follower)))
                .with_key("follower slot"),
        );
        Stack::new(children).into()
    };
    let old = Offset::new(65., 80.);
    let new = Offset::new(145., 140.);
    let layout = Offset::new(10., 10.);
    let mut runtime = Runtime::new(scene(Some(Offset::new(50., 60.)), false)).unwrap();
    runtime.run_frame(viewport()).unwrap();
    let root = runtime.tree().root().unwrap();
    let update = |runtime: &mut Runtime, leader, show_when_unlinked| {
        runtime
            .schedule_update(root, scene(leader, show_when_unlinked))
            .unwrap();
        runtime.run_frame(viewport()).unwrap();
    };
    let send = |runtime: &mut Runtime, phase, position| {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    };
    let tap = |runtime: &mut Runtime, position| {
        send(runtime, PointerPhase::Down, position);
        send(runtime, PointerPhase::Up, position);
    };
    tap(&mut runtime, old);
    assert_eq!(calls.get(), 1);

    send(&mut runtime, PointerPhase::Down, old);
    update(&mut runtime, None, false);
    send(&mut runtime, PointerPhase::Up, old);
    assert_eq!(calls.get(), 2, "admitted pointer retains its route");

    tap(&mut runtime, old);
    tap(&mut runtime, layout);
    assert_eq!(calls.get(), 2, "hidden orphan admits no new pointer");

    update(&mut runtime, Some(Offset::new(130., 120.)), false);
    tap(&mut runtime, old);
    tap(&mut runtime, new);
    assert_eq!(calls.get(), 3, "only the current leader position hits");

    update(&mut runtime, None, true);
    tap(&mut runtime, new);
    tap(&mut runtime, layout);
    assert_eq!(calls.get(), 4, "the visible orphan uses its layout slot");
}

#[test]
fn moving_a_leader_refreshes_a_stationary_mouse_over_its_follower() {
    for with_focus in [false, true] {
        check_stationary_follower_hover(with_focus);
    }
}

fn check_stationary_follower_hover(with_focus: bool) {
    use std::{cell::RefCell, time::Duration};

    use incular_config::Alignment;
    use incular_core::{PointerDeviceKind, PointerSampleMetadata, time::Instant};
    use incular_widgets::{
        Align, Focus, FocusNode, MouseCursor, MouseRegion, internal::TranslationController,
    };

    let link = LayerLink::new();
    let controller = TranslationController::new();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let focus = FocusNode::new();
    let leaf = SizedBox::new().width(20.).height(20.);
    let child: Widget = if with_focus {
        Focus::new(leaf).node(focus.clone()).into()
    } else {
        leaf.into()
    };
    let region = MouseRegion::new(child)
        .cursor(MouseCursor::Click)
        .on_enter({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("enter")
        })
        .on_exit({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("exit")
        });
    let follower = CompositedTransformFollower::new(link.clone(), region)
        .offset(Offset::new(5., 10.))
        .show_when_unlinked(false);
    let leader = Widget::translate(
        controller.clone(),
        CompositedTransformTarget::new(link, SizedBox::new().width(20.).height(20.)).into(),
    );
    let scene = Stack::new([
        Widget::from(Positioned::new(leader).left(50.).top(60.)),
        Widget::from(Positioned::fill(Align::new(Alignment::TOP_LEFT, follower))),
    ]);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.tree_mut().set_semantics_enabled(false);
    let now = Instant::now();
    runtime.run_frame_at(viewport(), now).unwrap();
    if with_focus {
        assert_eq!(focus.offset(), Offset::new(55., 70.));
    }
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 19,
            device: 2,
            kind: PointerDeviceKind::Mouse,
            buttons: 0,
            button: None,
            phase: PointerPhase::Move,
            position: Offset::new(65., 80.),
            sample: PointerSampleMetadata::EMPTY,
        },
        now + Duration::from_millis(1),
    );
    assert_eq!(&*trace.borrow(), &["enter"]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Click);

    controller.set_offset(Offset::new(80., 60.));
    let (_, frame) = runtime
        .run_frame_at(viewport(), now + Duration::from_millis(16))
        .unwrap();
    assert_eq!(frame.laid_out_render_objects, 0);
    assert_eq!(frame.repainted_render_objects, 0);
    assert_eq!(&*trace.borrow(), &["enter", "exit"]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
    if with_focus {
        assert_eq!(focus.offset(), Offset::new(135., 130.));
    }

    controller.set_offset(Offset::ZERO);
    runtime
        .run_frame_at(viewport(), now + Duration::from_millis(32))
        .unwrap();
    assert_eq!(&*trace.borrow(), &["enter", "exit", "enter"]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Click);
    if with_focus {
        assert_eq!(focus.offset(), Offset::new(55., 70.));
    }
}
