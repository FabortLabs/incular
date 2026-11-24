//! Native Windows wgpu comparison with a fitting or overflowing scroll child.
#![cfg(target_os = "windows")]
use incular_config::{Clip, Constraints, TransparencyMode};
use incular_core::{Color, Offset, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{
    Center, Positioned, SingleChildScrollView, SizedBox, Stack, Transform, Widget,
};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const N: u32 = 100;
fn scene(layout_overflow: bool, clip: Clip) -> Widget {
    let red = Widget::box_(Size::new(20., 20.), Color::rgba(255, 0, 0, 255));
    let child: Widget = if layout_overflow {
        SizedBox::new()
            .width(40.)
            .height(50.)
            .child(Stack::new([Positioned::new(red)
                .left(0.)
                .top(30.)
                .width(20.)
                .height(20.)]))
            .into()
    } else {
        Transform::translation(Offset::new(0., 30.), red).into()
    };
    Center::new(
        SizedBox::new()
            .width(40.)
            .height(40.)
            .child(SingleChildScrollView::new(child).clip_behavior(clip)),
    )
    .into()
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn only_translated_layout_overflow_clips_held_scroll_child_pixels() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 for native Windows readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(N, N)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(N, N),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();
    let mut runtime = Runtime::new(scene(false, Clip::HardEdge)).unwrap();
    for (frame, (layout, clip)) in [
        (false, Clip::HardEdge),
        (true, Clip::HardEdge),
        (false, Clip::HardEdge),
        (true, Clip::None),
        (true, Clip::HardEdge),
        (false, Clip::None),
    ]
    .into_iter()
    .enumerate()
    {
        if frame != 0 {
            let root = runtime.tree().root().unwrap();
            runtime.schedule_update(root, scene(layout, clip)).unwrap();
        }
        let (list, _) = runtime
            .run_frame(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        renderer.request_capture();
        assert!(matches!(
            renderer.render(&list, 1.).unwrap(),
            FrameOutcome::Presented(_)
        ));
        let pixels = renderer.take_capture().unwrap().unwrap();
        assert_eq!((pixels.width, pixels.height), (N, N));
        let outside = if layout && clip == Clip::HardEdge {
            0
        } else {
            255
        };
        for (y, red) in [(59, 0), (65, 255), (75, outside), (80, 0)] {
            let offset = ((y * N + 40) * 4) as usize;
            assert_eq!(
                pixels.rgba8[offset..offset + 4],
                [red, 0, 0, 255],
                "frame={frame}, y={y}"
            );
        }
    }
}
