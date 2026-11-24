//! Flutter 3.50.0 raster contracts C11–C14 on a real Windows wgpu surface.
//! The Flutter PictureRecorder reference and measurements are in
//! `audits/flutter-3.50.0-parity/raster-comparisons.md`.
//!
//! Run with `INCULAR_DESKTOP_LIVE_TESTS=1` on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::TransparencyMode;
use incular_core::{Color, Offset, Rect, Size};
use incular_platform::PhysicalSize;
use incular_rendering::{
    BlendMode, Border, Canvas, ColorFilter, FillRule, Paint, Path, RRect, Stroke,
};
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const WHITE: [u8; 4] = [255, 255, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];

#[derive(Clone, Copy)]
enum Scene {
    Border,
    Stroke,
    OrdinaryPath,
    PathDstOver,
    PathColorFilter,
}

impl Scene {
    fn label(self) -> &'static str {
        match self {
            Self::Border => "C11 zero-width rectangular border",
            Self::Stroke => "C12 zero-width stroked path",
            Self::OrdinaryPath => "control: ordinary red path",
            Self::PathDstOver => "C13 path Paint DstOver",
            Self::PathColorFilter => "C14 path Paint SrcIn color filter",
        }
    }

    fn is_hairline(self) -> bool {
        matches!(self, Self::Border | Self::Stroke)
    }

    fn expected_center(self) -> [u8; 4] {
        match self {
            Self::Border | Self::Stroke => WHITE,
            Self::OrdinaryPath => RED,
            Self::PathDstOver => BLUE,
            Self::PathColorFilter => GREEN,
        }
    }

    fn draw(self, canvas: &mut Canvas, rect: Rect, path: &Arc<Path>) {
        let red = Color::rgba(255, 0, 0, 255);
        let blue = Color::rgba(0, 0, 255, 255);
        let green = Color::rgba(0, 255, 0, 255);
        let background = if self.is_hairline() {
            Color::WHITE
        } else {
            blue
        };
        canvas.rect(
            Rect::from_origin_size(Offset::ZERO, Size::new(64., 64.)),
            background,
        );
        match self {
            Self::Border => canvas.border(RRect::uniform(rect, 0.), Border::new(0., red)),
            Self::Stroke => canvas.stroke_path(
                Arc::clone(path),
                red,
                Stroke {
                    width: 0.,
                    ..Stroke::default()
                },
            ),
            Self::OrdinaryPath | Self::PathDstOver | Self::PathColorFilter => {
                let paint = Paint::new().color(red);
                let paint = match self {
                    Self::PathDstOver => paint.blend_mode(BlendMode::DstOver),
                    Self::PathColorFilter => {
                        paint.color_filter(Some(ColorFilter::blend(green, BlendMode::SrcIn)))
                    }
                    _ => paint,
                };
                canvas.draw_path(Arc::clone(path), &paint, FillRule::NonZero);
            }
        }
    }
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before entering the event loop"
)]
fn flutter_350_hairlines_and_path_paint_match_gpu_readback() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 to run Windows GPU readback");
        return;
    }

    // Winit permits one EventLoop per process, so keep all cases in this test.
    let events = EventLoop::builder()
        .with_any_thread(true)
        .build()
        .expect("create the Windows event loop");
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(128, 128)),
        )
        .expect("create the hidden Windows test window");
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(128, 128),
        TransparencyMode::Opaque,
        Color::WHITE,
    ))
    .expect("create the Windows GPU renderer");
    let rect = Rect::from_origin_size(Offset::new(8., 8.), Size::new(48., 48.));
    let mut path = Path::builder();
    path.move_to(Offset::new(8., 8.))
        .line_to(Offset::new(56., 8.))
        .line_to(Offset::new(56., 56.))
        .line_to(Offset::new(8., 56.))
        .close();
    let path = Arc::new(path.build());
    let mut mismatches = Vec::new();

    for dpr in [1_u32, 2] {
        for scene in [
            Scene::OrdinaryPath,
            Scene::Border,
            Scene::Stroke,
            Scene::PathDstOver,
            Scene::PathColorFilter,
        ] {
            let label = scene.label();
            let mut canvas = Canvas::default();
            scene.draw(&mut canvas, rect, &path);
            renderer.request_capture();
            let outcome = renderer
                .render(&canvas.finish(), f64::from(dpr))
                .expect("submit the live parity frame");
            assert!(
                matches!(outcome, FrameOutcome::Presented(_)),
                "{label} at DPR {dpr}: frame not presented"
            );
            let frame = renderer
                .take_capture()
                .expect("a presented frame must have a capture")
                .expect("read the GPU capture");
            let pixel = |x: u32, y: u32| -> [u8; 4] {
                let i = ((y * frame.width + x) * 4) as usize;
                frame.rgba8[i..i + 4].try_into().unwrap()
            };
            let center = pixel(32 * dpr, 32 * dpr);
            let expected = scene.expected_center();
            if center != expected {
                mismatches.push(format!(
                    "{label} at DPR {dpr}: center {center:?}, expected {expected:?}"
                ));
            }
            if scene.is_hairline() {
                let outside = pixel(2 * dpr, 2 * dpr);
                if outside != WHITE {
                    mismatches.push(format!(
                        "{label} at DPR {dpr}: outside {outside:?}, expected {WHITE:?}"
                    ));
                }
                // Sum coverage across the straight top edge rather than pinning
                // how antialiasing divides coverage between adjacent rows.
                let edge_green =
                    std::array::from_fn::<_, 5, _>(|i| pixel(32 * dpr, 8 * dpr - 2 + i as u32)[1]);
                let coverage: f32 = edge_green
                    .into_iter()
                    .map(|green| f32::from(255 - green) / 255.)
                    .sum();
                if (coverage - 1.).abs() > 0.15 {
                    mismatches.push(format!(
                        "{label} at DPR {dpr}: top-edge coverage {coverage} physical px (green bytes {edge_green:?}), expected 1 ± 0.15"
                    ));
                }
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "Flutter 3.50.0 raster mismatches:\n{}",
        mismatches.join("\n")
    );
}
