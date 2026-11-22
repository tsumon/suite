use crate::monitor::MonitorCapture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PixelRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub fn from_corners(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let left = x1.min(x2);
        let top = y1.min(y2);
        let right = x1.max(x2);
        let bottom = y1.max(y2);
        Self::new(left, top, right - left, bottom - top)
    }

    pub fn intersect(self, other: Self) -> Self {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= left || bottom <= top {
            Self::default()
        } else {
            Self::new(left, top, right - left, bottom - top)
        }
    }

    pub fn clamp_to(self, width: i32, height: i32) -> Self {
        self.intersect(Self::new(0, 0, width, height))
    }

    pub const fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.right() && py < self.bottom()
    }
}

#[derive(Debug, Clone)]
pub struct PixelBuffer {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub bgra: Vec<u8>,
}

impl PixelBuffer {
    pub fn new(width: i32, height: i32, bgra: Vec<u8>, stride: i32) -> Result<Self, String> {
        if width <= 0 || height <= 0 {
            return Err("Buffer must be at least 1x1.".to_string());
        }
        if stride < width * 4 {
            return Err("Stride smaller than width * 4.".to_string());
        }
        if bgra.len() < (stride * height) as usize {
            return Err("Buffer smaller than stride * height.".to_string());
        }
        Ok(Self {
            width,
            height,
            stride,
            bgra,
        })
    }

    pub fn allocate(width: i32, height: i32) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let stride = w * 4;
        Self {
            width: w,
            height: h,
            stride,
            bgra: vec![0u8; (stride * h) as usize],
        }
    }

    pub fn release_pixels(&mut self) {
        self.bgra = Vec::new();
        self.width = 0;
        self.height = 0;
        self.stride = 0;
    }

    pub fn crop(&self, rect: PixelRect) -> Result<Self, String> {
        let clipped = rect.clamp_to(self.width, self.height);
        if clipped.is_empty() {
            return Err("Crop rectangle is empty.".to_string());
        }
        let mut dest = Self::allocate(clipped.width, clipped.height);
        let row_bytes = (clipped.width * 4) as usize;
        for y in 0..clipped.height {
            let src_idx = ((clipped.y + y) * self.stride + clipped.x * 4) as usize;
            let dst_idx = (y * dest.stride) as usize;
            dest.bgra[dst_idx..dst_idx + row_bytes]
                .copy_from_slice(&self.bgra[src_idx..src_idx + row_bytes]);
        }
        Ok(dest)
    }

    pub fn blit_from(&mut self, src: &Self, src_rect: PixelRect, mut dest_x: i32, mut dest_y: i32) {
        let mut clipped_src = src_rect.clamp_to(src.width, src.height);
        if clipped_src.is_empty() {
            return;
        }
        let mut copy_w = clipped_src.width;
        let mut copy_h = clipped_src.height;
        if dest_x < 0 {
            copy_w += dest_x;
            clipped_src = PixelRect::new(clipped_src.x - dest_x, clipped_src.y, copy_w, copy_h);
            dest_x = 0;
        }
        if dest_y < 0 {
            copy_h += dest_y;
            clipped_src = PixelRect::new(clipped_src.x, clipped_src.y - dest_y, copy_w, copy_h);
            dest_y = 0;
        }
        if dest_x + copy_w > self.width {
            copy_w = self.width - dest_x;
        }
        if dest_y + copy_h > self.height {
            copy_h = self.height - dest_y;
        }
        if copy_w <= 0 || copy_h <= 0 {
            return;
        }
        let row_bytes = (copy_w * 4) as usize;
        for y in 0..copy_h {
            let src_idx = ((clipped_src.y + y) * src.stride + clipped_src.x * 4) as usize;
            let dst_idx = ((dest_y + y) * self.stride + dest_x * 4) as usize;
            self.bgra[dst_idx..dst_idx + row_bytes]
                .copy_from_slice(&src.bgra[src_idx..src_idx + row_bytes]);
        }
    }

    pub fn try_get_pixel(&self, x: i32, y: i32) -> Option<(u8, u8, u8, u8)> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || self.bgra.is_empty() {
            return None;
        }
        let idx = (y * self.stride + x * 4) as usize;
        Some((
            self.bgra[idx],
            self.bgra[idx + 1],
            self.bgra[idx + 2],
            self.bgra[idx + 3],
        ))
    }

    pub fn packed_bgra(&self) -> Vec<u8> {
        let row_bytes = (self.width * 4) as usize;
        if self.stride as usize == row_bytes {
            return self.bgra[..(row_bytes * self.height as usize)].to_vec();
        }
        let mut out = vec![0u8; row_bytes * self.height as usize];
        for y in 0..self.height {
            let src_idx = (y * self.stride) as usize;
            let dst_idx = y as usize * row_bytes;
            out[dst_idx..dst_idx + row_bytes]
                .copy_from_slice(&self.bgra[src_idx..src_idx + row_bytes]);
        }
        out
    }

    /// Returns a copy rotated 90 degrees clockwise.
    pub fn rotate_90_cw(&self) -> Self {
        if self.width <= 0 || self.height <= 0 || self.bgra.is_empty() {
            return self.clone();
        }
        let new_w = self.height;
        let new_h = self.width;
        let mut out = Self::allocate(new_w, new_h);
        for y in 0..self.height {
            let src_row = (y * self.stride) as usize;
            let dx = self.height - 1 - y;
            for x in 0..self.width {
                let si = src_row + (x * 4) as usize;
                let dy = x;
                let di = (dy * out.stride + dx * 4) as usize;
                out.bgra[di..di + 4].copy_from_slice(&self.bgra[si..si + 4]);
            }
        }
        out
    }

    /// Returns a horizontally mirrored copy.
    pub fn flip_horizontal(&self) -> Self {
        if self.width <= 0 || self.height <= 0 || self.bgra.is_empty() {
            return self.clone();
        }
        let mut out = Self::allocate(self.width, self.height);
        for y in 0..self.height {
            let src_row = (y * self.stride) as usize;
            let dst_row = (y * out.stride) as usize;
            for x in 0..self.width {
                let dx = self.width - 1 - x;
                let si = src_row + (x * 4) as usize;
                let di = dst_row + (dx * 4) as usize;
                out.bgra[di..di + 4].copy_from_slice(&self.bgra[si..si + 4]);
            }
        }
        out
    }
}

