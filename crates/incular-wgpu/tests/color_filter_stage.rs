//! The production color-matrix shader executes `ColorFilterStage`s exactly
//! as the renderer-neutral `ColorFilter::apply` reference does: Flutter's
//! `ColorFilter.linearToSrgbGamma`, `ColorFilter.srgbToLinearGamma`
//! (`sky_engine/lib/ui/painting.dart`, `ColorFilter`), and the
//! `ColorFilter.mode` blends no 4x5 matrix expresses.
//!
//! Skips without an adapter unless `INCULAR_WGPU_REQUIRE_GPU=1`.
use std::{borrow::Cow, time::Duration};

use incular_core::Color;
use incular_rendering::{BlendMode, ColorFilter, ColorFilterStage};
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

/// Runs the production color-matrix shader over one row of premultiplied
/// RGBA8 texels.
fn filter(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    filter: ColorFilter,
    pixels: &[u8],
) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: WIDTH,
        height: 1,
        depth_or_array_layers: 1,
    };
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("color filter input"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        source.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(WIDTH * 4),
            rows_per_image: Some(1),
        },
        size,
    );
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("color filter output"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let module: naga::Module = postcard::from_bytes(built_in_shaders::COLOR_MATRIX_SHADER).unwrap();
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production color matrix shader"),
        source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
    });
    let quad_attributes = wgpu::vertex_attr_array![0 => Float32x2];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("color filter stage regression"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &quad_attributes,
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
    // The `GpuColorMatrixParams` layout: 20 matrix floats, the encoded
    // stage (four u32) and the premultiplied blend color.
    let ([stage, mode], color) = filter.stage().encoded();
    let mut params = Vec::with_capacity(112);
    params.extend_from_slice(bytemuck::cast_slice(&filter.to_matrix()));
    params.extend_from_slice(bytemuck::cast_slice(&[stage, mode, 0, 0]));
    params.extend_from_slice(bytemuck::cast_slice(&color));
    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("color filter params"),
        contents: &params,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let source_view = source.create_view(&Default::default());
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("color filter bindings"),
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
                resource: params.as_entire_binding(),
            },
        ],
    });
    let quad: [[f32; 2]; 6] = [[0., 0.], [1., 0.], [0., 1.], [0., 1.], [1., 0.], [1., 1.]];
    let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("color filter quad"),
        contents: bytemuck::cast_slice(&quad),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("color filter readback"),
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
    let pixels = mapped[..(WIDTH * 4) as usize].to_vec();
    drop(mapped);
    readback.unmap();
    pixels
}

/// The reference result for one premultiplied texel.
fn reference(filter: ColorFilter, texel: &[u8]) -> [f32; 4] {
    let [r, g, b, a] = std::array::from_fn::<f32, 4, _>(|index| f32::from(texel[index]) / 255.);
    let straight = if a > 0. {
        [r / a, g / a, b / a, a]
    } else {
        [0.; 4]
    };
    let [r, g, b, a] = filter.apply(straight);
    [r * a, g * a, b * a, a]
}

#[test]
fn color_filter_stages_match_the_reference() {
    let Some((device, queue)) = device() else {
        return;
    };
    // Premultiplied: opaque mid-grey, opaque dark red, half-transparent
    // white, transparent.
    let pixels: [u8; 16] = [
        128, 128, 128, 255, 40, 0, 0, 255, 128, 128, 128, 128, 0, 0, 0, 0,
    ];
    let filters = [
        ColorFilter::linear_to_srgb_gamma(),
        ColorFilter::srgb_to_linear_gamma(),
        ColorFilter::blend(Color::rgba(0, 0, 255, 128), BlendMode::SrcOver),
        ColorFilter::blend(Color::rgba(255, 128, 0, 255), BlendMode::Multiply),
        ColorFilter::blend(Color::rgba(0, 255, 0, 200), BlendMode::Xor),
    ];
    for filter_value in filters {
        assert_ne!(filter_value.stage(), ColorFilterStage::None);
        let output = filter(&device, &queue, filter_value, &pixels);
        for texel in 0..WIDTH as usize {
            let range = texel * 4..texel * 4 + 4;
            let expected = reference(filter_value, &pixels[range.clone()]);
            let actual: [f32; 4] =
                std::array::from_fn(|index| f32::from(output[range.start + index]) / 255.);
            for channel in 0..4 {
                assert!(
                    (expected[channel] - actual[channel]).abs() <= 3. / 255.,
                    "{filter_value:?} texel {texel} channel {channel}: expected {expected:?}, got {actual:?}"
                );
            }
        }
    }
}
