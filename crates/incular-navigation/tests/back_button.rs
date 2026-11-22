//! Navigators register themselves as back gesture observers, so the system
//! back pops their routes without app code. The newest navigator is asked
//! first; one that cannot pop lets back fall through to the ones before it.

use incular_core::{Color, Size};
use incular_navigation::*;
use incular_widgets::{Widget, dispatch_pop_route};

fn page() -> Widget {
    Widget::box_(Size::new(1., 1.), Color::WHITE)
}

#[test]
fn system_back_pops_until_the_first_route_remains() {
    let navigator = Navigator::new();
    navigator.push_page(Page::new("home", page()));
    navigator.push_page(Page::new("detail", page()));

    assert!(dispatch_pop_route());
    assert_eq!(navigator.routes().len(), 1);
    assert_eq!(navigator.current().unwrap().name, "home");

    // The root route has nothing to pop to, so the host leaves the app.
    assert!(!dispatch_pop_route());
    assert_eq!(navigator.routes().len(), 1);
}

#[test]
fn a_dropped_navigator_no_longer_takes_system_back() {
    let outer = Navigator::new();
    outer.push_page(Page::new("home", page()));
    outer.push_page(Page::new("detail", page()));

    let inner = Navigator::new();
    inner.push_page(Page::new("a", page()));
    inner.push_page(Page::new("b", page()));
    let clone = inner.clone();
    drop(inner);
    // The clone keeps the navigator, and its registration, alive.
    assert!(dispatch_pop_route());
    assert_eq!(clone.routes().len(), 1);
    drop(clone);

    // Dropped: back pops the outer navigator instead.
    assert!(dispatch_pop_route());
    assert_eq!(outer.routes().len(), 1);
    assert!(!dispatch_pop_route());
}

#[test]
fn nested_navigators_pop_the_inner_one_first_then_fall_through() {
    let outer = Navigator::new();
    outer.push_page(Page::new("home", page()));
    outer.push_page(Page::new("detail", page()));

    // Created later, so it is asked first; with one route it cannot pop.
    let inner = Navigator::new();
    inner.push_page(Page::new("tab", page()));

    assert!(dispatch_pop_route());
    assert_eq!(outer.routes().len(), 1);
    assert_eq!(inner.routes().len(), 1);
    assert!(!dispatch_pop_route());
}
