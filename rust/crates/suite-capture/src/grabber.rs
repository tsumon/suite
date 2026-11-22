use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_OUTDUPL_FRAME_INFO, IDXGIAdapter, IDXGIFactory1, IDXGIOutput1,
    IDXGIResource,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::core::Interface;

use crate::monitor::{CaptureMethod, MonitorCapture, MonitorEnumerator, MonitorInfo};
use crate::pixel::{PixelBuffer, PixelRect};

pub struct GdiBitBltGrabber;

impl GdiBitBltGrabber {
    pub fn capture(monitor: &MonitorInfo) -> Result<PixelBuffer, String> {
        Self::capture_rect(monitor.bounds)
    }

    pub fn capture_rect(rect: PixelRect) -> Result<PixelBuffer, String> {
        if rect.is_empty() {
            return Err("Empty capture rect.".to_string());
        }
        let width = rect.width;
        let height = rect.height;

        unsafe {
            let hdc_screen = GetDC(None);
            if hdc_screen.is_invalid() {
                return Err("GetDC failed.".to_string());
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
            if hdc_mem.is_invalid() || hbmp.is_invalid() {
                if !hbmp.is_invalid() {
                    let _ = DeleteObject(hbmp.into());
                }
                if !hdc_mem.is_invalid() {
                    let _ = DeleteDC(hdc_mem);
                }
                let _ = ReleaseDC(None, hdc_screen);
                return Err("CreateCompatibleDC/Bitmap failed.".to_string());
            }

            let old = SelectObject(hdc_mem, hbmp.into());
            let blt_ok = BitBlt(hdc_mem, 0, 0, width, height, Some(hdc_screen), rect.x, rect.y, SRCCOPY);
            let res = if let Err(err) = blt_ok {
                Err(format!("BitBlt failed: {err}"))
            } else {
                let mut bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width,
                        biHeight: -height,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut pixels = vec![0u8; (width * 4 * height) as usize];
                let lines = GetDIBits(
                    hdc_mem,
                    hbmp,
                    0,
                    height as u32,
                    Some(pixels.as_mut_ptr().cast()),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );
                if lines == 0 {
                    Err("GetDIBits failed.".to_string())
                } else {
                    for chunk in pixels.as_chunks_mut::<4>().0 {
                        chunk[3] = 255;
                    }
                    PixelBuffer::new(width, height, pixels, width * 4)
                }
            };

            SelectObject(hdc_mem, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            res
        }
    }
}

pub struct DxgiDuplicationGrabber;

impl DxgiDuplicationGrabber {
    pub fn try_capture(monitor: &MonitorInfo) -> Option<PixelBuffer> {
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            let mut adapter_idx = 0u32;
            while let Ok(adapter1) = factory.EnumAdapters1(adapter_idx) {
                adapter_idx += 1;
                let adapter: IDXGIAdapter = adapter1.cast().ok()?;
                if let Some(buf) = Self::try_capture_on_adapter(&adapter, monitor) {
                    return Some(buf);
                }
            }
            None
        }
    }

    unsafe fn try_capture_on_adapter(adapter: &IDXGIAdapter, monitor: &MonitorInfo) -> Option<PixelBuffer> {
        let mut out_idx = 0u32;
        while let Ok(output) = unsafe { adapter.EnumOutputs(out_idx) } {
            out_idx += 1;
            let Ok(desc) = (unsafe { output.GetDesc() }) else {
                continue;
            };
            if monitor.handle != 0 && desc.Monitor.0 as isize != monitor.handle {
                continue;
            }
            if monitor.handle == 0 {
                let b = PixelRect::new(
                    desc.DesktopCoordinates.left,
                    desc.DesktopCoordinates.top,
                    desc.DesktopCoordinates.right - desc.DesktopCoordinates.left,
                    desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top,
                );
                if b != monitor.bounds {
                    continue;
                }
            }

            let output1: IDXGIOutput1 = output.cast().ok()?;
            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
            if unsafe {
                D3D11CreateDevice(
                    adapter,
                    D3D_DRIVER_TYPE_UNKNOWN,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
            }
            .is_err()
            {
                return None;
            }

            let device = device?;
            let context = context?;
            let dupl = unsafe { output1.DuplicateOutput(&device) }.ok()?;
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut res: Option<IDXGIResource> = None;
            if unsafe { dupl.AcquireNextFrame(800, &mut info, &mut res) }.is_err()
                && unsafe { dupl.AcquireNextFrame(800, &mut info, &mut res) }.is_err()
            {
                return None;
            }
            let Some(resource) = res else {
                let _ = unsafe { dupl.ReleaseFrame() };
                return None;
            };
            let tex_result = resource.cast::<ID3D11Texture2D>().ok().and_then(|tex| {
                unsafe { Self::copy_texture(&device, &context, &tex) }
            });
            let _ = unsafe { dupl.ReleaseFrame() };
            if tex_result.is_some() {
                return tex_result;
            }
        }
        None
    }

    unsafe fn copy_texture(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        texture: &ID3D11Texture2D,
    ) -> Option<PixelBuffer> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };
        if desc.Width == 0 || desc.Height == 0 {
            return None;
        }
        desc.MipLevels = 1;
        desc.ArraySize = 1;
        desc.SampleDesc.Count = 1;
        desc.SampleDesc.Quality = 0;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        if desc.Format.0 == 0 {
            desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        }

        let mut staging: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&desc, None, Some(&mut staging)) }.ok()?;
        let staging = staging?;
        unsafe { context.CopyResource(&staging, texture) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }.ok()?;
        if mapped.pData.is_null() {
            unsafe { context.Unmap(&staging, 0) };
            return None;
        }

        let width = desc.Width as i32;
        let height = desc.Height as i32;
        let stride = width * 4;
        let mut pixels = vec![0u8; (stride * height) as usize];
        let row_pitch = mapped.RowPitch as usize;
        let base_ptr = mapped.pData.cast::<u8>();
        for y in 0..height as usize {
            let src_row = unsafe { std::slice::from_raw_parts(base_ptr.add(y * row_pitch), stride as usize) };
            let dst_off = y * stride as usize;
            pixels[dst_off..dst_off + stride as usize].copy_from_slice(src_row);
        }
        unsafe { context.Unmap(&staging, 0) };
        for chunk in pixels.as_chunks_mut::<4>().0 {
            chunk[3] = 255;
        }
        PixelBuffer::new(width, height, pixels, stride).ok()
    }
}

