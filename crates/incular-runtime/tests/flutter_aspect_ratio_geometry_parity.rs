//! Flutter 3.50.0 widgets/aspect_ratio_test.dart retained geometry.

use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_widgets::{
    AspectRatio, Center, ConstrainedBox, Container, Directionality, Placeholder, ScrollController,
    SingleChildScrollView, TextDirection, Widget, internal::Key,
};

fn size_of(runtime: &Runtime, key: u64) -> Size {
    let tree = runtime.tree();
    let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(element).expect("fixture bounds").size
}

fn viewport() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

// aspect_ratio_test.dart:26, "Aspect ratio control test".
#[test]
fn aspect_ratio_sizes_the_child_under_loose_finite_constraints() {
    const CHILD: u64 = 1;
    for (ratio, expected) in [(2., Size::new(500., 250.)), (0.5, Size::new(250., 500.))] {
        let child = Widget::from(Container::new()).with_key(CHILD);
        let scene = Center::new(ConstrainedBox::new(
            Constraints::loose(Size::new(500., 500.)),
            AspectRatio::new(ratio, child),
        ));
        let mut runtime = Runtime::new(scene.into()).expect("mount");
        runtime.run_frame(viewport()).expect("layout");
        assert_eq!(size_of(&runtime, CHILD), expected, "aspect ratio {ratio}");
    }
}

// aspect_ratio_test.dart:37, "Aspect ratio infinite width".
#[test]
fn aspect_ratio_uses_finite_height_inside_horizontal_scroll_view() {
    const CHILD: u64 = 1;
    const SCROLL: u64 = 2;
    let controller = ScrollController::new();
    let child = Widget::from(Container::new()).with_key(CHILD);
    let view = SingleChildScrollView::new(AspectRatio::new(2., child))
        .scroll_direction(Axis::Horizontal)
        .controller(controller.clone());
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(Widget::from(view).with_key(SCROLL)),
    );
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    runtime.run_frame(viewport()).expect("layout");
    assert_eq!(size_of(&runtime, SCROLL), Size::new(800., 600.));
    assert_eq!(size_of(&runtime, CHILD), Size::new(1200., 600.));
    assert_eq!(controller.max_offset(), 400.);
}

// aspect_ratio_test.dart:54, "AspectRatio does not crash at zero area".
#[test]
fn aspect_ratio_can_lay_out_and_paint_a_zero_area_placeholder() {
    const ASPECT: u64 = 1;
    let aspect = Widget::from(AspectRatio::new(2., Placeholder::new())).with_key(ASPECT);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(aspect));
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    for (width, height, want) in [
        (1000, 750, Size::new(800., 400.)),
        (0, 0, Size::ZERO),
        (1000, 750, Size::new(800., 400.)),
    ] {
        let metrics = WindowMetrics::new(PhysicalSize::new(width, height), 1.25);
        runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
        let view = runtime.environment();
        assert_eq!((view.physical_width, view.physical_height), (width, height));
        assert_eq!(view.viewport, metrics.logical_size());
        runtime
            .run_frame(Constraints::tight(view.viewport))
            .expect("view frame");
        assert_eq!(size_of(&runtime, ASPECT), want);
    }
}
