use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

#[test]
fn unsupported_child_devices_do_not_join_or_delay_an_ancestor_arena() {
    let view = Size::new(100., 100.);
    let child = Rc::new(Cell::new(0));
    let parent = Rc::new(Cell::new(0));
    let detector = GestureDetector::new(Widget::box_(view, Color::WHITE))
        .supported_devices([PointerDeviceKind::Touch])
        .on_horizontal_drag_start({
            let child = child.clone();
            move |_| child.set(child.get() + 1)
        });
    let root = GestureDetector::new(detector)
        .supported_devices([PointerDeviceKind::Mouse])
        .on_horizontal_drag_start({
            let parent = parent.clone();
            move |_| parent.set(parent.get() + 1)
        });
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    let base = Instant::now();
    for (index, kind, expected) in [
        (0, PointerDeviceKind::Mouse, (0, 1)),
        (1, PointerDeviceKind::Touch, (1, 1)),
    ] {
        let mut send = |phase, millis| {
            let _ = tree.dispatch_pointer_gesture_in_window(
                0,
                1,
                kind,
                PointerEvent {
                    pointer: index + 1,
                    position: Offset::new(10., 10.),
                    phase,
                    time: base + Duration::from_millis(millis),
                },
            );
        };
        send(PointerPhase::Down, 20 * index);
        assert_eq!((child.get(), parent.get()), expected);
        send(PointerPhase::Up, 20 * index + 10);
    }
}
