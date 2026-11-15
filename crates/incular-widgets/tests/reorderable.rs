//! Focused behavior checks for the Flutter reorderable-list parity surface.

use incular_config::{Axis, Constraints};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::{PointerEvent, PointerGestureRecognizer};
use incular_scroll::ScrollController;
use incular_widgets::internal::{
    ReorderableDelayedDragStartListener, ReorderableDragStartListener, ReorderableList, Widget,
    WidgetTree,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

fn dispatch(
    tree: &mut WidgetTree,
    pointer: u64,
    time: std::time::Instant,
    phase: PointerPhase,
    position: Offset,
) {
    let _ = tree.dispatch_gesture(PointerEvent {
        pointer,
        position,
        phase,
        time,
    });
}

/// Runs 16 ms frames (pre-layout animation tick, then layout) for 400 ms
/// after `from`: long enough for Flutter's 250 ms drop animation.
fn settle(tree: &mut WidgetTree, from: std::time::Instant, size: Size) {
    for frame in 1..=25 {
        let _ = tree.tick_animations(from + Duration::from_millis(16 * frame));
        tree.layout(Constraints::tight(size)).expect("layout");
    }
}

/// A 60 px row keyed by its index, for [`row_y`].
fn keyed_row(index: usize) -> Widget {
    Widget::box_(Size::new(100., 60.), Color::rgba(index as u8, 0, 0, 255))
        .with_key(incular_widgets::internal::Key::Value(1000 + index as u64))
}

/// The global y of the keyed row's top edge.
fn row_y(tree: &WidgetTree, index: usize) -> f32 {
    let id = tree
        .element_with_key(&incular_widgets::internal::Key::Value(1000 + index as u64))
        .expect("row is mounted");
    let (transform, _) = tree.element_geometry(id).expect("row is laid out");
    transform.transform_point(Offset::ZERO).y
}

fn row(index: usize) -> Widget {
    Widget::box_(Size::new(100., 40.), Color::rgba(index as u8, 0, 0, 255))
}

#[test]
fn reorder_controller_adjusts_legacy_and_final_destinations() {
    let controller = incular_widgets::internal::SliverReorderController::new(4);

    assert!(controller.reorder(1, 4));
    assert_eq!(controller.order(), vec![0, 2, 3, 1]);
    assert!(!controller.reorder(3, 3));

    assert!(controller.reorder(3, 1));
    assert_eq!(controller.order(), vec![0, 1, 2, 3]);
    assert!(!controller.reorder(4, 0));
}

#[test]
fn box_reorderable_list_uses_listener_index_and_flutter_callback_conventions() {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let reorders = Rc::new(RefCell::new(Vec::new()));
    let scroll = ScrollController::new();
    let list = ReorderableList::new(4, |index| {
        ReorderableDragStartListener::new(index, row(index))
    })
    .item_extent(40.)
    .scroll_direction(Axis::Vertical)
    .controller(scroll.clone())
    .on_reorder({
        let reorders = reorders.clone();
        move |old, new| reorders.borrow_mut().push((old, new))
    })
    .on_reorder_start({
        let starts = starts.clone();
        move |index| starts.borrow_mut().push(index)
    })
    .on_reorder_end({
        let ends = ends.clone();
        move |index| ends.borrow_mut().push(index)
    });
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 140.)))
        .expect("layout");

    assert_eq!(scroll.max_offset(), 20.);
    let now = std::time::Instant::now();
    dispatch(&mut tree, 1, now, PointerPhase::Down, Offset::new(10., 20.));
    dispatch(
        &mut tree,
        1,
        now + Duration::from_millis(1),
        PointerPhase::Move,
        Offset::new(10., 24.),
    );
    dispatch(
        &mut tree,
        1,
        now + Duration::from_millis(2),
        PointerPhase::Move,
        Offset::new(10., 130.),
    );
    dispatch(
        &mut tree,
        1,
        now + Duration::from_millis(3),
        PointerPhase::Up,
        Offset::new(10., 130.),
    );

    // Flutter reports onReorder after the 250 ms drop animation.
    assert!(reorders.borrow().is_empty());
    settle(
        &mut tree,
        now + Duration::from_millis(3),
        Size::new(100., 140.),
    );
    assert_eq!(order.order(), vec![1, 2, 3, 0]);
    assert_eq!(*starts.borrow(), vec![0]);
    assert_eq!(*reorders.borrow(), vec![(0, 4)]);
    assert_eq!(*ends.borrow(), vec![4]);
}

