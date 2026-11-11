//! `ScrollController::animate_to`, Flutter's `ScrollPosition.animateTo`: a
//! request starts on the owning viewport's next frame, follows its curve over
//! its duration inside one Start/End bracket, and stops when something else
//! moves the position.

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_scroll::{ScrollController, ScrollNotificationType};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{SingleChildScrollView, SizedBox, Widget};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn mounted(controller: &ScrollController) -> WidgetTree {
    let scrolled: Widget =
        SingleChildScrollView::new(Widget::box_(Size::new(200., 1000.), Color::WHITE))
            .controller(controller.clone())
            .into();
    let mut tree = WidgetTree::new();
    tree.mount(SizedBox::from_dimensions(Some(200.), Some(100.), Some(scrolled)).into())
        .expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 100.)))
        .expect("layout");
    tree
}

#[test]
fn animate_to_follows_its_curve_and_brackets_once() {
    let controller = ScrollController::new();
    let mut tree = mounted(&controller);
    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = controller.add_listener({
        let events = events.clone();
        move |notification| {
            events.borrow_mut().push(notification.kind);
            false
        }
    });
    assert!(controller.animate_to(400., Duration::from_millis(100), Curve::Linear));
    assert!(controller.has_pending_animation());
    assert!(tree.has_pending_scroll_animations());
    assert_eq!(controller.offset(), 0., "nothing moves before a frame");

    let base = Instant::now();
    // The first frame starts the ticker at elapsed zero.
    assert!(tree.pump_scroll_flings(base, false));
    assert_eq!(controller.offset(), 0.);
    assert!(!controller.has_pending_animation());
    assert!(tree.pump_scroll_flings(base + Duration::from_millis(50), false));
    assert!(
        (controller.offset() - 200.).abs() < 0.5,
        "{}",
        controller.offset()
    );
    let mut frame = 0;
    while tree.pump_scroll_flings(base + Duration::from_millis(66 + frame * 16), false) {
        frame += 1;
        assert!(frame < 20, "the animation ends");
    }
    assert_eq!(controller.offset(), 400.);
    let kinds = events.borrow();
    assert_eq!(kinds.first(), Some(&ScrollNotificationType::Start));
    assert_eq!(kinds.last(), Some(&ScrollNotificationType::End));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == ScrollNotificationType::End)
            .count(),
        1
    );
}

#[test]
fn a_jump_stops_a_running_animation() {
    let controller = ScrollController::new();
    let mut tree = mounted(&controller);
    let base = Instant::now();
    controller.animate_to(800., Duration::from_millis(300), Curve::EaseInOut);
    tree.pump_scroll_flings(base, false);
    tree.pump_scroll_flings(base + Duration::from_millis(100), false);
    let mid = controller.offset();
    assert!(mid > 0. && mid < 800.);
    controller.jump_to(10.);
    assert!(!tree.pump_scroll_flings(base + Duration::from_millis(116), false));
    assert_eq!(controller.offset(), 10.);
}

#[test]
fn a_new_request_replaces_the_previous_one() {
    let controller = ScrollController::new();
    let mut tree = mounted(&controller);
    let base = Instant::now();
    controller.animate_to(800., Duration::from_millis(300), Curve::Linear);
    tree.pump_scroll_flings(base, false);
    tree.pump_scroll_flings(base + Duration::from_millis(30), false);
    controller.animate_to(0., Duration::from_millis(100), Curve::Linear);
    let mut now = base + Duration::from_millis(46);
    while tree.pump_scroll_flings(now, false) {
        now += Duration::from_millis(16);
    }
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn targets_past_the_content_stop_at_the_edge() {
    let controller = ScrollController::new();
    let mut tree = mounted(&controller);
    let base = Instant::now();
    controller.animate_to(5000., Duration::from_millis(100), Curve::Linear);
    let mut now = base;
    while tree.pump_scroll_flings(now, false) {
        now += Duration::from_millis(16);
    }
    assert_eq!(controller.offset(), controller.max_offset());
}

#[test]
fn reduced_motion_and_zero_duration_land_immediately() {
    let controller = ScrollController::new();
    let mut tree = mounted(&controller);
    controller.animate_to(120., Duration::from_millis(100), Curve::Linear);
    assert!(!tree.pump_scroll_flings(Instant::now(), true));
    assert_eq!(controller.offset(), 120.);
    controller.animate_to(30., Duration::ZERO, Curve::Linear);
    assert_eq!(controller.offset(), 30.);
}
