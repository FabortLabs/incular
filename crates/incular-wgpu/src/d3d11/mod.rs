use std::sync::{Arc, Mutex};

use windows::Win32::Graphics::{
    Direct3D::{
        D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP,
        D3D_FEATURE_LEVEL_11_0,
    },
    Direct3D11::*,
    Dxgi::*,
};
use windows::core::Interface;

use crate::RendererError;

pub use ::wgpu::*;
mod surface;
pub use surface::{CurrentSurfaceTexture, Surface, SurfaceTexture};
mod resources;
pub use resources::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferBinding, BufferSlice, Sampler, Texture, TextureView,
};
mod commands;
pub use commands::{
    CommandEncoder, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, TexelCopyTextureInfo,
};
mod pipeline;
pub use pipeline::RenderPipeline;

#[derive(Clone, Debug)]
pub struct Instance;
#[derive(Clone, Debug)]
pub struct Adapter;
#[derive(Debug)]
struct Context {
    raw: ID3D11Device,
    immediate: ID3D11DeviceContext1,
    factory: IDXGIFactory2,
    spare_encoder: Mutex<Option<ID3D11DeviceContext1>>,
    shaders: Mutex<
        std::collections::HashMap<
            &'static str,
            (
                windows::Win32::Graphics::Direct3D::ID3DBlob,
                windows::Win32::Graphics::Direct3D::ID3DBlob,
            ),
        >,
    >,
}
#[derive(Clone, Debug)]
pub struct Device(Arc<Context>);
pub type Queue = Device;

pub(crate) fn open_device() -> Result<(Instance, Adapter, Device, Queue), RendererError> {
    fn create(
        adapter: Option<&IDXGIAdapter>,
        kind: D3D_DRIVER_TYPE,
    ) -> windows::core::Result<Device> {
        let mut device = None;
        let mut context = None;
        // SAFETY: D3D initializes both output handles for the requested driver on success.
        unsafe {
            D3D11CreateDevice(
                adapter,
                kind,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let raw = device.expect("D3D11CreateDevice returned a device");
            let immediate = context
                .expect("D3D11CreateDevice returned a context")
                .cast::<ID3D11DeviceContext1>()?;
            let _ = immediate
                .cast::<ID3D11Multithread>()?
                .SetMultithreadProtected(true);
            let dxgi = raw.cast::<IDXGIDevice>()?;
            let factory = dxgi.GetAdapter()?.GetParent::<IDXGIFactory2>()?;
            Ok(Device(Arc::new(Context {
                raw,
                immediate,
                factory,
                spare_encoder: Mutex::default(),
                shaders: Mutex::default(),
            })))
        }
    }
    if let Ok(backends) = std::env::var("WGPU_BACKEND")
        && !backends.trim().is_empty()
        && !backends.split(',').any(|name| {
            matches!(
                name.trim().to_ascii_lowercase().as_str(),
                "d3d11" | "dx11" | "all"
            )
        })
    {
        return Err(RendererError::DriverInitialization(format!(
            "this build uses Direct3D 11; enable the requested WGPU backend as a Cargo feature to select {backends}"
        )));
    }
    let device = if let Some(request) = std::env::var("WGPU_ADAPTER_NAME")
        .ok()
        .filter(|name| !name.is_empty())
    {
        // SAFETY: DXGI returns owned COM references, and the fixed-size name buffer is read within bounds.
        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }.map_err(native_error)?;
        let query = request.to_lowercase();
        let mut selected = None;
        for i in 0.. {
            // SAFETY: DXGI returns an owned adapter or an end-of-list error for this index.
            let Ok(adapter) = (unsafe { factory.EnumAdapters1(i) }) else {
                break;
            };
            // SAFETY: the adapter is live and GetDesc1 returns a value with no borrowed storage.
            let desc = unsafe { adapter.GetDesc1() }.map_err(native_error)?;
            let n = desc
                .Description
                .iter()
                .position(|&ch| ch == 0)
                .unwrap_or(desc.Description.len());
            if String::from_utf16_lossy(&desc.Description[..n])
                .to_lowercase()
                .contains(&query)
            {
                selected = Some(
                    create(
                        Some(&adapter.cast().map_err(native_error)?),
                        D3D_DRIVER_TYPE_UNKNOWN,
                    )
                    .map_err(native_error)?,
                );
                break;
            }
        }
        selected.ok_or(RendererError::AdapterNameNotFound(request))?
    } else if let Some(preference) =
        PowerPreference::from_env().filter(|p| *p != PowerPreference::None)
    {
        // SAFETY: DXGI owns the adapter returned for the requested system power preference.
        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory6>() }.map_err(native_error)?;
        let preference = if preference == PowerPreference::LowPower {
            DXGI_GPU_PREFERENCE_MINIMUM_POWER
        } else {
            DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE
        };
        let adapter = unsafe { factory.EnumAdapterByGpuPreference::<IDXGIAdapter>(0, preference) }
            .map_err(native_error)?;
        create(Some(&adapter), D3D_DRIVER_TYPE_UNKNOWN).map_err(native_error)?
    } else {
        create(None, D3D_DRIVER_TYPE_HARDWARE)
            .or_else(|_| create(None, D3D_DRIVER_TYPE_WARP))
            .map_err(native_error)?
    };
    Ok((Instance, Adapter, device.clone(), device))
}
pub(super) fn native_error(error: windows::core::Error) -> RendererError {
    RendererError::DriverInitialization(format!("Direct3D 11: {error}"))
}

impl Device {
    pub fn limits(&self) -> Limits {
        Limits {
            max_texture_dimension_2d: 16384,
            ..Limits::default()
        }
    }
    pub fn features(&self) -> Features {
        Features::empty()
    }
    pub fn generate_allocator_report(&self) -> Option<AllocatorReport> {
        None
    }
}

mod capture;

pub(crate) mod shaders {
    include!(concat!(env!("OUT_DIR"), "/d3d11_shaders.rs"));
}
