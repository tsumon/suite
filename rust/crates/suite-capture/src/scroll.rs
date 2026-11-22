use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::grabber::GdiBitBltGrabber;
use crate::pixel::{PixelBuffer, PixelRect};

#[derive(Debug, Clone)]
pub struct ScrollCaptureOptions {
    pub sample_interval_ms: u64,
    pub idle_stop_ms: u64,
    pub max_height_px: i32,
    pub max_duration_ms: u64,
    pub max_wait_first_change_ms: u64,
}

impl Default for ScrollCaptureOptions {
    fn default() -> Self {
        Self {
            sample_interval_ms: 150,
            idle_stop_ms: 1500,
            max_height_px: 12_000,
            max_duration_ms: 45_000,
            max_wait_first_change_ms: 6_000,
        }
    }
}

pub struct ScrollCaptureResult {
    pub succeeded: bool,
    pub image: Option<PixelBuffer>,
    pub error: Option<String>,
}

pub struct ScrollCapture;

impl ScrollCapture {
    pub const BAD_REGION: &'static str = "没找到可滚动的窗口。";
    pub const CAPTURE_FAILED: &'static str = "这个窗口暂时滚不动长图。";
    pub const TOO_SHORT: &'static str = "这个窗口暂时滚不动长图。";
    pub const COPIED: &'static str = "已复制长图。";
    pub const COPIED_AND_SAVED: &'static str = "已复制长图并保存。";
    pub const BUSY: &'static str = "正在拼接长图…";
    pub const TIMEOUT: &'static str = "滚动超时，没有截到内容。";
    pub const PICK_HINT: &'static str = "点选窗口或拖选区域；Esc 取消。";
    pub const SCROLL_HINT: &'static str = "在选区内滚动；停滚约 1.5 秒后自动完成";

    pub fn capture_region<F>(
        screen_rect: PixelRect,
        options: &ScrollCaptureOptions,
        cancel_flag: &Arc<AtomicBool>,
        finish_now_flag: &Arc<AtomicBool>,
        mut on_progress: F,
    ) -> ScrollCaptureResult
    where
        F: FnMut(i32),
    {
        if screen_rect.is_empty() || screen_rect.width < 2 || screen_rect.height < 2 {
            return Self::fail(Self::BAD_REGION);
        }

        let idle_ms = options.idle_stop_ms.max(200);
        let sample_ms = options.sample_interval_ms.clamp(80, 1000);
        let max_h = options.max_height_px.max(screen_rect.height);
        let max_duration_ms = options.max_duration_ms.max(idle_ms * 20);

        let Ok(mut previous) = GdiBitBltGrabber::capture_rect(screen_rect) else {
            return Self::fail(Self::CAPTURE_FAILED);
        };
        if previous.width < 2 || previous.height < 2 {
            return Self::fail(Self::CAPTURE_FAILED);
        }

        let first_height = previous.height;
        let mut stitch = previous.clone();
        let mut stitched_h = first_height;
        let started = Instant::now();
        let mut last_change = started;
        let mut grew = false;
        let mut consecutive_rejects = 0i32;
        const MAX_CONSECUTIVE_REJECTS: i32 = 40;

        while stitched_h < max_h {
            if cancel_flag.load(Ordering::Relaxed) {
                return ScrollCaptureResult {
                    succeeded: false,
                    image: None,
                    error: None,
                };
            }
            if finish_now_flag.load(Ordering::Relaxed) {
                break;
            }
            if started.elapsed().as_millis() as u64 > max_duration_ms {
                break;
            }

            std::thread::sleep(Duration::from_millis(sample_ms));

            if cancel_flag.load(Ordering::Relaxed) {
                return ScrollCaptureResult {
                    succeeded: false,
                    image: None,
                    error: None,
                };
            }
            if finish_now_flag.load(Ordering::Relaxed) {
                break;
            }

            let Ok(next) = GdiBitBltGrabber::capture_rect(screen_rect) else {
                break;
            };

            if next.width != previous.width
                || next.height != previous.height
                || Self::buffers_visually_equal(&previous, &next)
            {
                let idle_for = last_change.elapsed().as_millis() as u64;
                if grew && idle_for >= idle_ms {
                    break;
                }
                if !grew
                    && started.elapsed().as_millis() as u64
                        >= options.max_wait_first_change_ms.max(idle_ms * 4)
                {
                    break;
                }
                continue;
            }

            let overlap = Self::estimate_overlap(&previous, &next);
            let mut append = (next.height - overlap).max(0);
            if append <= 0 {
                consecutive_rejects += 1;
                previous = next;
                last_change = Instant::now();
                if consecutive_rejects >= MAX_CONSECUTIVE_REJECTS && grew {
                    break;
                }
                continue;
            }

            consecutive_rejects = 0;
            let room = max_h - stitched_h;
            if append > room {
                append = room;
            }
            if append <= 0 {
                break;
            }

            let new_h = stitched_h + append;
            let mut grown = PixelBuffer::allocate(stitch.width, new_h);
            grown.blit_from(&stitch, PixelRect::new(0, 0, stitch.width, stitched_h), 0, 0);
            grown.blit_from(
                &next,
                PixelRect::new(0, next.height - append, next.width, append),
                0,
                stitched_h,
            );
            stitch = grown;
            stitched_h = new_h;
            grew = stitched_h > first_height + 2;
            last_change = Instant::now();
            on_progress(stitched_h);
            previous = next;

            if stitched_h >= max_h {
                break;
            }
        }

        if stitched_h <= first_height + 2 {
            let err = if started.elapsed().as_millis() as u64 >= max_duration_ms {
                Self::TIMEOUT
            } else {
                Self::TOO_SHORT
            };
            return Self::fail(err);
        }

        ScrollCaptureResult {
            succeeded: true,
            image: Some(stitch),
            error: None,
        }
    }

