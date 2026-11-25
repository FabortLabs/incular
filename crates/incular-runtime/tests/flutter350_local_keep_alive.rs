//! A materialized child can change its parent-data request locally.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GridView, KeepAlive, ListView, ScrollController, Semantics, SliverGridDelegate,
    Widget, internal::Key,
};

fn tile(i: usize, keep: bool) -> Widget {
    let item = Semantics::new(Widget::box_(Size::new(100., 50.), Color::WHITE))
        .label(format!("tile {i}"))
        .container(true);
    KeepAlive::new(keep, Widget::from(item).with_key(Key::from(i as u64))).into()
}

#[test]
fn local_request_keeps_then_disposes_an_offscreen_child() {
    let view = Constraints::tight(Size::new(100., 100.));
    for (fixed, grid) in [(false, false), (true, false), (false, true)] {
        let controller = ScrollController::new();
        let scroll: Widget = if grid {
            GridView::builder(
                40,
                SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(50.),
                |i| tile(i, false),
            )
            .cache_extent(0.)
            .controller(controller.clone())
            .into()
        } else {
            let list = ListView::builder(20, |i| tile(i, false))
                .cache_extent(0.)
                .controller(controller.clone());
            if fixed {
                list.item_extent(50.).into()
            } else {
                list.into()
            }
        };
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, scroll).into()).unwrap();
        runtime.run_frame(view).unwrap();
        let key = Key::from(0_u64);
        let target = runtime.tree().element_with_key(&key).unwrap();
        runtime.schedule_update(target, tile(0, true)).unwrap();
        runtime.run_frame(view).unwrap();
        assert!(controller.jump_to(300.));
        runtime.run_frame(view).unwrap();
        assert_eq!(
            runtime.tree().element_with_key(&key),
            Some(target),
            "fixed={fixed} grid={grid}"
        );
        assert!(
            !runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some("tile 0"))
        );

        // Clear the same mounted child's request without changing the
        // surrounding view or issuing another scroll.
        let away = controller.offset();
        runtime.schedule_update(target, tile(0, false)).unwrap();
        runtime.run_frame(view).unwrap();
        assert!(
            !runtime.tree().element_exists(target),
            "fixed={fixed} grid={grid}"
        );
        assert_eq!(controller.offset(), away);
        assert!(controller.jump_to(0.));
        runtime.run_frame(view).unwrap();
        assert_ne!(runtime.tree().element_with_key(&key).unwrap(), target);
    }
}

fn compare_original_keep_alive_rows(fixed: bool, grid: bool) {
    use incular_core::{
        InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata,
        time::Instant,
    };
    use incular_widgets::internal::PointerDeviceKind;
    use std::time::Duration;

    let bounds = Constraints::tight(Size::new(800., 600.));
    let row = move |i: usize, keep| {
        let height = if grid { 24.6 } else { 12.3 };
        let child = Semantics::new(Widget::box_(Size::new(100., height), Color::WHITE))
            .label(format!("tile {i}"))
            .container(true);
        Widget::from(KeepAlive::new(
            keep,
            Widget::from(child).with_key(Key::from(i as u64)),
        ))
    };
    let controller = ScrollController::new();
    let scroll: Widget = if grid {
        GridView::builder(
            100,
            SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(24.6),
            move |i| row(i, false),
        )
        .controller(controller.clone())
        .cache_extent(0.)
        .into()
    } else {
        let list = ListView::builder(100, move |i| row(i, false))
            .controller(controller.clone())
            .cache_extent(0.);
        if fixed {
            list.item_extent(12.3).into()
        } else {
            list.into()
        }
    };
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scroll).into()).unwrap();
    let mut time = Instant::now();
    let get = |runtime: &Runtime, index: u64| runtime.tree().element_with_key(&Key::from(index));
    let check = |stage: &str, runtime: &Runtime, present: &[u64], missing: &[u64]| {
        for &index in present {
            assert!(
                get(runtime, index).is_some(),
                "{stage}: missing {index}, fixed={fixed}, grid={grid}"
            );
        }
        for &index in missing {
            assert!(
                get(runtime, index).is_none(),
                "{stage}: unexpected {index}, fixed={fixed}, grid={grid}"
            );
        }
    };
    let drag = |runtime: &mut Runtime, time: &mut Instant, pointer: u64, dy: f32| {
        for (phase, y) in [
            (PointerPhase::Down, 300.),
            (PointerPhase::Move, 300. + dy.signum() * 20.),
            (PointerPhase::Move, 300. + dy),
            (PointerPhase::Up, 300. + dy),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Up {
                        0
                    } else {
                        PRIMARY_POINTER_BUTTON
                    },
                    button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(400., y),
                },
                *time,
            );
        }
        *time += Duration::from_millis(16);
        runtime.run_frame_at(bounds, *time).unwrap();
    };

    runtime.run_frame_at(bounds, time).unwrap();
    check("initial", &runtime, &[3, 30], &[59, 60, 61, 90]);
    drag(&mut runtime, &mut time, 1, -300.);
    check("after outward drag", &runtime, &[30, 59, 60, 61], &[3, 90]);

    let original = get(&runtime, 60).unwrap();
    runtime.schedule_update(original, row(60, true)).unwrap();
    drag(&mut runtime, &mut time, 2, 300.);
    check("after return drag", &runtime, &[3, 30, 60], &[59, 61, 90]);
    assert_eq!(get(&runtime, 60), Some(original));
    assert!(
        !runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("tile 60"))
    );

    runtime.schedule_update(original, row(60, false)).unwrap();
    time += Duration::from_millis(16);
    runtime.run_frame_at(bounds, time).unwrap();
    check("after withdrawing", &runtime, &[3, 30], &[59, 60, 61, 90]);
}

#[test]
fn flutter_original_fixed_list_keep_alive_after_touch_drags() {
    compare_original_keep_alive_rows(true, false);
}

#[test]
fn flutter_original_variable_list_keep_alive_after_touch_drags() {
    compare_original_keep_alive_rows(false, false);
}

#[test]
fn flutter_original_grid_keep_alive_after_touch_drags() {
    compare_original_keep_alive_rows(false, true);
}
