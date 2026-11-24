//! Native-scale Windows wgpu counterpart to the Flutter Stack clipping probe.
#![cfg(target_os = "windows")]

use incular_config::{Clip, Constraints, TransparencyMode};
use incular_core::{Color, Offset, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{Center, Positioned, SizedBox, Stack, Transform, Widget};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

fn scene(positioned: bool, clip: Clip) -> Widget {
    let child = Widget::box_(Size::new(20., 20.), Color::rgba(255, 0, 0, 255));
    let child: Widget = if positioned {
        Positioned::new(child)
            .left(30.)
            .top(0.)
            .width(20.)
            .height(20.)
            .into()
    } else {
        Transform::translation(Offset::new(30., 0.), child).into()
    };
    Center::new(
        SizedBox::new()
            .width(40.)
            .height(40.)
            .child(Stack::new([child]).clip_behavior(clip)),
    )
    .into()
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn stack_hard_edge_clips_direct_layout_overflow_only() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run native Windows wgpu readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(100, 100)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(100, 100),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();

    let mut mismatches = Vec::new();
    let mut runtime = Runtime::new(scene(false, Clip::HardEdge)).unwrap();
    for (frame, (positioned, clip)) in [
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
            runtime
                .schedule_update(root, scene(positioned, clip))
                .unwrap();
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
        assert_eq!((pixels.width, pixels.height), (100, 100));
        let expected_outside = if positioned && clip == Clip::HardEdge {
            0
        } else {
            255
        };
        for (x, r) in [(59, 0), (65, 255), (75, expected_outside), (80, 0)] {
            let i = (40 * 100 + x) * 4;
            let actual = <[u8; 4]>::try_from(&pixels.rgba8[i..i + 4]).unwrap();
            let expected = [r, 0, 0, 255];
            if actual != expected {
                mismatches.push((
                    frame,
                    positioned,
                    clip,
                    x,
                    actual,
                    expected,
                    format!("{:?}", list.commands()),
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "frame, positioned, clip, x, actual, expected: {mismatches:?}"
    );
}
