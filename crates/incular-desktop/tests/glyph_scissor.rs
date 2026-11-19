//! Native surface readback and instance residency on a long retained paragraph list.
#![cfg(target_os = "windows")]

use incular_config::{Constraints, TransparencyMode};
use incular_core::{Color, Offset, Rect, Size};
use incular_platform::PhysicalSize;
use incular_rendering::{DisplayList, PaintCommand};
use incular_wgpu::{FrameOutcome, WgpuRenderer, WindowSurfaceTarget};
use incular_widgets::{Column, Directionality, Text, TextDirection, Widget, internal::WidgetTree};
use std::sync::Arc;
use winit::{event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows, window::Window};

#[test]
#[ignore = "requires a native Windows GPU surface with readback"]
#[allow(deprecated)]
fn clipped_rows_allocate_only_visible_glyph_instances() {
    let events = EventLoop::builder().with_any_thread(true).build().unwrap();
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(320, 120)),
        )
        .unwrap();
    let mut renderer = pollster::block_on(WgpuRenderer::new(
        WindowSurfaceTarget::new(Arc::new(window)),
        PhysicalSize::new(320, 120),
        TransparencyMode::Opaque,
        Color::WHITE,
    ))
    .unwrap();
    let rows: Vec<Widget> = (0..1000)
        .map(|i| Text::new(format!("Message {i}")).into())
        .collect();
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Column::new(rows)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(320., 20_000.)))
        .unwrap();
    let list = tree.paint();
    // An explicit logical clip to the entire framebuffer is the reference.
    let mut clipped = DisplayList::new();
    clipped.push(PaintCommand::PushClip {
        rect: Rect::from_origin_size(Offset::ZERO, Size::new(320., 120.)),
    });
    clipped.extend_from(&list);
    clipped.push(PaintCommand::PopClip);
    renderer.request_capture();
    renderer.render(&clipped, 1.).unwrap();
    let reference = renderer.take_capture().unwrap().unwrap();
    renderer.request_capture();
    let FrameOutcome::Presented(stats) = renderer.render(&list, 1.).unwrap() else {
        panic!("frame skipped")
    };
    let actual = renderer.take_capture().unwrap().unwrap();
    assert_eq!(actual.rgba8, reference.rgba8);
    assert!((1..256).contains(&stats.glyph_instances), "{stats:?}");
    let instances = renderer
        .memory_usage()
        .into_iter()
        .find(|entry| entry.name == "Instance buffers")
        .unwrap();
    assert!(instances.bytes < 64 * 1024, "{instances:?}");
}