pub struct RegionCropper;

impl RegionCropper {
    pub fn crop(frames: &[MonitorCapture], virtual_rect: PixelRect) -> Result<PixelBuffer, String> {
        if virtual_rect.is_empty() {
            return Err("Empty selection rect.".to_string());
        }
        let mut dest = PixelBuffer::allocate(virtual_rect.width, virtual_rect.height);
        for frame in frames {
            let overlap = virtual_rect.intersect(frame.monitor.bounds);
            if overlap.is_empty() {
                continue;
            }
            let src_rect = PixelRect::new(
                Self::map(
                    overlap.x - frame.monitor.bounds.x,
                    frame.monitor.bounds.width,
                    frame.buffer.width,
                ),
                Self::map(
                    overlap.y - frame.monitor.bounds.y,
                    frame.monitor.bounds.height,
                    frame.buffer.height,
                ),
                Self::map(
                    overlap.width,
                    frame.monitor.bounds.width,
                    frame.buffer.width,
                )
                .max(1),
                Self::map(
                    overlap.height,
                    frame.monitor.bounds.height,
                    frame.buffer.height,
                )
                .max(1),
            );
            dest.blit_from(
                &frame.buffer,
                src_rect,
                overlap.x - virtual_rect.x,
                overlap.y - virtual_rect.y,
            );
        }
        Ok(dest)
    }

