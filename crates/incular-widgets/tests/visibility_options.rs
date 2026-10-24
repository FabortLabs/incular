//! Flutter `Visibility.maintainInteractivity` and `Visibility.maintain`.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{GestureDetector, Visibility, Widget};

fn taps_through(visibility: impl Fn(Widget) -> Visibility) -> (usize, Size) {
    let taps = Rc::new(Cell::new(0));
    let target = GestureDetector::new(Widget::box_(Size::new(40., 40.), Color::WHITE)).on_tap({
        let taps = taps.clone();
        move || taps.set(taps.get() + 1)
    });
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Widget::from(visibility(target.into())))
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .expect("layout");
    let size = tree
        .render_size(tree.render_id(root).expect("root render"))
        .expect("laid out root");
    let base = Instant::now();
    for (phase, millis) in [(PointerPhase::Down, 0), (PointerPhase::Up, 20)] {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(20., 20.),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    (taps.get(), size)
}

#[test]
fn hidden_maintained_size_ignores_pointers_without_interactivity() {
    let (taps, size) = taps_through(|child| {
        Visibility::new(child)
            .visible(false)
            .maintain_state(true)
            .maintain_animation(true)
            .maintain_size(true)
    });
    assert_eq!(size.height, 40.);
    assert_eq!(taps, 0);
}

#[test]
fn maintain_interactivity_keeps_a_hidden_child_hittable() {
    let (taps, size) = taps_through(|child| {
        Visibility::new(child)
            .visible(false)
            .maintain_state(true)
            .maintain_animation(true)
            .maintain_size(true)
            .maintain_interactivity(true)
    });
    assert_eq!(size.height, 40.);
    assert_eq!(taps, 1);
}

#[test]
fn maintain_constructor_keeps_everything() {
    let (taps, size) = taps_through(|child| Visibility::maintain(child).visible(false));
    assert_eq!(size.height, 40.);
    assert_eq!(taps, 1);
    assert_eq!(
        Visibility::maintain(Widget::box_(Size::ZERO, Color::WHITE)),
        Visibility::builder()
            .child(Widget::box_(Size::ZERO, Color::WHITE))
            .maintain_state(true)
            .maintain_animation(true)
            .maintain_size(true)
            .maintain_semantics(true)
            .maintain_interactivity(true)
            .build()
    );
}
