//! Flutter 3.50.0 raw mouse annotations, hit policy, and synthetic touch hover.

use incular_config::{Constraints, EdgeInsets};
use incular_core::{
    Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::Runtime;
use incular_widgets::{Center, HitTestBehavior, MouseRegion, Padding, SizedBox, Stack, Widget};
use std::{cell::RefCell, rc::Rc};

type Log = Rc<RefCell<Vec<(&'static str, PointerPhase, Offset)>>>;

fn observed(region: MouseRegion, name: &'static str, log: &Log) -> MouseRegion {
    let (enter, hover, exit) = (log.clone(), log.clone(), log.clone());
    region
        .on_enter(move |event| {
            enter
                .borrow_mut()
                .push((name, PointerPhase::Enter, event.local_position))
        })
        .on_hover(move |event| {
            hover
                .borrow_mut()
                .push((name, PointerPhase::Move, event.local_position))
        })
        .on_exit(move |event| {
            exit.borrow_mut()
                .push((name, PointerPhase::Exit, event.local_position))
        })
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn add_mouse(runtime: &mut Runtime, position: Offset) {
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 1,
        device: 1,
        kind: PointerDeviceKind::Mouse,
        position,
        change: HoverDeviceChange::Added,
    }));
}

fn motion(runtime: &mut Runtime, kind: PointerDeviceKind, position: Offset) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device: 1,
        kind,
        position,
        phase: PointerPhase::Move,
        buttons: 0,
        button: None,
        sample: PointerSampleMetadata::default(),
    });
}

#[test]
fn default_empty_region_enters_after_defer_to_child_stays_unclaimed() {
    let log = Log::default();
    let root = |defer| {
        let mut region = observed(MouseRegion::without_child(), "region", &log);
        if defer {
            region = region.hit_test_behavior(HitTestBehavior::DeferToChild);
        }
        Center::new(region).into()
    };
    let mut runtime = Runtime::new(root(true)).unwrap();
    frame(&mut runtime);
    add_mouse(&mut runtime, Offset::ZERO);
    assert!(log.take().is_empty());
    runtime
        .schedule_update(runtime.tree().root().unwrap(), root(false))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(log.take(), [("region", PointerPhase::Enter, Offset::ZERO)]);
}

fn layered(top: MouseRegion) -> Vec<&'static str> {
    let log = Log::default();
    let bottom = observed(MouseRegion::without_child(), "bottom", &log);
    let top = observed(top, "top", &log);
    let root = Stack::new([
        SizedBox::square(50.).child(bottom),
        SizedBox::square(50.).child(top),
    ]);
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    add_mouse(&mut runtime, Offset::ZERO);
    let mut result = log
        .take()
        .into_iter()
        .map(|(name, phase, _)| {
            assert_eq!(phase, PointerPhase::Enter);
            name
        })
        .collect::<Vec<_>>();
    result.sort_unstable();
    result
}

#[test]
fn non_opaque_deferred_region_hits_its_child_and_the_region_behind() {
    let top = MouseRegion::new(Widget::box_(Size::new(50., 50.), Color::WHITE))
        .opaque(false)
        .hit_test_behavior(HitTestBehavior::DeferToChild);
    assert_eq!(layered(top), ["bottom", "top"]);
}

#[test]
fn translucent_empty_region_also_reports_the_region_behind_it() {
    assert_eq!(
        layered(MouseRegion::without_child().hit_test_behavior(HitTestBehavior::Translucent)),
        ["bottom", "top"],
    );
}

#[test]
fn derived_default_and_regular_no_child_constructor_are_both_opaque() {
    assert_eq!(layered(MouseRegion::without_child()), ["top"]);
    assert_eq!(layered(MouseRegion::default()), ["top"]);
}

#[test]
fn a_buttonless_touch_hover_fires_hover_but_does_not_track_a_mouse() {
    let log = Log::default();
    let root = Center::new(observed(
        MouseRegion::new(SizedBox::square(100.)),
        "region",
        &log,
    ));
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    motion(
        &mut runtime,
        PointerDeviceKind::Touch,
        Offset::new(400., 300.),
    );
    assert_eq!(
        log.take(),
        [("region", PointerPhase::Move, Offset::new(50., 50.))]
    );
    assert!(!runtime.has_hover_device());
}

#[test]
fn leaving_a_nested_child_keeps_hovering_its_parent() {
    let log = Log::default();
    let child = observed(MouseRegion::without_child(), "child", &log);
    let padded = Padding::new(EdgeInsets::all(50.), child);
    let parent = observed(
        MouseRegion::new(SizedBox::square(200.).child(padded)),
        "parent",
        &log,
    );
    let mut runtime = Runtime::new(Center::new(parent).into()).unwrap();
    frame(&mut runtime);
    add_mouse(&mut runtime, Offset::new(400., 0.));
    log.take();
    motion(
        &mut runtime,
        PointerDeviceKind::Mouse,
        Offset::new(400., 300.),
    );
    assert_eq!(
        log.take(),
        [
            ("parent", PointerPhase::Enter, Offset::new(100., 100.)),
            ("child", PointerPhase::Enter, Offset::new(50., 50.)),
            ("child", PointerPhase::Move, Offset::new(50., 50.)),
            ("parent", PointerPhase::Move, Offset::new(100., 100.)),
        ]
    );
    motion(
        &mut runtime,
        PointerDeviceKind::Mouse,
        Offset::new(325., 300.),
    );
    assert_eq!(
        log.take(),
        [
            ("child", PointerPhase::Exit, Offset::new(-25., 50.)),
            ("parent", PointerPhase::Move, Offset::new(25., 100.)),
        ]
    );
}

#[test]
fn a_touch_contact_motion_is_not_a_hover_even_with_no_button_bits() {
    let log = Log::default();
    let root = Center::new(observed(
        MouseRegion::new(SizedBox::square(100.)),
        "region",
        &log,
    ));
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device: 1,
        kind: PointerDeviceKind::Touch,
        position: Offset::new(400., 300.),
        phase: PointerPhase::Down,
        buttons: 1,
        button: Some(incular_core::PRIMARY_POINTER_BUTTON),
        sample: PointerSampleMetadata::default(),
    });
    motion(
        &mut runtime,
        PointerDeviceKind::Touch,
        Offset::new(401., 300.),
    );
    assert!(log.take().is_empty());
    assert!(!runtime.has_hover_device());
}

#[test]
fn switching_opacity_under_a_still_pointer_releases_only_the_covered_region() {
    let log = Log::default();
    let root = |opaque| {
        let bottom = observed(MouseRegion::without_child(), "bottom", &log);
        let top = observed(MouseRegion::without_child().opaque(opaque), "top", &log);
        Widget::from(Stack::new([
            SizedBox::square(50.).child(bottom),
            SizedBox::square(50.).child(top),
        ]))
    };
    let mut runtime = Runtime::new(root(true)).unwrap();
    frame(&mut runtime);
    add_mouse(&mut runtime, Offset::new(10., 10.));
    assert_eq!(
        log.take(),
        [("top", PointerPhase::Enter, Offset::new(10., 10.))]
    );
    for (opaque, phase) in [(false, PointerPhase::Enter), (true, PointerPhase::Exit)] {
        runtime
            .schedule_update(runtime.tree().root().unwrap(), root(opaque))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(log.take(), [("bottom", phase, Offset::new(10., 10.))]);
    }
}