#[test]
fn adjusted_on_reorder_item_reports_post_removal_index_and_same_drop_ends() {
    let reorders = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let list = ReorderableList::new(3, |index| {
        ReorderableDragStartListener::new(index, row(index))
    })
    .item_extent(40.)
    .on_reorder_item({
        let reorders = reorders.clone();
        move |old, new| reorders.borrow_mut().push((old, new))
    })
    .on_reorder_end({
        let ends = ends.clone();
        move |index| ends.borrow_mut().push(index)
    });
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 140.)))
        .expect("layout");

    let now = std::time::Instant::now();
    dispatch(&mut tree, 2, now, PointerPhase::Down, Offset::new(10., 60.));
    // Flutter 3.41.7 widgets/reorderable_list.dart ReorderableDragStartListener
    // uses ImmediateMultiDragGestureRecognizer, which starts at pointer down.
    // Incular's listener drags through the pan recognizer, which accepts only
    // after kPanSlop = 36 (touch slops, no device kind; gestures/constants.dart).
    // The drag therefore moves 40px out and back, so the drop lands on the
    // starting slot exactly as in the original scenario.
    dispatch(
        &mut tree,
        2,
        now + Duration::from_millis(1),
        PointerPhase::Move,
        Offset::new(10., 100.),
    );
    dispatch(
        &mut tree,
        2,
        now + Duration::from_millis(2),
        PointerPhase::Move,
        Offset::new(10., 60.),
    );
    dispatch(
        &mut tree,
        2,
        now + Duration::from_millis(3),
        PointerPhase::Up,
        Offset::new(10., 60.),
    );

    assert_eq!(order.order(), vec![0, 1, 2]);
    assert!(reorders.borrow().is_empty());
    assert_eq!(*ends.borrow(), vec![1]);
}

#[test]
fn delayed_listener_rejects_early_motion_and_starts_after_long_press() {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let reorders = Rc::new(RefCell::new(Vec::new()));
    let list = ReorderableList::new(4, |index| {
        ReorderableDelayedDragStartListener::new(index, row(index))
    })
    .item_extent(40.)
    .on_reorder_item({
        let reorders = reorders.clone();
        move |old, new| reorders.borrow_mut().push((old, new))
    })
    .on_reorder_start({
        let starts = starts.clone();
        move |index| starts.borrow_mut().push(index)
    })
    .on_reorder_end({
        let ends = ends.clone();
        move |index| ends.borrow_mut().push(index)
    });
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 140.)))
        .expect("layout");

    let now = std::time::Instant::now();
    dispatch(&mut tree, 3, now, PointerPhase::Down, Offset::new(10., 20.));
    dispatch(
        &mut tree,
        3,
        now + Duration::from_millis(100),
        PointerPhase::Move,
        Offset::new(10., 100.),
    );
    dispatch(
        &mut tree,
        3,
        now + Duration::from_millis(101),
        PointerPhase::Up,
        Offset::new(10., 100.),
    );
    assert_eq!(order.order(), vec![0, 1, 2, 3]);
    assert!(starts.borrow().is_empty());

    dispatch(
        &mut tree,
        4,
        now + Duration::from_secs(1),
        PointerPhase::Down,
        Offset::new(10., 20.),
    );
    dispatch(
        &mut tree,
        4,
        now + Duration::from_secs(1) + PointerGestureRecognizer::LONG_PRESS_TIMEOUT,
        PointerPhase::Move,
        Offset::new(10., 20.),
    );
    dispatch(
        &mut tree,
        4,
        now + Duration::from_secs(1)
            + PointerGestureRecognizer::LONG_PRESS_TIMEOUT
            + Duration::from_millis(1),
        PointerPhase::Move,
        Offset::new(10., 130.),
    );
    dispatch(
        &mut tree,
        4,
        now + Duration::from_secs(1)
            + PointerGestureRecognizer::LONG_PRESS_TIMEOUT
            + Duration::from_millis(2),
        PointerPhase::Up,
        Offset::new(10., 130.),
    );

    settle(
        &mut tree,
        now + Duration::from_secs(1) + PointerGestureRecognizer::LONG_PRESS_TIMEOUT,
        Size::new(100., 140.),
    );
    assert_eq!(order.order(), vec![1, 2, 3, 0]);
    assert_eq!(*starts.borrow(), vec![0]);
    assert_eq!(*reorders.borrow(), vec![(0, 3)]);
    assert_eq!(*ends.borrow(), vec![4]);
}

