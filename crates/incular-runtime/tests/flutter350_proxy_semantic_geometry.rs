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
