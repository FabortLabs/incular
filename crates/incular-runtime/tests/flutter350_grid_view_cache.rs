//! Flutter GridView must rebuild an evicted tile on returning to the cache.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GridView, SizedBox, SliverGridDelegate, TextDirection, Widget,
    internal::{Key, PointerDeviceKind},
};

fn fling(runtime: &mut Runtime, view: Constraints, time: &mut Instant, pointer: u64, dy: f32) {
    let start = Offset::new(400., 300.);
    let event = |runtime: &mut Runtime, phase, position, time| {
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
                position,
            },
            time,
        );
    };
    event(runtime, PointerPhase::Down, start, *time);
    // The Flutter harness uses 50 interpolated moves at distance / speed;
    // 300px / 5000px per second gives 1.2ms between samples.
    for index in 0..=50 {
        event(
            runtime,
            PointerPhase::Move,
            start + Offset::new(0., dy * index as f32 / 50.),
            *time,
        );
        *time += Duration::from_micros(1200);
        if (index + 1) % 14 == 0 {
            runtime.run_frame_at(view, *time).unwrap();
        }
    }
    event(
        runtime,
        PointerPhase::Up,
        start + Offset::new(0., dy),
        *time,
    );
    for _ in 0..300 {
        *time += Duration::from_millis(16);
        runtime.run_frame_at(view, *time).unwrap();
        if !runtime.frame_requested() {
            return;
        }
    }
    panic!("GridView fling must settle");
}

#[test]
fn grid_builder_runs_again_for_tile_four_after_flinging_away_and_back() {
    let seen = Rc::new(Cell::new(0));
    let grid = GridView::builder(1000, SliverGridDelegate::fixed_cross_axis_count(3), {
        let seen = seen.clone();
        move |index| {
            if index == 4 {
                seen.set(seen.get() + 1);
            }
            Widget::from(SizedBox::new().width(200.).height(200.)).with_key(Key::from(index as u64))
        }
    });
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, grid).into()).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    let mut time = Instant::now();
    runtime.run_frame_at(view, time).unwrap();
    let target = Key::from(4_u64);
    let first = runtime.tree().element_with_key(&target).unwrap();
    assert_eq!(seen.get(), 1);
    fling(&mut runtime, view, &mut time, 1, -300.);
    assert!(!runtime.tree().element_exists(first));
    assert!(runtime.tree().element_with_key(&target).is_none());
    assert_eq!(seen.get(), 1);
    fling(&mut runtime, view, &mut time, 2, 300.);
    assert_ne!(runtime.tree().element_with_key(&target).unwrap(), first);
    assert_eq!(seen.get(), 2);
}
