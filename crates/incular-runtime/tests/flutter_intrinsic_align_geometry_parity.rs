//! Flutter 3.50.0 retained IntrinsicWidth and Align geometry.

use incular_config::{Alignment, Constraints};
use incular_core::Size;
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Center, Directionality, IntrinsicWidth, Placeholder, SizedBox, TextDirection, Widget,
    internal::Key,
};

const SUBJECT: u64 = 1;

fn intrinsic(step_width: Option<f32>, step_height: Option<f32>) -> Widget {
    let mut width = IntrinsicWidth::new(SizedBox::new().width(100.).height(50.));
    if let Some(step) = step_width {
        width = width.step_width(step);
    }
    if let Some(step) = step_height {
        width = width.step_height(step);
    }
    Center::new(Widget::from(width).with_key(SUBJECT)).into()
}

fn aligned(width_factor: Option<f32>, height_factor: Option<f32>) -> Widget {
    let mut align = Align::new(Alignment::CENTER, SizedBox::square(100.));
    if let Some(factor) = width_factor {
        align = align.width_factor(factor);
    }
    if let Some(factor) = height_factor {
        align = align.height_factor(factor);
    }
    Widget::from(align).with_key(SUBJECT)
}

fn size(runtime: &Runtime) -> Size {
    let tree = runtime.tree();
    let element = tree
        .element_with_key(&Key::from(SUBJECT))
        .expect("fixture key");
    tree.element_bounds(element).expect("subject bounds").size
}

// widgets/intrinsic_width_test.dart:9, zero and omitted-step geometry.
#[test]
fn intrinsic_width_omitted_and_zero_steps_match() {
    let frame = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(intrinsic(None, None)).expect("mount");
    runtime.run_frame(frame).expect("initial frame");
    assert_eq!(size(&runtime), Size::new(100., 50.));
    let root = runtime.tree().root().expect("root");
    runtime
        .tree_mut()
        .update(root, intrinsic(Some(0.), Some(0.)))
        .expect("update steps");
    runtime.run_frame(frame).expect("zero-step frame");
    assert_eq!(size(&runtime), Size::new(100., 50.));
}

// widgets/intrinsic_width_test.dart:9, invalid negative-step debug validation.
#[cfg(debug_assertions)]
#[test]
fn intrinsic_width_rejects_negative_width_or_height_steps() {
    let rejected = [(-1., 0.), (0., -1.)].map(|(width, height)| {
        std::panic::catch_unwind(|| intrinsic(Some(width), Some(height))).is_err()
    });
    assert_eq!(rejected, [true, true], "(width step, height step)");
    let rejected_builder = [(-1., 0.), (0., -1.)].map(|(width, height)| {
        std::panic::catch_unwind(|| {
            IntrinsicWidth::builder()
                .child(SizedBox::square(10.))
                .step_width(width)
                .step_height(height)
                .build()
        })
        .is_err()
    });
    assert_eq!(rejected_builder, [true, true]);
}

// widgets/intrinsic_width_test.dart:35, "IntrinsicWidth does not crash at zero area".
#[test]
fn intrinsic_width_placeholder_can_complete_a_zero_area_frame() {
    let width = Widget::from(IntrinsicWidth::new(Placeholder::new())).with_key(SUBJECT);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(width));
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    let metrics = WindowMetrics::new(PhysicalSize::new(0, 0), 1.);
    runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
    let view = runtime.environment();
    assert_eq!((view.physical_width, view.physical_height), (0, 0));
    runtime
        .run_frame(Constraints::tight(view.viewport))
        .expect("zero-area frame");
    assert_eq!(size(&runtime), Size::ZERO);
}

// rendering/positioned_box_test.dart:13, "RenderPositionedBox expands".
#[test]
fn align_expands_both_bounded_axes_when_factors_are_omitted() {
    let mut runtime = Runtime::new(aligned(None, None)).expect("mount");
    runtime
        .run_frame(Constraints::loose(Size::new(200., 200.)))
        .expect("frame");
    assert_eq!(size(&runtime), Size::new(200., 200.));
}

// rendering/positioned_box_test.dart:25, "RenderPositionedBox shrink wraps".
#[test]
fn align_relayout_shrink_wraps_just_the_axes_with_explicit_factors() {
    let frame = Constraints::loose(Size::new(200., 200.));
    let mut runtime = Runtime::new(aligned(Some(1.), None)).expect("mount");
    runtime.run_frame(frame).expect("width-factor frame");
    assert_eq!(size(&runtime), Size::new(100., 200.));
    let root = runtime.tree().root().expect("root");
    for (width, height, expected) in [
        (None, Some(1.), Size::new(200., 100.)),
        (Some(1.), Some(1.), Size::new(100., 100.)),
    ] {
        runtime
            .tree_mut()
            .update(root, aligned(width, height))
            .expect("update axis factors");
        runtime.run_frame(frame).expect("updated-factor frame");
        assert_eq!(size(&runtime), expected);
    }
}

// rendering/positioned_box_test.dart:50, actual-layout assertions only.
#[test]
fn align_factor_transition_includes_a_zero_axis_and_restores_fill() {
    let frame = Constraints::at_most(200., 200.);
    let mut runtime = Runtime::new(aligned(Some(1.), Some(0.))).unwrap();
    runtime.run_frame(frame).unwrap();
    let root = runtime.tree().root().unwrap();
    let render = runtime.tree().render_id(root).unwrap();
    assert_eq!(size(&runtime), Size::new(100., 0.));
    for (width, height, expected) in [
        (Some(0.5), Some(0.5), Size::new(50., 50.)),
        (None, None, Size::new(200., 200.)),
    ] {
        runtime
            .schedule_update(root, aligned(width, height))
            .unwrap();
        runtime.run_frame(frame).unwrap();
        assert_eq!(runtime.tree().render_id(root), Some(render));
        assert_eq!(size(&runtime), expected);
    }
}
