pub mod clipboard;
pub mod draw;
pub mod grabber;
pub mod history;
pub mod monitor;
pub mod ocr;
pub mod pixel;
pub mod png_io;
pub mod scroll;
pub mod window_picker;

pub use clipboard::ImageClipboard;
pub use draw::{AnnotationOp, PixelDraw, bake_annotations};
pub use grabber::{DxgiDuplicationGrabber, GdiBitBltGrabber, ScreenCapturePipeline};
pub use history::{HistoryEntry, ScreenshotHistory};
pub use monitor::{CaptureMethod, MonitorCapture, MonitorEnumerator, MonitorInfo};
pub use ocr::{OcrResult, WindowsOcr};
pub use pixel::{CaptureUx, PixelBuffer, PixelRect, RegionCropper, ReleaseAction, ResizeHandle};
pub use png_io::{decode_png, encode_png, load_png_file, save_png_atomic, save_png_to_path};
pub use scroll::{ScrollCapture, ScrollCaptureOptions, ScrollCaptureResult};
pub use window_picker::WindowPicker;

pub struct ColorPick;

impl ColorPick {
    pub const FAIL: &'static str = "取色失败，请再试一次。";

    pub fn to_hex(r: u8, g: u8, b: u8) -> String {
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    pub fn to_rgb(r: u8, g: u8, b: u8) -> String {
        format!("rgb({r}, {g}, {b})")
    }

    pub fn copied_message(value: &str) -> String {
        format!("已复制颜色 {value}。")
    }
}