pub struct ScreenCapturePipeline;

impl ScreenCapturePipeline {
    /// Prefer GDI first (avoids Win11 GraphicsCapture yellow-border flash), then DXGI fallback.
    pub fn capture(monitor: &MonitorInfo) -> Result<MonitorCapture, String> {
        if let Ok(gdi) = GdiBitBltGrabber::capture(monitor) {
            return Ok(MonitorCapture {
                monitor: monitor.clone(),
                buffer: gdi,
                method: CaptureMethod::GdiBitBlt,
            });
        }
        if let Some(dxgi) = DxgiDuplicationGrabber::try_capture(monitor) {
            return Ok(MonitorCapture {
                monitor: monitor.clone(),
                buffer: dxgi,
                method: CaptureMethod::DxgiDuplication,
            });
        }
        let gdi = GdiBitBltGrabber::capture(monitor)?;
        Ok(MonitorCapture {
            monitor: monitor.clone(),
            buffer: gdi,
            method: CaptureMethod::GdiBitBlt,
        })
    }

    pub fn capture_all() -> Result<Vec<MonitorCapture>, String> {
        let monitors = MonitorEnumerator::get_all();
        if monitors.is_empty() {
            return Err("No monitors to capture.".to_string());
        }
        let mut frames = Vec::with_capacity(monitors.len());
        for m in monitors {
            frames.push(Self::capture(&m)?);
        }
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Gdi::{CreateSolidBrush, FillRect};

    #[test]
    fn gdi_dib_extraction_from_memory_dc_succeeds() {
        unsafe {
            let hdc_screen = GetDC(None);
            assert!(!hdc_screen.is_invalid());
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            let hbmp = CreateCompatibleBitmap(hdc_screen, 4, 4);
            assert!(!hdc_mem.is_invalid() && !hbmp.is_invalid());
            let old = SelectObject(hdc_mem, hbmp.into());

            // Paint solid BGR=(0x44, 0x33, 0x22) -> COLORREF=0x00443322
            let brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x0044_3322));
            let rc = windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 4,
                bottom: 4,
            };
            FillRect(hdc_mem, &rc, brush);
            let _ = DeleteObject(brush.into());

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 4,
                    biHeight: -4,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; 4 * 4 * 4];
            let lines = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                4,
                Some(pixels.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            SelectObject(hdc_mem, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            assert_eq!(lines, 4);
            assert_eq!(&pixels[0..3], &[0x44, 0x33, 0x22]);
        }
    }

    #[test]
    fn capture_rect_rejects_empty_rect() {
        assert!(GdiBitBltGrabber::capture_rect(PixelRect::new(0, 0, 0, 10)).is_err());
    }

    #[test]
    fn live_capture_probe_handles_interactive_or_headless_session() {
        match ScreenCapturePipeline::capture_all() {
            Ok(frames) => {
                assert!(!frames.is_empty());
                assert!(frames.iter().all(|f| f.buffer.width > 0 && f.buffer.height > 0));
            }
            Err(err) => {
                // Non-interactive / locked sessions disallow screen BitBlt (ERROR_INVALID_HANDLE / access denied).
                assert!(err.contains("BitBlt failed"), "unexpected error: {err}");
            }
        }
    }
}
