//! Offscreen readback of the same wgpu pipelines and packed geometry used
//! by the renderer. The target starts with the child's pixels, then the
//! shader blend is constrained by the mask quad independently of alpha.

use super::lowering::{shader_mask_fill_instance, shader_mask_needs_stencil};
use crate::{
    compositor::scissor_rect,
    pipelines::{
        SharedPipelineResources, create_shared_pipeline_resources, create_stencil_attachment,
    },
    render_plan::{ClipRect, ClipState, blend_instance},
};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{BlendMode, Brush, CornerRadii, RRect};
use std::time::Duration;
use wgpu::util::DeviceExt;

const SIDE: u32 = 200;
const PITCH: u32 = 1024;
const TARGET: Size = Size {
    width: SIDE as f32,
    height: SIDE as f32,
};
const EXTENT: wgpu::Extent3d = wgpu::Extent3d {
    width: SIDE,
    height: SIDE,
    depth_or_array_layers: 1,
};
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: EXTENT,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn instance_buffer<T: bytemuck::Pod>(device: &wgpu::Device, instance: &T) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(instance),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

fn square(rect: Rect) -> RRect {
    RRect::new(rect, CornerRadii::ZERO)
}

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &SharedPipelineResources,
    angle: f32,
    color: Color,
    clip_to_quad: bool,
) -> [[u8; 4]; 5] {
    let mask_rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    let transform = Transform::translation(Offset::new(100., 100.))
        .then(Transform::rotation(angle))
        .then(Transform::translation(Offset::new(-50., -50.)));
    let bounds = transform.transform_rect_bbox(mask_rect);
    let needs_stencil = shader_mask_needs_stencil(bounds, transform, 1.);
    assert_eq!(needs_stencil, angle != 0.);
    let stenciled = needs_stencil && clip_to_quad;
    let mask = instance_buffer(
        device,
        &shader_mask_fill_instance(
            square(mask_rect),
            &Brush::Solid(color),
            transform,
            1.,
            TARGET,
        ),
    );
    // The white child paints 20 px past the mask's left edge.
    let child_rect = Rect::from_origin_size(Offset::new(-20., 0.), Size::new(120., 100.));
    let child = instance_buffer(
        device,
        &shader_mask_fill_instance(
            square(child_rect),
            &Brush::Solid(Color::WHITE),
            transform,
            1.,
            TARGET,
        ),
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    let draw_source = |encoder: &mut wgpu::CommandEncoder, buffer: &wgpu::Buffer| {
        let target = texture(device);
        let view = target.create_view(&Default::default());
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
        pass.set_pipeline(pipelines.square_rect_pipeline.get_for(false).unwrap());
        pass.set_bind_group(0, &pipelines.solid_gradient.bind_group, &[]);
        pass.set_bind_group(1, &pipelines.no_soft_clip.group, &[0]);
        pass.set_vertex_buffer(0, pipelines.mesh.slice(..));
        pass.set_vertex_buffer(1, buffer.slice(..));
        pass.draw(0..6, 0..1);
        drop(pass);
        target
    };
    let source = draw_source(&mut encoder, &mask);
    let destination = draw_source(&mut encoder, &child);
    let output = texture(device);
    encoder.copy_texture_to_texture(destination.as_image_copy(), output.as_image_copy(), EXTENT);
    let source_view = source.create_view(&Default::default());
    let destination_view = destination.create_view(&Default::default());
    let view = output.create_view(&Default::default());
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipelines.blend_bind_group_layout,
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
                resource: wgpu::BindingResource::Sampler(&pipelines.blend_sampler),
            },
        ],
    });
    let placed = crate::render_plan::Placed::new(Offset::ZERO, SIDE, SIDE);
    let blend = instance_buffer(
        device,
        &blend_instance(placed, placed, 1., BlendMode::Modulate),
    );
    let stencil = stenciled.then(|| create_stencil_attachment(device, SIDE, SIDE));
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: stencil.as_ref().map(|(_, view)| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }
            }),
            ..Default::default()
        });
        pass.set_vertex_buffer(0, pipelines.mesh.slice(..));
        if stenciled {
            pass.set_stencil_reference(0);
            pass.set_pipeline(
                pipelines
                    .stencil_rrect_increment_pipeline
                    .get_for(true)
                    .unwrap(),
            );
            pass.set_bind_group(0, &pipelines.solid_gradient.bind_group, &[]);
            pass.set_vertex_buffer(1, mask.slice(..));
            pass.draw(0..6, 0..1);
            pass.set_stencil_reference(1);
        }
        let [x, y, width, height] =
            scissor_rect(ClipState::Rect(ClipRect::hard(bounds)), SIDE, SIDE, 1.).unwrap();
        pass.set_scissor_rect(x, y, width, height);
        pass.set_pipeline(pipelines.blend_pipeline.get_for(stenciled).unwrap());
        pass.set_bind_group(0, &bindings, &[]);
        pass.set_vertex_buffer(1, blend.slice(..));
        pass.draw(0..6, 0..1);
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(PITCH * SIDE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PITCH),
                rows_per_image: Some(SIDE),
            },
        },
        EXTENT,
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
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap();
    let sample = |x: u32, y: u32| {
        let offset = (y * PITCH + x * 4) as usize;
        bytes[offset..offset + 4].try_into().unwrap()
    };
    let [beyond, overflow, covered] = [-30., -15., 50.].map(|x| {
        let world = transform.transform_point(Offset::new(x, 50.));
        sample(world.x.floor() as u32, world.y.floor() as u32)
    });
    let (outside, inside) = if angle == 0. {
        (sample(49, 100), sample(50, 100))
    } else {
        (sample(64, 64), sample(64, 65))
    };
    [beyond, overflow, covered, outside, inside]
}

#[test]
fn shader_mask_wgpu_readback_preserves_rotated_overflow_without_alpha_as_coverage() {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        assert_ne!(
            std::env::var("INCULAR_WGPU_REQUIRE_GPU").as_deref(),
            Ok("1")
        );
        eprintln!("shader-mask raster readback skipped: no adapter");
        return;
    };
    let info = adapter.get_info();
    eprintln!(
        "shader-mask raster readback: {} {:?}",
        info.name, info.backend
    );
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let pipelines =
        pollster::block_on(create_shared_pipeline_resources(&device, &queue, FORMAT)).unwrap();
    let red = Color::rgba(255, 0, 0, 255);
    let white = [255; 4];
    let r = [255, 0, 0, 255];
    let clear = [0; 4];
    let rotation = std::f32::consts::FRAC_PI_4;
    assert_eq!(
        capture(&device, &queue, &pipelines, 0., red, true),
        [clear, white, r, white, r],
    );
    assert_eq!(
        capture(&device, &queue, &pipelines, rotation, red, true),
        [clear, white, r, white, r],
    );
    assert_eq!(
        capture(
            &device,
            &queue,
            &pipelines,
            rotation,
            Color::TRANSPARENT,
            true
        ),
        [clear, white, clear, white, clear],
    );
    // Run the old AABB-only blend too: overflow falls inside its scissor,
    // so a transparent source incorrectly destroys those white pixels.
    assert_eq!(
        capture(&device, &queue, &pipelines, rotation, red, false),
        [clear, clear, r, clear, r],
    );
}
