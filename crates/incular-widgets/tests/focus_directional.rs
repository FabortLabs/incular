//! Keyboard focus geometry on a mounted tree: Flutter's default arrow-key
//! directional focus, FocusScope reading-order traversal, and the page and
//! line scroll that `WidgetsApp` binds to PageUp/PageDown and Ctrl+arrows.

use incular_config::Constraints;
use incular_core::{Code, Color, Size};
use incular_widgets::internal::{ActionId, WidgetTree, action};
use incular_widgets::{
    Directionality, FocusScope, ListView, Positioned, Row, Stack, TextDirection, Widget,
};

fn mount_and_layout(root: Widget, constraints: Constraints) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, root).into())
        .expect("mount");
    tree.layout(constraints).expect("layout");
    tree
}

fn button(id: u64, width: f32, height: f32) -> Widget {
    action(Size::new(width, height), Color::WHITE, ActionId(id))
}

#[test]
fn arrow_keys_move_focus_to_the_neighbour_in_a_row() {
    let tree = mount_and_layout(
        Row::new(vec![
            button(1, 40., 30.),
            button(2, 40., 30.),
            button(3, 40., 30.),
        ])
        .into(),
        Constraints::loose(Size::new(200., 60.)),
    );
    let ids = tree.focusable_elements();
    assert_eq!(ids.len(), 3);
    assert_eq!(
        tree.directional_focus_target(Some(ids[0]), Code::ArrowRight),
        Some(ids[1])
    );
    assert_eq!(
        tree.directional_focus_target(Some(ids[1]), Code::ArrowLeft),
        Some(ids[0])
    );
    // Flutter's closed-loop edge behaviour wraps to the far side.
    assert_eq!(
        tree.directional_focus_target(Some(ids[2]), Code::ArrowRight),
        Some(ids[0])
    );
}

#[test]
fn directional_focus_ignores_paging_keys() {
    let tree = mount_and_layout(
        Row::new(vec![button(1, 40., 30.), button(2, 40., 30.)]).into(),
        Constraints::loose(Size::new(200., 60.)),
    );
    let ids = tree.focusable_elements();
    assert_eq!(
        tree.directional_focus_target(Some(ids[0]), Code::PageDown),
        None
    );
}

#[test]
fn focus_scope_traverses_in_reading_order_by_default() {
    // The first child in widget order sits lower on screen, so reading order
    // visits the second child (top 0) first.
    let scope = FocusScope::new(Stack::new(vec![
        Widget::from(Positioned::new(button(1, 20., 20.)).top(40.)),
        Widget::from(Positioned::new(button(2, 20., 20.)).top(0.)),
    ]));
    let tree = mount_and_layout(scope.into(), Constraints::loose(Size::new(100., 100.)));
    let ids = tree.focusable_elements();
    assert_eq!(ids.len(), 2);
    assert_eq!(
        tree.element_bounds(ids[0]).map(|rect| rect.origin.y),
        Some(0.)
    );
}

#[test]
fn focus_scope_widget_order_keeps_registration_order() {
    let scope = FocusScope::new(Stack::new(vec![
        Widget::from(Positioned::new(button(1, 20., 20.)).top(40.)),
        Widget::from(Positioned::new(button(2, 20., 20.)).top(0.)),
    ]))
    .widget_order();
    let tree = mount_and_layout(scope.into(), Constraints::loose(Size::new(100., 100.)));
    let ids = tree.focusable_elements();
    assert_eq!(
        tree.element_bounds(ids[0]).map(|rect| rect.origin.y),
        Some(40.)
    );
}

#[test]
fn page_down_scrolls_the_nearest_viewport_by_eighty_percent() {
    let controller = incular_scroll::ScrollController::new();
    let tree = mount_and_layout(
        ListView::builder(30, |index| button(index as u64 + 1, 100., 50.))
            .item_extent(50.)
            .controller(controller.clone())
            .into(),
        Constraints::loose(Size::new(100., 200.)),
    );
    let focused = tree.focusable_elements()[0];
    assert!(tree.scroll_nearest_viewport(Some(focused), Code::PageDown, true));
    let animation = controller
        .take_pending_animation()
        .expect("page scroll is animated");
    // Flutter `ScrollAction`: 0.8 * viewport, 100 ms easeInOut.
    assert!((animation.target - 0.8 * controller.viewport_extent()).abs() < 1e-3);
    assert_eq!(animation.duration, std::time::Duration::from_millis(100));
}

#[test]
fn ctrl_arrow_scrolls_by_one_line() {
    let controller = incular_scroll::ScrollController::new();
    let tree = mount_and_layout(
        ListView::builder(30, |index| button(index as u64 + 1, 100., 50.))
            .item_extent(50.)
            .controller(controller.clone())
            .into(),
        Constraints::loose(Size::new(100., 200.)),
    );
    let focused = tree.focusable_elements()[0];
    assert!(tree.scroll_nearest_viewport(Some(focused), Code::ArrowDown, false));
    let animation = controller
        .take_pending_animation()
        .expect("line scroll is animated");
    assert!((animation.target - 50.).abs() < 1e-3);
}

#[test]
fn paging_keys_without_a_scroll_viewport_are_not_consumed() {
    let tree = mount_and_layout(
        Row::new(vec![button(1, 40., 30.)]).into(),
        Constraints::loose(Size::new(200., 60.)),
    );
    let focused = tree.focusable_elements()[0];
    assert!(!tree.scroll_nearest_viewport(Some(focused), Code::PageDown, true));
    assert!(!tree.scroll_nearest_viewport(None, Code::PageDown, true));
}
