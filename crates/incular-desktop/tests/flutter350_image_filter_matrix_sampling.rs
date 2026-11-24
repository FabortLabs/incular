//! Native Windows wgpu readback of the pinned ImageFiltered matrix sampling probe.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Constraints, TextDirection, TransparencyMode};
use incular_core::{Color, Size, Transform};
use incular_platform::PhysicalSize;
use incular_rendering::ImageFilter;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{Directionality, ImageFiltered, Positioned, Row, Stack, Widget};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WIDTH: u32 = 40;
const HEIGHT: u32 = 32;
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

fn sample<const N: usize>(
    renderer: &mut WgpuRenderer,
    scale: f32,
    enabled: bool,
    probes: [u32; N],
) -> [[u8; 4]; N] {
    let tile_size = Size::new(4., 8.);
    let tiles = Row::new([
        Widget::box_(tile_size, Color::rgba(255, 0, 0, 255)),
        Widget::box_(tile_size, Color::rgba(0, 0, 255, 255)),
    ])
    .main_axis_size(incular_config::MainAxisSize::Min);
    let filtered = ImageFiltered::new(ImageFilter::matrix(Transform::scale(scale)), tiles)
        .unwrap()
        .enabled(enabled);
    let root = Stack::new([Widget::from(Positioned::new(filtered).left(8.).top(8.))]);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, root).into()).unwrap();
    let size = Size::new(WIDTH as f32, HEIGHT as f32);
    let (list, _) = runtime.run_frame(Constraints::tight(size)).unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));
    probes.map(|x| {
        let start = ((12 * WIDTH + x) * 4) as usize;
        frame.rgba8[start..start + 4].try_into().unwrap()
    })
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_image_filter_matrix_default_sampling_blends_enlarged_edges() {
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

    let original_probes = [8, 10, 11, 12, 15, 16];
    let original = [RED, RED, RED, BLUE, BLUE, BLACK];
    assert_eq!(
        sample(&mut renderer, 1., true, original_probes),
        original,
        "identity"
    );
    assert_eq!(
        sample(&mut renderer, 2., false, original_probes),
        original,
        "disabled"
    );

    let probes = [12, 15, 16, 19, 25];
    let actual = sample(&mut renderer, 2., true, probes);
    assert_eq!(
        [actual[0], actual[3], actual[4]],
        [RED, BLUE, BLACK],
        "scaled tile extents"
    );
    let expected = [RED, [191, 0, 64, 255], [64, 0, 191, 255], BLUE, BLACK];
    assert!(
        actual
            .iter()
            .flatten()
            .zip(expected.iter().flatten())
            .all(|(a, b)| a.abs_diff(*b) <= 1),
        "pixel samples at x={probes:?}: {actual:?}, expected {expected:?}"
    );
}
