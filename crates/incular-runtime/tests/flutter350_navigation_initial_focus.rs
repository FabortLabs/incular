//! Mounted route autofocus, focus restoration, and nested traversal.
use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_navigation::{Navigator, Page};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    Center, Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode,
    FocusTraversalGroup, FocusTraversalOrder, GestureDetector, SizedBox, Widget,
};

type Outlet = Rc<RefCell<RouteOutlet>>;
const SIZE: Size = Size {
    width: 320.,
    height: 340.,
};

fn build_outlet(
    navigator: &Navigator,
    build: impl Fn(&Outlet) -> Widget + 'static,
) -> (Runtime, Outlet) {
    let bounded = |child| SizedBox::from_dimensions(Some(SIZE.width), Some(SIZE.height), child);
    let mut runtime = Runtime::new(bounded(None).into()).unwrap();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        navigator,
        &runtime.spawner().scope(),
    )));
    let for_builder = outlet.clone();
    runtime
        .register_builder(runtime.tree().root().unwrap(), move || {
            bounded(Some(build(&for_builder))).into()
        })
        .unwrap();
    runtime.run_frame(Constraints::tight(SIZE)).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    (runtime, outlet)
}

fn settle(runtime: &mut Runtime, outlet: &Outlet) {
    for _ in 0..4 {
        RouteOutlet::present_frame(outlet, runtime, Constraints::tight(SIZE)).unwrap();
    }
}

fn button(node: &FocusNode, ordered: bool, on_press: impl Fn() + 'static) -> Widget {
    let hit_target =
        GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).on_tap(on_press);
    let focus = Focus::new(hit_target).node(node.clone()).autofocus(true);
    if ordered {
        FocusTraversalOrder::new(focus).order(0.).into()
    } else {
        focus.into()
    }
}

fn tap_center(runtime: &mut Runtime) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(SIZE.width / 2., SIZE.height / 2.),
        });
    }
}

fn assert_initial_focus_navigation(ordered: bool) {
    let navigator = Navigator::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    let next = navigator.clone();
    let return_to = navigator.clone();
    let next_button = second.clone();
    let home = button(&first, ordered, move || {
        let return_to = return_to.clone();
        next.push_page(Page::new(
            "/second",
            Center::new(button(&next_button, ordered, move || {
                return_to.pop();
            })),
        ));
    });
    let home_id = navigator.push_page(Page::new("/", Center::new(home)));
    let (mut runtime, outlet) = build_outlet(&navigator, move |outlet| {
        let group = FocusTraversalGroup::new(outlet.borrow().widget());
        if ordered {
            group.ordered_with_widget_order().into()
        } else {
            group.widget_order().into()
        }
    });

    settle(&mut runtime, &outlet);
    let home_scope = first.host_enclosing_scope().expect("home page focus scope");
    assert!(first.has_primary_focus());
    assert!(home_scope.has_focus());

    tap_center(&mut runtime);
    settle(&mut runtime, &outlet);
    assert_ne!(navigator.current().unwrap().id, home_id);
    assert!(!first.has_focus());
    assert!(second.has_primary_focus());

    tap_center(&mut runtime);
    settle(&mut runtime, &outlet);
    assert_eq!(navigator.current().unwrap().id, home_id);
    assert!(first.has_primary_focus());
    assert!(home_scope.has_focus());
    assert!(!second.has_focus());
}

#[test]
fn widget_order_routes_autofocus_new_page_and_restore_previous_page() {
    assert_initial_focus_navigation(false);
}

#[test]
fn ordered_routes_autofocus_new_page_and_restore_previous_page() {
    assert_initial_focus_navigation(true);
}

#[test]
fn sequential_focus_enters_and_leaves_a_nested_navigator_in_both_directions() {
    let navigator = Navigator::new();
    let [first, middle, last] = std::array::from_fn(|_| FocusNode::new());
    let outer = FocusScopeNode::new();
    let item = |node: &FocusNode| Focus::new(SizedBox::square(100.)).node(node.clone());
    navigator.push_page(Page::new("/nested", item(&middle)));
    let before = first.clone();
    let after = last.clone();
    let (mut runtime, outlet) = build_outlet(&navigator, move |outlet| {
        let nested =
            SizedBox::from_dimensions(Some(100.), Some(100.), Some(outlet.borrow().widget()));
        let children = [
            Widget::from(item(&before)),
            Widget::from(nested),
            Widget::from(item(&after)),
        ];
        Directionality::new(
            TextDirection::Ltr,
            FocusTraversalGroup::new(FocusScope::new(Column::new(children)).node(outer.clone())),
        )
        .into()
    });
    settle(&mut runtime, &outlet);

    let nodes = [first, middle, last];
    let focused = |index| {
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(
                node.has_primary_focus(),
                i == index,
                "expected focus on {index}; checking {i}"
            );
        }
    };
    nodes[0].request_focus();
    settle(&mut runtime, &outlet);
    focused(0);
    for (from, to) in [(0, 1), (1, 2), (2, 0)] {
        assert!(runtime.next_focus_from(&nodes[from]));
        settle(&mut runtime, &outlet);
        focused(to);
    }
    for (from, to) in [(0, 2), (2, 1), (1, 0)] {
        assert!(runtime.previous_focus_from(&nodes[from]));
        settle(&mut runtime, &outlet);
        focused(to);
    }
}
