//! Exercise the embedded effect shaders on an asymmetric texture. Identity
//! passes must preserve both row order and pixel centers, including the edges.
use std::{borrow::Cow, time::Duration};
use wgpu::util::DeviceExt;

#[allow(dead_code)]
#[path = "../src/built_in_shaders.rs"]
mod built_in_shaders;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "GPU required"
        );
        return None;
    };
    Some(pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap())
}

fn identity_pass(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &[u8],
    params: &[u8],
    pixels: &[u8],
) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: 8,
        height: 8,
        depth_or_array_layers: 1,
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("effect sampling regression"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let source = device.create_texture(&descriptor);
    queue.write_texture(
        source.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(32),
            rows_per_image: Some(8),
        },
        size,
    );
    let destination = device.create_texture(&wgpu::TextureDescriptor {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ..descriptor
    });
    let source_view = source.create_view(&Default::default());
    let destination_view = destination.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("identity effect parameters"),
        contents: params,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let module: naga::Module = postcard::from_bytes(shader).unwrap();
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production effect shader"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
    });
    let attributes = wgpu::vertex_attr_array![0 => Float32x2];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("identity effect pass"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
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
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("identity effect bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    });
    let quad: [[f32; 2]; 6] = [[0., 0.], [1., 0.], [0., 1.], [0., 1.], [1., 0.], [1., 1.]];
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("effect quad"),
        contents: bytemuck::cast_slice(&quad),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("effect pixel readback"),
        size: 256 * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &destination_view,
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
        pass.set_bind_group(0, &bindings, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.draw(0..6, 0..1);
    }
    encoder.copy_texture_to_buffer(
        destination.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(8),
            },
        },
        size,
    );
    let submission = queue.submit(Some(encoder.finish()));
    let (send, receive) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap();
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
    let output = mapped
        .as_chunks::<256>()
        .0
        .iter()
        .flat_map(|row| &row[..32])
        .copied()
        .collect();
    drop(mapped);
    readback.unmap();
    output
}

#[test]
fn identity_effects_preserve_orientation_and_pixel_centers() {
    let Some((device, queue)) = device() else {
        return;
    };
    let pixels: Vec<u8> = (0..8)
        .flat_map(|y| (0..8).flat_map(move |x| [x * 29, y * 31, (x + y) * 15, 255]))
        .collect();
    let matrix = incular_rendering::ColorFilter::identity().to_matrix();
    let mut blur = [0_u32; 76];
    for (index, value) in [(2, 8_f32), (3, 8.), (4, 8.), (5, 8.), (6, 1.), (12, 1.)] {
        blur[index] = value.to_bits();
    }
    let mut resample = blur;
    resample[7] = 1_f32.to_bits();
    resample[9] = 1;
    for (name, shader, params) in [
        (
            "color matrix",
            built_in_shaders::COLOR_MATRIX_SHADER,
            bytemuck::cast_slice(&matrix),
        ),
        (
            "blur",
            built_in_shaders::BLUR_SHADER,
            bytemuck::cast_slice(&blur),
        ),
        (
            "resample",
            built_in_shaders::RESAMPLE_SHADER,
            bytemuck::cast_slice(&resample),
        ),
    ] {
        let actual = identity_pass(&device, &queue, shader, params, &pixels);
        for (index, (&actual, &expected)) in actual.iter().zip(&pixels).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 1,
                "{name}: byte {index}: {actual} != {expected}"
            );
        }
    }
}

#[test]
fn resampling_preserves_scaled_edge_coverage_and_pixel_centers() {
    let Some((device, queue)) = device() else {
        return;
    };
    let pixels: Vec<u8> = (0..8)
        .flat_map(|y| (0..8).flat_map(move |x| [x * 29, y * 31, (x + y) * 15, 255]))
        .collect();
    for factor in [0.5_f32, 2.0] {
        let mut params = [0_u32; 76];
        for (index, value) in [
            (2, 8_f32),
            (3, 8.),
            (4, 8.),
            (5, 8.),
            (6, factor),
            (7, factor),
        ] {
            params[index] = value.to_bits();
        }
        params[9] = 1;
        let actual = identity_pass(
            &device,
            &queue,
            built_in_shaders::RESAMPLE_SHADER,
            bytemuck::cast_slice(&params),
            &pixels,
        );
        for y in 0..8 {
            for x in 0..8 {
                let source_x = (x as f32 + 0.5) * factor;
                let source_y = (y as f32 + 0.5) * factor;
                let expected = if source_x >= 8. || source_y >= 8. {
                    [0_u8; 4]
                } else {
                    let cell_x = (source_x - 0.5).clamp(0., 7.);
                    let cell_y = (source_y - 0.5).clamp(0., 7.);
                    [
                        (cell_x * 29.).round() as u8,
                        (cell_y * 31.).round() as u8,
                        ((cell_x + cell_y) * 15.).round() as u8,
                        255,
                    ]
                };
                let pixel = &actual[(y * 8 + x) * 4..][..4];
                for (&actual, expected) in pixel.iter().zip(expected) {
                    assert!(
                        actual.abs_diff(expected) <= 1,
                        "factor {factor}, pixel ({x}, {y}): {pixel:?} != {expected:?}"
                    );
                }
            }
        }
    }
}
