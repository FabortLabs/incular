//! Actual Windows wgpu raster counterparts to the pinned opacity_raster_test.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{
    Column, Opacity, Padding, Positioned, ScrollController, SingleChildScrollView, SizedBox, Stack,
    Widget,
};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WIDTH: u32 = 100;
const HEIGHT: u32 = 80;
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn capture(renderer: &mut WgpuRenderer, runtime: &mut Runtime) -> Vec<u8> {
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

fn expect_blue(
    label: &str,
    rgba: &[u8],
    expected_blue: impl Fn(u32, u32) -> u8,
    mismatches: &mut Vec<String>,
) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let i = ((y * WIDTH + x) * 4) as usize;
            let expected = expected_blue(x, y);
            if rgba[i] != 0
                || rgba[i + 1] != 0
                || rgba[i + 2].abs_diff(expected) > 1
                || rgba[i + 3] != 255
            {
                mismatches.push(format!(
                    "{label}: first mismatching pixel ({x},{y}) = {:?}, expected [0,0,{expected},255]",
                    &rgba[i..i + 4]
                ));
                return;
            }
        }
    }
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_opacity_offsets_empty_and_isolated_group_render() {
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
    let mut mismatches = Vec::new();

    let scroll = ScrollController::new();
    let rows = (0..10).map(|_| {
        Opacity::new(
            0.5,
            Padding::all(5., Widget::box_(Size::new(90., 50.), BLUE)),
        )
    });
    let viewport = SingleChildScrollView::new(Column::new(rows)).controller(scroll.clone());
    let mut runtime = Runtime::new(viewport.into()).unwrap();
    for offset in [0, 17] {
        if offset != 0 {
            assert!(scroll.jump_to(offset as f32));
        }
        expect_blue(
            &format!("stripes at scroll offset {offset}"),
            &capture(&mut renderer, &mut runtime),
            |x, y| {
                if (5..95).contains(&x) && (5..55).contains(&((y + offset) % 60)) {
                    128
                } else {
                    0
                }
            },
            &mut mismatches,
        );
    }

    let empty = Opacity::new(0.5, SizedBox::shrink());
    expect_blue(
        "empty fractional Opacity",
        &capture(&mut renderer, &mut Runtime::new(empty.into()).unwrap()),
        |_, _| 0,
        &mut mismatches,
    );

    for group in [false, true] {
        let stack = Stack::new([8., 24.].map(|left| {
            let rect = Widget::box_(Size::new(28., 20.), BLUE);
            let child = Opacity::new(if group { 1. } else { 0.5 }, rect);
            Widget::from(Positioned::new(child).left(left).top(10.))
        }));
        let root = if group {
            Opacity::new(0.5, stack).into()
        } else {
            stack.into()
        };
        expect_blue(
            if group {
                "group opacity"
            } else {
                "separate opacity control"
            },
            &capture(&mut renderer, &mut Runtime::new(root).unwrap()),
            |x, y| {
                if !(10..30).contains(&y) || !(8..52).contains(&x) {
                    0
                } else if !group && (24..36).contains(&x) {
                    192
                } else {
                    128
                }
            },
            &mut mismatches,
        );
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
