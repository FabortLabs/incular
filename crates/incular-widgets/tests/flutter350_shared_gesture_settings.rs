use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, DeviceGestureSettings, RuntimeEnvironment, TextDirection};
use incular_core::{Offset, PointerPhase, Size, time::Instant};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    Center, CustomScrollView, Directionality, DragDropContext, Draggable, GestureDetector,
    HitTestBehavior, Sliver, SliverFixedExtentList, Text, Widget,
};

#[derive(Clone, Copy)]
enum RowKind {
    Detector,
    Draggable,
}

type Events = Rc<RefCell<Vec<&'static str>>>;

fn drag(row: RowKind, settings: DeviceGestureSettings) -> Vec<&'static str> {
    let events = Events::default();
    let captured = events.clone();
    let drops = DragDropContext::new();
    let sliver = SliverFixedExtentList::new(32, 50., move |index| {
        let label = Widget::from(Text::new(format!("List Item {index}")))
            .with_key(Key::Value(index as u64));
        let row: Widget = match row {
            RowKind::Detector => GestureDetector::new(label)
                .behavior(HitTestBehavior::Opaque)
                .on_vertical_drag_down({
                    let captured = captured.clone();
                    move |_| captured.borrow_mut().push("down")
                })
                .on_vertical_drag_update({
                    let captured = captured.clone();
                    move |_| captured.borrow_mut().push("update")
                })
                .on_vertical_drag_end(|_| {})
                .into(),
            RowKind::Draggable => Draggable::new(drops.clone(), index, label)
                .feedback(|_| Text::new("Dragging").into())
                .on_start({
                    let captured = captured.clone();
                    move |_| captured.borrow_mut().push("start")
                })
                .on_drag_update({
                    let captured = captured.clone();
                    move |_| captured.borrow_mut().push("update")
                })
                .on_drag_end({
                    let captured = captured.clone();
                    move |_| captured.borrow_mut().push("end")
                })
                .into(),
        };
        Center::new(row)
    });
    let view = Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new([Box::new(sliver) as Box<dyn Sliver>]),
    );
    let mut tree = WidgetTree::new();
    tree.set_environment(RuntimeEnvironment {
        gesture_settings: settings,
        ..RuntimeEnvironment::default()
    });
    tree.mount(view.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    let second = tree.element_with_key(&Key::Value(1)).unwrap();
    let bounds = tree.element_bounds(second).unwrap();
    let start = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    assert_eq!(start.y, 75.);
    let base = Instant::now();
    for (phase, fraction, elapsed_us) in [
        (PointerPhase::Down, 0., 0),
        (PointerPhase::Move, 0., 0),
        (PointerPhase::Move, 1. / 3., 16_666),
        (PointerPhase::Move, 2. / 3., 33_333),
        (PointerPhase::Move, 1., 50_000),
        (PointerPhase::Up, 1., 50_000),
    ] {
        let position = start + Offset::new(0., 5. * fraction);
        let hit = if phase == PointerPhase::Down {
            tree.pointer_hit_test(position)
        } else {
            Default::default()
        };
        let event = RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            if phase == PointerPhase::Up { 0 } else { 1 },
            position,
            phase,
            base + Duration::from_micros(elapsed_us),
        );
        tree.dispatch_pointer_gesture_raw_with_hit(1, event, &hit);
    }
    events.take()
}

fn configured() -> DeviceGestureSettings {
    DeviceGestureSettings::from_physical(Some(4.), 3.)
}

#[test]
fn vertical_detector_in_scroll_view_obeys_the_same_view_touch_slop() {
    let events = drag(RowKind::Detector, configured());
    assert_eq!(events.first(), Some(&"down"));
    assert!(events[1..].contains(&"update"), "{events:?}");
    assert_eq!(
        drag(RowKind::Detector, DeviceGestureSettings::default()),
        ["down"]
    );
}

#[test]
fn draggable_in_scroll_view_obeys_the_same_view_touch_slop() {
    let events = drag(RowKind::Draggable, configured());
    assert_eq!(events.first(), Some(&"start"));
    assert!(events[1..].contains(&"update"), "{events:?}");
    assert_eq!(events.last(), Some(&"end"));
    assert!(drag(RowKind::Draggable, DeviceGestureSettings::default()).is_empty());
}
