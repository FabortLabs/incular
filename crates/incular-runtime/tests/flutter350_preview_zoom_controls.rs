use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_controls::widget_previews::PreviewZoomControls;
use incular_core::{InputEvent, Offset, PointerPhase, Size, Transform, time::Instant};
use incular_runtime::Runtime;
use incular_semantics::Role;
use incular_widgets::{Directionality, TransformationController};

struct Fixture {
    runtime: Runtime,
    controller: TransformationController,
    now: Instant,
}

impl Fixture {
    fn new() -> Self {
        let controller = TransformationController::new();
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                PreviewZoomControls::new(controller.clone()),
            )
            .into(),
        )
        .unwrap();
        runtime.set_platform_semantics_enabled(true);
        let mut scene = Self {
            runtime,
            controller,
            now: Instant::now(),
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.now += Duration::from_millis(20);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(520., 60.)), self.now)
            .unwrap();
    }

    fn point(&self, key: &str) -> Offset {
        let tree = self.runtime.tree();
        let id = tree.element_with_key(&key.into()).unwrap();
        let rect = tree.element_bounds(id).unwrap();
        rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.)
    }

    fn tap(&mut self, key: &str) {
        let point = self.point(key);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            self.now += Duration::from_millis(1);
            let _ = self.runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: point,
                },
                self.now,
            );
        }
        self.frame();
    }

    fn assert_scale(&self, scale: f32, label: &str) {
        assert_eq!(self.controller.value().coefficients()[0], f64::from(scale));
        let tree = self.runtime.tree().semantics();
        let (_, slider) = tree
            .iter()
            .find(|(_, node)| node.role == Role::Slider)
            .unwrap();
        assert_eq!(slider.state.numeric_value, Some(f64::from(scale)));
        assert!(
            tree.iter()
                .any(|(_, node)| node.label.as_deref() == Some(label)),
            "{} lacks percentage {label}",
            self.runtime.tree().semantics_debug_dump()
        );
    }
}

#[test]
fn preview_zoom_initializes_to_one_and_bound_buttons_are_inert() {
    let mut scene = Fixture::new();
    scene.assert_scale(1., "100%");
    let tree = scene.runtime.tree().semantics();
    for name in ["Zoom in", "Zoom out", "Reset zoom"] {
        assert!(
            tree.iter()
                .any(|(_, node)| node.state.tooltip.as_deref() == Some(name))
        );
    }
    scene.tap("preview.zoom.out");
    scene.tap("preview.zoom.reset");
    scene.assert_scale(1., "100%");
}

#[test]
fn preview_zoom_buttons_move_in_linear_quarters() {
    let mut scene = Fixture::new();
    for (key, scale, label) in [
        ("preview.zoom.in", 1.25, "125%"),
        ("preview.zoom.in", 1.5, "150%"),
        ("preview.zoom.out", 1.25, "125%"),
        ("preview.zoom.out", 1., "100%"),
    ] {
        scene.tap(key);
        scene.assert_scale(scale, label);
    }
}

#[test]
fn reset_uses_identity_and_removes_preview_translation_too() {
    let mut scene = Fixture::new();
    scene.tap("preview.zoom.in");
    scene.tap("preview.zoom.in");
    scene.assert_scale(1.5, "150%");
    scene
        .controller
        .set_value(Transform::from_coefficients([1.5, 0., 0., 1.5, 12., -8.]));
    scene.frame();
    scene.tap("preview.zoom.reset");
    assert_eq!(scene.controller.value(), Transform::IDENTITY);
    scene.assert_scale(1., "100%");
}

#[test]
fn external_transform_changes_refresh_the_slider_and_visible_percentage() {
    let mut scene = Fixture::new();
    scene.assert_scale(1., "100%");
    scene.controller.set_value(Transform::scale(2.5));
    scene.frame();
    scene.assert_scale(2.5, "250%");
}

#[test]
fn slider_is_live_and_the_upper_button_clamps_without_overshooting() {
    let mut scene = Fixture::new();
    scene.tap("preview.zoom.slider");
    assert!(scene.controller.value().coefficients()[0] > 1.);
    scene.controller.set_value(Transform::scale(3.875));
    scene.frame();
    scene.tap("preview.zoom.in");
    scene.assert_scale(4., "400%");
    scene.tap("preview.zoom.in");
    scene.assert_scale(4., "400%");
}

#[test]
fn accessibility_steps_the_slider_without_rounding_the_shared_transform_to_the_track_step() {
    use incular_semantics::SemanticAction;

    let mut scene = Fixture::new();
    scene.controller.set_value(Transform::scale(1.253_125));
    scene.frame();
    scene.assert_scale(1.253_125, "125%");
    let (slider, _) = scene
        .runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.role == Role::Slider)
        .unwrap();
    assert!(
        scene
            .runtime
            .dispatch_semantic_action(slider, SemanticAction::Increment)
    );
    scene.frame();
    let increment = if cfg!(any(target_os = "ios", target_os = "macos")) {
        0.3
    } else {
        0.15
    };
    let actual = scene.controller.value().coefficients()[0] as f32;
    assert!((actual - (1.253_125 + increment)).abs() < 0.000_001);
}
