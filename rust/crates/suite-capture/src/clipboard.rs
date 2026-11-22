use std::time::Duration;

use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFOHEADER};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_DIB;
use windows::core::PCWSTR;

use crate::pixel::PixelBuffer;
use crate::png_io::{decode_png, encode_png};

const CF_UNICODETEXT_ID: u32 = 13;

pub struct ImageClipboard;

impl ImageClipboard {
    fn open_with_retry() -> Result<(), String> {
        for _ in 0..8 {
            if unsafe { OpenClipboard(Some(HWND::default())).is_ok() } {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(12));
        }
        Err("无法打开剪贴板。".to_string())
    }

    fn alloc_global(bytes: &[u8]) -> Result<HANDLE, String> {
        unsafe {
            let hmem: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes.len())
                .map_err(|_| "剪贴板内存分配失败。".to_string())?;
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                return Err("剪贴板内存锁定失败。".to_string());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(hmem);
            Ok(HANDLE(hmem.0))
        }
    }

    fn register_format(name: &str) -> u32 {
        let wide = to_wide_null(name);
        unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
    }

    fn build_cf_dib(buffer: &PixelBuffer) -> Vec<u8> {
        let w = buffer.width;
        let h = buffer.height;
        let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
        let row_bytes = (w * 4) as usize;
        let mut out = vec![0u8; header_size + row_bytes * (h as usize)];
        let hdr = BITMAPINFOHEADER {
            biSize: header_size as u32,
            biWidth: w,
            biHeight: h, // positive = bottom-up DIB (standard CF_DIB)
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: (row_bytes * (h as usize)) as u32,
            ..Default::default()
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                (&hdr as *const BITMAPINFOHEADER).cast::<u8>(),
                out.as_mut_ptr(),
                header_size,
            );
        }
        for y in 0..h {
            let src_y = h - 1 - y;
            let src_off = (src_y * buffer.stride) as usize;
            let dst_off = header_size + (y as usize) * row_bytes;
            out[dst_off..dst_off + row_bytes]
                .copy_from_slice(&buffer.bgra[src_off..src_off + row_bytes]);
        }
        out
    }

    pub fn copy(buffer: &PixelBuffer) -> Result<(), String> {
        let png_bytes = encode_png(buffer)?;
        let dib_bytes = Self::build_cf_dib(buffer);
        let fmt_png = Self::register_format("PNG");
        let fmt_image_png = Self::register_format("image/png");

        Self::open_with_retry()?;
        let res = (|| -> Result<(), String> {
            unsafe {
                EmptyClipboard().map_err(|_| "无法清空剪贴板。".to_string())?;
                let h_dib = Self::alloc_global(&dib_bytes)?;
                SetClipboardData(u32::from(CF_DIB.0), Some(h_dib))
                    .map_err(|_| "写入 CF_DIB 失败。".to_string())?;
                if fmt_png != 0 {
                    let h_png = Self::alloc_global(&png_bytes)?;
                    let _ = SetClipboardData(fmt_png, Some(h_png));
                }
                if fmt_image_png != 0 {
                    let h_img = Self::alloc_global(&png_bytes)?;
                    let _ = SetClipboardData(fmt_image_png, Some(h_img));
                }
            }
            Ok(())
        })();
        unsafe {
            let _ = CloseClipboard();
        }
        res
    }

    pub fn copy_text(text: &str) -> Result<(), String> {
        let wide = to_wide_null(text);
        let byte_len = wide.len() * std::mem::size_of::<u16>();
        let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), byte_len) };

        Self::open_with_retry()?;
        let res = (|| -> Result<(), String> {
            unsafe {
                EmptyClipboard().map_err(|_| "无法清空剪贴板。".to_string())?;
                let h = Self::alloc_global(bytes)?;
                SetClipboardData(CF_UNICODETEXT_ID, Some(h))
                    .map_err(|_| "写入剪贴板文字失败。".to_string())?;
            }
            Ok(())
        })();
        unsafe {
            let _ = CloseClipboard();
        }
        res
    }

    pub fn try_get() -> Option<PixelBuffer> {
        let fmt_png = Self::register_format("PNG");
        let fmt_image_png = Self::register_format("image/png");
        let has_png = fmt_png != 0 && unsafe { IsClipboardFormatAvailable(fmt_png).is_ok() };
        let has_img_png =
            fmt_image_png != 0 && unsafe { IsClipboardFormatAvailable(fmt_image_png).is_ok() };
        let has_dib = unsafe { IsClipboardFormatAvailable(u32::from(CF_DIB.0)).is_ok() };

        if !has_png && !has_img_png && !has_dib {
            return None;
        }

        Self::open_with_retry().ok()?;
        let result = (|| -> Option<PixelBuffer> {
            for &fmt in &[fmt_png, fmt_image_png] {
                if fmt == 0 || unsafe { IsClipboardFormatAvailable(fmt).is_err() } {
                    continue;
                }
                if let Some(bytes) = unsafe { Self::read_global_bytes(fmt) }
                    && let Ok(buf) = decode_png(&bytes)
                {
                    return Some(buf);
                }
            }

            if has_dib && let Some(bytes) = unsafe { Self::read_global_bytes(u32::from(CF_DIB.0)) } {
                return Self::parse_dib(&bytes);
            }
            None
        })();
        unsafe {
            let _ = CloseClipboard();
        }
        result
    }

    unsafe fn read_global_bytes(format: u32) -> Option<Vec<u8>> {
        let handle = unsafe { GetClipboardData(format) }.ok()?;
        let hmem = HGLOBAL(handle.0);
        let size = unsafe { GlobalSize(hmem) };
        if size == 0 {
            return None;
        }
        let ptr = unsafe { GlobalLock(hmem) };
        if ptr.is_null() {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), size) };
        let out = slice.to_vec();
        unsafe {
            let _ = GlobalUnlock(hmem);
        }
        Some(out)
    }

    fn parse_dib(bytes: &[u8]) -> Option<PixelBuffer> {
        let hdr_min = std::mem::size_of::<BITMAPINFOHEADER>();
        if bytes.len() < hdr_min {
            return None;
        }
        let mut hdr = BITMAPINFOHEADER::default();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (&mut hdr as *mut BITMAPINFOHEADER).cast::<u8>(),
                hdr_min,
            );
        }
        let w = hdr.biWidth;
        let top_down = hdr.biHeight < 0;
        let h = hdr.biHeight.abs();
        if w <= 0 || h <= 0 || hdr.biPlanes != 1 {
            return None;
        }
        let bpp = hdr.biBitCount;
        if bpp != 24 && bpp != 32 {
            return None;
        }
        let mut pixel_offset = hdr.biSize as usize;
        // BI_BITFIELDS (3) with 40-byte BITMAPINFOHEADER appends 3 DWORD masks (12 bytes)
        if hdr.biCompression == 3 && hdr.biSize == 40 {
            pixel_offset += 12;
        }
        if pixel_offset >= bytes.len() {
            return None;
        }
        let src_stride = (((w * i32::from(bpp) + 31) / 32) * 4) as usize;
        let needed = pixel_offset + src_stride * (h as usize);
        if bytes.len() < needed {
            return None;
        }
        let mut out = PixelBuffer::allocate(w, h);
        for y in 0..h {
            let src_row_idx = if top_down { y } else { h - 1 - y };
            let src_off = pixel_offset + (src_row_idx as usize) * src_stride;
            let dst_off = (y * out.stride) as usize;
            if bpp == 32 {
                out.bgra[dst_off..dst_off + (w as usize) * 4]
                    .copy_from_slice(&bytes[src_off..src_off + (w as usize) * 4]);
                for x in 0..w as usize {
                    // Preserve alpha if any non-zero, otherwise leave opaque? Keep DIB alpha if non-all-zero.
                    let _ = x;
                }
            } else {
                for x in 0..w as usize {
                    let si = src_off + x * 3;
                    let di = dst_off + x * 4;
                    out.bgra[di] = bytes[si];
                    out.bgra[di + 1] = bytes[si + 1];
                    out.bgra[di + 2] = bytes[si + 2];
                    out.bgra[di + 3] = 255;
                }
            }
        }
        if bpp == 32 && out.bgra.as_chunks::<4>().0.iter().all(|px| px[3] == 0) {
            for px in out.bgra.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
        }
        Some(out)
    }
}