    fn fail(msg: &str) -> ScrollCaptureResult {
        ScrollCaptureResult {
            succeeded: false,
            image: None,
            error: Some(msg.to_string()),
        }
    }

    pub fn buffers_visually_equal(a: &PixelBuffer, b: &PixelBuffer) -> bool {
        if a.width != b.width || a.height != b.height {
            return false;
        }
        let len = a.bgra.len().min(b.bgra.len());
        let step = ((len / 4096) & !3).max(4);
        let mut i = 0usize;
        while i < len {
            if a.bgra[i] != b.bgra[i] {
                return false;
            }
            if i + 2 < len && (a.bgra[i + 1] != b.bgra[i + 1] || a.bgra[i + 2] != b.bgra[i + 2]) {
                return false;
            }
            i += step;
        }
        true
    }

    pub fn estimate_overlap(previous: &PixelBuffer, next: &PixelBuffer) -> i32 {
        let w = previous.width.min(next.width);
        let h = previous.height.min(next.height);
        if w < 2 || h < 12 {
            return next.height;
        }
        const MIN_APPEND: i32 = 4;
        const PASS_MEAN_ABS: f64 = 12.0;

        let mut fingerprint_h = (h / 4).clamp(24, (h / 3).max(24));
        if fingerprint_h > h - MIN_APPEND {
            fingerprint_h = (h - MIN_APPEND).max(8);
        }

        let max_match_y = h - fingerprint_h;
        if max_match_y < 0 {
            return next.height;
        }

        let strip_y = previous.height - fingerprint_h;
        let mut best_y: i32 = -1;
        let mut best_score = f64::MAX;

        for y in 0..=max_match_y {
            let score = Self::mean_abs_diff_strip(previous, strip_y, next, y, fingerprint_h, w);
            if score < best_score {
                best_score = score;
                best_y = y;
            }
        }

        if best_y < 0 {
            return next.height;
        }

        let delta = (h - fingerprint_h) - best_y;
        if delta < MIN_APPEND || best_score > PASS_MEAN_ABS {
            return next.height;
        }

        let overlap = h - delta;
        let verify = Self::mean_abs_diff_strip(
            previous,
            previous.height - overlap,
            next,
            0,
            overlap,
            w,
        );
        if verify > PASS_MEAN_ABS * 1.25 {
            return next.height;
        }
        overlap
    }

    fn mean_abs_diff_strip(
        a: &PixelBuffer,
        a_y: i32,
        b: &PixelBuffer,
        b_y: i32,
        rows: i32,
        width: i32,
    ) -> f64 {
        if rows <= 0 || width <= 0 {
            return f64::MAX;
        }
        let y_step = (rows / 24).max(1);
        let x_step = (width / 48).max(1);
        let mut sum = 0u64;
        let mut samples = 0u64;

        let mut y = 0;
        while y < rows {
            let ai = ((a_y + y) * a.stride) as usize;
            let bi = ((b_y + y) * b.stride) as usize;
            let mut x = 0;
            while x < width {
                let ao = ai + (x * 4) as usize;
                let bo = bi + (x * 4) as usize;
                sum += u64::from(a.bgra[ao].abs_diff(b.bgra[bo]));
                sum += u64::from(a.bgra[ao + 1].abs_diff(b.bgra[bo + 1]));
                sum += u64::from(a.bgra[ao + 2].abs_diff(b.bgra[bo + 2]));
                samples += 3;
                x += x_step;
            }
            y += y_step;
        }

        if samples == 0 {
            f64::MAX
        } else {
            sum as f64 / samples as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_overlap_recovers_exact_scroll_delta() {
        // Build a synthetic 80x240 patterned document, take [0..120] and [40..160].
        let mut doc = PixelBuffer::allocate(80, 240);
        for y in 0..240 {
            for x in 0..80 {
                let i = (y * doc.stride + x * 4) as usize;
                let band = ((y * 7 + x * 3) ^ (y / 5 * 31)) as u8;
                doc.bgra[i] = band;
                doc.bgra[i + 1] = band.wrapping_mul(3).wrapping_add((y % 97) as u8);
                doc.bgra[i + 2] = (y.wrapping_mul(11) ^ x.wrapping_mul(5)) as u8;
                doc.bgra[i + 3] = 255;
            }
        }
        let frame1 = doc.crop(PixelRect::new(0, 0, 80, 120)).unwrap();
        let frame2 = doc.crop(PixelRect::new(0, 40, 80, 120)).unwrap();
        let overlap = ScrollCapture::estimate_overlap(&frame1, &frame2);
        assert_eq!(overlap, 80); // 120 - 40 = 80 rows overlap
    }
}
