use super::*;
use crate::{
    geometry::tessellate_hairline,
    pipelines::{path_instance_layout, path_vertex_layout},
};
use wgpu::util::DeviceExt;

const SIDE: u32 = 64; // one exactly aligned RGBA8 readback row

#[test]
fn hairline_path_shader_readback_keeps_the_same_single_pixel_coverage_under_scale() {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "wgpu adapter required"
        );
        eprintln!("hairline GPU readback skipped: no adapter");
        return;
    };
    let info = adapter.get_info();
    eprintln!(
        "hairline GPU readback adapter: {} {:?}",
        info.name, info.backend
    );
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hairline production path shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/path_shader.wgsl").into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hairline path readback"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(path_vertex_layout()), Some(path_instance_layout())],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let lut = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("unused gradient lookup for solid hairline"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let sampler = device.create_sampler(&Default::default());
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&lut),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let size = wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hairline readback"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(SIDE * SIDE * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut path = Path::builder();
    path.move_to(Offset::new(6., 0.))
        .line_to(Offset::new(40., 0.));
    let path = path.build();
    let stroke = Stroke {
        width: 0.,
        ..Stroke::default()
    };

    for (transform, scale) in [
        (Transform::translation(Offset::new(0., 32.25)), 1.),
        (
            Transform::translation(Offset::new(0., 16.125))
                .then(Transform::scale_non_uniform(2., 8.)),
            2.,
        ),
    ] {
        let placement = PathPlacement { transform, scale };
        let kind = stroke_mesh_kind(stroke, placement).expect("visible stroke");
        let PathMeshKind::Stroke {
            device_linear: Some(linear),
            ..
        } = kind
        else {
            panic!("zero stroke should plan a hairline mesh");
        };
        let mesh = tessellate_hairline(&path, stroke, hairline_transform(linear)).unwrap();
        let params = path_instance(
            &Brush::Solid(Color::WHITE),
            kind,
            placement,
            [SIDE as f32; 2],
        );
        let buffer = |bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytes,
                usage,
            })
        };
        let vertices = buffer(
            bytemuck::cast_slice(&mesh.gpu_vertices()),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = buffer(
            bytemuck::cast_slice(&mesh.indices),
            wgpu::BufferUsages::INDEX,
        );
        let instances = buffer(bytemuck::bytes_of(&params), wgpu::BufferUsages::VERTEX);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_vertex_buffer(1, instances.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
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
        let (send, receive) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap()
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let alpha = [30, 31, 32, 33].map(|y| mapped[((y * SIDE + 32) * 4 + 3) as usize]);
        drop(mapped);
        readback.unmap();
        for (actual, expected) in alpha.into_iter().zip([0, 64, 191, 0]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "{transform:?} {scale}: {alpha:?}"
            );
        }
    }
}
