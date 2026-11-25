use super::path_paint;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, GradientStops, SweepGradient, TileMode};
use std::{
    borrow::Cow,
    f32::consts::{FRAC_PI_2, FRAC_PI_4},
    time::Duration,
};
use wgpu::util::DeviceExt;

const SIDE: u32 = 64;
const CLAMP_PIXELS: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [131, 0, 123, 255],
    [0, 0, 255, 255],
    [0, 0, 255, 255],
];
const CLEAR: [u8; 4] = [0; 4];
const POINTS: [(u32, u32); 4] = [(52, 32), (32, 52), (12, 32), (32, 12)];

const TEST_ENTRY: &str = r#"
struct Fixture { grad: vec4<f32>, ext: vec4<f32>, options: vec4<f32> }
@group(0) @binding(2) var<uniform> fixture: Fixture;
@vertex fn test_vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  let p = array(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
  return vec4(p[i], 0., 1.);
}
@fragment fn test_fragment(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
  return gradient_sample(p.xy, fixture.options.x, fixture.grad, fixture.ext);
}
"#;

fn samples(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    lut: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    brush: &Brush,
) -> [[u8; 4]; 4] {
    let (_, gradient, options, ext) = path_paint(brush);
    assert_eq!(options[0], 5.);
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sweep sector brush parameters"),
        contents: bytemuck::cast_slice(&[gradient, ext, options]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sweep sector shader bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(lut),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    });
    let size = wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    };
    let image = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sweep sector image"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = image.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sweep sector readback"),
        size: u64::from(SIDE * SIDE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        render.set_pipeline(pipeline);
        render.set_bind_group(0, &group, &[]);
        render.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        image.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIDE * 4),
                rows_per_image: Some(SIDE),
            },
        },
        size,
    );
    let submission = queue.submit(Some(encoder.finish()));
    let (send, receive) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).unwrap();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    let result = POINTS.map(|(x, y)| {
        let ofs = ((y * SIDE + x) * 4) as usize;
        <[u8; 4]>::try_from(&mapped[ofs..ofs + 4]).unwrap()
    });
    drop(mapped);
    readback.unmap();
    result
}

#[test]
fn sweep_sector_shader_readback_matches_pinned_axis_samples() {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "wgpu adapter required for this readback"
        );
        eprintln!("sweep sector readback skipped: no wgpu adapter");
        return;
    };
    let info = adapter.get_info();
    eprintln!(
        "sweep sector readback adapter: {} {:?}",
        info.name, info.backend
    );
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let lut = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("red-blue gradient LUT"),
        size: wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let data: Vec<u8> = (0..=255_u8).flat_map(|b| [255 - b, 0, b, 255]).collect();
    queue.write_texture(
        lut.as_image_copy(),
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(1024),
            rows_per_image: Some(1),
        },
        lut.size(),
    );
    let lut = lut.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(64., 64.));
    let mut sector = SweepGradient::new(
        Offset::new(32., 32.),
        FRAC_PI_4,
        GradientStops::even(&[Color::rgba(255, 0, 0, 255), Color::rgba(0, 0, 255, 255)]),
    )
    .with_end_angle(3. * FRAC_PI_4);
    let clamp = Brush::SweepGradient(sector.clone());
    sector.tile_mode = TileMode::Decal;
    let decal = Brush::SweepGradient(sector);
    let rotated = decal.rotated(FRAC_PI_2, rect);
    for (name, shader) in [
        ("rectangle", include_str!("shaders/square_rect_shader.wgsl")),
        ("rounded", include_str!("shaders/rrect_shader.wgsl")),
        ("path", include_str!("shaders/path_shader.wgsl")),
    ] {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!("{shader}\n{TEST_ENTRY}"))),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("test_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("test_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        for (brush, expected) in [
            (&clamp, CLAMP_PIXELS),
            (&decal, [CLEAR, CLAMP_PIXELS[1], CLEAR, CLEAR]),
            (&rotated, [CLEAR, CLEAR, CLAMP_PIXELS[1], CLEAR]),
        ] {
            let found = samples(&device, &queue, &pipeline, &lut, &sampler, brush);
            assert!(
                found
                    .iter()
                    .flatten()
                    .zip(expected.iter().flatten())
                    .all(|(a, b)| a.abs_diff(*b) <= 2),
                "{name}: {found:?} != {expected:?}"
            );
        }
    }
}
