use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

use crate::pixel::PixelBuffer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrResult {
    pub succeeded: bool,
    pub text: Option<String>,
    pub error: Option<String>,
}

impl OcrResult {
    pub fn ok(text: String) -> Self {
        Self {
            succeeded: true,
            text: Some(text),
            error: None,
        }
    }

    pub fn fail(error: &str) -> Self {
        Self {
            succeeded: false,
            text: None,
            error: Some(error.to_string()),
        }
    }
}

pub struct WindowsOcr;

impl WindowsOcr {
    pub const COPIED: &'static str = "已复制文字。";
    pub const NO_TEXT: &'static str = "没识别到文字。";
    pub const NO_LANGUAGE: &'static str = "系统缺少 OCR 语言包，请在 Windows 语言设置里安装后再试。";
    pub const ENGINE_UNAVAILABLE: &'static str = "本机暂时无法识字。";
    pub const EMPTY_IMAGE: &'static str = "没识别到文字。";
    pub const FAILED: &'static str = "本机暂时无法识字。";
    pub const CLIPBOARD_FAILED: &'static str = "文字已识别，但没法写入剪贴板，请再试一次。";

    /// Run an MTA WinRT scope safely on a dedicated worker thread so an STA UI thread never deadlocks.
    fn on_mta_thread<T, F>(job: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        std::thread::Builder::new()
            .name("suite-winrt-ocr".to_string())
            .spawn(move || {
                let ro_ok = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
                let res = job();
                if ro_ok {
                    unsafe {
                        RoUninitialize();
                    }
                }
                res
            })
            .map_err(|e| format!("Failed to spawn OCR thread: {e}"))?
            .join()
            .map_err(|_| "OCR thread panicked".to_string())
    }

    pub fn probe_available() -> Option<String> {
        Self::on_mta_thread(|| {
            Self::try_create_engine().map(|(_, tag)| tag)
        })
        .ok()
        .flatten()
    }

    pub fn recognize_blocking(image: PixelBuffer) -> OcrResult {
        if image.width <= 0 || image.height <= 0 || image.bgra.is_empty() {
            return OcrResult::fail(Self::EMPTY_IMAGE);
        }
        Self::on_mta_thread(move || Self::recognize_in_mta(&image))
            .unwrap_or_else(|_| OcrResult::fail(Self::FAILED))
    }

    fn recognize_in_mta(image: &PixelBuffer) -> OcrResult {
        let Some((engine, _tag)) = Self::try_create_engine() else {
            let has_langs = OcrEngine::AvailableRecognizerLanguages()
                .ok()
                .and_then(|v| v.Size().ok())
                .unwrap_or(0)
                > 0;
            return OcrResult::fail(if has_langs {
                Self::ENGINE_UNAVAILABLE
            } else {
                Self::NO_LANGUAGE
            });
        };

        let Ok(bitmap) = Self::to_software_bitmap(image) else {
            return OcrResult::fail(Self::FAILED);
        };
        let Ok(async_op) = engine.RecognizeAsync(&bitmap) else {
            return OcrResult::fail(Self::FAILED);
        };
        let Ok(raw) = async_op.get() else {
            return OcrResult::fail(Self::FAILED);
        };
        let text = raw
            .Text()
            .map(|h| h.to_string_lossy().trim().to_string())
            .unwrap_or_default();
        if text.is_empty() {
            OcrResult::fail(Self::NO_TEXT)
        } else {
            OcrResult::ok(text)
        }
    }

    fn try_create_engine() -> Option<(OcrEngine, String)> {
        if let Some(res) = Self::try_create_from_available() {
            return Some(res);
        }
        if let Ok(from_profile) = OcrEngine::TryCreateFromUserProfileLanguages() {
            let tag = from_profile
                .RecognizerLanguage()
                .ok()
                .and_then(|l| l.LanguageTag().ok())
                .map(|h| h.to_string_lossy())
                .unwrap_or_else(|| "profile".to_string());
            return Some((from_profile, tag));
        }
        None
    }

    fn try_create_from_available() -> Option<(OcrEngine, String)> {
        let langs = OcrEngine::AvailableRecognizerLanguages().ok()?;
        let count = langs.Size().ok()?;
        if count == 0 {
            return None;
        }
        let mut items: Vec<(i32, String, Language)> = Vec::with_capacity(count as usize);
        for i in 0..count {
            if let Ok(lang) = langs.GetAt(i) {
                let tag = lang
                    .LanguageTag()
                    .map(|h| h.to_string_lossy())
                    .unwrap_or_default();
                let lower = tag.to_ascii_lowercase();
                let rank = if lower.starts_with("zh-hans") {
                    0
                } else if lower.starts_with("zh") {
                    1
                } else {
                    2
                };
                items.push((rank, tag, lang));
            }
        }
        items.sort_by_key(|a| a.0);

        for (_, tag, lang) in items {
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
                return Some((engine, tag));
            }
        }
        None
    }

    fn to_software_bitmap(image: &PixelBuffer) -> Result<SoftwareBitmap, String> {
        let packed = image.packed_bgra();
        let writer = DataWriter::new().map_err(|e| format!("DataWriter::new failed: {e}"))?;
        writer
            .WriteBytes(&packed)
            .map_err(|e| format!("WriteBytes failed: {e}"))?;
        let ibuf = writer
            .DetachBuffer()
            .map_err(|e| format!("DetachBuffer failed: {e}"))?;
        SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &ibuf,
            BitmapPixelFormat::Bgra8,
            image.width,
            image.height,
            BitmapAlphaMode::Ignore,
        )
        .map_err(|e| format!("CreateCopyWithAlphaFromBuffer failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocr_probe_does_not_crash() {
        let tag = WindowsOcr::probe_available();
        // Should return Some(lang) on Windows with OCR pack or None cleanly without panicking.
        let _ = tag;
    }
}
