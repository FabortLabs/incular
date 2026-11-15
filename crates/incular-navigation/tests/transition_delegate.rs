//! Flutter's `TransitionDelegate` for declarative page updates
//! (`widgets/navigator.dart`, navigation-011): `DefaultTransitionDelegate`
//! pushes only a new top and pops only the removed old top, and a custom
//! delegate can change each record's decision.

use std::{cell::RefCell, rc::Rc};

use incular_core::{Color, Size};
use incular_navigation::*;
use incular_widgets::Widget;

fn page(name: &str) -> Page {
    Page::new(name, Widget::box_(Size::new(1., 1.), Color::WHITE))
        .key(PageKey::new(name).expect("valid key"))
}

/// A navigator whose decisions are recorded, as the route outlet does.
fn presented() -> Navigator {
    let navigator = Navigator::new();
    let _ = navigator.take_page_transitions();
    navigator
}

fn id(navigator: &Navigator, name: &str) -> RouteId {
    navigator
        .routes()
        .into_iter()
        .find(|route| route.name == name)
        .map(|route| route.id)
        .expect("route is live")
}

#[test]
fn decisions_are_not_kept_until_a_presenter_reads_them() {
    let navigator = Navigator::new();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    assert!(navigator.take_page_transitions().is_empty());
    // The first read starts recording.
    navigator.set_pages([page("a")]).expect("pages");
    assert!(!navigator.take_page_transitions().is_empty());
}

#[test]
fn only_the_new_top_pushes_and_the_rest_are_added() {
    let navigator = presented();
    navigator
        .set_pages([page("a"), page("b"), page("c")])
        .expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(
        decisions.entering(id(&navigator, "a")),
        Some(PageEntering::Add)
    );
    assert_eq!(
        decisions.entering(id(&navigator, "b")),
        Some(PageEntering::Add)
    );
    assert_eq!(
        decisions.entering(id(&navigator, "c")),
        Some(PageEntering::Push)
    );
    // Taking drains them.
    assert!(navigator.take_page_transitions().is_empty());
}

#[test]
fn kept_routes_have_no_entering_decision() {
    let navigator = presented();
    navigator.set_pages([page("a")]).expect("pages");
    let a = id(&navigator, "a");
    let _ = navigator.take_page_transitions();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(decisions.entering(a), None);
    assert_eq!(
        decisions.entering(id(&navigator, "b")),
        Some(PageEntering::Push)
    );
}

