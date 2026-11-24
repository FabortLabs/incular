//! Manually dispatched notifications from Flutter 3.50.0 notification_test.
//! These do not use a layout notifier, scroll event, or framework emission.

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Builder, NotificationDispatcher, NotificationListener, SizedBox, Widget, internal::WidgetTree,
};
use std::{cell::RefCell, ptr, rc::Rc};

struct ManualNotification(u32);

#[derive(Debug, Eq, PartialEq)]
struct Delivery {
    listener: &'static str,
    notification: *const ManualNotification,
    payload: u32,
}

type Log = Rc<RefCell<Vec<Delivery>>>;

struct Scene {
    _tree: WidgetTree,
    source: NotificationDispatcher,
    deliveries: Log,
}

impl Scene {
    fn mount(inner_stops: bool) -> Self {
        let origin = Rc::new(RefCell::new(None));
        let descendant = Builder::new({
            let origin = origin.clone();
            move |context| {
                *origin.borrow_mut() = Some(NotificationDispatcher::of(context));
                SizedBox::shrink().into()
            }
        });
        let deliveries = Log::default();
        let listen = |child: Widget, listener: &'static str, stops: bool| -> Widget {
            let deliveries = deliveries.clone();
            NotificationListener::new(child)
                .on(move |event: &ManualNotification| {
                    deliveries.borrow_mut().push(Delivery {
                        listener,
                        notification: ptr::from_ref(event),
                        payload: event.0,
                    });
                    stops
                })
                .into()
        };
        let mut tree = WidgetTree::new();
        tree.mount(listen(
            listen(descendant.into(), "b", inner_stops),
            "a",
            true,
        ))
        .expect("mount nested listeners");
        tree.layout(Constraints::loose(Size::new(800., 600.)))
            .expect("build mounted descendant context");
        let source = origin
            .take()
            .expect("capture mounted descendant dispatcher");
        Self {
            _tree: tree,
            source,
            deliveries,
        }
    }

    fn assert_dispatch(&self, destinations: &[&'static str]) {
        assert!(self.deliveries.borrow().is_empty());
        let event = ManualNotification(17);
        assert!(
            self.source.dispatch(&event),
            "a listener consumed the event"
        );
        let expected: Vec<_> = destinations
            .iter()
            .map(|&listener| Delivery {
                listener,
                notification: ptr::from_ref(&event),
                payload: 17,
            })
            .collect();
        assert_eq!(*self.deliveries.borrow(), expected);
    }
}

/// notification_test.dart:15 "Notification basics - dispatch".
#[test]
fn notification_dispatch_visits_inner_then_outer_with_the_same_value() {
    Scene::mount(false).assert_dispatch(&["b", "a"]);
}

/// notification_test.dart:43 "Notification basics - cancel".
#[test]
fn notification_dispatch_stops_when_the_inner_listener_consumes_it() {
    Scene::mount(true).assert_dispatch(&["b"]);
}
