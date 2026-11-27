use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ScrollView, WidgetTree};
use incular_widgets::{
    Center, LayoutChangedNotification, NotificationListener, Placeholder, ScrollController,
    ScrollNotificationType, SizeChangedLayoutNotification, SizeChangedLayoutNotifier, SizedBox,
    Widget,
};

#[test]
fn size_change_notifies_the_current_base_listener_but_not_on_mount() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let received = events.clone();
    let scene = move |width: f32, first: bool| {
        let received = received.clone();
        Center::new(
            NotificationListener::new(SizeChangedLayoutNotifier::notifying(
                SizedBox::new().width(width).height(100.),
            ))
            .on(move |notice: &LayoutChangedNotification| {
                assert!(!first, "first size assignment does not send a notification");
                received.borrow_mut().push(*notice);
                true
            }),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(100., true)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    assert!(events.borrow().is_empty());
    tree.update(root, scene(200., false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(
        *events.borrow(),
        [LayoutChangedNotification::SizeChanged(
            SizeChangedLayoutNotification
        )]
    );
    tree.layout(constraints).unwrap();
    assert_eq!(
        events.borrow().len(),
        1,
        "identical relayout was not a size change"
    );
}

#[test]
fn zero_area_notifier_sizes_to_zero_without_sending_an_initial_event() {
    let calls = Rc::new(Cell::new(0));
    let event = calls.clone();
    let notifier = Widget::from(SizeChangedLayoutNotifier::notifying(Placeholder::new()))
        .with_key(incular_widgets::internal::Key::Value(10));
    let mut tree = WidgetTree::new();
    tree.mount(
        Center::new(NotificationListener::new(notifier).on(
            move |_: &LayoutChangedNotification| {
                event.set(event.get() + 1);
                false
            },
        ))
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let retained = tree
        .element_with_key(&incular_widgets::internal::Key::Value(10))
        .unwrap();
    assert_eq!(
        tree.render_size(tree.render_id(retained).unwrap()),
        Some(Size::ZERO)
    );
    let _ = tree.paint();
    assert_eq!(calls.get(), 0);
}

#[test]
fn base_layout_listener_catches_scroll_activity_but_not_metrics_and_can_stop_bubbling() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let controller = ScrollController::new();
    let viewport = ScrollView::vertical(
        controller.clone(),
        Widget::box_(Size::new(100., 600.), Color::WHITE),
    );
    let inner = NotificationListener::new(viewport).on({
        let seen = seen.clone();
        move |notice: &LayoutChangedNotification| {
            let LayoutChangedNotification::Scroll(scroll) = notice else {
                return false;
            };
            seen.borrow_mut().push(scroll.kind);
            scroll.kind == ScrollNotificationType::Update
        }
    });
    let outside = Rc::new(Cell::new(0));
    let event = outside.clone();
    let outer = NotificationListener::new(inner).on_notification(move |scroll| {
        if scroll.kind == ScrollNotificationType::Update {
            event.set(event.get() + 1);
        }
        false
    });
    let mut tree = WidgetTree::new();
    tree.mount(outer.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 200.)))
        .unwrap();
    assert!(!seen.borrow().contains(&ScrollNotificationType::Metrics));
    assert!(controller.jump_to(20.));
    assert!(seen.borrow().contains(&ScrollNotificationType::Update));
    assert_eq!(outside.get(), 0);
}
