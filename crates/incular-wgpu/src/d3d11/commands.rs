use super::{
    BindGroup, BufferSlice, Device, RenderPipeline, Texture, TextureView, resources::Binding,
};
use ::wgpu::{Color, Extent3d, LoadOp, Operations, TexelCopyBufferLayout, TextureFormat};
use std::{ops::Range, sync::Arc};
use windows::{
    Win32::{
        Foundation::RECT,
        Graphics::{Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, Direct3D11::*, Dxgi::Common::*},
    },
    core::Interface,
};

pub type TexelCopyTextureInfo<'a> = ::wgpu::TexelCopyTextureInfoBase<&'a Texture>;

pub struct RenderPassColorAttachment<'a> {
    pub view: &'a TextureView,
    pub depth_slice: Option<u32>,
    pub resolve_target: Option<&'a TextureView>,
    pub ops: Operations<Color>,
}
pub struct RenderPassDepthStencilAttachment<'a> {
    pub view: &'a TextureView,
    pub depth_ops: Option<Operations<f32>>,
    pub stencil_ops: Option<Operations<u32>>,
}
#[derive(Default)]
pub struct RenderPassDescriptor<'a> {
    pub label: Option<&'a str>,
    pub color_attachments: &'a [Option<RenderPassColorAttachment<'a>>],
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'a>>,
    pub timestamp_writes: Option<()>,
    pub occlusion_query_set: Option<()>,
    pub multiview_mask: Option<std::num::NonZeroU32>,
}
pub struct CommandEncoder {
    device: Device,
    context: ID3D11DeviceContext1,
}
pub struct CommandBuffer {
    commands: ID3D11CommandList,
}
fn region(origin: ::wgpu::Origin3d, size: Extent3d) -> D3D11_BOX {
    D3D11_BOX {
        left: origin.x,
        top: origin.y,
        front: 0,
        right: origin.x + size.width,
        bottom: origin.y + size.height,
        back: 1,
    }
}
fn upload(
    context: &ID3D11DeviceContext1,
    dst: TexelCopyTextureInfo<'_>,
    data: &[u8],
    layout: TexelCopyBufferLayout,
    size: Extent3d,
) {
    let pixel = if dst.texture.format == TextureFormat::R8Unorm {
        1
    } else {
        4
    };
    let pitch = layout.bytes_per_row.unwrap_or(size.width * pixel);
    let data = &data[layout.offset as usize..];
    assert!(
        size.height > 0
            && pitch >= size.width * pixel
            && data.len()
                >= (size.height - 1) as usize * pitch as usize + (size.width * pixel) as usize
    );
    // SAFETY: the checked byte range contains all rows consumed by the texture update.
    unsafe {
        context.UpdateSubresource1(
            &dst.texture.raw,
            0,
            Some(&region(dst.origin, size)),
            data.as_ptr().cast(),
            pitch,
            0,
            0,
        );
    }
}
impl Device {
    pub fn create_command_encoder(
        &self,
        _: &::wgpu::CommandEncoderDescriptor<'_>,
    ) -> CommandEncoder {
        let context = self
            .0
            .spare_encoder
            .lock()
            .expect("encoder pool")
            .take()
            .unwrap_or_else(|| {
                let mut context = None;
                // SAFETY: the returned deferred context belongs to this graphics device.
                unsafe { self.0.raw.CreateDeferredContext(0, Some(&mut context)) }
                    .expect("create D3D11 command context");
                context
                    .expect("created deferred context")
                    .cast()
                    .expect("D3D11.1 context")
            });
        CommandEncoder {
            device: self.clone(),
            context,
        }
    }
    pub fn write_texture(
        &self,
        dst: TexelCopyTextureInfo<'_>,
        data: &[u8],
        layout: TexelCopyBufferLayout,
        size: Extent3d,
    ) {
        upload(&self.0.immediate, dst, data, layout, size);
    }
    pub fn submit(&self, commands: impl IntoIterator<Item = CommandBuffer>) {
        for command in commands {
            // SAFETY: command lists originate from this device and own their resource references.
            unsafe {
                self.0
                    .immediate
                    .ExecuteCommandList(&command.commands, false)
            };
        }
    }
}
impl CommandEncoder {
    pub fn finish(self) -> CommandBuffer {
        let mut list = None;
        // SAFETY: the unique deferred context is closed exactly once.
        unsafe { self.context.FinishCommandList(false, Some(&mut list)) }
            .expect("finish command list");
        *self.device.0.spare_encoder.lock().expect("encoder pool") = Some(self.context);
        CommandBuffer {
            commands: list.expect("created command list"),
        }
    }
    pub fn copy_texture_to_texture(
        &mut self,
        src: TexelCopyTextureInfo<'_>,
        dst: TexelCopyTextureInfo<'_>,
        size: Extent3d,
    ) {
        // SAFETY: retained compositor produces matching formats and in-bounds pixel ranges.
        // SAFETY: Both resources belong to the recording device and the compositor bounds the copied region.
        unsafe {
            self.context.CopySubresourceRegion(
                &dst.texture.raw,
                0,
                dst.origin.x,
                dst.origin.y,
                0,
                &src.texture.raw,
                0,
                Some(&region(src.origin, size)),
            )
        }
    }
    pub fn begin_render_pass<'a>(&'a mut self, d: &RenderPassDescriptor<'_>) -> Pass<'a> {
        debug_assert!(
            d.timestamp_writes.is_none()
                && d.occlusion_query_set.is_none()
                && d.multiview_mask.is_none(),
            "unsupported pass options in {:?}",
            d.label
        );
        let target = d.color_attachments[0]
            .as_ref()
            .expect("renderer color target");
        debug_assert!(target.depth_slice.is_none() && target.resolve_target.is_none());
        let view = &target.view;
        let depth = d.depth_stencil_attachment.as_ref();
        debug_assert!(depth.is_none_or(|d| d.depth_ops.is_none()));
        // SAFETY: views were created for the matching render target and stencil usage. Clear prior bindings before switching targets.
        unsafe {
            self.context.ClearState();
            self.context.OMSetRenderTargets(
                Some(std::slice::from_ref(&view.rtv)),
                depth.and_then(|d| d.view.dsv.as_ref()),
            );
            if let LoadOp::Clear(c) = target.ops.load {
                self.context.ClearRenderTargetView(
                    view.rtv.as_ref().expect("color target"),
                    &[c.r as f32, c.g as f32, c.b as f32, c.a as f32],
                );
            }
            if let Some(d) = depth
                && let Some(Operations {
                    load: LoadOp::Clear(value),
                    ..
                }) = d.stencil_ops
            {
                self.context.ClearDepthStencilView(
                    d.view.dsv.as_ref().expect("depth stencil view"),
                    D3D11_CLEAR_STENCIL.0,
                    0.0,
                    value as u8,
                );
            }
            self.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                Width: view.texture.width as f32,
                Height: view.texture.height as f32,
                MaxDepth: 1.0,
                ..Default::default()
            }]));
            self.context.RSSetScissorRects(Some(&[RECT {
                left: 0,
                top: 0,
                right: view.texture.width as i32,
                bottom: view.texture.height as i32,
            }]));
            self.context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        }
        Pass {
            context: &self.context,
            pipeline: None,
            vertices: Default::default(),
            reference: 0,
        }
    }
}
pub struct Pass<'a> {
    context: &'a ID3D11DeviceContext1,
    pipeline: Option<Arc<super::pipeline::Pipeline>>,
    reference: u32,
    vertices: [Option<(ID3D11Buffer, u32)>; 2],
}
impl Pass<'_> {
    pub fn set_pipeline(&mut self, p: &RenderPipeline) {
        let p = &p.0;
        // SAFETY: all pipeline handles belong to the pass's device and are retained by the command list.
        unsafe {
            self.context.VSSetShader(&p.vs, None);
            self.context.PSSetShader(&p.ps, None);
            self.context.IASetInputLayout(&p.input);
            self.context.OMSetBlendState(&p.blend, None, u32::MAX);
            self.context
                .OMSetDepthStencilState(&p.depth, self.reference);
            self.context.RSSetState(&p.raster);
        }
        let previous = self.pipeline.as_ref().map(|pipeline| pipeline.strides);
        self.pipeline = Some(p.clone());
        for slot in 0..self.vertices.len() {
            if previous.is_none_or(|strides| strides[slot] != p.strides[slot]) {
                self.bind_vertex(slot);
            }
        }
    }
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        // SAFETY: One initialized RECT is borrowed for the call.
        unsafe {
            self.context.RSSetScissorRects(Some(&[RECT {
                left: x as i32,
                top: y as i32,
                right: (x + w) as i32,
                bottom: (y + h) as i32,
            }]))
        }
    }
    pub fn set_stencil_reference(&mut self, r: u32) {
        self.reference = r;
        if let Some(p) = &self.pipeline {
            // SAFETY: The pass retains this depth-stencil state.
            unsafe { self.context.OMSetDepthStencilState(&p.depth, r) }
        }
    }
    pub fn set_vertex_buffer(&mut self, slot: u32, b: BufferSlice<'_>) {
        let slot = slot as usize;
        self.vertices[slot] = b.buffer.raw.clone().map(|buffer| (buffer, b.offset));
        self.bind_vertex(slot);
    }
    fn bind_vertex(&self, slot: usize) {
        if let (Some((buffer, offset)), Some(pipeline)) = (&self.vertices[slot], &self.pipeline) {
            // SAFETY: the one-element buffer, stride and offset arrays are live for this call.
            unsafe {
                self.context.IASetVertexBuffers(
                    slot as u32,
                    1,
                    Some(&Some(buffer.clone())),
                    Some(&pipeline.strides[slot]),
                    Some(offset),
                )
            }
        }
    }
    pub fn set_index_buffer(&mut self, b: BufferSlice<'_>, _: ::wgpu::IndexFormat) {
        // SAFETY: The buffer is retained and its byte offset is aligned for the renderer’s u32 indices.
        unsafe {
            self.context
                .IASetIndexBuffer(b.buffer.raw.as_ref(), DXGI_FORMAT_R32_UINT, b.offset)
        }
    }
    pub fn set_bind_group(&mut self, _: u32, g: &BindGroup, offsets: &[u32]) {
        // SAFETY: bindings reference retained resources; buffer offsets are in 16-byte units in D3D11.1.
        unsafe {
            for (slot, resource) in g.0.iter() {
                match resource {
                    Binding::Texture(v) => self
                        .context
                        .PSSetShaderResources(*slot, Some(std::slice::from_ref(&v.srv))),
                    Binding::Sampler(s) => self
                        .context
                        .PSSetSamplers(*slot, Some(&[Some(s.0.clone())])),
                    Binding::Uniform(b, offset, size, dyn_index) => {
                        let address = offset + dyn_index.map_or(0, |i| offsets[i]);
                        debug_assert!(address + size <= b.size);
                        let uniforms = b
                            .uniforms
                            .as_ref()
                            .expect("uniform binding")
                            .lock()
                            .expect("effect uniforms");
                        let constant = Some(
                            uniforms
                                .get(&address)
                                .expect("effect slot uploaded before drawing")
                                .1
                                .clone(),
                        );
                        self.context
                            .PSSetConstantBuffers(*slot, Some(std::slice::from_ref(&constant)));
                        self.context
                            .VSSetConstantBuffers(*slot, Some(std::slice::from_ref(&constant)));
                    }
                }
            }
        }
    }
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        // SAFETY: All vertex streams and the pipeline were bound on this deferred context.
        unsafe {
            self.context.DrawInstanced(
                vertices.len() as u32,
                instances.len() as u32,
                vertices.start,
                instances.start,
            )
        }
    }
    pub fn draw_indexed(&mut self, indices: Range<u32>, base: i32, instances: Range<u32>) {
        // SAFETY: The retained path mesh supplies the bound index and vertex ranges.
        unsafe {
            self.context.DrawIndexedInstanced(
                indices.len() as u32,
                instances.len() as u32,
                indices.start,
                base,
                instances.start,
            )
        }
    }
}
