//! Flutter `MouseTracker` callback order for nested `MouseRegion`s: enter
//! events run outer-to-inner (reverse hit-test order), exit events run
//! inner-to-outer (hit-test order).

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::{MouseRegion, Widget, internal::WidgetTree};

fn hover_at(position: Offset) -> RawPointerEvent {
    RawPointerEvent::new(
        0,
        1,
        PointerDeviceKind::Mouse,
        0,
        position,
        PointerPhase::Move,
        Instant::now(),
    )
}

#[test]
fn nested_regions_enter_outer_first_and_exit_inner_first() {
    let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let (inner_enter, inner_exit) = (log.clone(), log.clone());
    let (outer_enter, outer_exit) = (log.clone(), log.clone());
    let inner: Widget = MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .on_enter(move |_| inner_enter.borrow_mut().push("inner_enter"))
        .on_exit(move |_| inner_exit.borrow_mut().push("inner_exit"))
        .into();
    let root: Widget = MouseRegion::new(inner)
        .on_enter(move |_| outer_enter.borrow_mut().push("outer_enter"))
        .on_exit(move |_| outer_exit.borrow_mut().push("outer_exit"))
        .into();

    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");

    let _ = tree.dispatch_raw_pointer_in_window(7, hover_at(Offset::new(20., 20.)));
    assert_eq!(*log.borrow(), ["outer_enter", "inner_enter"]);

    let _ = tree.dispatch_raw_pointer_in_window(7, hover_at(Offset::new(150., 150.)));
    assert_eq!(
        *log.borrow(),
        ["outer_enter", "inner_enter", "inner_exit", "outer_exit"]
    );
}
