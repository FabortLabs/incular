//! Regression tests for Flutter `ScrollPosition.jumpTo` notifications and
//! out-of-range springs (scroll-001, scroll-005), the initial offset
//! (scroll-003), page mapping (scroll-002), and bouncing overscroll
//! (scroll-011).

use std::{cell::RefCell, rc::Rc};

use incular_scroll::*;

type Kinds = Rc<RefCell<Vec<ScrollNotificationType>>>;

fn record(controller: &ScrollController) -> (Kinds, ScrollNotificationSubscription) {
    let kinds: Kinds = Rc::new(RefCell::new(Vec::new()));
    let log = kinds.clone();
    let subscription = controller.add_listener(move |notification: ScrollNotification| {
        log.borrow_mut().push(notification.kind);
        false
    });
    (kinds, subscription)
}

fn attached_tree(controller: &ScrollController, tree: u64) -> MetricAttachment {
    controller
        .try_attach(MetricOwner::of_tree(tree))
        .expect("controller is free to attach")
}

fn publish(attachment: &MetricAttachment, content: f32, viewport: f32) {
    attachment
        .update_extents(ViewportMetricsUpdate::new(
            content,
            viewport,
            ScrollPhysics::clamping(),
        ))
        .expect("the attachment owns the controller");
}

/// Flutter `jumpTo` with a changed position: one Start, one Update, one End,
/// in that order (`scroll_position_with_single_context.dart:197-206`).
#[test]
fn in_range_jump_emits_start_update_end_in_order() {
    let controller = ScrollController::new();
    controller.update_extents(1000., 100.).unwrap();
    let (kinds, _subscription) = record(&controller);

    assert!(controller.jump_to(125.));

    assert_eq!(
        *kinds.borrow(),
        vec![
            ScrollNotificationType::Start,
            ScrollNotificationType::Update,
            ScrollNotificationType::End,
        ]
    );
    assert_eq!(controller.offset(), 125.);
}

/// Flutter emits no notifications when `pixels == value`.
#[test]
fn jump_to_the_current_position_is_silent() {
    let controller = ScrollController::new();
    controller.update_extents(1000., 100.).unwrap();
    assert!(controller.jump_to(125.));
    let (kinds, _subscription) = record(&controller);

    assert!(!controller.jump_to(125.));

    assert!(kinds.borrow().is_empty());
}

/// Without an attached viewport nothing springs an out-of-range position back,
/// so the jump clamps to the content bounds immediately.
#[test]
fn unattached_out_of_range_jump_clamps() {
    let controller = ScrollController::new();
    controller.update_extents(2000., 600.).unwrap();

    assert!(controller.jump_to(100_000.));

    assert_eq!(controller.offset(), 1400.);
    assert!(!controller.has_pending_settle());
}

/// Attached out-of-range jumps keep the requested offset and ask the viewport
/// to spring it back (Flutter `goBallistic(0)`), instead of clamping.
#[test]
fn attached_out_of_range_jump_is_sprung_back_not_clamped() {
    let controller = ScrollController::new();
    let attachment = attached_tree(&controller, 7);
    publish(&attachment, 2000., 600.);
    let (kinds, _subscription) = record(&controller);

    assert!(controller.jump_to(100_000.));

    assert_eq!(controller.offset(), 100_000.);
    assert!(controller.has_pending_settle());
    assert_eq!(
        *kinds.borrow(),
        vec![
            ScrollNotificationType::Start,
            ScrollNotificationType::Update,
            ScrollNotificationType::End,
        ]
    );
    let settle = controller
        .take_pending_settle()
        .expect("out-of-range position needs a spring");
    assert!(settle.final_x() <= 1400. + 1e-3);
    assert!(!controller.has_pending_settle());
}

/// A republication with unchanged extents must not clamp a position that is
/// still springing back.
#[test]
fn republication_keeps_a_position_that_is_springing_back() {
    let controller = ScrollController::new();
    let attachment = attached_tree(&controller, 8);
    publish(&attachment, 2000., 600.);
    controller.jump_to(5000.);

    publish(&attachment, 2000., 600.);

    assert_eq!(controller.offset(), 5000.);
}

