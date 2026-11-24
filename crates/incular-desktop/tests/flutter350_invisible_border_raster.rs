//! Windows wgpu pixels for Flutter's BoxDecoration none-style border.
#![cfg(target_os = "windows")]

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Size};
use incular_platform::PhysicalSize;
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{Border, BorderSide, BorderStyle, BoxDecoration, Center, Container, Widget};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

const N: u32 = 60;
const TINT: [u8; 4] = [217, 11, 79, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];
fn scene(style: Option<BorderStyle>) -> Widget {
    let decoration = if let Some(style) = style {
        let [r, g, b, a] = TINT;
        BoxDecoration::new().border(Border::all(BorderSide::new(
            Color::rgba(r, g, b, a),
            2.,
            style,
        )))
    } else {
        BoxDecoration::new()
    };
    Center::new(
        Container::new()
            .width(40.)
            .height(40.)
            .decoration(decoration)
            .child(Widget::box_(Size::new(40., 40.), Color::WHITE)),
    )
    .into()
}

#[test]
#[allow(
    deprecated,
    reason = "create a hidden window before running the native event loop"
)]
fn invisible_border_reserves_two_pixels_without_drawing_a_line() {
    if std::env::var("INCULAR_DESKTOP_LIVE_TESTS").as_deref() != Ok("1") {
        eprintln!("Set INCULAR_DESKTOP_LIVE_TESTS=1 for native Windows readback");
        return;
    }
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(N, N)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(N, N),
        TransparencyMode::Opaque,
        Color::BLACK,
    ))
    .unwrap();
    let mut runtime = Runtime::new(scene(None)).unwrap();
    for (frame, (style, edge)) in [
        (None, WHITE),
        (Some(BorderStyle::Solid), TINT),
        (Some(BorderStyle::None), BLACK),
        (None, WHITE),
    ]
    .into_iter()
    .enumerate()
    {
        if frame != 0 {
            let root = runtime.tree().root().unwrap();
            runtime.schedule_update(root, scene(style)).unwrap();
        }
        let (list, _) = runtime
            .run_frame(Constraints::tight(Size::new(N as f32, N as f32)))
            .unwrap();
        renderer.request_capture();
        assert!(matches!(
            renderer.render(&list, 1.).unwrap(),
            FrameOutcome::Presented(_)
        ));
        let pixels = renderer.take_capture().unwrap().unwrap();
        assert_eq!((pixels.width, pixels.height), (N, N));
        for (x, expected) in [
            (9, BLACK),
            (10, edge),
            (11, edge),
            (12, WHITE),
            (47, WHITE),
            (48, edge),
            (49, edge),
            (50, BLACK),
        ] {
            let offset = ((30 * N + x) * 4) as usize;
            assert_eq!(
                pixels.rgba8[offset..offset + 4],
                expected,
                "style={style:?}, x={x}"
            );
        }
    }
}