    pub fn dominant_method(frames: &[MonitorCapture], virtual_rect: PixelRect) -> crate::monitor::CaptureMethod {
        let mut best = crate::monitor::CaptureMethod::None;
        let mut best_area = -1i64;
        for frame in frames {
            let overlap = virtual_rect.intersect(frame.monitor.bounds);
            let area = i64::from(overlap.width) * i64::from(overlap.height);
            if area > best_area {
                best_area = area;
                best = frame.method;
            }
        }
        best
    }

    fn map(value: i32, from: i32, to: i32) -> i32 {
        if from <= 0 {
            0
        } else {
            ((value as f64) * (to as f64 / from as f64)).round() as i32
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseAction {
    Stay,
    CommitHover,
    CommitDrag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    None = 0,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

pub struct CaptureUx;

impl CaptureUx {
    pub const DRAG_THRESHOLD_PX: i32 = 4;
    pub const MIN_COMMIT_PX: i32 = 4;
    pub const RESIZE_GRIP_PX: i32 = 10;
    pub const ANNOTATE_ARM_MS: u64 = 400;
    pub const OVERLAY_HINT: &'static str = "悬停点窗，拖动手选。松手出工具栏。Esc 取消。";

    pub fn open_annotation_after_region(pin_after_capture: bool, annotate_modifier: bool) -> bool {
        annotate_modifier || !pin_after_capture
    }

    pub fn is_drag(start_x: i32, start_y: i32, x: i32, y: i32) -> bool {
        let dx = i64::from(x - start_x);
        let dy = i64::from(y - start_y);
        dx * dx + dy * dy >= i64::from(Self::DRAG_THRESHOLD_PX * Self::DRAG_THRESHOLD_PX)
    }

    pub fn decide_release(
        is_dragging: bool,
        drag_width: i32,
        drag_height: i32,
        has_hover: bool,
    ) -> ReleaseAction {
        if is_dragging {
            if drag_width >= Self::MIN_COMMIT_PX && drag_height >= Self::MIN_COMMIT_PX {
                ReleaseAction::CommitDrag
            } else if has_hover {
                ReleaseAction::CommitHover
            } else {
                ReleaseAction::Stay
            }
        } else if has_hover {
            ReleaseAction::CommitHover
        } else {
            ReleaseAction::Stay
        }
    }

    pub fn size_chip(width: i32, height: i32) -> String {
        format!("{width} × {height} px")
    }

    pub fn move_rect(origin: PixelRect, dx: i32, dy: i32, clamp_bounds: PixelRect) -> PixelRect {
        if origin.is_empty() || clamp_bounds.is_empty() {
            return origin;
        }
        let w = origin.width;
        let h = origin.height;
        let mut x = origin.x + dx;
        let mut y = origin.y + dy;
        let min_x = clamp_bounds.x;
        let min_y = clamp_bounds.y;
        let max_x = clamp_bounds.right() - w;
        let max_y = clamp_bounds.bottom() - h;
        if max_x < min_x {
            x = min_x;
        } else {
            x = x.clamp(min_x, max_x);
        }
        if max_y < min_y {
            y = min_y;
        } else {
            y = y.clamp(min_y, max_y);
        }
        PixelRect::new(x, y, w, h)
    }

    pub fn union_monitor_bounds(bounds: &[PixelRect], fallback: PixelRect) -> PixelRect {
        if bounds.is_empty() {
            return fallback;
        }
        let mut left = i32::MAX;
        let mut top = i32::MAX;
        let mut right = i32::MIN;
        let mut bottom = i32::MIN;
        for b in bounds {
            if b.is_empty() {
                continue;
            }
            left = left.min(b.x);
            top = top.min(b.y);
            right = right.max(b.right());
            bottom = bottom.max(b.bottom());
        }
        if right <= left || bottom <= top {
            fallback
        } else {
            PixelRect::new(left, top, right - left, bottom - top)
        }
    }

    pub fn index_of_largest_overlap(monitor_bounds: &[PixelRect], selection: PixelRect) -> Option<usize> {
        if monitor_bounds.is_empty() || selection.is_empty() {
            return None;
        }
        let mut best: Option<usize> = None;
        let mut best_area = -1i64;
        for (i, b) in monitor_bounds.iter().enumerate() {
            let overlap = selection.intersect(*b);
            if overlap.is_empty() {
                continue;
            }
            let area = i64::from(overlap.width) * i64::from(overlap.height);
            if area > best_area {
                best_area = area;
                best = Some(i);
            }
        }
        best
    }

    pub fn hit_test_resize_handle(sel: PixelRect, x: i32, y: i32, grip_px: i32) -> ResizeHandle {
        if sel.is_empty() || grip_px < 1 {
            return ResizeHandle::None;
        }
        if x < sel.x || y < sel.y || x >= sel.right() || y >= sel.bottom() {
            return ResizeHandle::None;
        }
        let grip_x = grip_px.min(sel.width.max(1));
        let grip_y = grip_px.min(sel.height.max(1));
        let dist_w = x - sel.x;
        let dist_e = sel.right() - 1 - x;
        let dist_n = y - sel.y;
        let dist_s = sel.bottom() - 1 - y;

        let mut near_w = dist_w < grip_x;
        let mut near_e = dist_e < grip_x;
        let mut near_n = dist_n < grip_y;
        let mut near_s = dist_s < grip_y;

        if near_w && near_e {
            if dist_w <= dist_e {
                near_e = false;
            } else {
                near_w = false;
            }
        }
        if near_n && near_s {
            if dist_n <= dist_s {
                near_s = false;
            } else {
                near_n = false;
            }
        }

        match (near_n, near_s, near_w, near_e) {
            (true, false, true, false) => ResizeHandle::NW,
            (true, false, false, true) => ResizeHandle::NE,
            (false, true, true, false) => ResizeHandle::SW,
            (false, true, false, true) => ResizeHandle::SE,
            (true, false, false, false) => ResizeHandle::N,
            (false, true, false, false) => ResizeHandle::S,
            (false, false, true, false) => ResizeHandle::W,
            (false, false, false, true) => ResizeHandle::E,
            _ => ResizeHandle::None,
        }
    }

    pub fn resize_rect(start: PixelRect, handle: ResizeHandle, x: i32, y: i32) -> PixelRect {
        let mut l = start.x;
        let mut t = start.y;
        let mut r = start.right();
        let mut b = start.bottom();
        match handle {
            ResizeHandle::N => t = y,
            ResizeHandle::S => b = y,
            ResizeHandle::E => r = x,
            ResizeHandle::W => l = x,
            ResizeHandle::NE => return Self::scale_corner(start, start.x, start.bottom(), x, y),
            ResizeHandle::NW => return Self::scale_corner(start, start.right(), start.bottom(), x, y),
            ResizeHandle::SE => return Self::scale_corner(start, start.x, start.y, x, y),
            ResizeHandle::SW => return Self::scale_corner(start, start.right(), start.y, x, y),
            ResizeHandle::None => {}
        }
        PixelRect::from_corners(l, t, r, b)
    }

    fn scale_corner(start: PixelRect, fixed_x: i32, fixed_y: i32, x: i32, y: i32) -> PixelRect {
        if start.width <= 0 || start.height <= 0 {
            return PixelRect::from_corners(fixed_x, fixed_y, x, y);
        }
        let sx = (x - fixed_x) as f64 / start.width as f64;
        let sy = (y - fixed_y) as f64 / start.height as f64;
        let mut scale = if sx.abs() > sy.abs() { sx } else { sy };
        if scale.abs() < 0.01 {
            scale = 0.01;
        }
        let w = Self::MIN_COMMIT_PX.max((start.width as f64 * scale.abs()).round() as i32);
        let h = Self::MIN_COMMIT_PX.max((start.height as f64 * scale.abs()).round() as i32);
        let nx = if x < fixed_x { fixed_x - w } else { fixed_x };
        let ny = if y < fixed_y { fixed_y - h } else { fixed_y };
        PixelRect::new(nx, ny, w, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{CaptureMethod, MonitorInfo};

    #[test]
    fn region_cropper_stitches_across_monitors() {
        let mut left_buf = PixelBuffer::allocate(100, 100);
        left_buf.bgra.fill(10);
        let mut right_buf = PixelBuffer::allocate(100, 100);
        right_buf.bgra.fill(200);

        let frames = vec![
            MonitorCapture {
                monitor: MonitorInfo {
                    handle: 1,
                    bounds: PixelRect::new(0, 0, 100, 100),
                    work_area: PixelRect::new(0, 0, 100, 100),
                    device_name: "M1".into(),
                    is_primary: true,
                    dpi_x: 96,
                    dpi_y: 96,
                },
                buffer: left_buf,
                method: CaptureMethod::GdiBitBlt,
            },
            MonitorCapture {
                monitor: MonitorInfo {
                    handle: 2,
                    bounds: PixelRect::new(100, 0, 100, 100),
                    work_area: PixelRect::new(100, 0, 100, 100),
                    device_name: "M2".into(),
                    is_primary: false,
                    dpi_x: 96,
                    dpi_y: 96,
                },
                buffer: right_buf,
                method: CaptureMethod::GdiBitBlt,
            },
        ];

        let cropped = RegionCropper::crop(&frames, PixelRect::new(90, 10, 20, 10)).unwrap();
        assert_eq!(cropped.width, 20);
        assert_eq!(cropped.height, 10);
        assert_eq!(cropped.try_get_pixel(5, 5), Some((10, 10, 10, 10)));
        assert_eq!(cropped.try_get_pixel(15, 5), Some((200, 200, 200, 200)));
    }

    #[test]
    fn resize_handle_corners_beat_edges() {
        let sel = PixelRect::new(100, 100, 200, 100);
        assert_eq!(
            CaptureUx::hit_test_resize_handle(sel, 102, 102, 10),
            ResizeHandle::NW
        );
        assert_eq!(
            CaptureUx::hit_test_resize_handle(sel, 150, 102, 10),
            ResizeHandle::N
        );
        assert_eq!(
            CaptureUx::hit_test_resize_handle(sel, 150, 150, 10),
            ResizeHandle::None
        );
    }

    #[test]
    fn rotate_90_cw_and_flip_horizontal_preserve_exact_pixels() {
        let mut buf = PixelBuffer::allocate(3, 2);
        // Set (0,0)=R, (2,0)=G, (0,1)=B
        let set = |b: &mut PixelBuffer, x: i32, y: i32, rgba: [u8; 4]| {
            let i = (y * b.stride + x * 4) as usize;
            b.bgra[i..i + 4].copy_from_slice(&rgba);
        };
        set(&mut buf, 0, 0, [0, 0, 255, 255]);
        set(&mut buf, 2, 0, [0, 255, 0, 255]);
        set(&mut buf, 0, 1, [255, 0, 0, 255]);

        let rot = buf.rotate_90_cw();
        assert_eq!((rot.width, rot.height), (2, 3));
        assert_eq!(rot.try_get_pixel(1, 0), Some((0, 0, 255, 255)));
        assert_eq!(rot.try_get_pixel(1, 2), Some((0, 255, 0, 255)));
        assert_eq!(rot.try_get_pixel(0, 0), Some((255, 0, 0, 255)));

        let flipped = buf.flip_horizontal();
        assert_eq!((flipped.width, flipped.height), (3, 2));
        assert_eq!(flipped.try_get_pixel(2, 0), Some((0, 0, 255, 255)));
        assert_eq!(flipped.try_get_pixel(0, 0), Some((0, 255, 0, 255)));
        assert_eq!(flipped.try_get_pixel(2, 1), Some((255, 0, 0, 255)));
    }
}
