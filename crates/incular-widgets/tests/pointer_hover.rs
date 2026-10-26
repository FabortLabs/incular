use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON, Size};
use incular_gestures::{MouseCursor, PointerDeviceKind, RawPointerEvent};
use incular_layout::Constraints;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Color, MouseRegion, TapRegion, TapRegionSurface, Widget};
use std::{cell::Cell, rc::Rc, time::Instant};

fn event(phase: PointerPhase, position: Offset) -> RawPointerEvent {
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

fn button_event(button: u32) -> RawPointerEvent {
    RawPointerEvent {
        buttons: button,
        button: Some(button),
        phase: PointerPhase::Down,
        ..event(PointerPhase::Down, Offset::new(20., 20.))
    }
}

fn layout(tree: &mut WidgetTree, root: impl Into<Widget>) {
    tree.mount(root.into()).expect("mount pointer tree");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout pointer tree");
}

#[test]
fn native_surface_exit_clears_mouse_region_hover_exactly_once() {
    let enters = Rc::new(Cell::new(0));
    let exits = Rc::new(Cell::new(0));
    let mut tree = WidgetTree::new();
    layout(
        &mut tree,
        MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .on_enter({
                let enters = enters.clone();
                move |_| enters.set(enters.get() + 1)
            })
            .on_exit({
                let exits = exits.clone();
                move |_| exits.set(exits.get() + 1)
            }),
    );

    assert!(
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Enter, Offset::ZERO))
            .is_none()
    );
    assert_eq!(
        enters.get(),
        0,
        "surface enter has no reliable local position"
    );
    let _ =
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Move, Offset::new(20., 20.)));
    assert_eq!(enters.get(), 1);

    let _ =
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Exit, Offset::new(20., 20.)));
    assert_eq!(exits.get(), 1);
    let _ =
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Exit, Offset::new(20., 20.)));
    assert_eq!(
        exits.get(),
        1,
        "duplicate native exits must not re-exit stale regions"
    );

    let _ =
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Move, Offset::new(20., 20.)));
    assert_eq!(enters.get(), 2, "a later positioned sample can enter again");
}

/// Two stacked 100x100 regions; returns how often each saw the pointer enter.
fn stacked_enters(top_opaque: bool) -> (usize, usize) {
    let below = Rc::new(Cell::new(0));
    let above = Rc::new(Cell::new(0));
    let mut tree = WidgetTree::new();
    let lower: Widget = MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .on_enter({
            let below = below.clone();
            move |_| below.set(below.get() + 1)
        })
        .into();
    let upper: Widget = MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::BLACK))
        .opaque(top_opaque)
        .on_enter({
            let above = above.clone();
            move |_| above.set(above.get() + 1)
        })
        .into();
    layout(&mut tree, incular_widgets::Stack::new([lower, upper]));
    let _ =
        tree.dispatch_raw_pointer_in_window(7, event(PointerPhase::Move, Offset::new(20., 20.)));
    (below.get(), above.get())
}

#[test]
fn opaque_mouse_region_hides_regions_behind_it() {
    // Flutter 3.41.7 rendering/proxy_box.dart RenderMouseRegion.hitTest is
    // `super.hitTest(...) && _opaque`: an opaque region ends the hit test, so
    // the sibling painted behind it is never reached.
    assert_eq!(stacked_enters(true), (0, 1));
}

#[test]
fn non_opaque_mouse_region_lets_regions_behind_it_see_the_pointer() {
    // With `opaque: false` the region still receives the pointer itself (its
    // entry is added) but hitTest returns false, so the hit test carries on
    // to the sibling behind it and both regions are hovered.
    assert_eq!(stacked_enters(false), (1, 1));
}

#[test]
fn deferred_mouse_cursor_falls_through_to_ancestor_without_changing_hit_testing() {
    let child = MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .cursor(MouseCursor::Defer);
    let mut tree = WidgetTree::new();
    layout(&mut tree, MouseRegion::new(child).cursor(MouseCursor::Text));
    let point = Offset::new(40., 40.);
    let hit_before = tree.hit_test(point);
    assert_eq!(tree.mouse_cursor_at(point), MouseCursor::Text);
    assert_eq!(tree.hit_test(point), hit_before);

    let child = MouseRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
        .cursor(MouseCursor::Click);
    let mut tree = WidgetTree::new();
    layout(&mut tree, MouseRegion::new(child).cursor(MouseCursor::Text));
    assert_eq!(tree.mouse_cursor_at(point), MouseCursor::Click);
}

#[test]
fn tap_region_is_primary_only_while_raw_secondary_input_remains_distinct() {
    let taps = Rc::new(Cell::new(0));
    let region = TapRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).on_tap_inside({
        let taps = taps.clone();
        move |_| taps.set(taps.get() + 1)
    });
    let mut tree = WidgetTree::new();
    layout(&mut tree, TapRegionSurface::new(region));

    let _ = tree.dispatch_raw_pointer_in_window(7, button_event(SECONDARY_POINTER_BUTTON));
    assert_eq!(taps.get(), 0);
    let _ = tree.dispatch_raw_pointer_in_window(7, button_event(PRIMARY_POINTER_BUTTON));
    assert_eq!(taps.get(), 1);
}
