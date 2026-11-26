//! Flutter parity regressions for `PageController` navigation (scroll-002),
//! `PageView.onPageChanged` (scroll-007), and `ListWheelScrollView` initial
//! selection and cross-axis centring (scroll-009, scroll-010).

use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_scroll::ScrollController;
use incular_widgets::{
    ChangeReportingBehavior, Directionality, ListView, ListWheelViewport, PageController, PageView,
    WheelChildDelegate, Widget, internal::WidgetTree,
};

fn numbered(count: usize) -> Vec<Widget> {
    (0..count)
        .map(|_| incular_widgets::SizedBox::new().width(300.).into())
        .collect()
}

/// Flutter `PageController.initialPage`: the first layout places the position
/// on the initial page, before any `jumpToPage`.
#[test]
fn initial_page_places_the_position_on_first_publication() {
    let page = PageController::new().with_initial_page(3);
    page.update_extents(4000., 800.)
        .expect("free controller publishes");

    assert_eq!(page.offset(), 2400.);
    assert_eq!(page.page(), Some(3.));
}

/// Flutter `jumpToPage` with `viewportFraction`: page n is `n * viewport *
/// fraction` pixels in, and `page` reads it back.
#[test]
fn jump_to_page_uses_viewport_fraction() {
    let page = PageController::new().with_viewport_fraction(0.5);
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(numbered(10)).controller(page.clone()),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();

    assert!(page.jump_to_page(2));
    assert_eq!(page.offset(), 800.);
    assert_eq!(page.page(), Some(2.));
}

#[test]
fn an_explicit_initial_queue_works_without_pretending_a_view_is_attached() {
    let page = PageController::new();
    assert_eq!(page.page(), None);
    assert!(!page.jump_to_page(2));
    assert!(page.queue_initial_page(1));

    page.update_extents(3000., 600.)
        .expect("free controller publishes");
    assert_eq!(page.offset(), 600.);
}

#[test]
fn changing_viewport_kind_cannot_send_page_commands_into_an_unrelated_list() {
    let scroll = ScrollController::new();
    let page = PageController::from(scroll.clone());
    let mut tree = WidgetTree::new();
    let list = || {
        let items = (0..20).map(|_| incular_widgets::SizedBox::new().height(100.));
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            ListView::new(items).controller(scroll.clone()),
        ))
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let root = tree.mount(list()).unwrap();
    tree.layout(bounds).unwrap();

    assert!(scroll.has_clients());
    assert!(!page.jump_to_page(2));
    assert!(!page.animate_to_page(
        2,
        std::time::Duration::from_millis(300),
        incular_animation::Curve::Ease
    ));
    assert_eq!(scroll.offset(), 0.);

    tree.update(
        root,
        Directionality::new(
            TextDirection::Ltr,
            PageView::new(numbered(10)).controller(page.clone()),
        )
        .into(),
    )
    .unwrap();
    tree.layout(bounds).unwrap();
    assert!(page.jump_to_page(1));
    assert_eq!(page.page(), Some(1.));

    tree.update(root, list()).unwrap();
    tree.layout(bounds).unwrap();
    let list_offset = scroll.offset();
    assert!(!page.jump_to_page(0));
    assert_eq!(scroll.offset(), list_offset);
}

/// Flutter `page` is `null` until the position has a viewport.
#[test]
fn page_is_none_before_layout() {
    assert_eq!(PageController::new().page(), None);
}

/// `on_page_changed` reports each new rounded page once, starting from the
/// initial page so mounting never reports a change (Flutter `_lastReportedPage`).
#[test]
fn page_change_is_reported_once_per_rounded_page() {
    let page = PageController::new().with_initial_page(0);
    page.update_extents(3200., 800.)
        .expect("free controller publishes");

    assert_eq!(page.take_page_change(0., 3200., 800., 1.), None);
    assert_eq!(page.take_page_change(800., 3200., 800., 1.), Some(1));
    assert_eq!(page.take_page_change(820., 3200., 800., 1.), None);
    assert_eq!(page.take_page_change(1700., 3200., 800., 1.), Some(2));
}

/// Flutter `ListWheelViewport` seeds its last reported item with the initial
/// item, so the item the wheel starts on is not a selection change at mount.
#[test]
fn list_wheel_does_not_report_its_initial_item_at_mount() {
    let controller = ScrollController::new();
    let reports = Rc::new(Cell::new(0_u32));
    let mut viewport = ListWheelViewport::new(
        controller.clone(),
        50.0,
        WheelChildDelegate::children(numbered(12)),
    );
    let counter = reports.clone();
    viewport.set_selection_callback(
        ChangeReportingBehavior::OnScrollUpdate,
        Some(move |_index: usize| counter.set(counter.get() + 1)),
    );

    let layout = viewport
        .layout(Size::new(100.0, 200.0))
        .expect("free controller publishes");

    assert_eq!(layout.selected_index, Some(0));
    assert_eq!(reports.get(), 0);
}

/// Flutter centres wheel children on the cross axis when they are narrower
/// than the viewport (`ListWheelChildBoxDelegate` / `ListWheelViewport`).
#[test]
fn list_wheel_centres_narrow_children_on_the_cross_axis() {
    let controller = ScrollController::new();
    let mut viewport =
        ListWheelViewport::new(controller, 50.0, WheelChildDelegate::children(numbered(4)));

    let layout = viewport
        .layout_with_measure(Size::new(800.0, 200.0), |_, _| Size::new(300.0, 50.0))
        .expect("free controller publishes");

    let child = layout
        .children
        .first()
        .expect("the wheel materializes at least one child");
    assert_eq!(child.untransformed_rect.origin.x, 250.0);
}
