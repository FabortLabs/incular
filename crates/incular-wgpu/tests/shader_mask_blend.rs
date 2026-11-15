//! The destination-reading blend shader that executes shader masks (and
//! blend groups) must match the renderer-neutral reference formula for the
//! modes Flutter's `ShaderMask` commonly uses. `BlendMode::Modulate` is
//! Flutter's `ShaderMask.blendMode` default (`shader_mask.dart`); before it
//! had its own branch the shader treated it as an unknown artistic mode.
//!
//! Skips without an adapter unless `INCULAR_WGPU_REQUIRE_GPU=1`.
use std::{borrow::Cow, time::Duration};

use incular_rendering::{BlendMode, blend_premultiplied};
use wgpu::util::DeviceExt;

#[allow(dead_code)]
#[path = "../src/built_in_shaders.rs"]
mod built_in_shaders;

const WIDTH: u32 = 4;

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

fn texture(device: &wgpu::Device, queue: &wgpu::Queue, pixels: &[u8]) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blend input"),
        size: wgpu::Extent3d {
            width: WIDTH,
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
    queue.write_texture(
        texture.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WIDTH * 4),
            rows_per_image: Some(1),
        },
        texture.size(),
    );
    texture
}

/// Runs the production blend shader once: `source` (the mask) over
/// `destination` (the child) with `mode`, both premultiplied RGBA8.
fn blend(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mode: BlendMode,
    source: &[u8],
    destination: &[u8],
) -> Vec<u8> {
    let source = texture(device, queue, source);
    let destination = texture(device, queue, destination);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blend output"),
        size: source.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let module: naga::Module = postcard::from_bytes(built_in_shaders::BLEND_SHADER).unwrap();
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production blend shader"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
    });
    let quad_attributes = wgpu::vertex_attr_array![0 => Float32x2];
    let instance_attributes = wgpu::vertex_attr_array![
        1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4
    ];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blend shader regression"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &quad_attributes,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: 80,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance_attributes,
                }),
            ],
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
    let source_view = source.create_view(&Default::default());
    let destination_view = destination.create_view(&Default::default());
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blend bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&destination_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let quad: [[f32; 2]; 6] = [[0., 0.], [1., 0.], [0., 1.], [0., 1.], [1., 0.], [1., 1.]];
    let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("blend quad"),
        contents: bytemuck::cast_slice(&quad),
        usage: wgpu::BufferUsages::VERTEX,
    });
    // rect (NDC), uv, alpha, color, options (mode, target width, height).
    let instance: [f32; 20] = [
        -1.,
        -1.,
        2.,
        2.,
        0.,
        0.,
        1.,
        1.,
        1.,
        0.,
        0.,
        0.,
        1.,
        1.,
        1.,
        1.,
        mode.code() as f32,
        WIDTH as f32,
        1.,
        0.,
    ];
    let instance = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("blend instance"),
        contents: bytemuck::cast_slice(&instance),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("blend readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = output.create_view(&Default::default());
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
        pass.set_bind_group(0, &bindings, &[]);
        pass.set_vertex_buffer(0, quad.slice(..));
        pass.set_vertex_buffer(1, instance.slice(..));
        pass.draw(0..6, 0..1);
    }
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        output.size(),
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
    let pixels = mapped[..(WIDTH * 4) as usize].to_vec();
    drop(mapped);
    readback.unmap();
    pixels
}

fn unit(pixel: &[u8]) -> [f32; 4] {
    std::array::from_fn(|index| f32::from(pixel[index]) / 255.)
}

#[test]
fn shader_mask_blend_modes_match_the_reference_formula() {
    let Some((device, queue)) = device() else {
        return;
    };
    // Mask texels (premultiplied): opaque white, half-transparent white,
    // transparent, opaque red. Child texels: opaque mid-grey, opaque green,
    // opaque blue, half-transparent white.
    let mask: [u8; 16] = [
        255, 255, 255, 255, 128, 128, 128, 128, 0, 0, 0, 0, 255, 0, 0, 255,
    ];
    let child: [u8; 16] = [
        128, 128, 128, 255, 0, 255, 0, 255, 0, 0, 255, 255, 128, 128, 128, 128,
    ];
    for mode in [
        BlendMode::Modulate,
        BlendMode::SrcIn,
        BlendMode::DstIn,
        BlendMode::SrcAtop,
    ] {
        let output = blend(&device, &queue, mode, &mask, &child);
        for texel in 0..WIDTH as usize {
            let range = texel * 4..texel * 4 + 4;
            let expected = blend_premultiplied(
                mode,
                unit(&mask[range.clone()]),
                unit(&child[range.clone()]),
            );
            let actual = unit(&output[range]);
            for channel in 0..4 {
                assert!(
                    (expected[channel] - actual[channel]).abs() <= 2. / 255.,
                    "{mode:?} texel {texel} channel {channel}: expected {expected:?}, got {actual:?}"
                );
            }
        }
    }
}
