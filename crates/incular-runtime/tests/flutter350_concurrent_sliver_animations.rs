//! Sibling sliver transitions are driven by the same frame clock.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, Sliver, SliverAnimatedOpacity, SliverCrossAxisGroup,
    SliverMainAxisGroup, SliverToBoxAdapter, TextDirection, Widget,
};

fn alphas(scene: &DisplayList) -> Vec<f32> {
    let mut values = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
            _ => None,
        })
        .collect::<Vec<_>>();
    values.sort_by(f32::total_cmp);
    values
}

fn close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} != {expected:?}");
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 0.0001),
        "{actual:?} != {expected:?}"
    );
}

#[derive(Clone, Copy, Debug)]
enum Group {
    Viewport,
    MainAxis,
    CrossAxis,
}

#[test]
fn fades_in_a_viewport_share_frame_time() {
    check(Group::Viewport);
}

#[test]
fn fades_in_a_main_axis_group_share_frame_time() {
    check(Group::MainAxis);
}

#[test]
fn fades_in_a_cross_axis_group_share_frame_time() {
    check(Group::CrossAxis);
}

fn check(group: Group) {
    let ended = Rc::new([const { Cell::new(0) }; 3]);
    let view = |targets: [f32; 3]| -> Widget {
        let slivers = targets
            .into_iter()
            .enumerate()
            .map(|(i, target)| -> Box<dyn Sliver> {
                let ended = ended.clone();
                Box::new(
                    SliverAnimatedOpacity::new(
                        target,
                        Duration::from_millis(200),
                        SliverToBoxAdapter::new(Widget::box_(Size::new(50., 40.), Color::WHITE)),
                    )
                    .on_end(move || ended[i].set(ended[i].get() + 1)),
                )
            })
            .collect::<Vec<_>>();
        let slivers = match group {
            Group::Viewport => slivers,
            Group::MainAxis => vec![Box::new(SliverMainAxisGroup::new(slivers)) as Box<dyn Sliver>],
            Group::CrossAxis => {
                vec![Box::new(SliverCrossAxisGroup::new(slivers)) as Box<dyn Sliver>]
            }
        };
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
    };

    let bounds = Constraints::tight(Size::new(400., 400.));
    let now = Instant::now();
    let mut runtime = Runtime::new(view([1.; 3])).unwrap();
    runtime.run_frame_at(bounds, now).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, view([0.2, 0.4, 0.6]))
        .unwrap();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(20))
        .unwrap();

    let (midpoint, stats) = runtime
        .run_frame_at(bounds, now + Duration::from_millis(120))
        .unwrap();
    assert!(stats.requested_another_frame, "group {group:?}");
    close(&alphas(&midpoint), &[0.6, 0.7, 0.8]);
    assert!(ended.iter().all(|v| v.get() == 0));

    let (endpoint, _) = runtime
        .run_frame_at(bounds, now + Duration::from_millis(220))
        .unwrap();
    close(&alphas(&endpoint), &[0.2, 0.4, 0.6]);
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(236))
        .unwrap();
    assert!(
        ended.iter().all(|v| v.get() == 1),
        "group {group:?}: {ended:?}"
    );
}
