//! Flutter's shrink-wrapping viewport bounces at both content edges.

use std::time::Duration;

use incular_config::{Axis, Constraints, TargetPlatform};
use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, ListView, RawPointerEvent, ScrollBehavior, ScrollConfiguration,
    ScrollController, ScrollPhysics, SizedBox, TextDirection, Widget,
};

#[test]
fn explicit_bouncing_physics_overscrolls_in_both_directions() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        check_edges(axis, TargetPlatform::Linux, Some(ScrollPhysics::BOUNCING));
    }
}

#[test]
fn apple_default_configuration_overscrolls_in_both_directions() {
    for platform in [TargetPlatform::Ios, TargetPlatform::MacOs] {
        for axis in [Axis::Vertical, Axis::Horizontal] {
            check_edges(axis, platform, None);
        }
    }
}

fn check_edges(axis: Axis, platform: TargetPlatform, physics: Option<ScrollPhysics>) {
    let viewport = Size::new(800., 600.);
    let scroll_extent = axis.main_extent(viewport);
    let max = 1000. - scroll_extent;
    let constraints = Constraints::tight(viewport);
    let ctl = ScrollController::new();
    let list = ListView::new((0u64..20).map(|i| Widget::from(SizedBox::square(50.)).with_key(i)))
        .item_extent(50.)
        .shrink_wrap(true)
        .scroll_direction(axis)
        .controller(ctl.clone());
    let list = if let Some(physics) = physics {
        list.physics(physics)
    } else {
        list
    };
    let root = Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(list).behavior(ScrollBehavior::new().platform(platform)),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let now = Instant::now();
    runtime.run_frame_at(constraints, now).unwrap();

    let center = Offset::new(400., 300.);
    let track = |runtime: &mut Runtime, pointer, phase, position, ms| {
        let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            position,
            phase,
            now + Duration::from_millis(ms),
        ));
    };
    let edge_of = |runtime: &Runtime, index: u64, trailing: bool| {
        let tree = runtime.tree();
        let target = tree.element_with_key(&index.into()).unwrap();
        let rect = tree.element_bounds(target).unwrap();
        let start = match axis {
            Axis::Vertical => rect.origin.y,
            Axis::Horizontal => rect.origin.x,
        };
        start
            + if trailing {
                axis.main_extent(rect.size)
            } else {
                0.
            }
    };
    assert_eq!(ctl.offset(), 0.);
    assert_eq!(ctl.max_offset(), max);
    for (pointer, trailing, pixels, item, edge, sign, start) in [
        (1, false, 0., 0u64, 0., 1., 10),
        (2, true, max, 19u64, scroll_extent, -1., 3010),
    ] {
        if trailing {
            assert!(ctl.jump_to(max));
            runtime
                .run_frame_at(constraints, now + Duration::from_millis(start - 2))
                .unwrap();
        }
        assert_eq!(ctl.offset(), pixels);
        assert_eq!(edge_of(&runtime, item, trailing), edge);
        let moved = center + axis.offset(sign * 25., 0.);
        track(&mut runtime, pointer, PointerPhase::Down, center, start);
        track(&mut runtime, pointer, PointerPhase::Move, moved, start + 16);
        runtime
            .run_frame_at(constraints, now + Duration::from_millis(start + 16))
            .unwrap();
        assert_eq!(
            ctl.offset(),
            pixels - sign * 25.,
            "{axis:?} {platform:?} trailing={trailing}"
        );
        assert_eq!(edge_of(&runtime, item, trailing), edge + sign * 25.);
        track(&mut runtime, pointer, PointerPhase::Up, moved, start + 100);
        for step in 0..150 {
            let (_, stats) = runtime
                .run_frame_at(
                    constraints,
                    now + Duration::from_millis(start + 100 + step * 16),
                )
                .unwrap();
            if !stats.requested_another_frame && ctl.offset() == pixels {
                break;
            }
        }
        assert_eq!(ctl.offset(), pixels);
        assert_eq!(edge_of(&runtime, item, trailing), edge);
    }
}

#[test]
fn apple_shrink_wrapping_grid_inside_a_column_lays_out_and_accepts_a_drag() {
    use incular_config::{CrossAxisAlignment, MainAxisAlignment};
    use incular_widgets::{Column, GridView, SliverGridDelegate, Text};

    for platform in [TargetPlatform::Ios, TargetPlatform::MacOs] {
        let ctl = ScrollController::new();
        let grid = GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(3)
                .child_aspect_ratio(3.)
                .main_axis_spacing(3.)
                .cross_axis_spacing(3.),
            ["a", "b", "c"].map(|s| Widget::from(Text::new(s)).with_key(s)),
        )
        .shrink_wrap(true)
        .controller(ctl.clone());
        let layout = Column::new([grid])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_alignment(MainAxisAlignment::Center);
        let scene = Directionality::new(
            TextDirection::Ltr,
            ScrollConfiguration::new(layout).behavior(ScrollBehavior::new().platform(platform)),
        );
        let mut runtime = Runtime::new(scene.into()).unwrap();
        let bounds = Constraints::tight(Size::new(800., 600.));
        let now = Instant::now();
        runtime.run_frame_at(bounds, now).unwrap();

        let tile = runtime.tree().element_with_key(&"b".into()).unwrap();
        let before = runtime.tree().element_bounds(tile).unwrap();
        let expected_height = (800. - 6.) / 9.;
        assert!((ctl.viewport_extent() - expected_height).abs() < 0.0001);
        assert_eq!(ctl.max_offset(), 0.);

        let center = before.origin + Offset::new(before.size.width / 2., before.size.height / 2.);
        for (phase, y, ms) in [
            (PointerPhase::Down, 0., 10),
            (PointerPhase::Move, 200., 30),
            (PointerPhase::Up, 200., 150),
        ] {
            let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
                1,
                0,
                PointerDeviceKind::Touch,
                if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                center + Offset::new(0., y),
                phase,
                now + Duration::from_millis(ms),
            ));
            runtime
                .run_frame_at(bounds, now + Duration::from_millis(ms))
                .unwrap();
        }

        assert_eq!(ctl.offset(), 0.);
        assert_eq!(runtime.tree().element_bounds(tile).unwrap(), before);
    }
}
