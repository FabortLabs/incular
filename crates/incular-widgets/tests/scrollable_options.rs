//! Flutter `Scrollable`/`ScrollView` options: `clipBehavior` and
//! `dragStartBehavior`.

use std::time::{Duration, Instant};

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, DragStartBehavior, Offset, PointerPhase, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{CustomScrollView, GridView, ListView, PageView, SingleChildScrollView};
use incular_widgets::{Directionality, SliverGridDelegate, Widget};

fn rows(count: usize) -> Vec<Widget> {
    (0..count)
        .map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE))
        .collect()
}

fn mount(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    let _ = tree.paint();
    tree
}

fn clip_layers(tree: &WidgetTree) -> usize {
    tree.compositor_debug_tree().matches("ClipRect").count()
}

#[test]
fn scroll_views_clip_hard_edge_by_default() {
    for widget in [
        Widget::from(ListView::new(rows(10))),
        Widget::from(SingleChildScrollView::new(Widget::box_(
            Size::new(100., 400.),
            Color::WHITE,
        ))),
        Widget::from(GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(1),
            rows(10),
        )),
        Widget::from(PageView::new(rows(3))),
        Widget::from(CustomScrollView::new(Vec::new())),
    ] {
        assert_eq!(clip_layers(&mount(widget)), 1);
    }
}

#[test]
fn clip_none_paints_scrolled_content_without_a_clip_layer() {
    for widget in [
        Widget::from(ListView::new(rows(10)).clip_behavior(Clip::None)),
        Widget::from(
            SingleChildScrollView::new(Widget::box_(Size::new(100., 400.), Color::WHITE))
                .clip_behavior(Clip::None),
        ),
        Widget::from(
            GridView::new(SliverGridDelegate::fixed_cross_axis_count(1), rows(10))
                .clip_behavior(Clip::None),
        ),
        Widget::from(PageView::new(rows(3)).clip_behavior(Clip::None)),
        Widget::from(CustomScrollView::new(Vec::new()).clip_behavior(Clip::None)),
    ] {
        assert_eq!(clip_layers(&mount(widget)), 0);
    }
}

#[test]
fn clip_none_still_translates_scrolled_content() {
    let controller = ScrollController::new();
    let mut tree = mount(
        ListView::new(rows(10))
            .controller(controller.clone())
            .clip_behavior(Clip::None)
            .into(),
    );
    assert!(controller.jump_to(30.));
    let _ = tree.update_compositor(Instant::now());
    assert!(
        tree.compositor_debug_tree().contains("-30"),
        "{}",
        tree.compositor_debug_tree()
    );
}

/// Drags a vertical list upwards: 10, then 15 more (past kTouchSlop 18 for
/// the unknown device kind of `dispatch_gesture`), then 10 more.
fn drag_list(behavior: DragStartBehavior) -> f32 {
    let controller = ScrollController::new();
    let mut tree = mount(
        ListView::new(rows(10))
            .controller(controller.clone())
            .drag_start_behavior(behavior)
            .into(),
    );
    let base = Instant::now();
    for (phase, y, millis) in [
        (PointerPhase::Down, 80., 0),
        (PointerPhase::Move, 70., 10),
        (PointerPhase::Move, 55., 20),
        (PointerPhase::Move, 45., 30),
    ] {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(50., y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    controller.offset()
}

#[test]
fn start_behavior_scrolls_only_after_the_slop() {
    // `DragStartBehavior.start`: the 25 px that won the arena are not
    // applied; only the following 10 px scroll.
    assert_eq!(drag_list(DragStartBehavior::Start), 10.);
}

#[test]
fn down_behavior_applies_the_slop_movement() {
    // `DragStartBehavior.down`: the drag starts at the press and the 25 px
    // that won the arena are the first update, then 10 px more.
    assert_eq!(drag_list(DragStartBehavior::Down), 35.);
}

fn stored_list(
    bucket: &incular_widgets::PageStorageBucket,
    controller: ScrollController,
) -> WidgetTree {
    mount(
        incular_widgets::PageStorage::with_bucket(
            bucket.clone(),
            ListView::new(rows(10))
                .controller(controller)
                .page_storage_key(incular_widgets::PageStorageKey::new("list")),
        )
        .into(),
    )
}

#[test]
fn keep_scroll_offset_saves_and_restores_through_page_storage() {
    let bucket = incular_widgets::PageStorageBucket::default();
    let first = ScrollController::new();
    assert!(
        first.keep_scroll_offset(),
        "Flutter defaults keepScrollOffset to true"
    );
    let _tree = stored_list(&bucket, first.clone());
    assert!(first.jump_to(40.));
    let key = incular_widgets::PageStorageKey::new("list");
    assert_eq!(
        bucket.read_state(&key).and_then(|value| value.as_f64()),
        Some(40.)
    );
    // A new view and controller under the same key starts where the first
    // one stopped.
    let second = ScrollController::new();
    let _tree = stored_list(&bucket, second.clone());
    assert_eq!(second.offset(), 40.);
}

#[test]
fn without_keep_scroll_offset_nothing_is_stored() {
    let bucket = incular_widgets::PageStorageBucket::default();
    let controller = ScrollController::new().with_keep_scroll_offset(false);
    let _tree = stored_list(&bucket, controller.clone());
    assert!(controller.jump_to(40.));
    assert!(bucket.is_empty());
}

fn tap_behind_short_list(behavior: incular_widgets::HitTestBehavior) -> usize {
    let taps = std::rc::Rc::new(std::cell::Cell::new(0));
    let background =
        incular_widgets::GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .on_tap({
                let taps = taps.clone();
                move || taps.set(taps.get() + 1)
            });
    // Two 40 px rows leave the bottom 20 px of the viewport empty.
    let mut tree = mount(
        incular_widgets::Stack::new(vec![
            Widget::from(background),
            Widget::from(ListView::new(rows(2)).hit_test_behavior(behavior)),
        ])
        .into(),
    );
    let base = Instant::now();
    for (phase, millis) in [(PointerPhase::Down, 0), (PointerPhase::Up, 20)] {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(50., 90.),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    taps.get()
}

#[test]
fn scrollables_are_opaque_by_default() {
    assert_eq!(
        tap_behind_short_list(incular_widgets::HitTestBehavior::Opaque),
        0
    );
}

#[test]
fn defer_to_child_lets_presses_on_empty_space_through() {
    assert_eq!(
        tap_behind_short_list(incular_widgets::HitTestBehavior::DeferToChild),
        1
    );
}
