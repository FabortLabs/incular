use super::Device;
use crate::pipelines::{PipelineContract, color_target_state, depth_stencil_state, stream_layouts};
use ::wgpu::{BlendFactor, StencilOperation, TextureFormat, VertexFormat, VertexStepMode};
use std::{ffi::CStr, sync::Arc};
use windows::{
    Win32::Graphics::{
        Direct3D::{Fxc::D3DCompile, ID3DBlob},
        Direct3D11::*,
        Dxgi::Common::*,
    },
    core::{PCSTR, Result},
};

#[derive(Clone, Debug)]
pub struct RenderPipeline(pub(super) Arc<Pipeline>);
#[derive(Debug)]
pub(super) struct Pipeline {
    pub vs: ID3D11VertexShader,
    pub ps: ID3D11PixelShader,
    pub input: ID3D11InputLayout,
    pub blend: ID3D11BlendState,
    pub depth: ID3D11DepthStencilState,
    pub raster: ID3D11RasterizerState,
    pub strides: [u32; 2],
}
fn compile(src: &[u8], entry: &CStr, profile: &CStr) -> Result<ID3DBlob> {
    let mut code = None;
    let mut message = None;
    // SAFETY: compiler reads the supplied full source slice; C strings and out pointers remain valid through the call.
    unsafe {
        D3DCompile(
            src.as_ptr().cast(),
            src.len(),
            PCSTR::null(),
            None,
            None,
            PCSTR(entry.as_ptr().cast()),
            PCSTR(profile.as_ptr().cast()),
            0,
            0,
            &mut code,
            Some(&mut message),
        )?;
    }
    Ok(code.expect("compiled shader bytecode"))
}
fn bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: ID3DBlob owns the contiguous immutable compiler output for the duration of this borrow.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}
fn factor(f: BlendFactor) -> D3D11_BLEND {
    match f {
        BlendFactor::One => D3D11_BLEND_ONE,
        BlendFactor::Zero => D3D11_BLEND_ZERO,
        BlendFactor::SrcAlpha => D3D11_BLEND_SRC_ALPHA,
        BlendFactor::OneMinusSrcAlpha => D3D11_BLEND_INV_SRC_ALPHA,
        BlendFactor::DstAlpha => D3D11_BLEND_DEST_ALPHA,
        BlendFactor::OneMinusDstAlpha => D3D11_BLEND_INV_DEST_ALPHA,
        _ => unreachable!("renderer blend factor"),
    }
}
impl Device {
    pub(crate) fn pipeline(
        &self,
        c: &PipelineContract,
        format: TextureFormat,
        stenciled: bool,
        _: Option<&super::BindGroupLayout>,
    ) -> std::result::Result<RenderPipeline, String> {
        self.create_pipeline(c, format, stenciled)
            .map(|pipeline| RenderPipeline(Arc::new(pipeline)))
            .map_err(|error| error.to_string())
    }
    fn create_pipeline(
        &self,
        c: &PipelineContract,
        fmt: TextureFormat,
        stenciled: bool,
    ) -> Result<Pipeline> {
        let (vs_code, ps_code) = {
            let mut shaders = self.0.shaders.lock().expect("shader cache");
            match shaders.entry(c.shader_module_label) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.get().clone(),
                std::collections::hash_map::Entry::Vacant(entry) => entry
                    .insert((
                        compile(c.shader, c"vs_main", c"vs_5_0")?,
                        compile(c.shader, c"fs_main", c"ps_5_0")?,
                    ))
                    .clone(),
            }
        };
        let mut elements = vec![];
        let mut strides = [0; 2];
        for (slot, stream) in stream_layouts(c.streams).into_iter().enumerate() {
            let Some(s) = stream else { continue };
            strides[slot] = s.array_stride as u32;
            for a in s.attributes {
                let instance = s.step_mode == VertexStepMode::Instance;
                elements.push(D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: PCSTR(c"LOC".as_ptr().cast()),
                    SemanticIndex: a.shader_location,
                    Format: match a.format {
                        VertexFormat::Float32 => DXGI_FORMAT_R32_FLOAT,
                        VertexFormat::Float32x2 => DXGI_FORMAT_R32G32_FLOAT,
                        VertexFormat::Float32x4 => DXGI_FORMAT_R32G32B32A32_FLOAT,
                        _ => unreachable!("renderer vertex format"),
                    },
                    InputSlot: slot as u32,
                    AlignedByteOffset: a.offset as u32,
                    InputSlotClass: if instance {
                        D3D11_INPUT_PER_INSTANCE_DATA
                    } else {
                        D3D11_INPUT_PER_VERTEX_DATA
                    },
                    InstanceDataStepRate: u32::from(instance),
                });
            }
        }
        let target = color_target_state(fmt, c.blend, c.color_writes);
        let blend_state = target.blend.unwrap_or(::wgpu::BlendState::REPLACE);
        let mut blend_desc = D3D11_BLEND_DESC::default();
        blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: target.blend.is_some().into(),
            SrcBlend: factor(blend_state.color.src_factor),
            DestBlend: factor(blend_state.color.dst_factor),
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: factor(blend_state.alpha.src_factor),
            DestBlendAlpha: factor(blend_state.alpha.dst_factor),
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: target.write_mask.bits() as u8,
        };
        let ds = depth_stencil_state(c.stencil, stenciled);
        let op = ds
            .as_ref()
            .map_or(StencilOperation::Keep, |s| s.stencil.front.pass_op);
        let face = D3D11_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: match op {
                StencilOperation::Keep => D3D11_STENCIL_OP_KEEP,
                StencilOperation::IncrementClamp => D3D11_STENCIL_OP_INCR_SAT,
                StencilOperation::DecrementClamp => D3D11_STENCIL_OP_DECR_SAT,
                _ => unreachable!("renderer stencil operation"),
            },
            StencilFunc: D3D11_COMPARISON_EQUAL,
        };
        let ds_desc = D3D11_DEPTH_STENCIL_DESC {
            DepthFunc: D3D11_COMPARISON_ALWAYS,
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
            StencilEnable: ds.is_some().into(),
            StencilReadMask: 255,
            StencilWriteMask: 255,
            FrontFace: face,
            BackFace: face,
            ..Default::default()
        };
        let rs_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            DepthClipEnable: true.into(),
            ScissorEnable: true.into(),
            ..Default::default()
        };
        let (mut vs, mut ps, mut input, mut blend, mut depth, mut raster) =
            (None, None, None, None, None, None);
        let dev = &self.0.raw;
        // SAFETY: descriptor arrays, shader blobs and all COM out parameters remain valid for their creation calls.
        unsafe {
            dev.CreateVertexShader(bytes(&vs_code), None, Some(&mut vs))?;
            dev.CreatePixelShader(bytes(&ps_code), None, Some(&mut ps))?;
            dev.CreateInputLayout(&elements, bytes(&vs_code), Some(&mut input))?;
            dev.CreateBlendState(&blend_desc, Some(&mut blend))?;
            dev.CreateDepthStencilState(&ds_desc, Some(&mut depth))?;
            dev.CreateRasterizerState(&rs_desc, Some(&mut raster))?;
        }
        Ok(Pipeline {
            vs: vs.expect("created vertex shader"),
            ps: ps.expect("created pixel shader"),
            input: input.expect("created input layout"),
            blend: blend.expect("created blend state"),
            depth: depth.expect("created depth state"),
            raster: raster.expect("created rasterizer"),
            strides,
        })
    }
}
