//! Scroll bubbling registrations survive unchanged frames and follow retained
//! topology when sources, listeners, lazy children or motion exits change.

use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_scroll::{ScrollController, ScrollNotificationType};
use incular_widgets::internal::{
    CustomScrollView, ElementId, Key, NotificationListener, ScrollView, Sliver,
    SliverFixedExtentList, WidgetTree,
};
use incular_widgets::motion::{MotionExt, MotionValue, Transition, ms};
use incular_widgets::{Column, GlobalKey, SizedBox, Widget};

type Events = Rc<RefCell<Vec<(&'static str, usize)>>>;

fn constraints() -> Constraints {
    Constraints::tight(Size::new(100., 200.))
}

fn scroll(controller: &ScrollController) -> Widget {
    ScrollView::vertical(
        controller.clone(),
        Widget::box_(Size::new(100., 600.), Color::WHITE),
    )
}

fn bounded(child: impl Into<Widget>) -> Widget {
    SizedBox::from_size(Size::new(100., 100.))
        .child(child)
        .into()
}

fn listen(child: Widget, label: &'static str, events: &Events) -> Widget {
    let events = events.clone();
    NotificationListener::new(child)
        .on_notification(move |notification| {
            if notification.kind == ScrollNotificationType::Update {
                events.borrow_mut().push((label, notification.depth));
            }
            false
        })
        .into()
}

fn mounted(widget: Widget) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(constraints()).expect("initial layout");
    (tree, root)
}

fn frame(tree: &mut WidgetTree, now: Instant) {
    tree.tick_animations(now);
    tree.layout(constraints()).expect("layout");
    tree.update_compositor(now).expect("compositor");
    let _ = tree.paint();
}

#[test]
fn cached_layout_keeps_existing_registration_order() {
    let controller = ScrollController::new();
    let events = Events::default();
    let (mut tree, _) = mounted(listen(scroll(&controller), "widget", &events));
    let direct_events = events.clone();
    let _direct = controller.add_listener(move |notification| {
        if notification.kind == ScrollNotificationType::Update {
            direct_events
                .borrow_mut()
                .push(("direct", notification.depth));
        }
        false
    });
    let before = tree.diagnostics();

    for _ in 0..4 {
        tree.layout(constraints()).expect("cached layout");
    }
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert!(controller.jump_to(20.));
    assert_eq!(*events.borrow(), [("widget", 0), ("direct", 0)]);
}

#[test]
fn compositor_motion_keeps_registration_order_without_rebuilding_content() {
    let controller = ScrollController::new();
    let events = Events::default();
    let x = MotionValue::new(0.);
    let bound = x.clone();
    let moving = scroll(&controller).motion(move |m| m.rest(|p| p.x(&bound)));
    let (mut tree, _) = mounted(listen(moving, "widget", &events));
    let mut now = Instant::now();
    frame(&mut tree, now);
    let direct_events = events.clone();
    let _direct = controller.add_listener(move |notification| {
        if notification.kind == ScrollNotificationType::Update {
            direct_events
                .borrow_mut()
                .push(("direct", notification.depth));
        }
        false
    });
    let before = tree.diagnostics();

    for step in 1..=4 {
        x.set(step as f32);
        now += ms(16);
        frame(&mut tree, now);
    }
    let after = tree.diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(after.layouts, before.layouts);
    assert!(controller.jump_to(20.));
    assert_eq!(*events.borrow(), [("widget", 0), ("direct", 0)]);
}

#[test]
fn callback_replacement_removal_and_readdition_refresh_registrations() {
    let controller = ScrollController::new();
    let events = Events::default();
    let source = scroll(&controller);
    let (mut tree, root) = mounted(listen(source.clone(), "old", &events));

    tree.update(root, listen(source.clone(), "new", &events))
        .expect("replace callback");
    assert!(controller.jump_to(20.));
    assert_eq!(*events.borrow(), [("new", 0)]);

    tree.update(root, NotificationListener::new(source.clone()).into())
        .expect("remove callback");
    tree.layout(constraints()).expect("layout without callback");
    events.borrow_mut().clear();
    assert!(controller.jump_to(40.));
    assert!(events.borrow().is_empty());

    tree.update(root, listen(source, "returned", &events))
        .expect("readd callback");
    tree.layout(constraints()).expect("layout with callback");
    assert!(controller.jump_to(60.));
    assert_eq!(*events.borrow(), [("returned", 0)]);
}

