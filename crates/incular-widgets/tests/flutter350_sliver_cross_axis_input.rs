//! Flutter 3.50.0 cross-axis group gesture coordinates and click routing.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_scroll::ScrollController;
use incular_widgets::{
    CustomScrollView, Directionality, GestureDetector, HitTestBehavior, SizedBox, Sliver,
    SliverCrossAxisGroup, SliverList, SliverToBoxAdapter, Widget, internal::WidgetTree,
};

fn scene(axis: Axis, controller: &ScrollController, slivers: Vec<Box<dyn Sliver>>) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(vec![
            Box::new(SliverCrossAxisGroup::new(slivers)) as Box<dyn Sliver>
        ])
        .controller(controller.clone())
        .scroll_direction(axis),
    )
    .into()
}

fn layout(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::tight(size)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

fn tap(tree: &mut WidgetTree, point: Offset, time: Instant) {
    for (phase, elapsed) in [(PointerPhase::Down, 0), (PointerPhase::Up, 20)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position: point,
            phase,
            time: time + Duration::from_millis(elapsed),
        });
    }
}

#[test]
fn cross_axis_tap_down_reports_the_correct_lane_and_local_and_global_positions() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let received = Rc::new(RefCell::new(Vec::new()));
        let lanes: Vec<Box<dyn Sliver>> = (0..2)
            .map(|index| {
                let received = received.clone();
                let detector = GestureDetector::new(SizedBox::new().width(20.).height(20.))
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap_down(move |details| {
                        received.borrow_mut().push((
                            index,
                            details.local_position,
                            details.global_position,
                        ));
                    });
                Box::new(SliverToBoxAdapter::new(detector)) as Box<dyn Sliver>
            })
            .collect();
        let mut tree = WidgetTree::new();
        tree.mount(scene(axis, &ScrollController::new(), lanes))
            .unwrap();
        layout(&mut tree, axis.size(20., 40.));
        let time = Instant::now();
        let mut expected = Vec::new();
        for (i, (lane, cross, local)) in [(0, 5., 5.), (0, 15., 15.), (1, 25., 5.), (1, 35., 15.)]
            .into_iter()
            .enumerate()
        {
            let point = axis.offset(15., cross);
            tap(
                &mut tree,
                point,
                time + Duration::from_millis(i as u64 * 150),
            );
            expected.push((lane, axis.offset(15., local), point));
            assert_eq!(*received.borrow(), expected, "{axis:?} at {point:?}");
        }
    }
}

#[test]
fn a_rebuild_routes_the_tap_only_to_the_selected_lazy_lane_and_tile() {
    let clicked = Rc::new(Cell::new(None));
    let controller = ScrollController::new();
    let build = |selected| {
        let lanes: Vec<Box<dyn Sliver>> = [300., 100.]
            .into_iter()
            .enumerate()
            .map(|(lane, height)| {
                let clicked = clicked.clone();
                Box::new(SliverList::builder(20, move |row| {
                    let tile = SizedBox::new().height(height);
                    if selected == (lane, row) {
                        let clicked = clicked.clone();
                        GestureDetector::new(tile)
                            .behavior(HitTestBehavior::Opaque)
                            .on_tap(move || clicked.set(Some((lane, row))))
                            .into()
                    } else {
                        Widget::from(tile)
                    }
                })) as Box<dyn Sliver>
            })
            .collect();
        scene(Axis::Vertical, &controller, lanes)
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build((0, 0))).unwrap();
    let size = Size::new(300., 600.);
    layout(&mut tree, size);
    let time = Instant::now();
    tap(&mut tree, Offset::new(75., 150.), time);
    assert_eq!(clicked.take(), Some((0, 0)));

    tree.update(root, build((1, 2))).unwrap();
    layout(&mut tree, size);
    tap(
        &mut tree,
        Offset::new(75., 150.),
        time + Duration::from_millis(150),
    );
    assert_eq!(clicked.get(), None);
    tap(
        &mut tree,
        Offset::new(225., 250.),
        time + Duration::from_millis(300),
    );
    assert_eq!(clicked.get(), Some((1, 2)));
}
