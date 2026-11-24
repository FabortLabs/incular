//! Solid BoxDecoration pixels on a real Windows wgpu surface.
//! Reference: the pinned Flutter elliptical_border_radius_raster_test.dart.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{BorderRadius, BoxDecoration, DecoratedBox, Radius, SizedBox};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const SIDE: u32 = 64;
const SAMPLES: [(u32, u32); 6] = [(0, 0), (1, 10), (62, 10), (1, 53), (62, 53), (32, 32)];
const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn paint(renderer: &mut WgpuRenderer, radius: Option<BorderRadius>) -> [[u8; 4]; 6] {
    let mut decoration = BoxDecoration::new().color(Color::WHITE);
    if let Some(radius) = radius {
        decoration = decoration.border_radius(radius);
    }
    let root = DecoratedBox::new(SizedBox::shrink()).decoration(decoration);
    let mut runtime = Runtime::new(root.into()).unwrap();
    let constraints = Constraints::tight(Size::new(SIDE as f32, SIDE as f32));
    let (list, _) = runtime.run_frame(constraints).unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    SAMPLES.map(|(x, y)| {
        let start = ((y * frame.width + x) * 4) as usize;
        frame.rgba8[start..start + 4].try_into().unwrap()
    })
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_box_decoration_preserves_elliptical_corner_radii_in_raster() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run Windows GPU readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(SIDE, SIDE)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(SIDE, SIDE),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();

    assert_eq!(paint(&mut renderer, None), [WHITE; 6], "square control");
    assert_eq!(
        paint(&mut renderer, Some(BorderRadius::circular(24.))),
        [BLACK, BLACK, BLACK, BLACK, BLACK, WHITE],
        "circular control at {SAMPLES:?}"
    );
    assert_eq!(
        paint(
            &mut renderer,
            Some(BorderRadius::all(Radius::elliptical(24., 8.)))
        ),
        [BLACK, WHITE, WHITE, WHITE, WHITE, WHITE],
        "24×8 elliptical corners at {SAMPLES:?}"
    );
}
