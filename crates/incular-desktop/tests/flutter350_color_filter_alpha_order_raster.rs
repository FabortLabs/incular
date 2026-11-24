//! Actual Windows wgpu readback for straight-RGBA ColorFiltered matrix
//! and Opacity nesting. Reference: pinned color_filter_alpha_order_raster_test.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_rendering::ColorFilter;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{ClipRect, ColorFiltered, Opacity, Positioned, SizedBox, Stack, Widget};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const OPAQUE: Color = Color::rgba(64, 96, 128, 255);
const HALF: Color = Color::rgba(64, 96, 128, 128);
const BLACK: [u8; 4] = [0, 0, 0, 255];
const MATTE: [u8; 4] = [0, 32, 0, 255];
const FILTERED: [u8; 4] = [128, 128, 64, 255];
const FILTERED_HALF: [u8; 4] = [64, 64, 32, 255];
const MATRIX: [f32; 20] = [
    0., 0., 1., 0., 0., // R' = B.
    0., 1., 0., 0., 32., // G' = G + 32.
    1., 0., 0., 0., 0., // B' = R.
    0., 0., 0., 0., 255., // A' = 255.
];

fn fill(color: Color) -> Widget {
    Widget::box_(Size::new(32., 24.), color)
}

fn filter(child: impl Into<Widget>) -> Widget {
    ColorFiltered::new(ColorFilter::flutter_matrix(MATRIX), child).into()
}

fn filter_preserving_alpha(child: Widget) -> Widget {
    let mut matrix = MATRIX;
    matrix[18] = 1.;
    matrix[19] = 0.;
    ColorFiltered::new(ColorFilter::flutter_matrix(matrix), child).into()
}

fn capture(renderer: &mut WgpuRenderer, child: Widget) -> Vec<u8> {
    let child = Widget::from(Positioned::new(child).left(8.).top(12.));
    let mut runtime = Runtime::new(Stack::new([child]).into()).unwrap();
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(WIDTH as f32, HEIGHT as f32)))
        .unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));
    frame.rgba8
}

fn first_mismatch(label: &str, rgba: &[u8], expected: [u8; 4], inside: bool) -> Option<String> {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if ((8..40).contains(&x) && (12..36).contains(&y)) != inside {
                continue;
            }
            let index = ((y * WIDTH + x) * 4) as usize;
            let actual = &rgba[index..index + 4];
            if actual.iter().zip(expected).any(|(a, b)| a.abs_diff(b) > 1) {
                return Some(format!(
                    "{label} pixel ({x},{y}) = {actual:?}, expected {expected:?}"
                ));
            }
        }
    }
    None
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_color_filtered_matrix_recovers_alpha_and_respects_opacity_order() {
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
    let cases = [
        (
            "unfiltered translucent",
            fill(HALF),
            [32, 48, 64, 255],
            BLACK,
        ),
        (
            "alpha-preserving matrix",
            filter_preserving_alpha(fill(HALF)),
            FILTERED_HALF,
            BLACK,
        ),
        ("filtered opaque", filter(fill(OPAQUE)), FILTERED, MATTE),
        ("filtered translucent", filter(fill(HALF)), FILTERED, MATTE),
        (
            "filter then opacity",
            Opacity::new(0.5, filter(fill(OPAQUE))).into(),
            FILTERED_HALF,
            BLACK,
        ),
        (
            "opacity then filter",
            filter(Opacity::new(0.5, fill(OPAQUE))),
            FILTERED,
            MATTE,
        ),
        (
            "clipped filter",
            ClipRect::new(filter(fill(OPAQUE))).into(),
            FILTERED,
            BLACK,
        ),
        ("empty filter", filter(SizedBox::shrink()), BLACK, BLACK),
    ];
    let mismatches: Vec<_> = cases
        .into_iter()
        .flat_map(|(label, child, source, backdrop)| {
            let rgba = capture(&mut renderer, child);
            [
                first_mismatch(label, &rgba, source, true),
                first_mismatch(label, &rgba, backdrop, false),
            ]
            .into_iter()
            .flatten()
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
