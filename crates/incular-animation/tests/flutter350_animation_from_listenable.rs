use std::{cell::RefCell, rc::Rc};

use incular_animation::{AnimationStatus, DoubleAnimation};
use incular_core::{Offset, ValueNotifier};

#[test]
fn notifier_animation_delegates_values_and_is_forward_without_a_clock() {
    let source = ValueNotifier::new(0.);
    let animation = DoubleAnimation::from_value_listenable(source.clone());
    assert_eq!(animation.status(), AnimationStatus::Forward);
    assert!(animation.controllers().is_empty());
    assert_eq!(animation.value(), 0.);
    source.set_value(1.);
    assert_eq!(animation.value(), 1.);

    let values = Rc::new(RefCell::new(Vec::new()));
    let observer = animation.observe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
    });
    let _never = animation.observe_status(|_| panic!("notifier status must not change"));
    source.set_value(0.5);
    assert_eq!(values.take(), [0.5]);
    assert!(!source.set_value(0.5));
    assert!(values.borrow().is_empty());
    drop(observer);
    source.set_value(0.2);
    assert_eq!(animation.value(), 0.2);
    assert!(values.borrow().is_empty());
}

#[test]
fn notifier_animation_reads_through_an_optional_transformer() {
    let source = ValueNotifier::new(0.);
    let animation =
        DoubleAnimation::from_value_listenable_with(source.clone(), |value| value / 10.);
    assert_eq!(animation.status(), AnimationStatus::Forward);
    assert_eq!(animation.value(), 0.);
    source.set_value(10.);
    assert_eq!(animation.value(), 1.);
    assert_eq!(
        source.value(),
        10.,
        "the transformer does not replace the notifier value"
    );
}

#[test]
fn notifier_animation_drives_a_typed_callback_without_clamping_progress() {
    let source = ValueNotifier::new(0.);
    let offset = DoubleAnimation::from_value_listenable(source.clone())
        .drive(|value| Offset::new(0., value));
    assert_eq!(offset.status(), AnimationStatus::Forward);
    assert_eq!(offset.value(), Offset::ZERO);
    source.set_value(10.);
    assert_eq!(offset.value(), Offset::new(0., 10.));

    let values = Rc::new(RefCell::new(Vec::new()));
    let observer = offset.observe({
        let values = values.clone();
        move |value| values.borrow_mut().push(value)
    });
    source.set_value(0.5);
    assert_eq!(values.take(), [Offset::new(0., 0.5)]);
    drop(observer);
    source.set_value(0.2);
    assert_eq!(offset.value(), Offset::new(0., 0.2));
    assert!(values.borrow().is_empty());
}
