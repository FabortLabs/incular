use super::{Device, Texture};
use crate::{CapturedFrame, SurfaceAlphaRepresentation};
use std::time::{Duration, Instant};
use windows::Win32::Graphics::{Direct3D11::*, Dxgi::DXGI_ERROR_WAS_STILL_DRAWING};
use windows::core::Result;

impl Device {
    pub(crate) fn capture(
        &self,
        image: &Texture,
        alpha: SurfaceAlphaRepresentation,
    ) -> Result<CapturedFrame> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: the texture is live, and GetDesc initializes its complete descriptor.
        unsafe { image.raw.GetDesc(&mut desc) };
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut staging = None;
        // SAFETY: the staging image has the source dimensions, format and sample count.
        unsafe {
            self.0
                .raw
                .CreateTexture2D(&desc, None, Some(&mut staging))?
        };
        let staging = staging.expect("created staging texture");
        let ctx = &self.0.immediate;
        // SAFETY: both textures belong to the same device and have matching subresources.
        unsafe {
            ctx.CopyResource(&staging, &image.raw);
            ctx.Flush();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let width = image.width as usize;
        let height = image.height as usize;
        let mut rgba8 = Vec::with_capacity(width * height * 4);
        let mut mapping = D3D11_MAPPED_SUBRESOURCE::default();
        loop {
            // SAFETY: the staging texture has CPU read access. Successful Map owns the mapping until Unmap.
            match unsafe {
                ctx.Map(
                    &staging,
                    0,
                    D3D11_MAP_READ,
                    D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                    Some(&mut mapping),
                )
            } {
                Ok(()) => break,
                Err(e) if e.code() == DXGI_ERROR_WAS_STILL_DRAWING && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                Err(e) => return Err(e),
            }
        }
        // SAFETY: DXGI_FORMAT_B8G8R8A8_UNORM has four bytes per pixel. Each mapped row
        // extends through RowPitch and is readable until Unmap below.
        unsafe {
            for y in 0..height {
                let row = std::slice::from_raw_parts(
                    mapping
                        .pData
                        .cast::<u8>()
                        .add(y * mapping.RowPitch as usize),
                    width * 4,
                );
                for &[b, g, r, a] in row.as_chunks::<4>().0 {
                    rgba8.extend_from_slice(&alpha.to_straight_rgba8(image.format, [r, g, b, a]));
                }
            }
            ctx.Unmap(&staging, 0);
        }
        Ok(CapturedFrame {
            width: image.width,
            height: image.height,
            rgba8,
        })
    }
}
