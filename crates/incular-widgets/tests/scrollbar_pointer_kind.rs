//! Flutter `ScrollbarPainter.hitTestInteractive` and
//! `hitTestOnlyThumbInteractive` for the overlay scrollbar: touch and
//! trackpad presses hit within `_kMinInteractiveSize` (48) of the thumb;
//! mouse, stylus and unknown pointers hit the track itself.

mod common;

use common::*;
use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_scroll::ScrollController;
use incular_widgets::PointerDeviceKind;

fn mount() -> (WidgetTree, ScrollController) {
    let controller = ScrollController::new();
    controller.set_scrollbar_thumb_visibility(true);
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::SingleChildScrollView::new(Widget::box_(
            Size::new(100., 1_000.),
            Color::WHITE,
        ))
        .controller(controller.clone())
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    settle_scrollbar_fade(&mut tree);
    (tree, controller)
}

/// A point 20 px left of the thumb's centre: inside the 48 px square
/// around the thumb, outside the narrower track.
fn beside_thumb(tree: &WidgetTree) -> Offset {
    let geometry = tree.scrollbar_diagnostics().pop().unwrap();
    let center = Offset::new(
        geometry.thumb.origin.x + geometry.thumb.size.width / 2.,
        geometry.thumb.origin.y + geometry.thumb.size.height / 2.,
    );
    let point = Offset::new(center.x - 20., center.y);
    assert!(
        !geometry.track.contains(point),
        "precondition: off the track"
    );
    point
}

#[test]
fn a_mouse_press_beside_the_track_misses() {
    let (mut tree, _) = mount();
    let point = beside_thumb(&tree);
    for kind in [
        PointerDeviceKind::Mouse,
        PointerDeviceKind::Stylus,
        PointerDeviceKind::Unknown,
    ] {
        assert!(!tree.scrollbar_pointer_with_kind(PointerPhase::Down, point, kind));
    }
}

#[test]
fn a_touch_press_near_the_thumb_drags_it() {
    for kind in [PointerDeviceKind::Touch, PointerDeviceKind::Trackpad] {
        let (mut tree, controller) = mount();
        let point = beside_thumb(&tree);
        assert!(tree.scrollbar_pointer_with_kind(PointerPhase::Down, point, kind));
        let moved = point + Offset::new(0., 40.);
        assert!(tree.scrollbar_pointer_with_kind(PointerPhase::Move, moved, kind));
        assert!(tree.scrollbar_pointer_with_kind(PointerPhase::Up, moved, kind));
        // A thumb drag, not a track page (which jumps a whole viewport).
        assert!(controller.offset() > 0. && controller.offset() != 100.);
    }
}
