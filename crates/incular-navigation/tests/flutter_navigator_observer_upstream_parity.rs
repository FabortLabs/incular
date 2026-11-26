//! Imperative Navigator observer counterparts from Flutter 3.50.0.
//! Exercise the public route stack and subscription; route names are pushed
//! directly rather than resolved by a WidgetsApp or driven with taps.

use incular_navigation::{NavigationEvent, Navigator, NavigatorObserver, Route};
use incular_widgets::SizedBox;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, PartialEq, Eq)]
enum Record {
    Pushed(String),
    Popped(String),
    Removed(String, Option<String>),
    Replaced(String, String),
    Active(Option<String>, Option<String>),
}

type Log = Rc<RefCell<Vec<Record>>>;

fn observe(navigator: &Navigator) -> (Log, NavigatorObserver) {
    let log = Log::default();
    let token = navigator.observe({
        let log = log.clone();
        move |event| {
            let record = match event {
                NavigationEvent::Pushed { route } => Record::Pushed(route.name),
                NavigationEvent::Popped { route } => Record::Popped(route.name),
                NavigationEvent::Removed { route, previous } => {
                    Record::Removed(route.name, previous.map(|route| route.name))
                }
                NavigationEvent::Replaced { previous, route } => {
                    Record::Replaced(previous.name, route.name)
                }
                NavigationEvent::ActiveRouteChanged { previous, current } => Record::Active(
                    previous.map(|route| route.name),
                    current.map(|route| route.name),
                ),
            };
            log.borrow_mut().push(record);
        }
    });
    (log, token)
}

fn route(name: &str) -> Route {
    Route::new(name, SizedBox::shrink())
}

fn current(navigator: &Navigator) -> String {
    navigator.current().expect("current route").name
}

fn active(previous: Option<&str>, current: &str) -> Record {
    Record::Active(previous.map(str::to_owned), Some(current.to_owned()))
}

/// navigator_test.dart:779 "Push and pop should trigger the observers".
#[test]
fn push_and_pop_observer_reports_the_changed_and_previous_routes() {
    let navigator = Navigator::new();
    let (log, _observer) = observe(&navigator);
    navigator.push(route("/"));
    assert_eq!(current(&navigator), "/");
    assert_eq!(log.take(), [Record::Pushed("/".into()), active(None, "/")]);

    navigator.push(route("/A"));
    assert_eq!(current(&navigator), "/A");
    assert_eq!(
        log.take(),
        [Record::Pushed("/A".into()), active(Some("/"), "/A")]
    );

    assert_eq!(navigator.pop().expect("pop /A").name, "/A");
    assert_eq!(current(&navigator), "/");
    assert_eq!(
        log.take(),
        [Record::Popped("/A".into()), active(Some("/A"), "/")]
    );
}

/// navigator_test.dart:848 "Add and remove an observer should work".
#[test]
fn attaching_an_observer_does_not_replay_and_dropping_it_stops_delivery() {
    let navigator = Navigator::new();
    let (original, _observer1) = observe(&navigator);
    navigator.push(route("/"));
    assert_eq!(
        original.take(),
        [Record::Pushed("/".into()), active(None, "/")]
    );

    let (late, observer2) = observe(&navigator);
    assert!(
        late.borrow().is_empty(),
        "adding an observer does not replay the initial route"
    );
    navigator.push(route("/A"));
    assert_eq!(
        late.take(),
        [Record::Pushed("/A".into()), active(Some("/"), "/A")]
    );
    assert_eq!(
        original.take(),
        [Record::Pushed("/A".into()), active(Some("/"), "/A")]
    );

    drop(observer2);
    assert_eq!(navigator.pop().expect("pop /A").name, "/A");
    assert_eq!(
        original.take(),
        [Record::Popped("/A".into()), active(Some("/A"), "/")]
    );
    assert!(
        late.borrow().is_empty(),
        "removed observer no longer receives events"
    );
}