#[test]
fn callback_drop_unwind_keeps_remaining_subscribers_for_a_successful_retry() {
    struct PanicOnce(Rc<Cell<bool>>);

    impl Drop for PanicOnce {
        fn drop(&mut self) {
            assert!(
                !self.0.replace(false),
                "callback capture dropped while armed"
            );
        }
    }

    let first = ScrollController::new();
    let second = ScrollController::new();
    let first_source = bounded(scroll(&first));
    let second_source = bounded(scroll(&second));
    let events = Events::default();
    let armed = Rc::new(Cell::new(false));
    let capture = PanicOnce(armed.clone());
    let first_events = events.clone();
    let original: Widget = NotificationListener::new(first_source.clone())
        .on_notification(move |notification| {
            let _retained_capture = &capture;
            if notification.kind == ScrollNotificationType::Update {
                first_events
                    .borrow_mut()
                    .push(("old first", notification.depth));
            }
            false
        })
        .into();
    let (mut tree, root) = mounted(
        Column::new([
            original,
            listen(second_source.clone(), "removed second", &events),
        ])
        .into(),
    );
    assert!(first.jump_to(10.));
    assert!(second.jump_to(10.));
    assert_eq!(*events.borrow(), [("old first", 0), ("removed second", 0)]);
    events.borrow_mut().clear();

    armed.set(true);
    let replacement: Widget = Column::new([
        listen(first_source, "replacement", &events),
        NotificationListener::new(second_source).into(),
    ])
    .into();
    let result = catch_unwind(AssertUnwindSafe(|| tree.update(root, replacement)));
    assert!(result.is_err(), "the first old callback destructor unwound");
    assert!(!armed.get(), "the destructor panics only once");

    tree.layout(constraints())
        .expect("retry subscription refresh");
    assert!(first.jump_to(20.));
    assert!(second.jump_to(20.));
    assert_eq!(
        *events.borrow(),
        [("replacement", 0)],
        "retry installs the new listener and removes the unvisited old one"
    );
}

#[test]
fn replacing_a_controller_releases_the_old_source_subscription() {
    let previous = ScrollController::new();
    let replacement = ScrollController::new();
    let events = Events::default();
    let (mut tree, root) = mounted(listen(scroll(&previous), "listener", &events));
    let source = tree.children(root).expect("listener child")[0];

    // Update only the source: the listener descriptor and ancestry stay put.
    tree.update(source, scroll(&replacement))
        .expect("replace source controller");
    tree.layout(constraints()).expect("replacement layout");
    events.borrow_mut().clear();
    assert!(previous.jump_to(20.));
    assert!(events.borrow().is_empty(), "old source was detached");
    assert!(replacement.jump_to(30.));
    assert_eq!(*events.borrow(), [("listener", 0)]);
}

#[test]
fn nested_sources_keep_depth_and_innermost_stop_order() {
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    let events = Events::default();
    let stop = Rc::new(Cell::new(false));
    let observed_stop = stop.clone();
    let inner_events = events.clone();
    let inner_listener: Widget = NotificationListener::new(scroll(&inner))
        .on_notification(move |notification| {
            if notification.kind == ScrollNotificationType::Update {
                inner_events
                    .borrow_mut()
                    .push(("inner", notification.depth));
            }
            observed_stop.get()
        })
        .into();
    let outer_source = ScrollView::vertical(
        outer,
        Column::new([
            bounded(inner_listener),
            Widget::box_(Size::new(100., 600.), Color::WHITE),
        ]),
    );
    let (mut tree, _) = mounted(listen(outer_source, "outer", &events));

    tree.layout(constraints()).expect("cached nested layout");
    assert!(inner.jump_to(20.));
    assert_eq!(*events.borrow(), [("inner", 0), ("outer", 1)]);
    events.borrow_mut().clear();
    stop.set(true);
    tree.layout(constraints()).expect("another cached layout");
    assert!(inner.jump_to(40.));
    assert_eq!(*events.borrow(), [("inner", 0)]);
}

