//! Equivalent gradient shaders sampled on a native Windows wgpu surface.
//! Reference: the pinned Flutter gradient_stop_sampling_test.dart.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::TransparencyMode;
use incular_core::{Color, Offset, Rect, Size};
use incular_platform::PhysicalSize;
use incular_rendering::{
    Brush, Canvas, GradientStop, GradientStops, LinearGradient, RRect, RadialGradient,
    SweepGradient,
};
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const SIDE: u32 = 64;
const CENTER: Offset = Offset::new(32.5, 32.5);
const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const TRANSPARENT_BLUE: Color = Color::rgba(0, 0, 255, 0);
const OPAQUE_CONTROL: [[u8; 4]; 3] = [[64, 0, 191, 255], [128, 0, 128, 255], [191, 0, 64, 255]];
const STRAIGHT_OVER_BLACK: [[u8; 4]; 3] = [[16, 0, 48, 255], [64, 0, 64, 255], [143, 0, 48, 255]];

#[derive(Clone, Copy, Debug)]
enum Geometry {
    Linear,
    Radial,
    Sweep,
}

impl Geometry {
    fn samples(self) -> [(u32, u32); 3] {
        match self {
            Self::Linear => [(20, 32), (32, 32), (44, 32)],
            Self::Radial => [(36, 32), (40, 32), (44, 32)],
            Self::Sweep => [(32, 48), (16, 32), (32, 16)],
        }
    }

    fn brush(self, first: Color) -> Brush {
        let stops = GradientStops::new(vec![
            GradientStop {
                offset: 0.,
                color: first,
            },
            GradientStop {
                offset: 1.,
                color: RED,
            },
        ]);
        match self {
            Self::Linear => Brush::LinearGradient(LinearGradient::new(
                Offset::new(8.5, 32.5),
                Offset::new(56.5, 32.5),
                stops,
            )),
            Self::Radial => Brush::RadialGradient(RadialGradient::new(CENTER, 16., stops)),
            Self::Sweep => Brush::SweepGradient(SweepGradient::new(CENTER, 0., stops)),
        }
    }
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_gradient_stops_render_straight_color_before_compositing() {
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
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(SIDE as f32, SIDE as f32));
    let mut mismatches = Vec::new();
    for geometry in [Geometry::Linear, Geometry::Radial, Geometry::Sweep] {
        for (first, expected) in [
            (BLUE, OPAQUE_CONTROL),
            (TRANSPARENT_BLUE, STRAIGHT_OVER_BLACK),
        ] {
            let mut canvas = Canvas::default();
            canvas.rect(bounds, Color::BLACK);
            canvas.rrect(RRect::uniform(bounds, 0.), geometry.brush(first));
            renderer.request_capture();
            let outcome = renderer.render(&canvas.finish(), 1.).unwrap();
            assert!(matches!(outcome, FrameOutcome::Presented(_)));
            let frame = renderer.take_capture().unwrap().unwrap();
            let actual = geometry.samples().map(|(x, y)| {
                let start = ((y * frame.width + x) * 4) as usize;
                <[u8; 4]>::try_from(&frame.rgba8[start..start + 4]).unwrap()
            });
            let matches = actual
                .iter()
                .flatten()
                .zip(expected.iter().flatten())
                .all(|(actual, expected)| actual.abs_diff(*expected) <= 1);
            if !matches {
                mismatches.push(format!(
                    "{geometry:?} first={first:?}: quarter sample RGBA {actual:?}, expected {expected:?}"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
