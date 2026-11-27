use super::{BufferUsages, Device, TextureFormat, TextureUsages};
use std::{
    ops::{Bound, RangeBounds},
    sync::{Arc, Mutex},
};
use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*};
pub(super) fn format(f: TextureFormat) -> DXGI_FORMAT {
    match f {
        TextureFormat::Rgba8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
        TextureFormat::Bgra8Unorm => DXGI_FORMAT_B8G8R8A8_UNORM,
        TextureFormat::Rgba8UnormSrgb => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        TextureFormat::Bgra8UnormSrgb => DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        TextureFormat::R8Unorm => DXGI_FORMAT_R8_UNORM,
        TextureFormat::Depth24PlusStencil8 => DXGI_FORMAT_D24_UNORM_S8_UINT,
        _ => unreachable!("renderer texture format"),
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub(super) raw: ID3D11Texture2D,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) format: TextureFormat,
    usage: TextureUsages,
}
impl Texture {
    pub(super) fn surface(raw: ID3D11Texture2D, width: u32, height: u32) -> Self {
        Self {
            raw,
            width,
            height,
            format: TextureFormat::Bgra8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
        }
    }
    pub fn create_view(&self, _: &::wgpu::TextureViewDescriptor<'_>) -> TextureView {
        let mut v = TextureView {
            texture: self.clone(),
            srv: None,
            rtv: None,
            dsv: None,
        };
        // SAFETY: each view matches its texture's creation flags and uses the native format.
        unsafe {
            let dev = self.raw.GetDevice().expect("live texture has a device");
            if self.usage.contains(TextureUsages::TEXTURE_BINDING) {
                dev.CreateShaderResourceView(&self.raw, None, Some(&mut v.srv))
                    .expect("sampled texture");
            }
            if self.usage.contains(TextureUsages::RENDER_ATTACHMENT) {
                if self.format == TextureFormat::Depth24PlusStencil8 {
                    dev.CreateDepthStencilView(&self.raw, None, Some(&mut v.dsv))
                        .expect("stencil view");
                } else {
                    dev.CreateRenderTargetView(&self.raw, None, Some(&mut v.rtv))
                        .expect("color view");
                }
            }
        }
        v
    }
}
#[derive(Clone, Debug)]
pub struct TextureView {
    pub(super) texture: Texture,
    pub(super) srv: Option<ID3D11ShaderResourceView>,
    pub(super) rtv: Option<ID3D11RenderTargetView>,
    pub(super) dsv: Option<ID3D11DepthStencilView>,
}
type UniformSlots = std::collections::BTreeMap<u32, (usize, ID3D11Buffer)>;
#[derive(Clone, Debug)]
pub struct Buffer {
    pub(super) raw: Option<ID3D11Buffer>,
    pub(super) size: u32,
    pub(super) uniforms: Option<Arc<Mutex<UniformSlots>>>,
}
impl Buffer {
    pub fn size(&self) -> u64 {
        u64::from(self.size)
    }
    pub fn slice<R: RangeBounds<u64>>(&self, r: R) -> BufferSlice<'_> {
        let offset = match r.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        BufferSlice {
            buffer: self,
            offset: u32::try_from(offset).expect("D3D11 buffer offset"),
        }
    }
    pub fn as_entire_binding(&self) -> BindingResource<'_> {
        BindingResource::Buffer(BufferBinding {
            buffer: self,
            offset: 0,
            size: None,
        })
    }
}
pub struct BufferSlice<'a> {
    pub(super) buffer: &'a Buffer,
    pub(super) offset: u32,
}
#[derive(Clone, Debug)]
pub struct Sampler(pub(super) ID3D11SamplerState);
#[derive(Clone, Debug)]
pub struct BindGroupLayout(Vec<u32>);
pub struct BufferBinding<'a> {
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub size: Option<::wgpu::BufferSize>,
}
pub enum BindingResource<'a> {
    Sampler(&'a Sampler),
    TextureView(&'a TextureView),
    Buffer(BufferBinding<'a>),
}
pub struct BindGroupEntry<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}
pub struct BindGroupDescriptor<'a> {
    pub label: Option<&'a str>,
    pub layout: &'a BindGroupLayout,
    pub entries: &'a [BindGroupEntry<'a>],
}
#[derive(Clone, Debug)]
pub(super) enum Binding {
    Sampler(Sampler),
    Texture(TextureView),
    Uniform(Buffer, u32, u32, Option<usize>),
}
#[derive(Clone, Debug)]
pub struct BindGroup(pub(super) Arc<[(u32, Binding)]>);
impl Device {
    pub fn create_texture(&self, d: &::wgpu::TextureDescriptor<'_>) -> Texture {
        let mut flags = 0;
        if d.usage.contains(TextureUsages::TEXTURE_BINDING) {
            flags |= D3D11_BIND_SHADER_RESOURCE.0;
        }
        if d.usage.contains(TextureUsages::RENDER_ATTACHMENT) {
            flags |= if d.format == TextureFormat::Depth24PlusStencil8 {
                D3D11_BIND_DEPTH_STENCIL
            } else {
                D3D11_BIND_RENDER_TARGET
            }
            .0;
        }
        let desc = D3D11_TEXTURE2D_DESC {
            Width: d.size.width,
            Height: d.size.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format(d.format),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: flags as u32,
            ..Default::default()
        };
        let mut raw = None;
        // SAFETY: initialized descriptor and owned output slot, with no initial pointer.
        unsafe { self.0.raw.CreateTexture2D(&desc, None, Some(&mut raw)) }
            .expect("allocate renderer texture");
        Texture {
            raw: raw.expect("created texture"),
            width: desc.Width,
            height: desc.Height,
            format: d.format,
            usage: d.usage,
        }
    }
    fn allocate_buffer(&self, size: u64, usage: BufferUsages, initial: Option<&[u8]>) -> Buffer {
        let size = u32::try_from(size.max(4)).expect("D3D11 buffer size");
        let mut flags = 0;
        for (usage_bit, flag) in [
            (BufferUsages::VERTEX, D3D11_BIND_VERTEX_BUFFER),
            (BufferUsages::INDEX, D3D11_BIND_INDEX_BUFFER),
        ] {
            if usage.contains(usage_bit) {
                flags |= flag.0;
            }
        }
        let mut raw = None;
        if flags != 0 {
            let desc = D3D11_BUFFER_DESC {
                ByteWidth: size,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: flags as u32,
                ..Default::default()
            };
            let data = initial.map(|bytes| {
                assert_eq!(bytes.len(), size as usize);
                D3D11_SUBRESOURCE_DATA {
                    pSysMem: bytes.as_ptr().cast(),
                    ..Default::default()
                }
            });
            // SAFETY: the optional initial slice contains exactly ByteWidth bytes.
            unsafe {
                self.0.raw.CreateBuffer(
                    &desc,
                    data.as_ref().map(std::ptr::from_ref),
                    Some(&mut raw),
                )
            }
            .expect("allocate renderer buffer");
        } else {
            assert!(
                usage.contains(BufferUsages::UNIFORM),
                "unsupported renderer buffer usage"
            );
        }
        Buffer {
            raw,
            uniforms: usage.contains(BufferUsages::UNIFORM).then(Default::default),
            size,
        }
    }
    pub fn create_buffer(&self, d: &::wgpu::BufferDescriptor<'_>) -> Buffer {
        self.allocate_buffer(d.size, d.usage, None)
    }
    pub fn create_buffer_init(&self, d: &::wgpu::util::BufferInitDescriptor<'_>) -> Buffer {
        self.allocate_buffer(d.contents.len() as u64, d.usage, Some(d.contents))
    }
    pub fn write_buffer(&self, b: &Buffer, offset: u64, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let start = u32::try_from(offset).expect("buffer offset");
        let end = start
            .checked_add(u32::try_from(data.len()).expect("D3D11 buffer update size"))
            .expect("buffer range");
        assert!(end <= b.size);
        // A dedicated uniform per ring slot also works on drivers without
        // D3D11.1 partial constant-buffer updates or offset binding.
        if let Some(uniforms) = &b.uniforms {
            let mut uniforms = uniforms.lock().expect("effect uniforms");
            if let Some((length, raw)) = uniforms.get(&start) {
                assert_eq!(*length, data.len(), "uniform slot size changed");
                // SAFETY: each slot holds one fixed-size effect parameter block.
                unsafe {
                    self.0
                        .immediate
                        .UpdateSubresource(raw, 0, None, data.as_ptr().cast(), 0, 0)
                };
            } else {
                assert_eq!(data.len() % 16, 0);
                let desc = D3D11_BUFFER_DESC {
                    ByteWidth: data.len() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                };
                let init = D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.as_ptr().cast(),
                    ..Default::default()
                };
                let mut raw = None;
                // SAFETY: init points to the full parameter block specified by ByteWidth.
                unsafe { self.0.raw.CreateBuffer(&desc, Some(&init), Some(&mut raw)) }
                    .expect("create effect uniform");
                uniforms.insert(start, (data.len(), raw.expect("created effect uniform")));
            }
            return;
        }
        let region = D3D11_BOX {
            left: start,
            right: end,
            top: 0,
            bottom: 1,
            front: 0,
            back: 1,
        };
        // SAFETY: source and byte range were checked. Context1 supports partial constant-buffer writes.
        unsafe {
            self.0.immediate.UpdateSubresource1(
                b.raw.as_ref().expect("GPU upload buffer"),
                0,
                if start == 0 && end == b.size {
                    None
                } else {
                    Some(&region)
                },
                data.as_ptr().cast(),
                0,
                0,
                0,
            );
        }
    }
    pub fn create_bind_group_layout(
        &self,
        d: &::wgpu::BindGroupLayoutDescriptor<'_>,
    ) -> BindGroupLayout {
        BindGroupLayout(
            d.entries
                .iter()
                .filter(|e| {
                    matches!(
                        e.ty,
                        ::wgpu::BindingType::Buffer {
                            has_dynamic_offset: true,
                            ..
                        }
                    )
                })
                .map(|e| e.binding)
                .collect(),
        )
    }
    pub fn create_bind_group(&self, d: &BindGroupDescriptor<'_>) -> BindGroup {
        debug_assert!(d.entries.len() <= 16, "too many bindings in {:?}", d.label);
        BindGroup(
            d.entries
                .iter()
                .map(|e| {
                    (
                        e.binding,
                        match &e.resource {
                            BindingResource::TextureView(v) => Binding::Texture((*v).clone()),
                            BindingResource::Sampler(s) => Binding::Sampler((*s).clone()),
                            BindingResource::Buffer(b) => Binding::Uniform(
                                b.buffer.clone(),
                                b.offset as u32,
                                b.size.map_or(b.buffer.size, |s| s.get() as u32),
                                d.layout.0.iter().position(|&slot| slot == e.binding),
                            ),
                        },
                    )
                })
                .collect(),
        )
    }
    pub fn create_sampler(&self, d: &::wgpu::SamplerDescriptor<'_>) -> Sampler {
        let mut raw = None;
        let address = |mode| match mode {
            ::wgpu::AddressMode::Repeat => D3D11_TEXTURE_ADDRESS_WRAP,
            ::wgpu::AddressMode::MirrorRepeat => D3D11_TEXTURE_ADDRESS_MIRROR,
            _ => D3D11_TEXTURE_ADDRESS_CLAMP,
        };
        let desc = D3D11_SAMPLER_DESC {
            Filter: if d.mag_filter == ::wgpu::FilterMode::Linear {
                D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT
            } else {
                D3D11_FILTER_MIN_MAG_MIP_POINT
            },
            AddressU: address(d.address_mode_u),
            AddressV: address(d.address_mode_v),
            AddressW: address(d.address_mode_w),
            MaxLOD: f32::MAX,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            ..Default::default()
        };
        // SAFETY: descriptor has valid filters, address modes and lifetime.
        unsafe { self.0.raw.CreateSamplerState(&desc, Some(&mut raw)) }.expect("create sampler");
        Sampler(raw.expect("created sampler"))
    }
}
