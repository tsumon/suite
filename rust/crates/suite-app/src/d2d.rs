use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_ROUNDED_RECT,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWriteCreateFactory, IDWriteFactory,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::core::PCWSTR;

use suite_capture::PixelBuffer;
use suite_contracts::{NetSpeedSettings, RateFormatter, TaskbarSlotChrome, ThemeKind};
use suite_platform::constants::to_wide_null;

const RAINBOW_RGB: [(u8, u8, u8); 6] = [
    (0xFF, 0x6A, 0x8A),
    (0xFF, 0xB8, 0x6A),
    (0xE8, 0xE0, 0x70),
    (0x5E, 0xDC, 0x9A),
    (0x5A, 0xB8, 0xFF),
    (0xA8, 0x78, 0xFF),
];

pub struct D2dRenderer {
    _d2d_factory: ID2D1Factory,
    dwrite_factory: IDWriteFactory,
    dc_target: ID2D1DCRenderTarget,
    mem_dc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    bits: *mut u8,
    dib_w: i32,
    dib_h: i32,
}

impl D2dRenderer {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let d2d_factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
                    .map_err(|e| format!("D2D1CreateFactory failed: {e}"))?;
            let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
                .map_err(|e| format!("DWriteCreateFactory failed: {e}"))?;

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            };
            let dc_target = d2d_factory
                .CreateDCRenderTarget(&props)
                .map_err(|e| format!("CreateDCRenderTarget failed: {e}"))?;
            dc_target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);

            let mem_dc = CreateCompatibleDC(None);
            if mem_dc.is_invalid() {
                return Err("CreateCompatibleDC failed".to_string());
            }

            Ok(Self {
                _d2d_factory: d2d_factory,
                dwrite_factory,
                dc_target,
                mem_dc,
                dib: HBITMAP::default(),
                old_obj: HGDIOBJ::default(),
                bits: std::ptr::null_mut(),
                dib_w: 0,
                dib_h: 0,
            })
        }
    }

    fn ensure_dib(&mut self, w: i32, h: i32) -> Result<(), String> {
        let w = w.max(1);
        let h = h.max(1);
        if self.dib_w == w && self.dib_h == h && !self.dib.is_invalid() && !self.bits.is_null() {
            return Ok(());
        }
        unsafe {
            self.release_dib();
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let hbmp = CreateDIBSection(Some(self.mem_dc), &bmi, DIB_RGB_COLORS, &mut ptr, None, 0)
                .map_err(|e| format!("CreateDIBSection failed: {e}"))?;
            if hbmp.is_invalid() || ptr.is_null() {
                return Err("CreateDIBSection returned null".to_string());
            }
            self.old_obj = SelectObject(self.mem_dc, hbmp.into());
            self.dib = hbmp;
            self.bits = ptr.cast::<u8>();
            self.dib_w = w;
            self.dib_h = h;
        }
        Ok(())
    }

    unsafe fn release_dib(&mut self) {
        if !self.dib.is_invalid() {
            unsafe {
                if !self.old_obj.is_invalid() {
                    SelectObject(self.mem_dc, self.old_obj);
                    self.old_obj = HGDIOBJ::default();
                }
                let _ = DeleteObject(self.dib.into());
            }
            self.dib = HBITMAP::default();
            self.bits = std::ptr::null_mut();
            self.dib_w = 0;
            self.dib_h = 0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_netspeed_frame(
        &mut self,
        w: i32,
        h: i32,
        dpi: u32,
        tray_dark: bool,
        theme: ThemeKind,
        settings: &NetSpeedSettings,
        down_bps: f64,
        up_bps: f64,
        adapter_missing: bool,
    ) -> Option<&[u8]> {
        let w = w.max(1);
        let h = h.max(1);
        self.ensure_dib(w, h).ok()?;

        let dark = tray_dark || theme == ThemeKind::Dark;
        let ink_argb = if dark { 0xFF_F0F0F0 } else { 0xFF_1A1A1A };
        let zero_argb = 0xFF_888888u32;
        let down_default = if dark { 0xFF_63C7FF } else { 0xFF_086FA8 };
        let up_default = if dark { 0xFF_7EE787 } else { 0xFF_16803C };

        let (down_text, up_text, single_text) = if adapter_missing {
            let d = "↓ —".to_string();
            let u = "↑ 选网卡".to_string();
            let s = if settings.show_download && settings.show_upload {
                "↓ —  ↑ 选网卡".to_string()
            } else if settings.show_download {
                d.clone()
            } else {
                u.clone()
            };
            (d, u, s)
        } else {
            let d = format!("↓ {}", RateFormatter::format_bytes_per_second(down_bps));
            let u = format!("↑ {}", RateFormatter::format_bytes_per_second(up_bps));
            let s = if settings.show_download && settings.show_upload {
                format!("{d}  {u}")
            } else if settings.show_download {
                d.clone()
            } else {
                u.clone()
            };
            (d, u, s)
        };

        let resolve = |custom: Option<u32>, def: u32, bps: f64| -> u32 {
            if let Some(c) = custom {
                c
            } else if adapter_missing {
                ink_argb
            } else if bps.abs() < 0.5 {
                zero_argb
            } else {
                def
            }
        };
        let down_col = resolve(settings.down_color_argb, down_default, down_bps);
        let up_col = resolve(settings.up_color_argb, up_default, up_bps);

        let font_dip = TaskbarSlotChrome::clamp_font_size(settings.font_size_dip) as f32;
        let scale = (dpi.max(96) as f32) / 96.0;
        let font_px = font_dip * scale;
        let line_height_px = font_px * (TaskbarSlotChrome::LINE_HEIGHT as f32);
        let visible_lines = settings.visible_line_count().max(1) as f32;
        let content_h = line_height_px * visible_lines;
        let leftover = (h as f32) - content_h;
        let offset_y = if leftover > 0.0 {
            leftover / 2.0 + (TaskbarSlotChrome::OPTICAL_NUDGE_DIP as f32) * scale
        } else {
            0.0
        };
        let pad_x = (TaskbarSlotChrome::PAD_HORIZONTAL_DIP as f32) * scale;

        let font_family = to_wide_null("Consolas");
        let locale = to_wide_null("zh-CN");
        let weight = if settings.bold {
            DWRITE_FONT_WEIGHT_BOLD
        } else {
            DWRITE_FONT_WEIGHT_NORMAL
        };

        unsafe {
            let fmt = self
                .dwrite_factory
                .CreateTextFormat(
                    PCWSTR(font_family.as_ptr()),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STRETCH_NORMAL,
                    font_px,
                    PCWSTR(locale.as_ptr()),
                )
                .ok()?;

            let rc = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            self.dc_target.BindDC(self.mem_dc, &rc).ok()?;
            self.dc_target.BeginDraw();

            if settings.background_transparent {
                self.dc_target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }));
            } else {
                let bg = settings.background_argb.unwrap_or(0xFF_1E1E1E);
                self.dc_target.Clear(Some(&argb_to_d2d_opaque(bg)));
            }

            if !settings.two_line && (settings.show_download || settings.show_upload) {
                let col = if settings.show_download { down_col } else { up_col };
                if let Ok(brush) = self.dc_target.CreateSolidColorBrush(&argb_to_d2d(col), None) {
                    let wt: Vec<u16> = single_text.encode_utf16().collect();
                    let rect_f = D2D_RECT_F {
                        left: pad_x,
                        top: offset_y,
                        right: (w as f32) - 2.0,
                        bottom: h as f32,
                    };
                    self.dc_target.DrawText(
                        &wt,
                        &fmt,
                        &rect_f,
                        &brush,
                        D2D1_DRAW_TEXT_OPTIONS_CLIP,
                        DWRITE_MEASURING_MODE_NATURAL,
                    );
                }
            } else {
                let mut y = offset_y;
                if settings.show_download {
                    if let Ok(brush) = self.dc_target.CreateSolidColorBrush(&argb_to_d2d(down_col), None) {
                        let wt: Vec<u16> = down_text.encode_utf16().collect();
                        let rect_f = D2D_RECT_F {
                            left: pad_x,
                            top: y,
                            right: (w as f32) - 2.0,
                            bottom: h as f32,
                        };
                        self.dc_target.DrawText(
                            &wt,
                            &fmt,
                            &rect_f,
                            &brush,
                            D2D1_DRAW_TEXT_OPTIONS_CLIP,
                            DWRITE_MEASURING_MODE_NATURAL,
                        );
                    }
                    y += line_height_px;
                }
                if settings.show_upload
                    && let Ok(brush) = self.dc_target.CreateSolidColorBrush(&argb_to_d2d(up_col), None)
                {
                    let wt: Vec<u16> = up_text.encode_utf16().collect();
                    let rect_f = D2D_RECT_F {
                        left: pad_x,
                        top: y,
                        right: (w as f32) - 2.0,
                        bottom: h as f32,
                    };
                    self.dc_target.DrawText(
                        &wt,
                        &fmt,
                        &rect_f,
                        &brush,
                        D2D1_DRAW_TEXT_OPTIONS_CLIP,
                        DWRITE_MEASURING_MODE_NATURAL,
                    );
                }
            }

            if settings.show_border {
                let border_col = if dark {
                    D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.16,
                    }
                } else {
                    D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.16,
                    }
                };
                if let Ok(brush) = self.dc_target.CreateSolidColorBrush(&border_col, None) {
                    let brc = D2D_RECT_F {
                        left: 0.5,
                        top: 0.5,
                        right: (w as f32) - 0.5,
                        bottom: (h as f32) - 0.5,
                    };
                    self.dc_target.DrawRectangle(&brc, &brush, 1.0, None);
                }
            }

            if self.dc_target.EndDraw(None, None).is_err() {
                return None;
            }
            let len = (w * h * 4) as usize;
            Some(std::slice::from_raw_parts(self.bits, len))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_pin_frame(
        &mut self,
        image: &PixelBuffer,
        img_w: i32,
        img_h: i32,
        glow_pad: i32,
        phase: f32,
        far_alpha: f32,
        near_alpha: f32,
        window_opacity: f32,
    ) -> Option<(i32, i32, Vec<u8>)> {
        let img_w = img_w.max(1);
        let img_h = img_h.max(1);
        let total_w = img_w + glow_pad * 2;
        let total_h = img_h + glow_pad * 2;
        self.ensure_dib(total_w, total_h).ok()?;

        let packed = image.packed_bgra();
        unsafe {
            let rc = RECT {
                left: 0,
                top: 0,
                right: total_w,
                bottom: total_h,
            };
            self.dc_target.BindDC(self.mem_dc, &rc).ok()?;
            self.dc_target.BeginDraw();
            self.dc_target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            let p = glow_pad as f32;
            let iw = img_w as f32;
            let ih = img_h as f32;

            // Layered rainbow outer glow rings
            let far_col = sample_rainbow(phase, far_alpha * 0.45 * window_opacity);
            if let Ok(far_brush) = self.dc_target.CreateSolidColorBrush(&far_col, None) {
                for &(expand, stroke) in &[(7.0f32, 8.0f32), (4.5f32, 5.0f32)] {
                    let rr = D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: p - expand,
                            top: p - expand,
                            right: p + iw + expand,
                            bottom: p + ih + expand,
                        },
                        radiusX: 7.0,
                        radiusY: 7.0,
                    };
                    self.dc_target.DrawRoundedRectangle(&rr, &far_brush, stroke, None);
                }
            }

            let near_col = sample_rainbow(phase + 0.35, near_alpha * window_opacity);
            if let Ok(near_brush) = self.dc_target.CreateSolidColorBrush(&near_col, None) {
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: p - 1.2,
                        top: p - 1.2,
                        right: p + iw + 1.2,
                        bottom: p + ih + 1.2,
                    },
                    radiusX: 3.0,
                    radiusY: 3.0,
                };
                self.dc_target.DrawRoundedRectangle(&rr, &near_brush, 2.2, None);
            }

            let props = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };
            let size = windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                width: image.width as u32,
                height: image.height as u32,
            };
            if let Ok(bmp) = self.dc_target.CreateBitmap(
                size,
                Some(packed.as_ptr().cast()),
                (image.width * 4) as u32,
                &props,
            ) {
                let dst_rc = D2D_RECT_F {
                    left: p,
                    top: p,
                    right: p + iw,
                    bottom: p + ih,
                };
                let src_rc = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: image.width as f32,
                    bottom: image.height as f32,
                };
                self.dc_target.DrawBitmap(
                    &bmp,
                    Some(&dst_rc),
                    window_opacity,
                    windows::Win32::Graphics::Direct2D::D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                    Some(&src_rc),
                );
            }

            if self.dc_target.EndDraw(None, None).is_err() {
                return None;
            }
            let len = (total_w * total_h * 4) as usize;
            let bytes = std::slice::from_raw_parts(self.bits, len).to_vec();
            Some((total_w, total_h, bytes))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_on_buffer(
        &mut self,
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        text: &str,
        b: u8,
        g: u8,
        r: u8,
        font_size: f32,
    ) {
        if text.is_empty() || buffer.width <= 0 || buffer.height <= 0 {
            return;
        }
        let w = buffer.width;
        let h = buffer.height;
        if self.ensure_dib(w, h).is_err() {
            return;
        }
        let row_bytes = (w * 4) as usize;
        unsafe {
            for row in 0..h {
                let src_off = (row * buffer.stride) as usize;
                let dst_ptr = self.bits.add((row as usize) * row_bytes);
                std::ptr::copy_nonoverlapping(
                    buffer.bgra.as_ptr().add(src_off),
                    dst_ptr,
                    row_bytes,
                );
            }
        }

        let font_family = to_wide_null("Microsoft YaHei UI");
        let locale = to_wide_null("zh-CN");
        unsafe {
            let Ok(fmt) = self.dwrite_factory.CreateTextFormat(
                PCWSTR(font_family.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STRETCH_NORMAL,
                font_size.max(8.0),
                PCWSTR(locale.as_ptr()),
            ) else {
                return;
            };
            let rc = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            if self.dc_target.BindDC(self.mem_dc, &rc).is_err() {
                return;
            }
            self.dc_target.BeginDraw();
            let color = D2D1_COLOR_F {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                a: 1.0,
            };
            if let Ok(brush) = self.dc_target.CreateSolidColorBrush(&color, None) {
                let wt: Vec<u16> = text.encode_utf16().collect();
                let rect_f = D2D_RECT_F {
                    left: x as f32,
                    top: y as f32,
                    right: w as f32,
                    bottom: h as f32,
                };
                self.dc_target.DrawText(
                    &wt,
                    &fmt,
                    &rect_f,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            if self.dc_target.EndDraw(None, None).is_err() {
                return;
            }
            for row in 0..h {
                let dst_off = (row * buffer.stride) as usize;
                let src_ptr = self.bits.add((row as usize) * row_bytes);
                std::ptr::copy_nonoverlapping(
                    src_ptr,
                    buffer.bgra.as_mut_ptr().add(dst_off),
                    row_bytes,
                );
            }
            for chunk in buffer.bgra.as_chunks_mut::<4>().0 {
                chunk[3] = 255;
            }
        }
    }

    pub fn draw_text_raw_into_dc(
        &mut self,
        size: (i32, i32),
        bg_color: D2D1_COLOR_F,
        text_color: D2D1_COLOR_F,
        text: &str,
        font_size: f32,
        pad: (f32, f32),
    ) -> Option<&[u8]> {
        let (w, h) = size;
        let (pad_x, pad_y) = pad;
        self.ensure_dib(w, h).ok()?;
        let font_family = to_wide_null("Microsoft YaHei UI");
        let locale = to_wide_null("zh-CN");
        unsafe {
            let fmt = self
                .dwrite_factory
                .CreateTextFormat(
                    PCWSTR(font_family.as_ptr()),
                    None,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STRETCH_NORMAL,
                    font_size,
                    PCWSTR(locale.as_ptr()),
                )
                .ok()?;
            let rc = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            self.dc_target.BindDC(self.mem_dc, &rc).ok()?;
            self.dc_target.BeginDraw();
            self.dc_target.Clear(Some(&bg_color));
            if let Ok(brush) = self.dc_target.CreateSolidColorBrush(&text_color, None) {
                let wt: Vec<u16> = text.encode_utf16().collect();
                let rect_f = D2D_RECT_F {
                    left: pad_x,
                    top: pad_y,
                    right: (w as f32) - pad_x,
                    bottom: h as f32,
                };
                self.dc_target.DrawText(
                    &wt,
                    &fmt,
                    &rect_f,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            if self.dc_target.EndDraw(None, None).is_err() {
                return None;
            }
            Some(std::slice::from_raw_parts(self.bits, (w * h * 4) as usize))
        }
    }
}

impl Drop for D2dRenderer {
    fn drop(&mut self) {
        unsafe {
            self.release_dib();
            if !self.mem_dc.is_invalid() {
                let _ = DeleteDC(self.mem_dc);
            }
        }
    }
}

fn argb_to_d2d(argb: u32) -> D2D1_COLOR_F {
    let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
    let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
    let b = (argb & 0xFF) as f32 / 255.0;
    D2D1_COLOR_F { r, g, b, a }
}

fn argb_to_d2d_opaque(argb: u32) -> D2D1_COLOR_F {
    let mut c = argb_to_d2d(argb);
    c.a = 1.0;
    c
}

fn sample_rainbow(phase: f32, alpha: f32) -> D2D1_COLOR_F {
    let n = RAINBOW_RGB.len() as f32;
    let p = (phase.rem_euclid(1.0)) * n;
    let idx0 = (p.floor() as usize) % RAINBOW_RGB.len();
    let idx1 = (idx0 + 1) % RAINBOW_RGB.len();
    let t = p - p.floor();
    let c0 = RAINBOW_RGB[idx0];
    let c1 = RAINBOW_RGB[idx1];
    D2D1_COLOR_F {
        r: (f32::from(c0.0) * (1.0 - t) + f32::from(c1.0) * t) / 255.0,
        g: (f32::from(c0.1) * (1.0 - t) + f32::from(c1.1) * t) / 255.0,
        b: (f32::from(c0.2) * (1.0 - t) + f32::from(c1.2) * t) / 255.0,
        a: alpha.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d2d_dc_render_target_writes_premul_alpha() {
        let mut r = D2dRenderer::new().expect("D2dRenderer::new should succeed");
        let settings = NetSpeedSettings::default();
        let bytes = r
            .render_netspeed_frame(148, 48, 96, true, ThemeKind::Dark, &settings, 2048.0, 1024.0, false)
            .expect("render_netspeed_frame should return buffer");
        assert_eq!(bytes.len(), 148 * 48 * 4);
        // Corner pixel at (0,0) should remain transparent (alpha == 0)
        assert_eq!(bytes[3], 0);
        // Text area should have non-zero alpha pixels
        let any_text_alpha = bytes.as_chunks::<4>().0.iter().any(|px| px[3] > 0);
        assert!(any_text_alpha, "DirectWrite should render anti-aliased text pixels");
    }
}
