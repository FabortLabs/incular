//! `GestureDetector` force presses through the retained gesture arena
//! (gestures-008). The press carries the native sample, as the runtime
//! dispatches it, so its normalized pressure reaches Flutter's
//! `ForcePressGestureRecognizer` (`gestures/force_press.dart`: start 0.4,
//! peak 0.85, linear interpolation over `pressureMin` 0 and `pressureMax` 1).

mod common;

use common::*;
use incular_config::Constraints;
use incular_core::{
    NormalizedPressure, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, PointerSampleMetadata, Size,
};
use incular_gestures::{PointerDeviceKind, PointerEvent};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Log(RefCell<Vec<String>>);

impl Log {
    fn push(&self, entry: String) {
        self.0.borrow_mut().push(entry);
    }
    fn entries(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

fn mount(log: &Rc<Log>, with_tap: bool) -> WidgetTree {
    let mut detector = GestureDetector::new(Widget::box_(Size::new(100., 80.), Color::WHITE))
        .on_force_press_start({
            let log = log.clone();
            move |details| log.push(format!("start {:.2}", details.pressure))
        })
        .on_force_press_peak({
            let log = log.clone();
            move |details| log.push(format!("peak {:.2}", details.pressure))
        })
        .on_force_press_update({
            let log = log.clone();
            move |details| log.push(format!("update {:.2}", details.pressure))
        })
        .on_force_press_end({
            let log = log.clone();
            move |_| log.push("end".into())
        });
    if with_tap {
        detector = detector.on_tap({
            let log = log.clone();
            move || log.push("tap".into())
        });
    }
    let mut tree = WidgetTree::new();
    tree.mount(detector.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 80.)))
        .expect("layout");
    tree
}

/// A stylus contact through `pressures` (one event each: the press, then
/// moves), then its release. `None` reports no pressure sensing.
fn press(tree: &mut WidgetTree, kind: PointerDeviceKind, pressures: &[Option<f64>]) {
    let base = Instant::now();
    let position = Offset::new(20., 20.);
    let last = pressures.len();
    for (index, pressure) in pressures
        .iter()
        .copied()
        .chain(std::iter::once(None))
        .enumerate()
    {
        let phase = match index {
            0 => PointerPhase::Down,
            index if index == last => PointerPhase::Up,
            _ => PointerPhase::Move,
        };
        let sample = PointerSampleMetadata {
            pressure: pressure.and_then(NormalizedPressure::new),
            stylus: None,
        };
        let hit = tree.pointer_hit_test(position);
        let _ = tree.dispatch_pointer_gesture_with_hit_sample(
            0,
            0,
            kind,
            if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            sample,
            PointerEvent {
                pointer: 7,
                position,
                phase,
                time: base + Duration::from_millis(index as u64 * 16),
            },
            &hit,
        );
    }
}

#[test]
fn a_rising_pressure_starts_peaks_updates_and_ends_the_force_press() {
    let log = Rc::new(Log::default());
    let mut tree = mount(&log, false);
    press(
        &mut tree,
        PointerDeviceKind::Stylus,
        &[Some(0.2), Some(0.5), Some(0.9)],
    );
    // Flutter: start once past 0.4, peak once past 0.85, an update for each
    // event of the started press, and the end on release.
    assert_eq!(
        log.entries(),
        [
            "start 0.50",
            "update 0.50",
            "peak 0.90",
            "update 0.90",
            "end"
        ]
    );
}

#[test]
fn a_device_without_pressure_never_force_presses_and_still_taps() {
    let log = Rc::new(Log::default());
    let mut tree = mount(&log, true);
    press(&mut tree, PointerDeviceKind::Mouse, &[None]);
    assert_eq!(log.entries(), ["tap"]);
}

#[test]
fn a_started_force_press_wins_the_arena_over_the_tap() {
    let log = Rc::new(Log::default());
    let mut tree = mount(&log, true);
    press(
        &mut tree,
        PointerDeviceKind::Stylus,
        &[Some(0.1), Some(0.6)],
    );
    assert_eq!(log.entries(), ["start 0.60", "update 0.60", "end"]);
}

#[test]
fn a_light_press_lets_the_tap_win() {
    let log = Rc::new(Log::default());
    let mut tree = mount(&log, true);
    press(
        &mut tree,
        PointerDeviceKind::Stylus,
        &[Some(0.1), Some(0.2)],
    );
    // The force press stays below its start pressure, so the arena sweep on
    // release gives the press to the tap.
    assert_eq!(log.entries(), ["tap"]);
}
