//! Flutter 3.50.0 paired gesture audit at ab2912035e.

use std::{
    cell::Cell,
    f32::consts::FRAC_PI_2,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_widgets::{GestureDetector, Transform, Widget, internal::WidgetTree};

fn horizontal_drag_starts(angle: f32, motion: Offset) -> u32 {
    let starts = Rc::new(Cell::new(0));
    let inner = GestureDetector::new(Widget::box_(Size::new(200.0, 200.0), Color::WHITE))
        .on_horizontal_drag_start({
            let starts = Rc::clone(&starts);
            move |_| starts.set(starts.get() + 1)
        });
    // The outer tap keeps the press from resolving by default to the only
    // recognizer and bypassing the drag threshold.
    let outer = GestureDetector::new(Transform::rotation(angle, inner)).on_tap(|| {});
    let mut tree = WidgetTree::new();
    tree.mount(outer.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200.0, 200.0)))
        .unwrap();

    let base = Instant::now();
    let start = Offset::new(50.0, 50.0);
    let end = start + motion;
    for (phase, position, millis) in [
        (PointerPhase::Down, start, 0),
        (PointerPhase::Move, end, 10),
    ] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position,
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    let result = starts.get();
    tree.dispatch_gesture(PointerEvent {
        pointer: 1,
        position: end,
        phase: PointerPhase::Cancel,
        time: base + Duration::from_millis(11),
    });
    result
}

// C09: Flutter monodrag projects into the local axis before testing physical slop.
#[test]
fn rotated_horizontal_drag_recognizes_global_vertical_movement() {
    assert_eq!(
        horizontal_drag_starts(0.0, Offset::new(20.0, 0.0)),
        1,
        "unrotated horizontal control",
    );
    assert_eq!(
        horizontal_drag_starts(FRAC_PI_2, Offset::new(0.0, 17.0)),
        0,
        "rotated movement below touch slop",
    );
    assert_eq!(
        horizontal_drag_starts(FRAC_PI_2, Offset::new(0.0, 20.0)),
        1,
        "rotated movement beyond local horizontal touch slop",
    );
}