#[test]
fn box_reorderable_list_requires_an_enabled_listener_for_a_drag() {
    let order = {
        let list = ReorderableList::new(2, row)
            .item_extent(40.)
            .with_reorder_controller(incular_widgets::internal::SliverReorderController::new(2));
        let order = list.reorder_controller();
        let mut tree = WidgetTree::new();
        tree.mount(list.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 80.)))
            .expect("layout");
        let now = std::time::Instant::now();
        dispatch(&mut tree, 5, now, PointerPhase::Down, Offset::new(10., 20.));
        dispatch(
            &mut tree,
            5,
            now + Duration::from_millis(1),
            PointerPhase::Move,
            Offset::new(10., 65.),
        );
        dispatch(
            &mut tree,
            5,
            now + Duration::from_millis(2),
            PointerPhase::Up,
            Offset::new(10., 65.),
        );
        order.order()
    };
    assert_eq!(order, vec![0, 1]);

    let disabled = ReorderableDragStartListener::new(0, row(0)).enabled(false);
    let list = ReorderableList::new(2, move |index| {
        if index == 0 {
            disabled.clone()
        } else {
            ReorderableDragStartListener::new(index, row(index))
        }
    })
    .item_extent(40.);
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 80.)))
        .expect("layout");
    let now = std::time::Instant::now();
    dispatch(&mut tree, 6, now, PointerPhase::Down, Offset::new(10., 20.));
    dispatch(
        &mut tree,
        6,
        now + Duration::from_millis(1),
        PointerPhase::Move,
        Offset::new(10., 80.),
    );
    dispatch(
        &mut tree,
        6,
        now + Duration::from_millis(2),
        PointerPhase::Up,
        Offset::new(10., 80.),
    );
    assert_eq!(order.order(), vec![0, 1]);
}

/// gestures-006: the long-press deadline fires on a frame tick (before any
/// movement), as it does in a running app, and the drag must still start.
/// Flutter's `ReorderableDelayedDragStartListener` is one
/// `DelayedMultiDragGestureRecognizer` with the `kLongPressTimeout` delay.
#[test]
fn delayed_listener_starts_reorder_when_deadline_fires_on_frame() {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let list = ReorderableList::new(4, |index| {
        ReorderableDelayedDragStartListener::new(index, row(index))
    })
    .item_extent(40.)
    .on_reorder_start({
        let starts = starts.clone();
        move |index| starts.borrow_mut().push(index)
    });
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 140.)))
        .expect("layout");

    let now = std::time::Instant::now();
    let hold = PointerGestureRecognizer::LONG_PRESS_TIMEOUT;
    dispatch(&mut tree, 5, now, PointerPhase::Down, Offset::new(10., 20.));
    // A frame lands after the deadline; the timer path accepts the drag.
    tree.pump_gesture_timers(now + hold + Duration::from_millis(1));
    assert_eq!(*starts.borrow(), vec![0]);
    dispatch(
        &mut tree,
        5,
        now + hold + Duration::from_millis(2),
        PointerPhase::Move,
        Offset::new(10., 130.),
    );
    dispatch(
        &mut tree,
        5,
        now + hold + Duration::from_millis(3),
        PointerPhase::Up,
        Offset::new(10., 130.),
    );
    settle(&mut tree, now + hold, Size::new(100., 140.));
    assert_eq!(order.order(), vec![1, 2, 3, 0]);
}

#[test]
fn drop_index_follows_the_proxy_geometry_not_the_pointer_row() {
    // Flutter 3.41.7 widgets/reorderable_list.dart _dragUpdateItems: row 0
    // (60 px) dragged down 150 px has its proxy at 150..210. Row 2 (120..180)
    // claims the slot because the proxy's start lies in its leading half, so
    // the insertion index is 2 (harness gestures__reorderable_list_long_press
    // reports onReorderEnd(2)), although the pointer ends over row 2's end.
    let ends = Rc::new(RefCell::new(Vec::new()));
    let reorders = Rc::new(RefCell::new(Vec::new()));
    let list = ReorderableList::new(3, |index| {
        ReorderableDelayedDragStartListener::new(
            index,
            Widget::box_(Size::new(100., 60.), Color::rgba(index as u8, 0, 0, 255)),
        )
    })
    .item_extent(60.)
    .on_reorder_end({
        let ends = ends.clone();
        move |index| ends.borrow_mut().push(index)
    })
    .on_reorder({
        let reorders = reorders.clone();
        move |from, to| reorders.borrow_mut().push((from, to))
    });
    let order = list.reorder_controller();
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 600.)))
        .expect("layout");

    let now = std::time::Instant::now();
    let hold = PointerGestureRecognizer::LONG_PRESS_TIMEOUT;
    dispatch(&mut tree, 5, now, PointerPhase::Down, Offset::new(10., 30.));
    tree.pump_gesture_timers(now + hold + Duration::from_millis(1));
    dispatch(
        &mut tree,
        5,
        now + hold + Duration::from_millis(2),
        PointerPhase::Move,
        Offset::new(10., 180.),
    );
    dispatch(
        &mut tree,
        5,
        now + hold + Duration::from_millis(3),
        PointerPhase::Up,
        Offset::new(10., 180.),
    );
    assert_eq!(*ends.borrow(), vec![2]);
    assert!(reorders.borrow().is_empty());
    settle(&mut tree, now + hold, Size::new(100., 600.));
    assert_eq!(*reorders.borrow(), vec![(0, 2)]);
    assert_eq!(order.order(), vec![1, 0, 2]);
}

