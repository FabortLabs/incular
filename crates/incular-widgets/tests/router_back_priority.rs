//! Flutter's `BackButtonDispatcher` priority across nested routers
//! (`widgets/router.dart`, navigation-006): a `ChildBackButtonDispatcher`
//! receives back requests before its parent's own callbacks once it calls
//! `takePriority`, the newest child to take priority goes first, and a child
//! whose callbacks are all removed is forgotten.

use std::{cell::RefCell, rc::Rc};

use incular_widgets::{
    BackButtonDispatcher, BasicRouterDelegate, RootBackButtonDispatcher, Router, RouterConfig,
    SizedBox, Widget,
};

type Log = Rc<RefCell<Vec<&'static str>>>;

/// A callback that records `name` and answers `handled`.
fn recorder(log: &Log, name: &'static str, handled: bool) -> impl Fn() -> bool + 'static {
    let log = log.clone();
    move || {
        log.borrow_mut().push(name);
        handled
    }
}

#[test]
fn a_child_is_consulted_only_after_it_takes_priority() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let _root_callback = root.add_callback(recorder(&log, "root", true));
    let child = root.create_child();
    let _child_callback = child.add_callback(recorder(&log, "child", true));

    // Flutter: a parent's `_children` holds only children that deferred.
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["root"]);

    log.borrow_mut().clear();
    child.take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["child"]);
}

#[test]
fn an_unhandled_child_falls_back_to_the_parent_callbacks() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let _root_callback = root.add_callback(recorder(&log, "root", true));
    let child = root.create_child();
    let _child_callback = child.add_callback(recorder(&log, "child", false));
    child.take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["child", "root"]);
}

#[test]
fn the_newest_child_to_take_priority_goes_first() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let first = root.create_child();
    let second = root.create_child();
    let _first_callback = first.add_callback(recorder(&log, "first", false));
    let _second_callback = second.add_callback(recorder(&log, "second", false));
    first.take_priority();
    second.take_priority();
    assert!(!root.dispatch_back());
    assert_eq!(*log.borrow(), ["second", "first"]);

    // Taking priority again moves `first` back to the front.
    log.borrow_mut().clear();
    first.take_priority();
    assert!(!root.dispatch_back());
    assert_eq!(*log.borrow(), ["first", "second"]);
}

#[test]
fn a_parent_taking_priority_clears_its_children() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let _root_callback = root.add_callback(recorder(&log, "root", true));
    let child = root.create_child();
    let _child_callback = child.add_callback(recorder(&log, "child", true));
    child.take_priority();
    // Flutter `BackButtonDispatcher.takePriority` clears `_children`.
    root.dispatcher().take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["root"]);
}

#[test]
fn removing_the_last_child_callback_forgets_the_child() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let _root_callback = root.add_callback(recorder(&log, "root", true));
    let child = root.create_child();
    let child_callback = child.add_callback(recorder(&log, "child", true));
    child.take_priority();
    drop(child_callback);
    assert!(!child.dispatcher().has_callbacks());
    // A later callback does not restore priority until the child takes it.
    let _again = child.add_callback(recorder(&log, "child again", true));
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["root"]);
}

#[test]
fn grandchildren_reach_the_root_through_their_parent() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let child = root.create_child();
    let grandchild = child.create_child();
    let _child_callback = child.add_callback(recorder(&log, "child", false));
    let _grandchild_callback = grandchild.add_callback(recorder(&log, "grandchild", true));
    child.take_priority();
    grandchild.take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["grandchild"]);
}

#[test]
fn defer_to_ignores_a_foreign_child() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let _root_callback = root.add_callback(recorder(&log, "root", true));
    let other = BackButtonDispatcher::default();
    let foreign = other.create_child();
    let _foreign_callback = foreign.add_callback(recorder(&log, "foreign", true));
    root.dispatcher().defer_to(&foreign);
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["root"]);
}

/// A router that records its delegate's pops.
fn router(
    log: &Log,
    name: &'static str,
    handled: bool,
    dispatcher: impl Into<BackButtonDispatcher>,
) -> Widget {
    let delegate = BasicRouterDelegate::<String>::new(|| SizedBox::shrink().into());
    delegate.on_pop_route(recorder(log, name, handled));
    Router::from_config(RouterConfig::new(delegate).with_back_button_dispatcher(dispatcher))
        .into_widget()
}

#[test]
fn a_nested_router_with_priority_pops_before_the_root_router() {
    let log = Log::default();
    let root = RootBackButtonDispatcher::new();
    let child = root.create_child();
    let _outer = router(&log, "outer", true, root.clone());
    let _inner = router(&log, "inner", true, child.clone());

    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["outer"]);

    log.borrow_mut().clear();
    child.take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["inner"]);

    // The inner delegate declines: the outer router pops instead.
    log.borrow_mut().clear();
    let declining = root.create_child();
    let _declining = router(&log, "declining", false, declining.clone());
    declining.take_priority();
    assert!(root.dispatch_back());
    assert_eq!(*log.borrow(), ["declining", "inner"]);
}
