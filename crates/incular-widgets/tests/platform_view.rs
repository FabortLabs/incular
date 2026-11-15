//! Flutter's `PlatformViewSurface` (support-001): the view takes the
//! biggest size its constraints allow, forwards the pointers in its area by
//! hit-test behavior, and reports where it is.

use std::{cell::Cell, rc::Rc, time::Instant};

use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Rect, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    HitTestBehavior, Listener, Padding, PlatformViewCommand, PlatformViewController,
    PlatformViewHitTestBehavior, PlatformViewSurface, Positioned, RawPointerEvent, Stack, Widget,
};

fn controller() -> PlatformViewController {
    PlatformViewController::new("map", vec![1, 2], TextDirection::Ltr)
}

/// A padded view over a listener that counts the presses behind it.
fn mount(
    controller: &PlatformViewController,
    behavior: PlatformViewHitTestBehavior,
) -> (WidgetTree, Rc<Cell<usize>>) {
    let behind = Rc::new(Cell::new(0));
    let counter = behind.clone();
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(Stack::new([
        Widget::from(
            Listener::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
                .behavior(HitTestBehavior::Opaque)
                .on_pointer_down(move |_| counter.set(counter.get() + 1)),
        ),
        Padding::new(
            EdgeInsets::all(10.),
            PlatformViewSurface::new(controller.clone()).hit_test_behavior(behavior),
        )
        .into(),
    ])))
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    (tree, behind)
}

fn press(tree: &mut WidgetTree, x: f32, y: f32) {
    for (phase, buttons) in [(PointerPhase::Down, 1), (PointerPhase::Up, 0)] {
        tree.dispatch_raw_pointer(RawPointerEvent::new(
            7,
            1,
            PointerDeviceKind::Touch,
            buttons,
            Offset::new(x, y),
            phase,
            Instant::now(),
        ));
    }
}

fn pointers(controller: &PlatformViewController) -> Vec<(PointerPhase, Offset)> {
    controller
        .take_commands()
        .into_iter()
        .filter_map(|command| match command {
            PlatformViewCommand::Pointer { event, .. } => Some((event.phase, event.local_position)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_view_fills_its_constraints_and_reports_its_placement() {
    let view = controller();
    let (tree, _) = mount(&view, PlatformViewHitTestBehavior::Opaque);
    let placements = tree.platform_view_placements();
    assert_eq!(placements.len(), 1);
    let expected = rect(10., 10., 180., 180.);
    assert_eq!(placements[0].controller, view);
    assert_eq!(placements[0].bounds, expected);
    assert_eq!(placements[0].visible_bounds, Some(expected));
}

#[test]
fn hit_test_behavior_decides_who_receives_pointers() {
    let view = controller();
    let (mut tree, behind) = mount(&view, PlatformViewHitTestBehavior::Opaque);
    let _ = view.take_commands();
    press(&mut tree, 30., 40.);
    assert_eq!(
        pointers(&view),
        [
            (PointerPhase::Down, Offset::new(20., 30.)),
            (PointerPhase::Up, Offset::new(20., 30.))
        ],
        "local positions"
    );
    assert_eq!(behind.get(), 0, "opaque hides what is behind");
    // Outside the view, only what is behind it is hit.
    press(&mut tree, 5., 5.);
    assert!(pointers(&view).is_empty());
    assert_eq!(behind.get(), 1);

    let view = controller();
    let (mut tree, behind) = mount(&view, PlatformViewHitTestBehavior::Translucent);
    let _ = view.take_commands();
    press(&mut tree, 30., 40.);
    assert_eq!(pointers(&view).len(), 2);
    assert_eq!(behind.get(), 1, "translucent lets what is behind be hit");

    let view = controller();
    let (mut tree, behind) = mount(&view, PlatformViewHitTestBehavior::Transparent);
    let _ = view.take_commands();
    press(&mut tree, 30., 40.);
    assert!(pointers(&view).is_empty());
    assert_eq!(behind.get(), 1, "transparent passes pointers through");
}

#[test]
fn visible_bounds_are_clipped_to_the_ancestors() {
    let view = controller();
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(Stack::new([Widget::from(
        Positioned::new(PlatformViewSurface::new(view.clone()))
            .left(150.)
            .top(150.)
            .width(100.)
            .height(100.),
    )])))
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    let placement = &tree.platform_view_placements()[0];
    assert_eq!(placement.bounds, rect(150., 150., 100., 100.));
    assert_eq!(placement.visible_bounds, Some(rect(150., 150., 50., 50.)));
}

#[test]
fn controllers_queue_creation_first_and_drop_commands_after_dispose() {
    let view = controller();
    view.set_layout_direction(TextDirection::Ltr);
    view.set_layout_direction(TextDirection::Rtl);
    view.clear_focus();
    let commands = view.take_commands();
    assert!(matches!(
        &commands[0],
        PlatformViewCommand::Create { view_type, creation_params, .. }
            if view_type == "map" && creation_params == &[1, 2]
    ));
    assert_eq!(
        commands[1..],
        [
            PlatformViewCommand::SetDirection {
                id: view.id(),
                direction: TextDirection::Rtl
            },
            PlatformViewCommand::ClearFocus { id: view.id() },
        ],
        "an unchanged direction sends nothing"
    );
    view.dispose();
    view.dispose();
    view.clear_focus();
    assert_eq!(
        view.take_commands(),
        [PlatformViewCommand::Dispose { id: view.id() }]
    );

    // A view disposed before its creation was collected never reaches a host.
    let unborn = controller();
    unborn.dispose();
    assert!(unborn.take_commands().is_empty());
    assert_ne!(unborn.id(), view.id());
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}
