//! Headless coverage readback for the production rounded-rectangle shader.
use std::time::Duration;

use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Border, RRect};
use wgpu::util::DeviceExt;

use crate::built_in_shaders::{RRECT_SHADER, SQUARE_RECT_SHADER, decode};
use crate::pipelines::{VertexStreams, stream_layouts};
use crate::render_plan::border_instance;

const SIDE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn a_subpixel_rounded_border_does_not_leave_a_stronger_edge_than_flutter() {
    check_shader(
        RRECT_SHADER,
        8.,
        &[
            (2., [0, 192, 255, 64, 0, 0]),
            (0.03125, [2, 5, 0, 0, 0, 0]),
            (0.001, [0, 0, 0, 0, 0, 0]),
            (16., [0, 191, 255, 255, 255, 255]),
        ],
    );
}

#[test]
fn the_subpixel_square_border_keeps_its_own_straight_edge_coverage() {
    check_shader(
        SQUARE_RECT_SHADER,
        0.,
        &[(2., [0, 192, 255, 64, 0, 0]), (0.03125, [0, 8, 0, 0, 0, 0])],
    );
}

fn check_shader(shader: &'static [u8], radius: f32, cases: &[(f32, [u8; 6])]) {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "GPU required for rounded border readback"
        );
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production rounded border readback"),
        source: decode(shader),
    });
    let targets = [Some(wgpu::ColorTargetState {
        format: FORMAT,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rounded border alpha"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &stream_layouts(VertexStreams::RRectInstances),
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    });
    let size = wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rounded border offscreen pixels"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());
    let unused_gradient = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("unused gradient in solid-border test"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rounded border gradient bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&unused_gradient),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("unit quad"),
        contents: bytemuck::cast_slice(&[
            [0f32, 0.],
            [1., 0.],
            [0., 1.],
            [0., 1.],
            [1., 0.],
            [1., 1.],
        ]),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rounded border RGBA readback"),
        size: u64::from(SIDE * SIDE * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bounds = Rect::from_origin_size(Offset::new(16.25, 16.25), Size::new(32., 32.));
    let rrect = RRect::uniform(bounds, radius);
    // The pinned Flutter Canvas test gives these alpha ramps. R8 rounding
    // and Flutter antialias quantization need not choose the same last bit.
    for &(width, expected) in cases {
        let params = border_instance(
            rrect,
            Border::new(width, Color::BLACK),
            Transform::IDENTITY,
            1.,
            SIDE as f32,
            SIDE as f32,
        );
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("production rounded border instance"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rounded border raster"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
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
            pass.set_vertex_buffer(1, vertices.slice(..));
            pass.draw(0..6, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
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
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
        let pixels = readback.slice(..).get_mapped_range().unwrap();
        let alpha = |x, y| pixels[((y * SIDE + x) * 4 + 3) as usize];
        let actual = [15, 16, 17, 18, 19, 32].map(|x| alpha(x, 32));
        if radius > 0. && width == 0.03125 {
            assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] <= 12));
            // Stay close to Flutter on the curve too. The shader estimates
            // the normal from a pixel quad; Skia's curve AA uses a
            // different sampling pattern from the straight part of the rim.
            let corners = [(18, 18), (17, 18), (19, 18), (18, 17), (18, 19), (19, 19)]
                .map(|(x, y)| alpha(x, y));
            for (got, want) in corners.into_iter().zip([4, 0, 3, 0, 3, 0]) {
                assert!(got.abs_diff(want) <= 3, "thin corner alpha: {corners:?}");
            }
        }
        if width == 0.001 {
            assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] <= 1));
        }
        drop(pixels);
        readback.unmap();
        for (got, reference) in actual.into_iter().zip(expected) {
            assert!(
                got.abs_diff(reference) <= 1,
                "width {width}: native={actual:?}, reference={expected:?}"
            );
        }
    }
}
