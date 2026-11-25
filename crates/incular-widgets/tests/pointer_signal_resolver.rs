//! `PointerSignalResolver`, Flutter's way for one handler to claim a wheel
//! event: the innermost registration wins, and a scroll view that would move
//! takes part in the same innermost-first order.

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Listener, PointerSignalResolver, RawPointerEvent, SingleChildScrollView, SizedBox, Widget,
};
use std::cell::Cell;
use std::rc::Rc;

fn claiming(child: Widget, claimed: &Rc<Cell<u32>>) -> Widget {
    let claimed = claimed.clone();
    Listener::new(child)
        .on_pointer_signal(move |event: RawPointerEvent| {
            let claimed = claimed.clone();
            PointerSignalResolver::register(&event, move |_| claimed.set(claimed.get() + 1));
        })
        .into()
}

fn scroll_view(controller: &ScrollController, content: Widget) -> Widget {
    let scrolled: Widget = SingleChildScrollView::new(content)
        .controller(controller.clone())
        .into();
    SizedBox::from_dimensions(Some(200.), Some(100.), Some(scrolled)).into()
}

fn mount(root: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 100.)))
        .expect("layout");
    tree
}

fn content() -> Widget {
    Widget::box_(Size::new(200., 1000.), Color::WHITE)
}

#[test]
fn a_listener_inside_a_scroll_view_claims_the_wheel() {
    let controller = ScrollController::new();
    let claimed = Rc::new(Cell::new(0));
    let mut tree = mount(scroll_view(&controller, claiming(content(), &claimed)));
    assert!(tree.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.)));
    assert_eq!(claimed.get(), 1);
    assert_eq!(controller.offset(), 0., "the scroll view lost the event");
}

#[test]
fn a_scroll_view_inside_a_listener_claims_the_wheel() {
    let controller = ScrollController::new();
    let claimed = Rc::new(Cell::new(0));
    let mut tree = mount(claiming(scroll_view(&controller, content()), &claimed));
    assert!(tree.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.)));
    assert_eq!(claimed.get(), 0, "the listener lost the event");
    assert_eq!(controller.offset(), 40.);
}

#[test]
fn a_scroll_view_at_its_end_leaves_the_wheel_to_the_listener() {
    let controller = ScrollController::new();
    let claimed = Rc::new(Cell::new(0));
    let mut tree = mount(claiming(scroll_view(&controller, content()), &claimed));
    // Upwards at offset zero the scroll view cannot move, so it does not
    // register and the listener outside it wins.
    assert!(tree.scroll_at(Offset::new(50., 50.), Offset::new(0., -40.)));
    assert_eq!(claimed.get(), 1);
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn only_the_innermost_registration_runs() {
    let controller = ScrollController::new();
    let outer = Rc::new(Cell::new(0));
    let inner = Rc::new(Cell::new(0));
    let mut tree = mount(claiming(
        scroll_view(&controller, claiming(content(), &inner)),
        &outer,
    ));
    tree.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.));
    assert_eq!((inner.get(), outer.get()), (1, 0));
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn register_outside_a_dispatch_runs_right_away() {
    let ran = Rc::new(Cell::new(false));
    let event = RawPointerEvent::scroll(
        0,
        0,
        incular_widgets::PointerDeviceKind::Mouse,
        Offset::ZERO,
        Offset::new(0., 1.),
        incular_core::time::Instant::now(),
    );
    let flag = ran.clone();
    PointerSignalResolver::register(&event, move |_| flag.set(true));
    assert!(ran.get());
}

#[test]
fn no_registration_is_a_noop_and_allows_platform_default() {
    let mut tree = mount(content());
    let mut responses = Vec::new();
    assert!(!tree.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.)));
    assert!(!tree.scroll_at_with_platform_response(
        Offset::new(50., 50.),
        Offset::new(0., 40.),
        false,
        |allowed| responses.push(allowed),
    ));
    assert_eq!(responses, [true]);
}

#[test]
fn observation_alone_does_not_suppress_platform_default() {
    let observed = Rc::new(Cell::new(0));
    let mut tree = mount(
        Listener::new(content())
            .on_pointer_signal({
                let observed = observed.clone();
                move |_| observed.set(observed.get() + 1)
            })
            .into(),
    );
    let mut responded = false;
    assert!(tree.scroll_at_with_platform_response(
        Offset::new(50., 50.),
        Offset::new(0., 40.),
        false,
        |allowed| {
            assert_eq!(observed.get(), 1);
            responded = allowed;
        },
    ));
    assert!(responded);
}

#[test]
fn first_registration_receives_its_event_once_and_resolver_can_be_reused() {
    let received = Rc::new(std::cell::RefCell::new(Vec::new()));
    let inner = Listener::new(content()).on_pointer_signal({
        let received = received.clone();
        move |event| {
            let received = received.clone();
            PointerSignalResolver::register(&event, move |winning| {
                assert_eq!(winning, event);
                received.borrow_mut().push(winning.scroll_delta);
            });
        }
    });
    let outer = Listener::new(inner).on_pointer_signal(|event| {
        PointerSignalResolver::register(&event, |_| panic!("second registration won"));
    });
    let mut tree = mount(outer.into());
    for dy in [40., -20.] {
        assert!(tree.scroll_at_with_platform_response(
            Offset::new(50., 50.),
            Offset::new(0., dy),
            false,
            |_| panic!("claimed event notified platform default"),
        ));
    }
    assert_eq!(
        *received.borrow(),
        [Offset::new(0., 40.), Offset::new(0., -20.)]
    );
}

#[test]
fn first_transformed_listener_keeps_its_own_local_coordinates() {
    let received = Rc::new(Cell::new(None));
    let inner = Listener::new(content()).on_pointer_signal({
        let received = received.clone();
        move |event| {
            let received = received.clone();
            PointerSignalResolver::register(&event, move |winning| {
                received.set(Some((winning.position, winning.local_position)));
            });
        }
    });
    let outer = Listener::new(incular_widgets::Transform::new(
        incular_core::Transform::translation(Offset::new(10., 20.)),
        inner,
    ))
    .on_pointer_signal(|event| {
        assert_eq!(event.local_position, Offset::new(50., 50.));
        PointerSignalResolver::register(&event, |_| panic!("outer transformed event won"));
    });
    let mut tree = mount(outer.into());
    assert!(tree.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.)));
    assert_eq!(
        received.get(),
        Some((Offset::new(50., 50.), Offset::new(40., 30.)))
    );
}
