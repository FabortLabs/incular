use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::{MouseCursor, PointerDeviceKind, RawPointerEvent};
use incular_widgets::{
    AbsorbPointer, GestureDetector, HitTestBehavior, IgnorePointer, MouseRegion, Positioned, Stack,
    Widget,
    internal::{ActionId, WidgetTree, action},
};
use std::{cell::RefCell, rc::Rc, time::Instant};

fn region(
    log: &Rc<RefCell<Vec<&'static str>>>,
    name: &'static str,
    child: impl Into<Widget>,
) -> MouseRegion {
    let log = log.clone();
    MouseRegion::new(child).on_enter(move |_| log.borrow_mut().push(name))
}

/// Overlapping layers that exercise every way the raw and retained routes
/// part: opaque and translucent regions, regions over ignored children,
/// absorbing and deferring wrappers.
fn layered(log: &Rc<RefCell<Vec<&'static str>>>) -> Widget {
    let square = |side| Widget::box_(Size::new(side, side), Color::WHITE);
    Stack::new([
        Widget::from(region(
            log,
            "behind",
            action(Size::new(60., 60.), Color::WHITE, ActionId(1)),
        )),
        // The detector's bounds hold the point, so the region claims it for
        // raw input, yet nothing in it is a retained target.
        Positioned::new(
            region(
                log,
                "opaque-over-ignored",
                GestureDetector::new(IgnorePointer::new(square(50.)))
                    .behavior(HitTestBehavior::DeferToChild)
                    .on_tap(|| {}),
            )
            .hit_test_behavior(HitTestBehavior::DeferToChild)
            .cursor(MouseCursor::Click)
            .opaque(true),
        )
        .left(20.)
        .top(20.)
        .into(),
        Positioned::new(
            region(log, "translucent", square(30.))
                .opaque(false)
                .hit_test_behavior(HitTestBehavior::Translucent)
                .cursor(MouseCursor::Text),
        )
        .left(40.)
        .top(0.)
        .into(),
        Positioned::new(AbsorbPointer::new(square(40.)))
            .left(0.)
            .top(60.)
            .into(),
        Positioned::new(
            GestureDetector::new(IgnorePointer::new(square(30.)))
                .behavior(HitTestBehavior::DeferToChild)
                .on_tap(|| {}),
        )
        .left(50.)
        .top(50.)
        .into(),
        Positioned::new(
            region(log, "deferring", IgnorePointer::new(square(30.)))
                .hit_test_behavior(HitTestBehavior::DeferToChild)
                .opaque(true),
        )
        .left(70.)
        .top(70.)
        .into(),
    ])
    .into()
}

fn mounted(root: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

fn grid() -> impl Iterator<Item = Offset> {
    (-1..=21).flat_map(|x| (-1..=21).map(move |y| Offset::new(x as f32 * 5., y as f32 * 5.)))
}

fn hover(phase: PointerPhase, position: Offset) -> RawPointerEvent {
    RawPointerEvent::new(
        0,
        1,
        PointerDeviceKind::Mouse,
        0,
        position,
        phase,
        Instant::now(),
    )
}

#[test]
fn one_walk_finds_the_same_retained_target_and_cursor() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let tree = mounted(layered(&log));
    for point in grid() {
        let hit = tree.pointer_hit_test(point);
        assert_eq!(hit.target(), tree.hit_test(point), "target at {point:?}");
        assert_eq!(
            tree.mouse_cursor_for_hit(&hit),
            tree.mouse_cursor_at(point),
            "cursor at {point:?}"
        );
        assert_eq!(
            tree.text_field_for_hit(&hit),
            tree.text_field_at(point),
            "text field at {point:?}"
        );
        assert_eq!(
            tree.selectable_text_for_hit(&hit),
            tree.selectable_text_at(point),
            "selectable text at {point:?}"
        );
    }
}

#[test]
fn an_opaque_region_over_ignored_content_blocks_raw_but_not_retained_hits() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut tree = mounted(layered(&log));
    let point = Offset::new(25., 25.);
    let hit = tree.pointer_hit_test(point);
    // The retained route looks behind the region's ignored child and finds
    // the action underneath; the raw route stops at the opaque region.
    let target = hit
        .target()
        .and_then(|render| tree.element_for_render(render))
        .expect("retained target");
    assert_eq!(tree.action_for_element(target), Some(ActionId(1)));
    let _ = tree.dispatch_raw_pointer_with_hit(7, hover(PointerPhase::Move, point), &hit);
    assert_eq!(*log.borrow(), ["opaque-over-ignored"]);

    // A translucent region lets the opaque one behind it enter too.
    let point = Offset::new(45., 25.);
    let hit = tree.pointer_hit_test(point);
    let _ = tree.dispatch_raw_pointer_with_hit(7, hover(PointerPhase::Move, point), &hit);
    assert_eq!(
        *log.borrow(),
        ["opaque-over-ignored", "translucent"],
        "only the translucent region enters; the opaque one is still hovered"
    );
    assert_eq!(tree.mouse_cursor_for_hit(&hit), MouseCursor::Text);
}

#[test]
fn hover_refresh_runs_enter_and_exit_without_hover_callbacks() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let hovered = |name: &'static str| {
        let enter = log.clone();
        let exit = log.clone();
        let hover = log.clone();
        MouseRegion::new(Widget::box_(Size::new(100., 50.), Color::WHITE))
            .on_enter(move |_| enter.borrow_mut().push(format!("enter {name}")))
            .on_exit(move |_| exit.borrow_mut().push(format!("exit {name}")))
            .on_hover(move |_| hover.borrow_mut().push(format!("hover {name}")))
    };
    let mut tree = mounted(
        Stack::new([
            Widget::from(Positioned::new(hovered("top")).left(0.).top(0.)),
            Positioned::new(hovered("bottom")).left(0.).top(50.).into(),
        ])
        .into(),
    );
    let still = Offset::new(10., 10.);
    let _ = tree.dispatch_raw_pointer_in_window(7, hover(PointerPhase::Move, still));
    assert_eq!(*log.borrow(), ["enter top", "hover top"]);

    // Replaying the same sample against an unchanged tree changes nothing.
    let hit = tree.pointer_hit_test(still);
    tree.refresh_mouse_hover_in_window(7, hover(PointerPhase::Move, still), &hit);
    assert_eq!(log.borrow().len(), 2);

    // Content that moved under the pointer: the refreshed hit test lands
    // on the bottom region, so hover moves without a hover callback.
    let moved = tree.pointer_hit_test(Offset::new(10., 60.));
    tree.refresh_mouse_hover_in_window(7, hover(PointerPhase::Move, still), &moved);
    assert_eq!(
        *log.borrow(),
        ["enter top", "hover top", "exit top", "enter bottom"]
    );
}
