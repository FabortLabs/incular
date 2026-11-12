//! Backdrop blur frame cost: a 1920x1080 physical window at 1.5x with four
//! clipped glass pills (`BackdropFilter::blur(12)`) and one rounded acrylic
//! panel (`blur(32)`) over a busy background, against the same background
//! without the filters.
//!
//! Opt-in: `INCULAR_BENCH=1 cargo test --release -p incular-parity-harness --test backdrop_bench`.
//! Each frame requests a capture, whose synchronous readback waits for the
//! GPU to finish the frame, so the wall time of `render` covers the frame's
//! GPU work. Both scenes pay the same readback; their difference is the
//! cost of the backdrop filters.

use std::sync::Arc;
use std::time::{Duration, Instant};

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BackdropFilter, BorderRadius, ClipRRect, Container, Directionality, Positioned, SizedBox,
    Stack, TextDirection, Widget,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const PHYSICAL: (u32, u32) = (1920, 1080);
const SCALE: f64 = 1.5;
const WARMUP: usize = 20;
const FRAMES: usize = 200;

fn at(left: f32, top: f32, width: f32, height: f32, child: impl Into<Widget>) -> Widget {
    Positioned::new(child)
        .left(left)
        .top(top)
        .width(width)
        .height(height)
        .into()
}

fn solid(color: Color) -> Widget {
    Container::new().color(color).into()
}

fn glass(radius: f32, sigma: f32) -> Widget {
    ClipRRect::new(
        BorderRadius::circular(radius),
        BackdropFilter::blur(sigma, solid(Color::rgba(255, 255, 255, 40))),
    )
    .into()
}

/// A stand-in for video: dense colored stripes over the whole window.
fn scene(filters: bool, logical: Size) -> Widget {
    let mut children: Vec<Widget> = vec![at(
        0.,
        0.,
        logical.width,
        logical.height,
        solid(Color::BLACK),
    )];
    for index in 0..64 {
        let x = index as f32 * 20.;
        let color = Color::rgba((index * 37 % 255) as u8, (index * 91 % 255) as u8, 200, 255);
        children.push(at(x, 0., 10., logical.height, solid(color)));
    }
    if filters {
        for pill in 0..4 {
            children.push(at(
                40. + 140. * pill as f32,
                40.,
                120.,
                36.,
                glass(18., 12.),
            ));
        }
        children.push(at(340., 160., 600., 400., glass(24., 32.)));
    }
    Directionality::new(
        TextDirection::Ltr,
        SizedBox::new()
            .width(logical.width)
            .height(logical.height)
            .child(Stack::new(children)),
    )
    .into()
}

struct Bench {
    report: Vec<String>,
}

impl Bench {
    fn run(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Incular backdrop bench")
                        .with_decorations(false)
                        .with_resizable(false)
                        .with_inner_size(winit::dpi::PhysicalSize::new(PHYSICAL.0, PHYSICAL.1)),
                )
                .expect("create window"),
        );
        let logical = Size::new(
            (f64::from(PHYSICAL.0) / SCALE) as f32,
            (f64::from(PHYSICAL.1) / SCALE) as f32,
        );
        for filters in [false, true] {
            let mut renderer = pollster::block_on(WgpuRenderer::new(
                WindowSurfaceTarget::new(window.clone()),
                PhysicalSize::new(PHYSICAL.0, PHYSICAL.1),
                TransparencyMode::Opaque,
                Color::BLACK,
            ))
            .expect("create renderer");
            let mut tree = WidgetTree::new();
            tree.mount(scene(filters, logical)).expect("mount scene");
            for _ in 0..2 {
                tree.layout(Constraints::tight(logical)).expect("layout");
                tree.update_compositor(incular_core::time::Instant::now())
                    .expect("composite");
            }
            let list = tree.paint();
            let mut times = Vec::with_capacity(FRAMES);
            let before = renderer.counters();
            for frame in 0..WARMUP + FRAMES {
                renderer.request_capture();
                let started = Instant::now();
                match renderer.render(&list, SCALE) {
                    Ok(FrameOutcome::Presented(_)) => {}
                    other => panic!("frame not presented: {other:?}"),
                };
                let _ = renderer.take_capture();
                let elapsed = started.elapsed();
                if frame >= WARMUP {
                    times.push(elapsed);
                }
            }
            let after = renderer.counters();
            times.sort_unstable();
            let mean = times.iter().sum::<Duration>() / times.len() as u32;
            let median = times[times.len() / 2];
            let frames = (WARMUP + FRAMES) as u64;
            self.report.push(format!(
                "{:<12} mean {:>7.3} ms  median {:>7.3} ms  effect submits/frame {}  \
                 blur H passes/frame {}  downsample/frame {}  effect texture creations/frame {}  \
                 captured texels/frame {}  blurred texels/frame {}",
                if filters { "backdrops" } else { "background" },
                mean.as_secs_f64() * 1e3,
                median.as_secs_f64() * 1e3,
                (after.effect_pass_submissions - before.effect_pass_submissions) / frames,
                (after.blur_horizontal_passes - before.blur_horizontal_passes) / frames,
                (after.blur_downsample_passes - before.blur_downsample_passes) / frames,
                (after.effect_texture_creations - before.effect_texture_creations) / frames,
                (after.backdrop_capture_texels - before.backdrop_capture_texels) / frames,
                (after.blur_output_texels - before.blur_output_texels) / frames,
            ));
        }
    }
}

impl ApplicationHandler for Bench {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.run(event_loop);
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    if std::env::var("INCULAR_BENCH").as_deref() != Ok("1") {
        eprintln!("backdrop_bench: skipped; set INCULAR_BENCH=1");
        return;
    }
    let mut bench = Bench { report: Vec::new() };
    EventLoop::new()
        .expect("event loop")
        .run_app(&mut bench)
        .expect("run event loop");
    for line in &bench.report {
        eprintln!("{line}");
    }
}
