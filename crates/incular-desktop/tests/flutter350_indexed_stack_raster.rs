//! Actual Windows wgpu raster for retained IndexedStack selection, at native scale.
#![cfg(target_os = "windows")]

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{Center, IndexedStack, Widget};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const RGBA: [[u8; 4]; 3] = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
fn capture(renderer: &mut WgpuRenderer, runtime: &mut Runtime) -> Vec<u8> {
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(WIDTH as f32, HEIGHT as f32)))
        .unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let pixels = renderer.take_capture().unwrap().unwrap();
    assert_eq!((pixels.width, pixels.height), (WIDTH, HEIGHT));
    pixels.rgba8
}
fn scene(children: &[Widget], index: impl Into<Option<usize>>) -> Widget {
    Center::new(IndexedStack::new(children.iter().cloned()).index(index)).into()
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn selected_color_replaces_previous_raster_without_affecting_edges() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run native Windows wgpu readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH, HEIGHT)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(WIDTH, HEIGHT),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();
    let children =
        RGBA.map(|[r, g, b, a]| Widget::box_(Size::new(28., 20.), Color::rgba(r, g, b, a)));
    let sample = |pixels: &[u8], x, y| {
        let index = ((y * WIDTH + x) * 4) as usize;
        <[u8; 4]>::try_from(&pixels[index..index + 4]).unwrap()
    };
    for (i, color) in RGBA.into_iter().enumerate() {
        let mut cold = Runtime::new(scene(&children, i)).unwrap();
        assert_eq!(
            sample(&capture(&mut renderer, &mut cold), 32, 24),
            color,
            "fresh selection {i}"
        );
    }
    let mut live = Runtime::new(scene(&children, 0)).unwrap();
    let mut mismatches = Vec::new();
    for (frame, i) in [Some(0), None, Some(1), Some(2), None, Some(0)]
        .into_iter()
        .enumerate()
    {
        if frame != 0 {
            let root = live.tree().root().unwrap();
            live.schedule_update(root, scene(&children, i)).unwrap();
        }
        let pixels = capture(&mut renderer, &mut live);
        for (x, y, inside) in [
            (32, 24, true),
            (18, 14, true),
            (45, 33, true),
            (17, 14, false),
            (46, 33, false),
            (18, 13, false),
            (45, 34, false),
        ] {
            let expected = i.filter(|_| inside).map_or([0, 0, 0, 255], |i| RGBA[i]);
            let actual = sample(&pixels, x, y);
            if actual != expected {
                mismatches.push((frame, i, x, y, actual, expected));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "frame, index, x, y, actual, expected: {mismatches:?}"
    );
}
