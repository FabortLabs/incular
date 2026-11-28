//! GPU readback of the shared production soft-clip shader, without a window.
use std::{borrow::Cow, time::Duration};

use incular_core::{Offset, Rect, Size, Transform};
use wgpu::util::DeviceExt;

#[path = "../src/affine_oval.rs"]
mod affine_oval;
use affine_oval::AffineOvalClip;

const SIDE: u32 = 64;
const BYTES_PER_ROW: u32 = SIDE * 4;

fn test_oval() -> (Rect, Transform, f32) {
    (
        Rect::from_origin_size(Offset::new(-18., -8.), Size::new(36., 16.)),
        Transform::translation(Offset::new(24., 24.))
            .then(Transform::rotation(0.65))
            .then(Transform::scale_non_uniform(1.1, 0.85)),
        1.25,
    )
}

#[test]
fn affine_shader_packing_maps_transformed_axes_back_to_unit_coordinates() {
    let (rect, transform, scale) = test_oval();
    let ellipse = AffineOvalClip::new(rect, transform, scale).expect("invertible oval");
    let [cx, cy, width, height] = ellipse.shader_rect(scale);
    assert_eq!([cx, cy], [30., 30.]);
    assert_eq!(ellipse.bounds, transform.transform_rect_bbox(rect));
    assert!(width > height);
    let [a, b, c, d] = ellipse.normalized_inverse;
    for (local, expected) in [
        (Offset::new(18., 0.), (1., 0.)),
        (Offset::new(0., 8.), (0., 1.)),
        (Offset::new(-18., 0.), (-1., 0.)),
    ] {
        let point = transform.transform_point(local);
        let dx = point.x * scale - cx;
        let dy = point.y * scale - cy;
        assert!((a * dx + b * dy - expected.0).abs() < 0.000_01);
        assert!((c * dx + d * dy - expected.1).abs() < 0.000_01);
    }
    assert!(AffineOvalClip::new(rect, Transform::scale_non_uniform(0., 1.), scale).is_none());
}

#[test]
fn affine_oval_edge_has_partial_symmetric_coverage_on_the_gpu() {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "GPU required for affine clip readback"
        );
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let source = concat!(
        include_str!("../src/clip_math.wgsl"),
        include_str!("../src/soft_clip.wgsl"),
        r#"
        @vertex
        fn vertex(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
            let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
            return vec4(points[vertex], 0.0, 1.0);
        }

        @fragment
        fn fragment(@builtin(position) point: vec4<f32>) -> @location(0) vec4<f32> {
            return vec4(clip_weight(point.xy));
        }
        "#
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shared affine clip coverage"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
    });
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("affine clip coverage"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let (rect, transform, scale) = test_oval();
    let ellipse = AffineOvalClip::new(rect, transform, scale).unwrap();
    let mut clips = [0_u32; 24];
    clips[12..16].copy_from_slice(&ellipse.shader_rect(scale).map(f32::to_bits));
    clips[16..20].copy_from_slice(&ellipse.normalized_inverse.map(f32::to_bits));
    clips[21] = 3;
    let clips = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("soft clip stack"),
        contents: bytemuck::cast_slice(&clips),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("target soft clip index"),
        contents: bytemuck::cast_slice(&[1_u32, 0, 0, 0]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let unused = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("unused ordinary shader resources"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[],
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("soft clip inputs"),
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: clips.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: index.as_entire_binding(),
            },
        ],
    });
    let size = wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("affine clip readback target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("affine clip mapped pixels"),
        size: u64::from(SIDE * BYTES_PER_ROW),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &unused, &[]);
        pass.set_bind_group(1, &bindings, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(BYTES_PER_ROW),
                rows_per_image: Some(SIDE),
            },
        },
        size,
    );
    let submission = queue.submit(Some(encoder.finish()));
    let (send, recv) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap()
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(5)),
        })
        .unwrap();
    recv.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap();
    let alpha = |x, y| bytes[((x + y * SIDE) * 4 + 3) as usize];
    let local_sample = |local| {
        let point = transform.transform_point(local);
        alpha(
            (point.x * scale).floor() as u32,
            (point.y * scale).floor() as u32,
        )
    };
    assert_eq!(local_sample(Offset::new(0., 0.)), 255);
    assert_eq!(local_sample(Offset::new(10., 2.)), 255);
    assert_eq!(local_sample(Offset::new(8., 11.)), 0);
    let mut partial = 0;
    for y in 0..60 {
        for x in 0..60 {
            let value = alpha(x, y);
            partial += usize::from((1..=254).contains(&value));
            assert!(
                value.abs_diff(alpha(59 - x, 59 - y)) <= 1,
                "pixel ({x}, {y})"
            );
        }
    }
    assert!(
        partial >= 30,
        "expected an antialiased edge, got {partial} pixels"
    );
}
