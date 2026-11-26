//! Flutter 3.50.0 basic_test.dart: fractional translation paint, hits, and semantics.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Rect, Size};
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::{Application, Runtime, Signal};
use incular_widgets::{
    Center, ColoredBox, Directionality, FractionalTranslation, GestureDetector, Listener,
    Semantics, SizedBox, Text, TextDirection, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn send_tap(runtime: &mut Runtime, position: Offset) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
}

fn hits_translated_child(fraction: f32) {
    let received = Rc::new(RefCell::new(Vec::new()));
    let child = SizedBox::from_size(Size::new(100., 100.))
        .child(ColoredBox::empty(Color::rgba(0, 0, 255, 255)));
    let listener = Listener::new(child).on_pointer_down({
        let received = received.clone();
        move |event| received.borrow_mut().push(event.local_position)
    });
    let child = FractionalTranslation::new(Offset::new(fraction, fraction), listener);
    let mut runtime = Runtime::new(Center::new(child).into()).unwrap();
    frame(&mut runtime);
    assert!(received.borrow().is_empty());
    let point = Offset::new(400. + 100. * fraction, 300. + 100. * fraction);
    send_tap(&mut runtime, point);
    assert_eq!(received.borrow().as_slice(), [Offset::new(50., 50.)]);
}

#[test]
fn reaches_a_child_entirely_inside_the_untranslated_bounds() {
    hits_translated_child(0.);
}

#[test]
fn reaches_a_child_partially_inside_the_untranslated_bounds() {
    hits_translated_child(0.5);
}

#[test]
fn reaches_a_child_completely_outside_the_untranslated_bounds() {
    hits_translated_child(1.);
}

#[test]
fn tap_moves_retained_semantic_bounds_at_device_scale_three() {
    let fraction = Signal::new(0.4);
    let mut runtime = Application::new({
        let fraction = fraction.clone();
        move |_| {
            let body = SizedBox::square(100.).child(Text::new("foo"));
            let tap = GestureDetector::new(body).on_tap({
                let fraction = fraction.clone();
                move || {
                    fraction.set(0.8);
                }
            });
            let offset = fraction.get();
            let translated = FractionalTranslation::new(Offset::new(offset, offset), tap);
            let semantic = Semantics::new(Widget::from(translated)).explicit_child_nodes(true);
            Directionality::new(TextDirection::Ltr, Center::new(semantic)).into()
        }
    })
    .unwrap()
    .into_runtime();
    runtime.handle_platform_event(PlatformEvent::Metrics(WindowMetrics::new(
        PhysicalSize::new(2400, 1800),
        3.,
    )));
    frame(&mut runtime);
    let label = |runtime: &Runtime| {
        runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(id, node)| {
                (node.label.as_deref() == Some("foo")).then_some((id, node.bounds))
            })
            .unwrap()
    };
    let (id, before) = label(&runtime);
    let physical = |bounds: Rect| Offset::new(bounds.origin.x * 3., bounds.origin.y * 3.);
    assert_eq!(physical(before), Offset::new(1170., 870.));
    assert_eq!(before.size, Size::new(100., 100.));
    send_tap(&mut runtime, Offset::new(400., 300.));
    frame(&mut runtime);
    let (after_id, after) = label(&runtime);
    assert_eq!(after_id, id);
    assert_eq!(after.size, before.size);
    assert_eq!(physical(after), Offset::new(1290., 990.));
}

#[test]
fn fractional_translation_does_not_crash_in_an_empty_view() {
    // The native API supplies a shrink child for Flutter's omitted child.
    let translation = FractionalTranslation::new(Offset::new(0.4, 0.4), SizedBox::shrink());
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(Widget::from(translation).with_key("fractional")),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    let tree = runtime.tree();
    let translated = tree.element_with_key(&"fractional".into()).unwrap();
    assert_eq!(tree.element_bounds(translated).unwrap().size, Size::ZERO);
}
