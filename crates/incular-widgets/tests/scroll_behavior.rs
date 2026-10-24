//! Flutter's `ScrollBehavior` (`widgets/scroll_configuration.dart`) and the
//! `ScrollConfiguration` that hands it to descendant scroll views.

use std::time::{Duration, Instant};

use incular_config::{Constraints, TargetPlatform};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_gestures::VelocityTrackerKind;
use incular_scroll::{ScrollController, ScrollDecelerationRate, ScrollPhysics};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    AndroidOverscrollIndicator, ListView, MultitouchDragStrategy, ScrollBehavior,
    ScrollConfiguration, Widget,
};

fn on(platform: TargetPlatform) -> ScrollBehavior {
    ScrollBehavior::new().platform(platform)
}

#[test]
fn physics_follow_the_platform() {
    assert_eq!(
        on(TargetPlatform::Ios).get_scroll_physics(),
        ScrollPhysics::clamping().bouncing().range_maintaining()
    );
    assert_eq!(
        on(TargetPlatform::MacOs).get_scroll_physics(),
        ScrollPhysics::clamping()
            .bouncing_with(ScrollDecelerationRate::Fast)
            .range_maintaining()
    );
    for platform in [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::Linux,
        TargetPlatform::Windows,
    ] {
        assert_eq!(
            on(platform).get_scroll_physics(),
            ScrollPhysics::clamping().range_maintaining()
        );
    }
    let fixed = ScrollPhysics::clamping().never_scrollable();
    assert_eq!(
        on(TargetPlatform::Ios).physics(fixed).get_scroll_physics(),
        fixed
    );
}

#[test]
fn platform_policies_match_flutter() {
    // `_kTouchLikeDeviceTypes`: no mouse.
    let behavior = ScrollBehavior::new();
    assert!(!behavior.accepts_drag(PointerDeviceKind::Mouse));
    for kind in [
        PointerDeviceKind::Touch,
        PointerDeviceKind::Stylus,
        PointerDeviceKind::InvertedStylus,
        PointerDeviceKind::Trackpad,
        PointerDeviceKind::Unknown,
    ] {
        assert!(behavior.accepts_drag(kind));
    }
    assert!(
        behavior
            .clone()
            .drag_devices([PointerDeviceKind::Mouse])
            .accepts_drag(PointerDeviceKind::Mouse)
    );
    // Scrollbars on desktop, glow on Android and Fuchsia.
    for platform in [
        TargetPlatform::Linux,
        TargetPlatform::MacOs,
        TargetPlatform::Windows,
    ] {
        assert!(on(platform).shows_scrollbar());
        assert_eq!(on(platform).overscroll_indicator(), None);
    }
    for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
        assert!(!on(platform).shows_scrollbar());
        assert_eq!(
            on(platform).overscroll_indicator(),
            Some(AndroidOverscrollIndicator::Glow)
        );
    }
    assert!(!on(TargetPlatform::Ios).shows_scrollbar());
    assert!(
        !on(TargetPlatform::Linux)
            .scrollbars(false)
            .shows_scrollbar()
    );
    assert_eq!(
        on(TargetPlatform::Android)
            .overscroll(false)
            .overscroll_indicator(),
        None
    );
    // Velocity trackers and multitouch strategies.
    assert_eq!(
        on(TargetPlatform::Ios).velocity_tracker(),
        VelocityTrackerKind::IosScrollViewFling
    );
    assert_eq!(
        on(TargetPlatform::MacOs).velocity_tracker(),
        VelocityTrackerKind::MacOsScrollViewFling
    );
    assert_eq!(
        on(TargetPlatform::Linux).velocity_tracker(),
        VelocityTrackerKind::LeastSquares
    );
    assert_eq!(
        on(TargetPlatform::Ios).get_multitouch_drag_strategy(),
        MultitouchDragStrategy::AverageBoundaryPointers
    );
    assert_eq!(
        on(TargetPlatform::Android).get_multitouch_drag_strategy(),
        MultitouchDragStrategy::LatestPointer
    );
}