#[test]
fn the_removed_top_pops_and_routes_removed_below_complete() {
    let navigator = presented();
    navigator
        .set_pages([page("a"), page("b"), page("c")])
        .expect("pages");
    let (a, b, c) = (
        id(&navigator, "a"),
        id(&navigator, "b"),
        id(&navigator, "c"),
    );
    let _ = navigator.take_page_transitions();
    navigator.set_pages([page("a")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    // Both sat above `a`; only the last of that chain pops.
    assert_eq!(decisions.exiting(b), Some((PageExiting::Complete, Some(a))));
    assert_eq!(decisions.exiting(c), Some((PageExiting::Pop, Some(a))));
}

#[test]
fn replacing_the_top_pushes_the_new_page_and_completes_the_old() {
    let navigator = presented();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let (a, b) = (id(&navigator, "a"), id(&navigator, "b"));
    let _ = navigator.take_page_transitions();
    navigator.set_pages([page("a"), page("c")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    // `b` sat above `a`, not above the new top `c`.
    assert_eq!(decisions.exiting(b), Some((PageExiting::Complete, Some(a))));
    assert_eq!(
        decisions.entering(id(&navigator, "c")),
        Some(PageEntering::Push)
    );
}

#[test]
fn routes_removed_above_a_kept_route_below_the_top_complete() {
    let navigator = presented();
    navigator
        .set_pages([page("a"), page("b"), page("c")])
        .expect("pages");
    let (a, b) = (id(&navigator, "a"), id(&navigator, "b"));
    let _ = navigator.take_page_transitions();
    // `b` sat above `a`, which is no longer the top: it leaves at once and
    // the new top `d` pushes over the kept `c`.
    navigator
        .set_pages([page("a"), page("c"), page("d")])
        .expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(decisions.exiting(b), Some((PageExiting::Complete, Some(a))));
    assert_eq!(
        decisions.entering(id(&navigator, "d")),
        Some(PageEntering::Push)
    );
}

#[test]
fn removing_every_page_pops_the_old_top() {
    let navigator = presented();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let (a, b) = (id(&navigator, "a"), id(&navigator, "b"));
    let _ = navigator.take_page_transitions();
    navigator.set_pages(Vec::<Page>::new()).expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(decisions.exiting(a), Some((PageExiting::Complete, None)));
    assert_eq!(decisions.exiting(b), Some((PageExiting::Pop, None)));
}

#[test]
fn a_custom_delegate_sees_default_decisions_and_overrides_them() {
    let navigator = presented();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let (a, b) = (id(&navigator, "a"), id(&navigator, "b"));
    let _ = navigator.take_page_transitions();
    let seen = Rc::new(RefCell::new(Vec::new()));
    navigator.set_transition_delegate({
        let seen = seen.clone();
        move |transition: &mut PageTransition| {
            for record in transition.history() {
                seen.borrow_mut()
                    .push((record.name().to_owned(), record.entering()));
            }
            for record in transition.exiting() {
                seen.borrow_mut()
                    .push((record.name().to_owned(), None::<PageEntering>));
                assert_eq!(record.exiting(), PageExiting::Complete);
            }
            // Add every new page without animation, and remove without one.
            for record in transition.history_mut() {
                record.mark_for_add();
            }
            for record in transition.exiting_mut() {
                record.mark_for_remove();
            }
        }
    });
    navigator.set_pages([page("a"), page("c")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    let c = id(&navigator, "c");
    assert_eq!(
        *seen.borrow(),
        [
            ("a".to_owned(), None),
            ("c".to_owned(), Some(PageEntering::Push)),
            ("b".to_owned(), None),
        ]
    );
    assert_eq!(decisions.entering(a), None);
    assert_eq!(decisions.entering(c), Some(PageEntering::Add));
    assert_eq!(decisions.exiting(b), Some((PageExiting::Remove, Some(a))));

    // The default delegate is restored.
    navigator.clear_transition_delegate();
    navigator.set_pages([page("a")]).expect("pages");
    assert_eq!(
        navigator.take_page_transitions().exiting(c),
        Some((PageExiting::Pop, Some(a)))
    );
}

#[test]
fn marking_a_kept_route_changes_nothing() {
    let navigator = presented();
    navigator.set_pages([page("a")]).expect("pages");
    let _ = navigator.take_page_transitions();
    navigator.set_transition_delegate(|transition: &mut PageTransition| {
        for record in transition.history_mut() {
            assert_eq!(record.is_entering(), record.name() == "b");
            assert_eq!(record.mark_for_push(), record.is_entering());
        }
    });
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(decisions.entering(id(&navigator, "a")), None);
}

#[test]
fn a_route_readded_before_the_presenter_reads_has_no_exit() {
    let navigator = presented();
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let _ = navigator.take_page_transitions();
    let b = id(&navigator, "b");
    navigator.set_pages([page("a")]).expect("pages");
    // A keyed page re-added mounts anew with a fresh identity; the old
    // route's exit stays pending.
    navigator.set_pages([page("a"), page("b")]).expect("pages");
    let decisions = navigator.take_page_transitions();
    assert_eq!(
        decisions.exiting(b).map(|(exiting, _)| exiting),
        Some(PageExiting::Pop)
    );
    assert_eq!(
        decisions.entering(id(&navigator, "b")),
        Some(PageEntering::Push)
    );
}