/// gestures-006: Flutter 3.41.7 widgets/reorderable_list.dart. The dragged
/// row is the proxy (`_DragItemProxy`) and follows the drag; the rows it
/// passes move aside over 250 ms with `Curves.easeInOut`
/// (`_ReorderableItemState.updateForGap`); the release animates the proxy
/// into its slot (`_DragInfo.end` reverses `_proxyAnimation`) and only then
/// reports `onReorder` (`_dropCompleted`). Values from the harness scenario
/// gestures__reorderable_list_long_press (row 1 at 59.517 one frame after the
/// gap starts moving).
#[test]
fn reorder_drag_moves_a_proxy_opens_a_gap_and_reports_after_the_drop() {
    let ends = Rc::new(RefCell::new(Vec::new()));
    let reorders = Rc::new(RefCell::new(Vec::new()));
    let list = ReorderableList::new(3, |index| {
        ReorderableDelayedDragStartListener::new(index, keyed_row(index))
    })
    .item_extent(60.)
    .on_reorder_end({
        let ends = ends.clone();
        move |index| ends.borrow_mut().push(index)
    })
    .on_reorder({
        let reorders = reorders.clone();
        move |from, to| reorders.borrow_mut().push((from, to))
    });
    let order = list.reorder_controller();
    let size = Size::new(100., 600.);
    let mut tree = WidgetTree::new();
    tree.mount(list.into()).unwrap();
    tree.layout(Constraints::tight(size)).expect("layout");

    let now = std::time::Instant::now();
    let hold = PointerGestureRecognizer::LONG_PRESS_TIMEOUT;
    dispatch(&mut tree, 7, now, PointerPhase::Down, Offset::new(10., 30.));
    tree.pump_gesture_timers(now + hold);
    let frame = now + hold + Duration::from_millis(16);
    dispatch(
        &mut tree,
        7,
        frame - Duration::from_millis(1),
        PointerPhase::Move,
        Offset::new(10., 180.),
    );
    let _ = tree.tick_animations(frame);
    tree.layout(Constraints::tight(size)).expect("layout");
    assert!(
        (row_y(&tree, 0) - 150.).abs() < 0.01,
        "the proxy follows the drag"
    );
    assert!(
        (row_y(&tree, 1) - 60.).abs() < 0.01,
        "the gap starts moving next frame"
    );
    assert!((row_y(&tree, 2) - 120.).abs() < 0.01);

    let next = frame + Duration::from_millis(16);
    let _ = tree.tick_animations(next);
    tree.layout(Constraints::tight(size)).expect("layout");
    assert!(
        (row_y(&tree, 1) - 59.517).abs() < 0.01,
        "{}",
        row_y(&tree, 1)
    );

    dispatch(
        &mut tree,
        7,
        next + Duration::from_millis(1),
        PointerPhase::Up,
        Offset::new(10., 180.),
    );
    assert_eq!(*ends.borrow(), vec![2]);
    let drop = next + Duration::from_millis(32);
    let _ = tree.tick_animations(drop);
    tree.layout(Constraints::tight(size)).expect("layout");
    let landing = row_y(&tree, 0);
    assert!(
        landing > 60. && landing < 150.,
        "the proxy drops toward 60: {landing}"
    );
    assert!(reorders.borrow().is_empty(), "onReorder waits for the drop");

    settle(&mut tree, drop, size);
    assert_eq!(*reorders.borrow(), vec![(0, 2)]);
    assert_eq!(order.order(), vec![1, 0, 2]);
    // `_dragReset`: the gaps reset and the builder (by index) fills the
    // slots again.
    for index in 0..3 {
        assert!((row_y(&tree, index) - 60. * index as f32).abs() < 0.01);
    }
}