fn list(controller: &ScrollController) -> Widget {
    ListView::new(
        (0..10)
            .map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE))
            .collect::<Vec<_>>(),
    )
    .controller(controller.clone())
    .into()
}

fn drag(tree: &mut WidgetTree, kind: PointerDeviceKind, from: f32, to: f32) {
    let base = Instant::now();
    let steps = [from, from, (from + to) / 2., to, to];
    for (index, y) in steps.iter().enumerate() {
        let phase = match index {
            0 => PointerPhase::Down,
            4 => PointerPhase::Up,
            _ => PointerPhase::Move,
        };
        tree.dispatch_pointer_gesture_in_window(
            0,
            1,
            kind,
            incular_gestures::PointerEvent {
                pointer: 1,
                position: Offset::new(50., *y),
                phase,
                // Slow enough that the release is not a fling.
                time: base + Duration::from_millis(index as u64 * 500),
            },
        );
    }
}

fn mount(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

#[test]
fn a_mouse_does_not_drag_scroll_by_default() {
    let controller = ScrollController::new();
    let mut tree = mount(list(&controller));
    drag(&mut tree, PointerDeviceKind::Mouse, 90., 10.);
    assert_eq!(controller.offset(), 0.);
    drag(&mut tree, PointerDeviceKind::Touch, 90., 10.);
    assert!(controller.offset() > 0.);
}

#[test]
fn a_configuration_can_let_the_mouse_drag() {
    let controller = ScrollController::new();
    let mut tree = mount(
        ScrollConfiguration::new(list(&controller))
            .behavior(ScrollBehavior::new().drag_devices([PointerDeviceKind::Mouse]))
            .into(),
    );
    drag(&mut tree, PointerDeviceKind::Mouse, 90., 10.);
    assert!(controller.offset() > 0.);
}

#[test]
fn views_without_physics_take_the_configurations() {
    // Bouncing physics let a drag pull the list past its start.
    let controller = ScrollController::new();
    let mut tree = mount(
        ScrollConfiguration::new(list(&controller))
            .behavior(on(TargetPlatform::Ios))
            .into(),
    );
    drag(&mut tree, PointerDeviceKind::Touch, 10., 90.);
    assert!(controller.offset() < 0.);
    // Clamping physics stop at the start; a view's own physics win.
    let controller = ScrollController::new();
    let mut tree = mount(
        ScrollConfiguration::new(
            ListView::new(
                (0..10)
                    .map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE))
                    .collect::<Vec<_>>(),
            )
            .controller(controller.clone())
            .physics(ScrollPhysics::clamping()),
        )
        .behavior(on(TargetPlatform::Ios))
        .into(),
    );
    drag(&mut tree, PointerDeviceKind::Touch, 10., 90.);
    assert_eq!(controller.offset(), 0.);
}

#[test]
fn horizontal_scrollbars_can_be_left_out() {
    // Material's `buildScrollbar` gives only vertical scroll views a
    // scrollbar; the base behavior gives both axes one on desktop.
    let row = |controller: &ScrollController| -> Widget {
        controller.set_scrollbar_thumb_visibility(true);
        ListView::new(
            (0..10)
                .map(|_| Widget::box_(Size::new(40., 100.), Color::WHITE))
                .collect::<Vec<_>>(),
        )
        .scroll_direction(incular_config::Axis::Horizontal)
        .controller(controller.clone())
        .into()
    };
    let visible = |behavior: ScrollBehavior| {
        let controller = ScrollController::new();
        let mut tree = mount(
            ScrollConfiguration::new(row(&controller))
                .behavior(behavior)
                .into(),
        );
        let start = Instant::now();
        tree.pump_scrollbar_fades(start);
        tree.pump_scrollbar_fades(start + Duration::from_millis(300));
        // The overlay track runs down the right edge, its thumb at the top.
        let hit = tree.scrollbar_pointer(PointerPhase::Down, Offset::new(97., 5.));
        tree.scrollbar_pointer(PointerPhase::Up, Offset::new(97., 5.));
        hit
    };
    assert!(visible(on(TargetPlatform::Linux)));
    assert!(!visible(
        on(TargetPlatform::Linux).horizontal_scrollbars(false)
    ));
}