#[test]
fn globally_keyed_source_follows_moves_between_unchanged_listeners() {
    let controller = ScrollController::new();
    let events = Events::default();
    let key = GlobalKey::new();
    let child = bounded(scroll(&controller)).with_key(key);
    let left: Widget = Column::new([child.clone()]).into();
    let right: Widget = Column::new(Vec::<Widget>::new()).into();
    let (mut tree, _) = mounted(
        Column::new([
            listen(left.with_key("left"), "left", &events),
            listen(right.with_key("right"), "right", &events),
        ])
        .into(),
    );
    let original = tree
        .element_with_key(&Key::Global(key))
        .expect("global child");
    let left = tree
        .element_with_key(&Key::from("left"))
        .expect("left slot");
    let right = tree
        .element_with_key(&Key::from("right"))
        .expect("right slot");

    for (from, to, label, offset) in [(left, right, "right", 20.), (right, left, "left", 40.)] {
        let empty: Widget = Column::new(Vec::<Widget>::new()).into();
        let full: Widget = Column::new([child.clone()]).into();
        let (from_key, to_key) = if from == left {
            ("left", "right")
        } else {
            ("right", "left")
        };
        tree.update(from, empty.with_key(from_key))
            .expect("park source");
        tree.update(to, full.with_key(to_key))
            .expect("retake source");
        tree.layout(constraints()).expect("layout after move");
        assert_eq!(tree.element_with_key(&Key::Global(key)), Some(original));
        events.borrow_mut().clear();
        assert!(controller.jump_to(offset));
        assert_eq!(*events.borrow(), [(label, 0)]);
    }
}

#[test]
fn lazy_materialization_adds_sources_and_expiry_removes_them() {
    let outer = ScrollController::new();
    let nested = (0..100)
        .map(|_| ScrollController::new())
        .collect::<Vec<_>>();
    let item_controllers = nested.clone();
    let items = SliverFixedExtentList::new(100, 100., move |index| {
        bounded(scroll(&item_controllers[index])).with_key(Key::Value(index as u64))
    });
    let view: Widget = CustomScrollView::new([Box::new(items) as Box<dyn Sliver>])
        .controller(outer.clone())
        .cache_extent(0.)
        .into();
    let events = Events::default();
    let (mut tree, _) = mounted(listen(view, "listener", &events));

    assert!(nested[0].jump_to(10.));
    assert_eq!(*events.borrow(), [("listener", 1)]);
    assert!(outer.jump_to(5_000.));
    tree.layout(constraints()).expect("new lazy window");
    events.borrow_mut().clear();
    assert!(nested[0].jump_to(20.));
    assert!(
        events.borrow().is_empty(),
        "expired source lost its listener"
    );
    assert!(nested[50].jump_to(10.));
    assert_eq!(*events.borrow(), [("listener", 1)]);
}

#[test]
fn departing_source_stays_subscribed_until_exit_and_can_revive() {
    let controller = ScrollController::new();
    let events = Events::default();
    let child = bounded(scroll(&controller)).with_key("moving").motion(|m| {
        m.exit(|p| p.opacity(0.))
            .transition(Transition::linear(ms(160)))
    });
    let children = |present| {
        let items = if present {
            vec![child.clone()]
        } else {
            Vec::new()
        };
        Widget::from(Column::new(items)).with_key("items")
    };
    let (mut tree, _) = mounted(listen(children(true), "listener", &events));
    let mut now = Instant::now();
    frame(&mut tree, now);
    let owner = tree.element_with_key(&Key::from("items")).expect("items");
    let original = tree.children(owner).expect("child")[0];

    tree.update(owner, children(false))
        .expect("begin departure");
    now += ms(48);
    frame(&mut tree, now);
    assert!(tree.element_exists(original), "exit still owns its subtree");
    events.borrow_mut().clear();
    assert!(controller.jump_to(20.));
    assert_eq!(*events.borrow(), [("listener", 0)]);

    tree.update(owner, children(true))
        .expect("revive departure");
    for _ in 0..20 {
        now += ms(16);
        frame(&mut tree, now);
    }
    assert_eq!(tree.children(owner), Some(&[original][..]));
    events.borrow_mut().clear();
    assert!(controller.jump_to(40.));
    assert_eq!(*events.borrow(), [("listener", 0)]);

    tree.update(owner, children(false)).expect("depart again");
    for _ in 0..30 {
        now += Duration::from_millis(16);
        frame(&mut tree, now);
    }
    assert!(!tree.element_exists(original), "settled exit was unmounted");
    events.borrow_mut().clear();
    assert!(controller.jump_to(60.));
    assert!(
        events.borrow().is_empty(),
        "settled source was unsubscribed"
    );
}
