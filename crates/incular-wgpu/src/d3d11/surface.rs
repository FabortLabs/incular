use super::{Adapter, Device, Instance, Texture};
use ::wgpu::{
    CompositeAlphaMode, DisplayAndWindowHandle, PresentMode, SurfaceCapabilities,
    SurfaceConfiguration, TextureFormat, TextureUsages, rwh::RawWindowHandle,
};
use std::sync::{Arc, Mutex};
use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            DirectComposition::*,
            Dxgi::{Common::*, *},
        },
    },
    core::Interface,
};

pub type Surface<'window> = WindowSurface;
pub struct WindowSurface {
    owner: Arc<dyn DisplayAndWindowHandle>,
    state: Mutex<Option<SwapChain>>,
    configuration_error: Mutex<Option<String>>,
}
struct SwapChain {
    chain: IDXGISwapChain1,
    _composition: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    width: u32,
    height: u32,
}
impl Instance {
    pub fn create_surface(
        &self,
        owner: Arc<dyn DisplayAndWindowHandle>,
    ) -> Result<WindowSurface, ::wgpu::CreateSurfaceError> {
        Ok(WindowSurface {
            owner,
            state: Mutex::default(),
            configuration_error: Mutex::default(),
        })
    }
}
impl WindowSurface {
    pub fn get_capabilities(&self, _: &Adapter) -> SurfaceCapabilities {
        SurfaceCapabilities {
            format_capabilities: vec![],
            formats: vec![TextureFormat::Bgra8Unorm],
            present_modes: vec![PresentMode::Fifo],
            alpha_modes: vec![
                CompositeAlphaMode::Opaque,
                CompositeAlphaMode::PreMultiplied,
            ],
            usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        }
    }
    pub fn get_default_config(
        &self,
        _: &Adapter,
        width: u32,
        height: u32,
    ) -> Option<SurfaceConfiguration> {
        Some(SurfaceConfiguration {
            color_space: Default::default(),
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8Unorm,
            width,
            height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
        })
    }
    pub fn configure(&self, device: &Device, config: &SurfaceConfiguration) {
        let mut state = self.state.lock().expect("swapchain lock");
        // SAFETY: the surface owns the window, and the device and composition tree outlive all acquired buffers.
        let configure = || -> windows::core::Result<SwapChain> {
            unsafe {
                let handle = self
                    .owner
                    .window_handle()
                    .map_err(|_| windows::core::Error::from_win32())?;
                let RawWindowHandle::Win32(win) = handle.as_raw() else {
                    return Err(windows::core::Error::from_win32());
                };
                let desc = DXGI_SWAP_CHAIN_DESC1 {
                    Width: config.width,
                    Height: config.height,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_STRETCH,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                    AlphaMode: if config.alpha_mode == CompositeAlphaMode::PreMultiplied {
                        DXGI_ALPHA_MODE_PREMULTIPLIED
                    } else {
                        DXGI_ALPHA_MODE_IGNORE
                    },
                    ..Default::default()
                };
                let chain =
                    device
                        .0
                        .factory
                        .CreateSwapChainForComposition(&device.0.raw, &desc, None)?;
                let dxgi = device.0.raw.cast::<IDXGIDevice>()?;
                let composition: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
                let target =
                    composition.CreateTargetForHwnd(HWND(win.hwnd.get() as *mut _), true)?;
                let visual = composition.CreateVisual()?;
                visual.SetContent(&chain)?;
                target.SetRoot(&visual)?;
                composition.Commit()?;
                Ok(SwapChain {
                    chain,
                    _composition: composition,
                    _target: target,
                    _visual: visual,
                    width: config.width,
                    height: config.height,
                })
            }
        };
        if let Some(existing) = state.as_mut() {
            // SAFETY: the presentation code releases acquired buffers before reconfiguration.
            if unsafe {
                existing.chain.ResizeBuffers(
                    2,
                    config.width,
                    config.height,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_SWAP_CHAIN_FLAG(0),
                )
            }
            .is_ok()
            {
                existing.width = config.width;
                existing.height = config.height;
                return;
            }
        }
        *state = None;
        match configure() {
            Ok(chain) => *state = Some(chain),
            Err(error) => {
                *self.configuration_error.lock().expect("surface error lock") =
                    Some(error.to_string())
            }
        }
    }
    pub(crate) fn check_configuration(&self) -> Result<(), crate::RendererError> {
        match self
            .configuration_error
            .lock()
            .expect("surface error lock")
            .take()
        {
            Some(reason) => Err(crate::RendererError::DriverInitialization(reason)),
            None => Ok(()),
        }
    }
    pub fn get_current_texture(&self) -> CurrentSurfaceTexture {
        let state = self.state.lock().expect("swapchain lock");
        let Some(state) = state.as_ref() else {
            return CurrentSurfaceTexture::Lost;
        };
        // SAFETY: buffer zero is the current render target of the flip swapchain.
        match unsafe { state.chain.GetBuffer(0) } {
            Ok(raw) => CurrentSurfaceTexture::Success(SurfaceTexture {
                texture: Texture::surface(raw, state.width, state.height),
                chain: state.chain.clone(),
            }),
            Err(_) => CurrentSurfaceTexture::Lost,
        }
    }
}
#[derive(Debug)]
pub struct SurfaceTexture {
    pub texture: Texture,
    chain: IDXGISwapChain1,
}
impl Device {
    pub fn present(&self, frame: SurfaceTexture) -> Result<(), crate::RendererError> {
        // SAFETY: this frame owns its swapchain and the submitted back buffer.
        unsafe { frame.chain.Present(1, DXGI_PRESENT(0)) }
            .ok()
            .map_err(super::native_error)
    }
}
#[derive(Debug)]
pub enum CurrentSurfaceTexture {
    Success(SurfaceTexture),
    Lost,
}
