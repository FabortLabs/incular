//! Actual Windows wgpu readback for straight-RGBA ColorFiltered matrix
//! and Opacity nesting. Reference: pinned color_filter_alpha_order_raster_test.
//! Run with INCULAR_DESKTOP_LIVE_TESTS=1 on a Windows GPU desktop.
#![cfg(target_os = "windows")]

use std::sync::Arc;

use incular_config::{Clip, Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_rendering::{BlendMode, ColorFilter};
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::internal::Blend;
use incular_widgets::{
    ClipRRect, ClipRect, ColorFiltered, Opacity, Padding, Positioned, SizedBox, Stack, Widget,
};
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

fn capture(renderer: &mut WgpuRenderer, runtime: &mut Runtime, child: Widget) -> Vec<u8> {
    let child = Widget::from(Positioned::new(child).left(8.).top(12.));
    // Layer identities are local to a retained tree. Reconcile each case
    // in one runtime so unrelated test trees cannot alias renderer caches.
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, Stack::new([child]).into())
        .unwrap();
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

fn first_mismatch(
    label: &str,
    rgba: &[u8],
    foreground: [u8; 4],
    backdrop: [u8; 4],
    clip_padding: u32,
) -> Option<String> {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let inside = (8 + clip_padding..40 + clip_padding).contains(&x)
                && (12 + clip_padding..36 + clip_padding).contains(&y);
            let outside_clip = clip_padding != 0
                && !((8..40 + 2 * clip_padding).contains(&x)
                    && (12..36 + 2 * clip_padding).contains(&y));
            let expected = if inside {
                foreground
            } else if outside_clip {
                BLACK
            } else {
                backdrop
            };
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
    let mut runtime = Runtime::new(Stack::new(Vec::<Widget>::new()).into()).unwrap();
    let mut half_alpha = MATRIX;
    half_alpha[19] = 128.;
    let padded_clip = ClipRect::new(Padding::all(
        4.,
        ColorFiltered::new(ColorFilter::flutter_matrix(half_alpha), fill(OPAQUE)),
    ));
    let cases = [
        (
            "unfiltered translucent",
            fill(HALF),
            [32, 48, 64, 255],
            BLACK,
            0,
        ),
        (
            "alpha-preserving matrix",
            filter_preserving_alpha(fill(HALF)),
            FILTERED_HALF,
            BLACK,
            0,
        ),
        ("filtered opaque", filter(fill(OPAQUE)), FILTERED, MATTE, 0),
        (
            "filtered translucent",
            filter(fill(HALF)),
            FILTERED,
            MATTE,
            0,
        ),
        (
            "filter then opacity",
            Opacity::new(0.5, filter(fill(OPAQUE))).into(),
            FILTERED_HALF,
            BLACK,
            0,
        ),
        (
            "opacity then filter",
            filter(Opacity::new(0.5, fill(OPAQUE))),
            FILTERED,
            MATTE,
            0,
        ),
        (
            "clipped filter",
            ClipRect::new(filter(fill(OPAQUE))).into(),
            FILTERED,
            BLACK,
            0,
        ),
        (
            "clip larger than translucent matte",
            padded_clip.into(),
            FILTERED_HALF,
            [0, 16, 0, 255],
            4,
        ),
        ("empty filter", filter(SizedBox::shrink()), BLACK, BLACK, 0),
    ];
    let mut mismatches: Vec<_> = cases
        .into_iter()
        .filter_map(|(label, child, source, backdrop, clip_padding)| {
            let rgba = capture(&mut renderer, &mut runtime, child);
            first_mismatch(label, &rgba, source, backdrop, clip_padding)
        })
        .collect();
    let pigment = Color::rgba(200, 40, 90, 160);
    for (mode, opaque, translucent) in [
        (BlendMode::Hue, [148, 94, 142, 255], [137, 60, 99, 255]),
        (
            BlendMode::Saturation,
            [70, 121, 211, 255],
            [98, 73, 134, 255],
        ),
        (BlendMode::Color, [169, 84, 141, 255], [147, 55, 99, 255]),
        (
            BlendMode::Luminosity,
            [66, 106, 176, 255],
            [96, 66, 117, 255],
        ),
    ] {
        for (destination, expected) in [
            (Color::rgba(80, 120, 190, 255), opaque),
            (Color::rgba(80, 120, 190, 128), translucent),
        ] {
            let color_filter = ClipRect::new(ColorFiltered::new(
                ColorFilter::blend(pigment, mode),
                fill(destination),
            ));
            // Isolate the destination from the black window background.
            let direct = ClipRRect::new(
                0.,
                Stack::new([fill(destination), Blend::new(mode, fill(pigment)).into()]),
            )
            .clip_behavior(Clip::AntiAliasWithSaveLayer);
            for (label, widget) in [
                ("ColorFiltered", Widget::from(color_filter)),
                ("direct blend", Widget::from(direct)),
            ] {
                let rgba = capture(&mut renderer, &mut runtime, widget);
                let label = format!("{label} {mode:?} over {destination:?}");
                mismatches.extend(first_mismatch(&label, &rgba, expected, BLACK, 0));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
