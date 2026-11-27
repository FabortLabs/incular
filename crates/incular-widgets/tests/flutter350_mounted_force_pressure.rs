use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Color, NormalizedPressure, Offset, PRIMARY_POINTER_BUTTON, PointerContactMetadata,
    PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_gestures::{
    ArenaGestureRecognizer, ForcePressGestureRecognizer, GestureRecognizerFactoryWithHandlers,
    RawPointerEvent,
};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{GestureDetector, RawGestureDetector, Widget};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(100., 50.))
}

fn leaf() -> Widget {
    Widget::box_(Size::new(100., 50.), Color::WHITE)
}

fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(bounds()).unwrap();
    tree
}

fn deliver(tree: &mut WidgetTree, phase: PointerPhase, raw: Option<f32>, portable: Option<f64>) {
    let sample = PointerSampleMetadata::new(portable.and_then(NormalizedPressure::new), None)
        .with_contact(PointerContactMetadata {
            raw_pressure: raw,
            pressure_min: 0.,
            pressure_max: if raw.is_some() { 10. } else { 1. },
            ..PointerContactMetadata::DEFAULT
        });
    let event = RawPointerEvent::new(
        1,
        5,
        PointerDeviceKind::Stylus,
        if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        Offset::new(
            if phase == PointerPhase::Down {
                20.
            } else {
                21.
            },
            10.,
        ),
        phase,
        Instant::now(),
    )
    .with_button((phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON))
    .with_sample(sample);
    let hit = tree.pointer_hit_test(event.position);
    tree.dispatch_raw_pointer_with_hit(0, event, &hit);
    tree.dispatch_pointer_gesture_raw_with_hit(0, event, &hit);
    tree.settle_uncontested_raw_arena(0, event);
}

#[test]
fn a_mounted_custom_interpolator_receives_raw_calibration_or_portable_fallback() {
    for physical in [false, true] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = Rc::clone(&seen);
        let detector =
            RawGestureDetector::new(leaf()).gesture(GestureRecognizerFactoryWithHandlers::new(
                move || {
                    let captured = Rc::clone(&captured);
                    ArenaGestureRecognizer::new(
                        ForcePressGestureRecognizer::new()
                            .interpolation(move |min, max, pressure| {
                                captured.borrow_mut().push((min, max, pressure));
                                (pressure - min) / (max - min)
                            })
                            .on_start(|_| {}),
                    )
                },
                |_| {},
            ));
        let mut tree = mount(detector);
        deliver(
            &mut tree,
            PointerPhase::Down,
            physical.then_some(0.),
            Some(0.),
        );
        deliver(
            &mut tree,
            PointerPhase::Move,
            physical.then_some(4.),
            Some(0.8),
        );
        deliver(&mut tree, PointerPhase::Up, None, None);
        let last = seen.borrow().last().copied();
        assert_eq!(
            last,
            if physical {
                Some((0., 10., 4.))
            } else {
                Some((0., 1., 0.8))
            }
        );
    }
}

#[test]
fn default_detector_keeps_adjusted_force_and_accepts_calibrated_only_sources() {
    for (portable, expected) in [(Some(0.8), 0.8), (None, 0.6)] {
        let starts = Rc::new(RefCell::new(Vec::new()));
        let captured = Rc::clone(&starts);
        let detector = GestureDetector::new(leaf()).on_force_press_start(move |event| {
            captured.borrow_mut().push(event.pressure);
        });
        let mut tree = mount(detector);
        deliver(
            &mut tree,
            PointerPhase::Down,
            Some(0.),
            portable.map(|_| 0.),
        );
        deliver(&mut tree, PointerPhase::Move, Some(6.), portable);
        assert_eq!(*starts.borrow(), [expected]);
        deliver(&mut tree, PointerPhase::Up, None, None);
    }
}
