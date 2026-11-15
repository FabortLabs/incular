//! Secondary and tertiary button taps through the retained gesture arena
//! (gestures-007). The press is hit-tested and dispatched with its button
//! mask, as the runtime does for a mouse; Flutter routes the press to the
//! `onSecondaryTap` recognizer and never to `onTap`.

mod common;

use common::*;
use incular_config::Constraints;
use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON, Size};
use incular_gestures::{PointerDeviceKind, PointerEvent};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

struct Counts {
    primary: Rc<Cell<u32>>,
    secondary: Rc<Cell<u32>>,
    tertiary: Rc<Cell<u32>>,
}

fn mount() -> (WidgetTree, Counts) {
    let counts = Counts {
        primary: Rc::new(Cell::new(0)),
        secondary: Rc::new(Cell::new(0)),
        tertiary: Rc::new(Cell::new(0)),
    };
    let detector = GestureDetector::new(Widget::box_(Size::new(100., 80.), Color::WHITE))
        .on_tap({
            let primary = counts.primary.clone();
            move || primary.set(primary.get() + 1)
        })
        .on_secondary_tap({
            let secondary = counts.secondary.clone();
            move || secondary.set(secondary.get() + 1)
        })
        .on_tertiary_tap_up({
            let tertiary = counts.tertiary.clone();
            move |_| tertiary.set(tertiary.get() + 1)
        });
    let mut tree = WidgetTree::new();
    tree.mount(detector.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 80.)))
        .expect("layout");
    (tree, counts)
}

/// One press and release at `position`, with the button mask of the press.
fn click(tree: &mut WidgetTree, buttons: u32, position: Offset) {
    let base = Instant::now();
    for (millis, phase) in [(0, PointerPhase::Down), (50, PointerPhase::Up)] {
        let hit = tree.pointer_hit_test(position);
        let event = PointerEvent {
            pointer: 60,
            position,
            phase,
            time: base + Duration::from_millis(millis),
        };
        let _ = tree.dispatch_pointer_gesture_with_hit_buttons(
            0,
            0,
            PointerDeviceKind::Mouse,
            buttons,
            event,
            &hit,
        );
    }
}

#[test]
fn a_secondary_click_fires_only_the_secondary_tap() {
    let (mut tree, counts) = mount();
    click(&mut tree, SECONDARY_POINTER_BUTTON, Offset::new(20., 20.));
    assert_eq!(counts.secondary.get(), 1);
    assert_eq!(counts.primary.get(), 0);
    assert_eq!(counts.tertiary.get(), 0);
}

#[test]
fn a_primary_click_still_fires_the_primary_tap() {
    let (mut tree, counts) = mount();
    click(&mut tree, PRIMARY_POINTER_BUTTON, Offset::new(20., 20.));
    assert_eq!(counts.primary.get(), 1);
    assert_eq!(counts.secondary.get(), 0);
}

#[test]
fn a_tertiary_click_fires_the_tertiary_tap_up() {
    let (mut tree, counts) = mount();
    click(&mut tree, 1 << 2, Offset::new(20., 20.));
    assert_eq!(counts.tertiary.get(), 1);
    assert_eq!(counts.primary.get(), 0);
    assert_eq!(counts.secondary.get(), 0);
}
