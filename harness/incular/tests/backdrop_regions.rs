//! Backdrop blur work stays proportional to what the clip shows: a clipped
//! small `BackdropFilter` captures and blurs about its clip plus the kernel
//! margin, not the window; a large sigma takes the downsample path; stages
//! sharing a `BackdropGroup` capture blur the same pixels as separate
//! captures.
//!
//! Renders through the real WGPU renderer in a native window, so it is
//! opt-in like the pixel harness: `INCULAR_PARITY=1 cargo test -p
//! incular-parity-harness --test backdrop_regions`.

use std::sync::Arc;

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_rendering::GaussianBlur;
use incular_wgpu::{FrameOutcome, GpuCounters, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BackdropFilter, BackdropGroup, BorderRadius, ClipRRect, Container, Directionality, Positioned,
    SizedBox, Stack, TextDirection, Widget,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const LOGICAL: (f32, f32) = (640., 480.);
const SCALE: f64 = 1.5;
const PHYSICAL: (u32, u32) = (960, 720);

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

/// Vertical stripes over the whole window, then `overlays`.
fn striped(overlays: Vec<Widget>) -> Widget {
    let mut children: Vec<Widget> = vec![at(0., 0., LOGICAL.0, LOGICAL.1, solid(Color::BLACK))];
    for index in 0..32 {
        let color = Color::rgba(
            (index * 53 % 255) as u8,
            255 - (index * 29 % 255) as u8,
            90,
            255,
        );
        children.push(at(index as f32 * 20., 0., 10., LOGICAL.1, solid(color)));
    }
    children.extend(overlays);
    Directionality::new(
        TextDirection::Ltr,
        SizedBox::new()
            .width(LOGICAL.0)
            .height(LOGICAL.1)
            .child(Stack::new(children)),
    )
    .into()
}

fn pill(left: f32, top: f32, sigma: f32, grouped: bool) -> Widget {
    let filter = if grouped {
        BackdropFilter::grouped(GaussianBlur::uniform(sigma), SizedBox::expand())
    } else {
        BackdropFilter::blur(sigma, SizedBox::expand())
    };
    at(
        left,
        top,
        120.,
        36.,
        ClipRRect::new(BorderRadius::circular(18.), filter),
    )
}

struct Frame {
    counters: GpuCounters,
    rgba: Vec<u8>,
}

fn delta(after: GpuCounters, before: GpuCounters) -> GpuCounters {
    GpuCounters {
        backdrop_capture_texels: after.backdrop_capture_texels - before.backdrop_capture_texels,
        blur_output_texels: after.blur_output_texels - before.blur_output_texels,
        blur_downsample_passes: after.blur_downsample_passes - before.blur_downsample_passes,
        blur_horizontal_passes: after.blur_horizontal_passes - before.blur_horizontal_passes,
        effect_pass_submissions: after.effect_pass_submissions - before.effect_pass_submissions,
        ..GpuCounters::default()
    }
}

fn render(window: &Arc<Window>, scene: Widget) -> Frame {
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(window.clone()),
        PhysicalSize::new(PHYSICAL.0, PHYSICAL.1),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .expect("create renderer");
    let mut tree = WidgetTree::new();
    tree.mount(scene).expect("mount scene");
    for _ in 0..2 {
        tree.layout(Constraints::tight(Size::new(LOGICAL.0, LOGICAL.1)))
            .expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
    }
    let list = tree.paint();
    let before = renderer.counters();
    renderer.request_capture();
    match renderer.render(&list, SCALE) {
        Ok(FrameOutcome::Presented(_)) => {}
        other => panic!("frame not presented: {other:?}"),
    }
    let counters = delta(renderer.counters(), before);
    let rgba = renderer
        .take_capture()
        .expect("capture requested")
        .expect("capture succeeded")
        .rgba8;
    Frame { counters, rgba }
}

fn summary(counters: &GpuCounters) -> String {
    format!(
        "captured {} texels, blurred {} texels, {} downsample passes, {} submissions",
        counters.backdrop_capture_texels,
        counters.blur_output_texels,
        counters.blur_downsample_passes,
        counters.effect_pass_submissions
    )
}

fn max_difference(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

fn check(window: &Arc<Window>) {
    let window_texels = u64::from(PHYSICAL.0) * u64::from(PHYSICAL.1);
    // A 120x36 pill at 1.5x is 180x54 device pixels; sigma 12 logical is
    // 18 physical, a 1/4 downsample with a 3 sigma + 2 x 4 pixel margin.
    let pill_output = 180 * 54;
    let margin = 54 + 8;
    let pill_input = (180 + 2 * margin) * (54 + 2 * margin);

    let small = render(window, striped(vec![pill(100., 100., 12., false)]));
    eprintln!("clipped pill: {}", summary(&small.counters));
    let output = small.counters.blur_output_texels;
    assert!(
        (pill_output..=182 * 56).contains(&output),
        "a clipped pill blurs about its clip, not the window: {output} texels"
    );
    let captured = small.counters.backdrop_capture_texels;
    assert!(
        captured <= pill_input + 4 * (180 + 54 + 2 * margin) && captured < window_texels / 8,
        "a clipped pill captures its clip plus the kernel margin: {captured} texels"
    );
    assert!(
        small.counters.blur_downsample_passes > 0,
        "sigma 18 physical downsamples"
    );
    assert_eq!(
        small.counters.effect_pass_submissions, 1,
        "the capture and every pass of one backdrop share one submission"
    );

    // Sigma 6 logical is 9 physical: above Impeller's 4, so it downsamples.
    let medium = render(window, striped(vec![pill(100., 100., 6., false)]));
    assert!(
        medium.counters.blur_downsample_passes > 0,
        "sigma 9 physical takes the downsample path"
    );
    // Sigma 2 logical is 3 physical: blurred at full resolution.
    let fine = render(window, striped(vec![pill(100., 100., 2., false)]));
    assert_eq!(fine.counters.blur_downsample_passes, 0);
    assert_eq!(fine.counters.blur_horizontal_passes, 1);

    // Without a clip a backdrop covers the whole window, as in Flutter.
    let unclipped = render(
        window,
        striped(vec![at(
            0.,
            0.,
            LOGICAL.0,
            LOGICAL.1,
            BackdropFilter::blur(12., SizedBox::expand()),
        )]),
    );
    assert_eq!(unclipped.counters.blur_output_texels, window_texels);
    assert_eq!(unclipped.counters.backdrop_capture_texels, window_texels);

    // Two pills far apart in one group share one capture covering both,
    // and blur exactly what two separate captures blur.
    let grouped = render(
        window,
        striped(vec![
            BackdropGroup::new(Stack::new([
                pill(40., 40., 12., true),
                pill(420., 360., 12., true),
            ]))
            .into(),
        ]),
    );
    let separate = render(
        window,
        striped(vec![
            pill(40., 40., 12., false),
            pill(420., 360., 12., false),
        ]),
    );
    eprintln!("grouped: {}", summary(&grouped.counters));
    eprintln!("separate: {}", summary(&separate.counters));
    assert_eq!(
        grouped.counters.blur_output_texels,
        separate.counters.blur_output_texels
    );
    assert!(
        grouped.counters.backdrop_capture_texels > separate.counters.backdrop_capture_texels,
        "the shared capture spans both pills"
    );
    let difference = max_difference(&grouped.rgba, &separate.rgba);
    assert!(
        difference <= 1,
        "a shared capture blurs the same pixels: max difference {difference}"
    );
}

struct Run {
    done: bool,
}

impl ApplicationHandler for Run {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Incular backdrop regions")
                        .with_decorations(false)
                        .with_resizable(false)
                        .with_inner_size(winit::dpi::PhysicalSize::new(PHYSICAL.0, PHYSICAL.1)),
                )
                .expect("create window"),
        );
        check(&window);
        self.done = true;
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    if std::env::var("INCULAR_PARITY").as_deref() != Ok("1") {
        eprintln!("backdrop_regions: skipped; set INCULAR_PARITY=1 (see harness/README.md)");
        return;
    }
    let mut run = Run { done: false };
    EventLoop::new()
        .expect("event loop")
        .run_app(&mut run)
        .expect("run event loop");
    assert!(run.done, "the checks ran");
}