/// Inside an open activity bracket a jump reports a bare Update and leaves the
/// bracket to its owner, so gesture and fling takeover keep their ordering.
#[test]
fn jump_inside_open_bracket_reports_only_update() {
    let controller = ScrollController::new();
    controller.update_extents(1000., 100.).unwrap();
    assert!(controller.begin_activity());
    let (kinds, _subscription) = record(&controller);

    assert!(controller.jump_to(50.));

    assert_eq!(*kinds.borrow(), vec![ScrollNotificationType::Update]);
    assert!(controller.end_activity());
}

/// Flutter `ScrollController.initialScrollOffset` is clamped to the first
/// content dimensions (`applyContentDimensions`).
#[test]
fn initial_scroll_offset_applies_and_clamps_on_first_publication() {
    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    controller.update_extents(1000., 400.).unwrap();
    assert_eq!(controller.offset(), 300.);

    let short = ScrollController::new().with_initial_scroll_offset(300.);
    short.update_extents(200., 100.).unwrap();
    assert_eq!(short.offset(), 100.);
}

/// A saved offset is not overwritten by the initial offset when it is applied
/// first: the initial value is consumed only once.
#[test]
fn initial_scroll_offset_is_applied_once() {
    let controller = ScrollController::new().with_initial_scroll_offset(300.);
    controller.update_extents(1000., 400.).unwrap();
    assert!(controller.jump_to(50.));

    controller.update_extents(1000., 400.).unwrap();

    assert_eq!(controller.offset(), 50.);
}

/// Flutter `PageMetrics.page` and `getPixelsFromPage` with `viewportFraction`
/// and the pad that centres pages above one.
#[test]
fn page_mapping_matches_flutter_with_viewport_fraction() {
    // Fraction one: page n is n viewports in.
    assert_eq!(pixels_for_page(2., 400., 1.), 800.);
    assert_eq!(page_for_pixels(800., 400., 1.), 2.);

    // Fraction 0.5: pages are half a viewport apart, no centring pad.
    assert_eq!(pixels_for_page(2., 400., 0.5), 400.);
    assert_eq!(page_for_pixels(400., 400., 0.5), 2.);

    // Fraction 1.5: initial pad is viewport * 0.25.
    assert_eq!(pixels_for_page(0., 400., 1.5), 100.);
    assert_eq!(pixels_for_page(1., 400., 1.5), 700.);
    assert!((page_for_pixels(700., 400., 1.5) - 1.).abs() < 1e-6);
}

/// Defers a page-based starting position until a viewport exists, then
/// resolves it against that viewport (Flutter `initialPage`).
#[test]
fn deferred_page_resolves_against_first_viewport() {
    let controller = ScrollController::new();
    assert!(controller.defer_page(2., 0.5));
    controller.update_extents(2000., 400.).unwrap();

    assert_eq!(controller.offset(), 400.);
}

/// Bouncing physics report no boundary overscroll for a drag past the edge
/// (Flutter `BouncingScrollPhysics.applyBoundaryConditions` returns zero),
/// while clamping physics report the clamped amount.
#[test]
fn bouncing_drag_past_edge_emits_no_overscroll() {
    let bouncing = ScrollController::new();
    bouncing.update_extents(1000., 100.).unwrap();
    let (bounce_kinds, _bounce) = record(&bouncing);
    let _ = bouncing.apply_physics(ScrollPhysics::clamping().bouncing(), -40.);
    assert!(
        !bounce_kinds
            .borrow()
            .contains(&ScrollNotificationType::Overscroll)
    );

    let clamping = ScrollController::new();
    clamping.update_extents(1000., 100.).unwrap();
    let (clamp_kinds, _clamp) = record(&clamping);
    let _ = clamping.apply_physics(ScrollPhysics::clamping(), -40.);
    assert!(
        clamp_kinds
            .borrow()
            .contains(&ScrollNotificationType::Overscroll)
    );
}