/// navigator_test.dart:1295 "pushReplacement correctly reports didReplace
/// to the observer". Preconstruct the same three-route history.
#[test]
fn replacement_after_pop_until_reports_two_pops_then_a_replace() {
    let navigator = Navigator::new();
    for name in ["/", "/A", "/A/B"] {
        navigator.push(route(name));
    }
    let (log, _observer) = observe(&navigator);
    assert_eq!(current(&navigator), "/A/B");

    assert!(navigator.pop_until(|candidate| candidate.name == "/"));
    assert_eq!(
        navigator.replace(route("/C")).expect("replace root").name,
        "/"
    );
    assert_eq!(current(&navigator), "/C");
    assert_eq!(
        log.take(),
        [
            Record::Popped("/A/B".into()),
            active(Some("/A/B"), "/A"),
            Record::Popped("/A".into()),
            active(Some("/A"), "/"),
            Record::Replaced("/".into(), "/C".into()),
            active(Some("/"), "/C"),
        ]
    );
}

/// navigator_test.dart:1677 "removeRoute": didRemove carries the route
/// below the removed route and is distinct from didPop.
#[test]
fn removing_routes_must_not_emit_pop_observations() {
    let navigator = Navigator::new();
    let root = navigator.push(route("/"));
    let a = navigator.push(route("/A"));
    let (log, _observer) = observe(&navigator);

    let popped_control = navigator.push(route("/B"));
    assert_eq!(
        navigator.pop().expect("normal pop control").id,
        popped_control
    );
    assert_eq!(
        log.take(),
        [
            Record::Pushed("/B".into()),
            active(Some("/A"), "/B"),
            Record::Popped("/B".into()),
            active(Some("/B"), "/A")
        ]
    );

    let b = navigator.push(route("/B"));
    assert_ne!(b, popped_control);
    log.take();
    assert_eq!(navigator.remove_route(b).expect("remove /B").id, b);
    assert_eq!(navigator.current().expect("/A remains").id, a);
    assert_eq!(navigator.remove_route(a).expect("remove /A").id, a);
    assert_eq!(navigator.current().expect("initial route remains").id, root);
    assert_eq!(navigator.routes().len(), 1);

    assert_eq!(
        log.take(),
        [
            Record::Removed("/B".into(), Some("/A".into())),
            active(Some("/B"), "/A"),
            Record::Removed("/A".into(), Some("/".into())),
            active(Some("/A"), "/"),
        ]
    );
}

#[test]
fn removing_a_middle_route_uses_its_neighbor_and_does_not_change_the_top() {
    let navigator = Navigator::new();
    let root = navigator.push(route("/"));
    let middle = navigator.push(route("/A"));
    let top = navigator.push(route("/B"));
    let (log, _observer) = observe(&navigator);
    assert_eq!(navigator.remove_route_below(top).unwrap().id, middle);
    assert_eq!(current(&navigator), "/B");
    assert_eq!(log.take(), [Record::Removed("/A".into(), Some("/".into()))]);
    assert_eq!(navigator.remove_route(root).unwrap().id, root);
    assert_eq!(current(&navigator), "/B");
    assert_eq!(log.take(), [Record::Removed("/".into(), None)]);
}

#[test]
fn a_batch_push_reports_removals_against_the_remaining_neighbor_before_top_change() {
    for keep_root in [true, false] {
        let navigator = Navigator::new();
        navigator.push(route("/"));
        navigator.push(route("/A"));
        navigator.push(route("/B"));
        let (log, _observer) = observe(&navigator);
        let new_id = navigator.push_and_remove_until(
            route("/C"),
            |route| keep_root && route.name == "/",
        );
        assert_eq!(navigator.current().unwrap().id, new_id);
        let previous = keep_root.then(|| String::from("/"));
        let mut expected = vec![
            Record::Pushed("/C".into()),
            Record::Removed("/B".into(), previous.clone()),
            Record::Removed("/A".into(), previous),
        ];
        if !keep_root {
            expected.push(Record::Removed("/".into(), None));
        }
        expected.push(active(Some("/B"), "/C"));
        assert_eq!(log.take(), expected);
    }
}
