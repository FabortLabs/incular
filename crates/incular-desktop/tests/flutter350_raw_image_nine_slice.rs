//! RawImage default-fit bounds from native Windows wgpu readback.
//! Reference: the pinned Flutter raw_image_nine_slice_test.dart.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Offset, Rect, Size};
use incular_image::ImageHandle;
use incular_platform::PhysicalSize;
use incular_rendering::FilterQuality;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{BoxFit, RawImage};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WIDTH: u32 = 100;
const HEIGHT: u32 = 60;
const CENTERED: [u32; 4] = [35, 15, 65, 45];
const FULL: [u32; 4] = [0, 0, WIDTH, HEIGHT];

fn bounds(
    renderer: &mut WgpuRenderer,
    source: &ImageHandle,
    has_slice: bool,
    fit: Option<BoxFit>,
) -> [u32; 4] {
    let size = Size::new(WIDTH as f32, HEIGHT as f32);
    let mut image = RawImage::new()
        .image(source.clone())
        .width(size.width)
        .height(size.height)
        .filter_quality(FilterQuality::None);
    if has_slice {
        image = image.center_slice(Rect::from_origin_size(
            Offset::new(10., 10.),
            Size::new(10., 10.),
        ));
    }
    if let Some(fit) = fit {
        image = image.fit(fit);
    }

    let mut runtime = Runtime::new(image.into()).unwrap();
    let (list, _) = runtime.run_frame(Constraints::tight(size)).unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));

    let (mut left, mut top, mut right, mut bottom) = (WIDTH, HEIGHT, 0, 0);
    let mut painted = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if frame.rgba8[((y * WIDTH + x) * 4) as usize] == 0 {
                continue;
            }
            painted += 1;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    assert!(painted > 0);
    assert_eq!(
        painted,
        (right - left) * (bottom - top),
        "no holes in the painted bounds"
    );
    [left, top, right, bottom]
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_raw_image_defaults_to_fill_only_with_a_center_slice() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run Windows GPU readback");
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
    let source = ImageHandle::from_rgba8(30, 30, vec![255; 30 * 30 * 4]).unwrap();

    assert_eq!(
        bounds(&mut renderer, &source, false, None),
        CENTERED,
        "no slice"
    );
    assert_eq!(
        bounds(&mut renderer, &source, true, Some(BoxFit::ScaleDown)),
        CENTERED,
        "explicit scaleDown with slice"
    );
    assert_eq!(
        bounds(&mut renderer, &source, true, Some(BoxFit::Fill)),
        FULL,
        "explicit fill with slice"
    );
    assert_eq!(
        bounds(&mut renderer, &source, true, None),
        FULL,
        "unset fit with slice (left, top, right exclusive, bottom exclusive)"
    );
}
