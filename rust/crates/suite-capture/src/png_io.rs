use std::path::{Path, PathBuf};

use chrono::Local;
use suite_contracts::SettingsPaths;

use crate::pixel::PixelBuffer;

pub fn encode_png(buffer: &PixelBuffer) -> Result<Vec<u8>, String> {
    if buffer.width <= 0 || buffer.height <= 0 || buffer.bgra.is_empty() {
        return Err("Empty pixel buffer.".to_string());
    }
    let w = buffer.width as u32;
    let h = buffer.height as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..buffer.height {
        let src_row = (y * buffer.stride) as usize;
        let dst_row = (y * buffer.width * 4) as usize;
        for x in 0..buffer.width as usize {
            let si = src_row + x * 4;
            let di = dst_row + x * 4;
            rgba[di] = buffer.bgra[si + 2];     // R
            rgba[di + 1] = buffer.bgra[si + 1]; // G
            rgba[di + 2] = buffer.bgra[si];     // B
            rgba[di + 3] = buffer.bgra[si + 3]; // A
        }
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG header failed: {e}"))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| format!("PNG write failed: {e}"))?;
    }
    Ok(out)
}

pub fn decode_png(bytes: &[u8]) -> Result<PixelBuffer, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG decode header failed: {e}"))?;
    let mut raw = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut raw)
        .map_err(|e| format!("PNG decode frame failed: {e}"))?;
    let w = info.width as i32;
    let h = info.height as i32;
    let stride = w * 4;
    let mut bgra = vec![0u8; (stride * h) as usize];
    let src = &raw[..info.buffer_size()];

    match info.color_type {
        png::ColorType::Rgba => {
            for (s, d) in src.as_chunks::<4>().0.iter().zip(bgra.as_chunks_mut::<4>().0) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = s[3];
            }
        }
        png::ColorType::Rgb => {
            for (s, d) in src.as_chunks::<3>().0.iter().zip(bgra.as_chunks_mut::<4>().0) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for (s, d) in src.as_chunks::<2>().0.iter().zip(bgra.as_chunks_mut::<4>().0) {
                d[0] = s[0];
                d[1] = s[0];
                d[2] = s[0];
                d[3] = s[1];
            }
        }
        png::ColorType::Grayscale => {
            for (&s, d) in src.iter().zip(bgra.as_chunks_mut::<4>().0) {
                d[0] = s;
                d[1] = s;
                d[2] = s;
                d[3] = 255;
            }
        }
        other => return Err(format!("Unsupported PNG color type: {other:?}")),
    }

    PixelBuffer::new(w, h, bgra, stride)
}

pub fn load_png_file(path: &Path) -> Result<PixelBuffer, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Read PNG failed: {e}"))?;
    decode_png(&bytes)
}

pub fn save_png_to_path(buffer: &PixelBuffer, path: &Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Create dir failed: {e}"))?;
    }
    let bytes = encode_png(buffer)?;
    let tmp = path.with_extension("png.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Write PNG tmp failed: {e}"))?;
    if let Err(err) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Move PNG failed: {err}"));
    }
    Ok(())
}

pub fn save_png_atomic(buffer: &PixelBuffer, configured_directory: Option<&str>) -> Result<PathBuf, String> {
    let dir = SettingsPaths::resolve_capture_directory(configured_directory);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Create capture dir failed: {e}"))?;
    let now = Local::now();
    let mut path = dir.join(format!("Suite-{}.png", now.format("%Y%m%d-%H%M%S")));
    if path.exists() {
        path = dir.join(format!("Suite-{}.png", now.format("%Y%m%d-%H%M%S-%3f")));
    }
    save_png_to_path(buffer, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::PixelDraw;
    use crate::pixel::PixelRect;

    #[test]
    fn png_encode_decode_roundtrip() {
        let mut buf = PixelBuffer::allocate(8, 6);
        PixelDraw::fill_color(&mut buf, PixelRect::new(0, 0, 8, 6), 11, 22, 33, 255);
        PixelDraw::fill_color(&mut buf, PixelRect::new(2, 2, 2, 2), 44, 55, 66, 200);
        let bytes = encode_png(&buf).unwrap();
        let decoded = decode_png(&bytes).unwrap();
        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 6);
        assert_eq!(decoded.try_get_pixel(0, 0), Some((11, 22, 33, 255)));
        assert_eq!(decoded.try_get_pixel(2, 2), Some((44, 55, 66, 200)));
    }
}
