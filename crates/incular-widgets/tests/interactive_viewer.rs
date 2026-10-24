//! Flutter `InteractiveViewer`: pans and scales its child, keeping the
//! viewport inside the boundary. Expected transforms are derived by hand from
//! `_matrixTranslate` and `_matrixScale`.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{InteractiveViewer, TransformationController, Widget};

/// A 200x200 child in a 100x100 viewport, laid out unconstrained so it keeps
/// its size.
fn viewer(controller: &TransformationController) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        InteractiveViewer::new(Widget::box_(Size::new(200., 200.), Color::WHITE))
            .constrained(false)
            .transformation_controller(controller.clone())
            .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

fn base() -> Instant {
    static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *BASE.get_or_init(Instant::now)
}

fn touch(tree: &mut WidgetTree, pointer: u64, x: f32, y: f32, phase: PointerPhase, millis: u64) {
    let base = base();
    tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer,
        position: Offset::new(x, y),
        phase,
        time: base + Duration::from_millis(millis),
    });
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
}

fn scale_and_translation(controller: &TransformationController) -> (f32, Offset) {
    let value = controller.value();
    let [a, ..] = value.to_kurbo().as_coeffs();
    (a as f32, value.translation_offset())
}

#[test]
fn defaults_follow_flutter() {
    let viewer = InteractiveViewer::new(Widget::box_(Size::ZERO, Color::WHITE));
    assert_eq!(viewer.min_scale_value(), 0.8);
    assert_eq!(viewer.max_scale_value(), 2.5);
}

#[test]
fn a_pan_is_clamped_to_the_child_boundary() {
    let controller = TransformationController::new();
    let mut tree = viewer(&controller);
    touch(&mut tree, 1, 50., 50., PointerPhase::Down, 0);
    // The focal point passes kPanSlop (36) at x = 10, where the gesture
    // starts (`DragStartBehavior.start`).
    touch(&mut tree, 1, 40., 50., PointerPhase::Move, 10);
    touch(&mut tree, 1, 10., 50., PointerPhase::Move, 20);
    touch(&mut tree, 1, 0., 50., PointerPhase::Move, 30);
    assert_eq!(
        scale_and_translation(&controller),
        (1., Offset::new(-10., 0.))
    );
    // A 200 px pan would show scene x 210..310; the boundary stops the
    // viewport at the child's right edge, 100..200.
    touch(&mut tree, 1, -200., 50., PointerPhase::Move, 40);
    assert_eq!(
        scale_and_translation(&controller),
        (1., Offset::new(-100., 0.))
    );
    // A slow release does not fling.
    touch(&mut tree, 1, -200., 50., PointerPhase::Up, 1_000);
    assert_eq!(
        scale_and_translation(&controller),
        (1., Offset::new(-100., 0.))
    );
}

#[test]
fn a_pinch_scales_about_the_focal_point_up_to_max_scale() {
    let controller = TransformationController::new();
    let mut tree = viewer(&controller);
    touch(&mut tree, 1, 40., 50., PointerPhase::Down, 0);
    touch(&mut tree, 2, 60., 50., PointerPhase::Down, 0);
    // Span 10 -> 30 passes kScaleSlop: the scale starts with focal (70, 50).
    touch(&mut tree, 2, 100., 50., PointerPhase::Move, 10);
    // Span 60 = 2 x 30, focal (100, 50): scale 2 keeping scene (70, 50)
    // under the focal point, so the translation is (100, 50) - 2 (70, 50).
    touch(&mut tree, 2, 160., 50., PointerPhase::Move, 20);
    assert_eq!(
        scale_and_translation(&controller),
        (2., Offset::new(-40., -50.))
    );
    // Span 110 asks for 3.67, clamped to maxScale 2.5; scene (70, 50) stays
    // under the focal point (150, 50).
    touch(&mut tree, 2, 260., 50., PointerPhase::Move, 30);
    assert_eq!(
        scale_and_translation(&controller),
        (2.5, Offset::new(-25., -75.))
    );
}

#[test]
fn to_scene_inverts_the_transform() {
    let controller = TransformationController::with_value(
        incular_core::Transform::translation(Offset::new(-40., -50.))
            .then(incular_core::Transform::scale(2.)),
    );
    assert_eq!(
        controller.to_scene(Offset::new(100., 50.)),
        Offset::new(70., 50.)
    );
}

#[test]
fn a_mouse_wheel_scales_about_the_pointer() {
    let controller = TransformationController::new();
    let mut tree = viewer(&controller);
    // `exp(-dy / scaleFactor)` with scaleFactor 200: dy = -200 ln 2 doubles.
    let dy = -200. * std::f32::consts::LN_2;
    assert!(tree.scroll_at(Offset::new(50., 40.), Offset::new(0., dy)));
    let (scale, translation) = scale_and_translation(&controller);
    assert!((scale - 2.).abs() < 1e-4);
    // Scene (50, 40) stays under the pointer: translation = (50, 40) - 2 (50, 40).
    assert!((translation.x + 50.).abs() < 1e-3);
    assert!((translation.y + 40.).abs() < 1e-3);
}

#[test]
fn a_fling_continues_with_friction_until_the_boundary() {
    let controller = TransformationController::new();
    let mut tree = viewer(&controller);
    // 20 px every 16 ms to the left: 1250 px/s at the release.
    for (index, x) in [90., 70., 50., 30., 10.].into_iter().enumerate() {
        let phase = if index == 0 {
            PointerPhase::Down
        } else {
            PointerPhase::Move
        };
        touch(&mut tree, 1, x, 50., phase, index as u64 * 16);
    }
    touch(&mut tree, 1, 10., 50., PointerPhase::Up, 64);
    assert_eq!(scale_and_translation(&controller).1, Offset::new(-40., 0.));
    // The friction simulation heads for -40 - 1250 / ln(0.0000135), about
    // -151.5, over ln(10 / 1250) / ln(0.0000135 / 100) s; the boundary stops
    // the viewport at the child's right edge.
    for millis in (80..=600).step_by(16) {
        tree.update_compositor(base() + Duration::from_millis(millis))
            .expect("frame");
    }
    let translation = scale_and_translation(&controller).1;
    assert_eq!(translation.x, -100.);
    assert!(translation.y.abs() < 1e-6);
}
