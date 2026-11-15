//! `MediaQueryData.gestureSettings` reaches the recognizers of a
//! `GestureDetector` (Flutter `GestureDetector.build` passes
//! `MediaQuery.maybeGestureSettingsOf(context)` to every recognizer
//! factory): the nearest `MediaQuery` override wins over the window's
//! environment, and the default keeps `kTouchSlop` (gestures-013).

mod common;

use common::*;
use incular_config::{Constraints, DeviceGestureSettings, RuntimeEnvironment};
use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size};
use incular_gestures::{PointerDeviceKind, PointerEvent};
use incular_widgets::MediaQuery;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

/// A detector whose tap competes with a vertical drag, so the drag starts
/// only once the pointer moves beyond the hit slop.
fn detector(started: &Rc<Cell<bool>>) -> Widget {
    let started = started.clone();
    GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .on_tap(|| {})
        .on_vertical_drag_start(move |_| started.set(true))
        .into()
}

fn settings(touch_slop: f32) -> RuntimeEnvironment {
    RuntimeEnvironment {
        gesture_settings: DeviceGestureSettings::new(Some(touch_slop)),
        ..RuntimeEnvironment::default()
    }
}

/// Presses at (20, 20) with a touch and moves straight down by `distance`.
fn drag_down(tree: &mut WidgetTree, distance: f32) {
    let base = Instant::now();
    let start = Offset::new(20., 20.);
    let hit = tree.pointer_hit_test(start);
    for (millis, phase, position) in [
        (0, PointerPhase::Down, start),
        (16, PointerPhase::Move, start + Offset::new(0., distance)),
    ] {
        let event = PointerEvent {
            pointer: 7,
            position,
            phase,
            time: base + Duration::from_millis(millis),
        };
        let _ = tree.dispatch_pointer_gesture_with_hit_buttons(
            0,
            0,
            PointerDeviceKind::Touch,
            PRIMARY_POINTER_BUTTON,
            event,
            &hit,
        );
    }
}

fn mount(widget: Widget, window: Option<RuntimeEnvironment>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    if let Some(environment) = window {
        tree.set_environment(environment);
    }
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

#[test]
fn default_settings_keep_the_flutter_touch_slop() {
    let started = Rc::new(Cell::new(false));
    let mut tree = mount(detector(&started), None);
    // 10 is within kTouchSlop (18).
    drag_down(&mut tree, 10.);
    assert!(!started.get());
}

#[test]
fn a_media_query_touch_slop_reaches_the_detector() {
    let started = Rc::new(Cell::new(false));
    let widget = MediaQuery::new(settings(6.), detector(&started)).into();
    let mut tree = mount(widget, None);
    drag_down(&mut tree, 10.);
    assert!(started.get());
}

#[test]
fn the_window_environment_applies_without_a_media_query() {
    let started = Rc::new(Cell::new(false));
    let mut tree = mount(detector(&started), Some(settings(6.)));
    drag_down(&mut tree, 10.);
    assert!(started.get());
}

#[test]
fn the_nearest_media_query_overrides_the_window() {
    let started = Rc::new(Cell::new(false));
    // The window reports a small slop; the subtree restores the default.
    let widget = MediaQuery::new(RuntimeEnvironment::default(), detector(&started)).into();
    let mut tree = mount(widget, Some(settings(6.)));
    drag_down(&mut tree, 10.);
    assert!(!started.get());
}
