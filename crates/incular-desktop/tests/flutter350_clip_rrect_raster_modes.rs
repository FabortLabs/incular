//! Native Windows wgpu readback for the pinned ClipRRect raster mode probe.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Clip, Constraints, StackFit, TextDirection, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{ClipRRect, Directionality, Positioned, Stack, Widget};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const SIDE: u32 = 48;
const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[derive(Debug)]
struct Samples {
    corner: [u8; 4],
    center: [u8; 4],
    partial: usize,
    red_fringe: usize,
}

fn capture(renderer: &mut WgpuRenderer, clip: Option<Clip>, paint_red_first: bool) -> Samples {
    let size = Size::new(40., 40.);
    let mut tiles = Vec::new();
    if paint_red_first {
        tiles.push(Widget::box_(size, Color::rgba(255, 0, 0, 255)));
    }
    tiles.push(Widget::box_(size, Color::WHITE));
    let rounded_clip = ClipRRect::new(12., Stack::new(tiles).fit(StackFit::Expand));
    let rounded_clip = match clip {
        None => rounded_clip,
        Some(clip) => rounded_clip.clip_behavior(clip),
    };
    let root = Stack::new([Widget::from(
        Positioned::new(rounded_clip)
            .left(4.)
            .top(4.)
            .width(40.)
            .height(40.),
    )]);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, root).into()).unwrap();
    let (display_list, _) = runtime
        .run_frame(Constraints::tight(Size::new(SIDE as f32, SIDE as f32)))
        .unwrap();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&display_list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    assert_eq!((frame.width, frame.height), (SIDE, SIDE));
    let pixel = |x, y| -> [u8; 4] {
        let index = ((y * SIDE + x) * 4) as usize;
        frame.rgba8[index..index + 4].try_into().unwrap()
    };
    let corner_pixels = (4..16).flat_map(|y| (4..16).map(move |x| pixel(x, y)));
    Samples {
        corner: pixel(4, 4),
        center: pixel(24, 24),
        partial: corner_pixels
            .clone()
            .filter(|p| p[1] > 0 && p[1] < 255)
            .count(),
        red_fringe: corner_pixels
            .filter(|p| u16::from(p[0]) > u16::from(p[1]) + 1)
            .count(),
    }
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_clip_rrect_modes_distinguish_aa_and_saved_compositing() {
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

    let none = capture(&mut renderer, Some(Clip::None), false);
    assert_eq!((none.corner, none.center), (WHITE, WHITE), "clip none");
    let hard = capture(&mut renderer, Some(Clip::HardEdge), false);
    let default_clip = capture(&mut renderer, None, false);
    let ordinary_overlap = capture(&mut renderer, None, true);
    let saved_overlap = capture(&mut renderer, Some(Clip::AntiAliasWithSaveLayer), true);
    for samples in [&hard, &default_clip, &ordinary_overlap, &saved_overlap] {
        assert_eq!((samples.corner, samples.center), (BLACK, WHITE));
    }
    assert_eq!(hard.partial, 0, "hardEdge has no partially covered pixels");
    assert!(
        default_clip.partial > 0
            && ordinary_overlap.partial > 0
            && ordinary_overlap.red_fringe > 0
            && saved_overlap.partial > 0
            && saved_overlap.red_fringe == 0,
        "default should smooth edges and permit red fringe when painting twice; \
         saveLayer should smooth edges without that fringe. Got default={default_clip:?}, \
         ordinary_overlap={ordinary_overlap:?}, saved_overlap={saved_overlap:?}"
    );
}
