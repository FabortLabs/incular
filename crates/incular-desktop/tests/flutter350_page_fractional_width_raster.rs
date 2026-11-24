//! Representative native wgpu pixels for PageView at the pinned precision-regression width.
#![cfg(target_os = "windows")]
use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size, time::Instant};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::Curve;
use incular_widgets::{
    Center, Directionality, PageController, PageView, ScrollPhysics, SizedBox, TextDirection,
    Widget,
};
use std::{sync::Arc, time::Duration};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

#[test]
#[allow(
    deprecated,
    reason = "create one hidden window before entering the event loop"
)]
fn fractional_width_keeps_the_animated_last_page_marker_in_the_exact_pixel_box() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run native Windows wgpu readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(512, 192)),
        )
        .unwrap();
    let mut gpu = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(512, 192),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();
    let c = PageController::new();
    let children = [[0, 0, 255], [0, 255, 0], [255, 0, 0]].map(|[red, green, blue]| {
        Center::new(Widget::box_(
            Size::new(60., 22.),
            Color::rgba(red, green, blue, 255),
        ))
    });
    let view = PageView::new(children)
        .controller(c.clone())
        .physics(ScrollPhysics::CLAMPING);
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(
            SizedBox::new()
                .width(411.42857142857144f64 as f32)
                .height(192.)
                .child(view),
        ),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let constraints = Constraints::tight(Size::new(512., 192.));
    let base = Instant::now();
    runtime.run_frame_at(constraints, base).unwrap();
    assert!(c.animate_to_page(2, Duration::from_millis(300), Curve::Ease));
    for n in 1..120 {
        runtime
            .run_frame_at(constraints, base + Duration::from_millis(16 * n))
            .unwrap();
    }
    let (list, _) = runtime
        .run_frame_at(constraints, base + Duration::from_millis(16 * 120))
        .unwrap();
    assert!((c.page().unwrap() - 2.).abs() < 1e-5);
    gpu.request_capture();
    assert!(matches!(
        gpu.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = gpu.take_capture().unwrap().unwrap();
    assert_eq!((frame.width, frame.height), (512, 192));
    let mut differences = Vec::new();
    for (x, y, red) in [
        (256, 96, true),
        (226, 85, true),
        (285, 106, true),
        (225, 85, false),
        (286, 106, false),
        (226, 84, false),
        (285, 107, false),
    ] {
        let i = (y * 512 + x) * 4;
        let expected = [if red { 255 } else { 0 }, 0, 0, 255];
        let actual = <[u8; 4]>::try_from(&frame.rgba8[i..i + 4]).unwrap();
        if actual != expected {
            differences.push((x, y, actual, expected));
        }
    }
    assert!(
        differences.is_empty(),
        "x, y, actual, expected: {differences:?}"
    );
}
