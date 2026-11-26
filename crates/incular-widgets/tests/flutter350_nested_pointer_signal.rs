use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Listener, PointerSignalResolver, SizedBox, Widget};

fn tree(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree
}

#[test]
fn nested_pointer_signals_do_not_erase_or_steal_the_outer_claim() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let inside = Rc::new(RefCell::new(tree(
        Listener::new(SizedBox::expand())
            .on_pointer_signal({
                let events = events.clone();
                move |event| {
                    let events = events.clone();
                    PointerSignalResolver::register(&event, move |winner| {
                        events.borrow_mut().push(("inner", winner.scroll_delta.y));
                        // This call is outside a listener dispatch, even though
                        // an outer listener is still on the call stack.
                        let events = events.clone();
                        PointerSignalResolver::register(&winner, move |same| {
                            events
                                .borrow_mut()
                                .push(("standalone", same.scroll_delta.y));
                        });
                    });
                }
            })
            .into(),
    )));
    let outside = Listener::new(SizedBox::expand())
        .on_pointer_signal({
            let events = events.clone();
            move |event| {
                let events = events.clone();
                PointerSignalResolver::register(&event, move |winner| {
                    events.borrow_mut().push(("outer", winner.scroll_delta.y));
                });
                assert!(
                    inside
                        .borrow_mut()
                        .scroll_at(Offset::new(50., 50.), Offset::new(0., -12.))
                );
            }
        })
        .into();
    let mut outer = tree(outside);
    let mut allowed_unwanted_default = false;
    assert!(outer.scroll_at_with_platform_response(
        Offset::new(50., 50.),
        Offset::new(0., 40.),
        false,
        |_| allowed_unwanted_default = true,
    ));
    assert!(!allowed_unwanted_default);
    assert_eq!(
        *events.borrow(),
        [("inner", -12.), ("standalone", -12.), ("outer", 40.)]
    );
}

#[test]
fn a_nested_listener_panic_does_not_retire_the_outer_signal() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let outer_wins = Rc::new(std::cell::Cell::new(0));
    let nested = Rc::new(RefCell::new(tree(
        Listener::new(SizedBox::expand())
            .on_pointer_signal(|_| panic!("nested listener failed"))
            .into(),
    )));
    let mut outer = tree(
        Listener::new(SizedBox::expand())
            .on_pointer_signal({
                let outer_wins = outer_wins.clone();
                move |event| {
                    let outer_wins = outer_wins.clone();
                    PointerSignalResolver::register(&event, move |winner| {
                        assert_eq!(winner.scroll_delta.y, 40.);
                        outer_wins.set(outer_wins.get() + 1);
                    });
                    assert!(
                        catch_unwind(AssertUnwindSafe(|| {
                            nested
                                .borrow_mut()
                                .scroll_at(Offset::new(50., 50.), Offset::new(0., -12.))
                        }))
                        .is_err()
                    );
                }
            })
            .into(),
    );
    for _ in 0..2 {
        assert!(outer.scroll_at(Offset::new(50., 50.), Offset::new(0., 40.)));
    }
    assert_eq!(outer_wins.get(), 2);
}
