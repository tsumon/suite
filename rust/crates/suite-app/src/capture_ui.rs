use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use suite_capture::{
    AnnotationOp, CaptureUx, ColorPick, ImageClipboard, MonitorCapture, MonitorEnumerator,
    PixelBuffer, PixelDraw, PixelRect, RegionCropper, ReleaseAction, ResizeHandle,
    ScreenCapturePipeline, WindowPicker, WindowsOcr, bake_annotations, save_png_to_path,
};
use suite_contracts::{AnnotateTool, AnnotateToolbar, CurveStyle, StackPlacement};
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CreateCompatibleBitmap,
    CreateCompatibleDC, CreatePen, CreateSolidBrush, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetStockObject,
    GetTextExtentPoint32W, HBITMAP, HBRUSH, HDC, HFONT, HOLLOW_BRUSH, LineTo, MoveToEx,
    PAINTSTRUCT, PS_SOLID, Polygon, Rectangle, RoundRect, SRCCOPY, SelectObject, SetBkColor,
    SetBkMode, SetTextColor, StretchDIBits, TRANSPARENT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, SetFocus, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT,
    VK_RETURN, VK_RIGHT, VK_SHIFT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, ES_AUTOHSCROLL, GetWindowLongPtrW,
    GetWindowTextW, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW, IDC_CROSS, IDC_SIZEALL, IDC_SIZENESW,
    IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, KillTimer, LoadCursorW, SW_HIDE, SW_SHOW, SetCursor,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT, WM_RBUTTONUP, WM_TIMER,
    WS_CHILD, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use crate::color_pick_ui::draw_magnifier_box;
use crate::d2d::D2dRenderer;
use crate::messages::{
    CaptureDoneMsg, ColorPickDoneMsg, OcrDoneMsg, WM_APP_CAPTURE_DONE, WM_APP_COLOR_PICK_DONE,
    WM_APP_OCR_DONE, post_boxed,
};
use crate::win32_util::{
    create_ui_font, hinstance, invalidate_all, register_window_class, s_px, save_png_dialog,
    set_control_font,
};

const CAPTURE_OVERLAY_CLASS: &str = "SuiteCaptureOverlayWnd";
const EDIT_CHILD_ID: usize = 5001;
const TIMER_STATUS_HIDE: usize = 2;

#[derive(Debug, Clone)]
pub struct CaptureToolPrefs {
    pub ellipse_mode: bool,
    pub curve_style: CurveStyle,
    pub blur_mode: bool,
    pub color_bgr: (u8, u8, u8),
    pub stroke_thickness: i32,
    pub marker_thickness: i32,
    pub mosaic_radius: i32,
    pub eraser_thickness: i32,
    pub number_radius: i32,
    pub text_size: i32,
}

impl Default for CaptureToolPrefs {
    fn default() -> Self {
        Self {
            ellipse_mode: false,
            curve_style: CurveStyle::Solid,
            blur_mode: false,
            color_bgr: (PixelDraw::STROKE_B, PixelDraw::STROKE_G, PixelDraw::STROKE_R),
            stroke_thickness: 4,
            marker_thickness: 14,
            mosaic_radius: 16,
            eraser_thickness: 18,
            number_radius: 16,
            text_size: 18,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayPhase {
    Select,
    Annotate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SubHit {
    ShapeRect,
    ShapeEllipse,
    CurveStyle(CurveStyle),
    MosaicPixel,
    MosaicBlur,
    SizeChip(i32),
    Swatch(usize),
}

struct CaptureShared {
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    prefs: Rc<RefCell<CaptureToolPrefs>>,
    frames: Vec<MonitorCapture>,
    dimmed_frames: Vec<PixelBuffer>,
    windows: Vec<HWND>,
    primary_hwnd: HWND,
    phase: OverlayPhase,
    show_magnifier: bool,
    pin_after_capture: bool,
    save_dir: String,
    finished: bool,

    // Select phase state
    hover: Option<PixelRect>,
    press: Option<(i32, i32)>,
    drag: Option<PixelRect>,
    dragging_select: bool,
    cursor_virtual: (i32, i32),

    // Annotate phase state
    armed_at: Instant,
    dom_mon_idx: usize,
    selection: PixelRect,
    original_crop: PixelBuffer,
    baked_crop: PixelBuffer,
    bake_dirty: bool,
    ops: Vec<AnnotationOp>,
    redo: Vec<AnnotationOp>,
    active_tool: Option<AnnotateTool>,
    hover_tool: Option<AnnotateTool>,
    hover_sub: Option<SubHit>,
    ellipse_mode: bool,
    curve_style: CurveStyle,
    blur_mode: bool,
    color_bgr: (u8, u8, u8),
    stroke_thickness: i32,
    marker_thickness: i32,
    mosaic_radius: i32,
    eraser_thickness: i32,
    number_radius: i32,
    text_size: i32,

    // Interactive draw/resize/move
    drawing_stroke: bool,
    draw_start_px: (i32, i32),
    draw_curr_px: (i32, i32),
    stroke_pts: Vec<(i32, i32)>,
    mosaic_draft: Option<PixelBuffer>,
    mosaic_draft_last: usize,
    resizing: bool,
    resize_handle: ResizeHandle,
    resize_origin: PixelRect,
    moving_sel: bool,
    move_origin: PixelRect,
    move_start_virt: (i32, i32),

    // Native Win32 IME text input child
    text_edit: HWND,
    text_anchor_px: (i32, i32),
    edit_bg_brush: HBRUSH,

    // Status pill & OCR
    ocr_busy: bool,
    status_text: Option<String>,
}

struct PerMonitorOverlay {
    mon_idx: usize,
    shared: Rc<RefCell<CaptureShared>>,
    font_ui: HFONT,
    font_small: HFONT,
    mem_dc: HDC,
    mem_bmp: HBITMAP,
    mem_w: i32,
    mem_h: i32,
}

pub struct CaptureSession {
    shared: Rc<RefCell<CaptureShared>>,
}

impl CaptureSession {
    pub fn begin(
        controller: HWND,
        d2d: Rc<RefCell<D2dRenderer>>,
        pin_after_capture: bool,
        show_magnifier: bool,
        save_dir: String,
        prefs: Rc<RefCell<CaptureToolPrefs>>,
    ) -> Result<Self, String> {
        let frames = ScreenCapturePipeline::capture_all()?;
        let dimmed_frames: Vec<PixelBuffer> = frames
            .iter()
            .map(|f| build_dimmed_copy(&f.buffer))
            .collect();

        let cross = unsafe { LoadCursorW(None, IDC_CROSS).ok() };
        register_window_class(
            CAPTURE_OVERLAY_CLASS,
            Some(capture_wnd_proc),
            true,
            cross,
        );

        let cursor_virtual = MonitorEnumerator::cursor_pos();
        let edit_bg_brush = unsafe { CreateSolidBrush(COLORREF(0x001E_1E1E)) };
        let dummy_buf = PixelBuffer::allocate(1, 1);
        let p = prefs.borrow().clone();

        let shared = Rc::new(RefCell::new(CaptureShared {
            controller,
            d2d,
            prefs,
            frames,
            dimmed_frames,
            windows: Vec::new(),
            primary_hwnd: HWND::default(),
            phase: OverlayPhase::Select,
            show_magnifier,
            pin_after_capture,
            save_dir,
            finished: false,
            hover: None,
            press: None,
            drag: None,
            dragging_select: false,
            cursor_virtual,
            armed_at: Instant::now(),
            dom_mon_idx: 0,
            selection: PixelRect::default(),
            original_crop: dummy_buf.clone(),
            baked_crop: dummy_buf,
            bake_dirty: false,
            ops: Vec::new(),
            redo: Vec::new(),
            active_tool: None,
            hover_tool: None,
            hover_sub: None,
            ellipse_mode: p.ellipse_mode,
            curve_style: p.curve_style,
            blur_mode: p.blur_mode,
            color_bgr: p.color_bgr,
            stroke_thickness: p.stroke_thickness,
            marker_thickness: p.marker_thickness,
            mosaic_radius: p.mosaic_radius,
            eraser_thickness: p.eraser_thickness,
            number_radius: p.number_radius,
            text_size: p.text_size,
            drawing_stroke: false,
            draw_start_px: (0, 0),
            draw_curr_px: (0, 0),
            stroke_pts: Vec::new(),
            mosaic_draft: None,
            mosaic_draft_last: 0,
            resizing: false,
            resize_handle: ResizeHandle::None,
            resize_origin: PixelRect::default(),
            moving_sel: false,
            move_origin: PixelRect::default(),
            move_start_virt: (0, 0),
            text_edit: HWND::default(),
            text_anchor_px: (0, 0),
            edit_bg_brush,
            ocr_busy: false,
            status_text: None,
        }));

        let cls = to_wide_null(CAPTURE_OVERLAY_CLASS);
        let title = to_wide_null("Suite Capture");
        let mon_info: Vec<(PixelRect, u32)> = shared
            .borrow()
            .frames
            .iter()
            .map(|f| (f.monitor.bounds, f.monitor.dpi_x.max(96)))
            .collect();

        let mut first_hwnd = HWND::default();
        for (idx, &(b, mon_dpi)) in mon_info.iter().enumerate() {
            let font_ui = create_ui_font(s_px(13, mon_dpi), false, false, "Microsoft YaHei UI");
            let font_small = create_ui_font(s_px(12, mon_dpi), false, false, "Microsoft YaHei UI");
            let state = Rc::new(RefCell::new(PerMonitorOverlay {
                mon_idx: idx,
                shared: Rc::clone(&shared),
                font_ui,
                font_small,
                mem_dc: HDC::default(),
                mem_bmp: HBITMAP::default(),
                mem_w: 0,
                mem_h: 0,
            }));
            let raw_ptr = Rc::into_raw(state);
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    PCWSTR(cls.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    WS_POPUP,
                    b.x,
                    b.y,
                    b.width,
                    b.height,
                    None,
                    None,
                    Some(hinstance()),
                    Some(raw_ptr.cast()),
                )
                .unwrap_or_default()
            };
            if hwnd.is_invalid() {
                unsafe {
                    drop(Rc::from_raw(raw_ptr));
                }
                continue;
            }
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            shared.borrow_mut().windows.push(hwnd);
            if first_hwnd.is_invalid() || b.contains(cursor_virtual.0, cursor_virtual.1) {
                first_hwnd = hwnd;
            }
        }

        if !first_hwnd.is_invalid() {
            shared.borrow_mut().primary_hwnd = first_hwnd;
            unsafe {
                let _ = SetWindowPos(
                    first_hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                        | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
                );
                let _ = SetForegroundWindow(first_hwnd);
                let _ = SetCapture(first_hwnd);
            }
        }

        Ok(Self { shared })
    }

    pub fn cancel(&self) {
        finish_capture_session(
            &self.shared,
            CaptureDoneMsg {
                cancelled: true,
                pin_requested: false,
                save_requested: false,
                image: None,
                selection: PixelRect::default(),
                dpi_x: 96,
                dpi_y: 96,
                error: None,
            },
        );
    }

    pub fn handle_ocr_done(&self, msg: OcrDoneMsg) {
        let mut s = self.shared.borrow_mut();
        s.ocr_busy = false;
        if s.finished {
            return;
        }
        let status = if msg.result.succeeded {
            if let Some(txt) = msg.result.text {
                match ImageClipboard::copy_text(&txt) {
                    Ok(()) => WindowsOcr::COPIED.to_string(),
                    Err(_) => WindowsOcr::CLIPBOARD_FAILED.to_string(),
                }
            } else {
                WindowsOcr::NO_TEXT.to_string()
            }
        } else {
            msg.result.error.unwrap_or_else(|| WindowsOcr::FAILED.to_string())
        };
        s.status_text = Some(status);
        let target = s.windows.get(s.dom_mon_idx).copied().unwrap_or(HWND(msg.overlay_hwnd as *mut _));
        drop(s);
        if !target.is_invalid() {
            unsafe {
                let _ = SetTimer(Some(target), TIMER_STATUS_HIDE, 2400, None);
            }
            invalidate_all(target);
        }
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn build_dimmed_copy(src: &PixelBuffer) -> PixelBuffer {
    let mut out = src.clone();
    // Apply 0x6E/255 (~43% black dim) quickly across BGR pixels
    const KEEP: u32 = 255 - 0x6E;
    for px in out.bgra.as_chunks_mut::<4>().0 {
        px[0] = ((u32::from(px[0]) * KEEP) >> 8) as u8;
        px[1] = ((u32::from(px[1]) * KEEP) >> 8) as u8;
        px[2] = ((u32::from(px[2]) * KEEP) >> 8) as u8;
        px[3] = 255;
    }
    out
}

fn finish_capture_session(shared: &Rc<RefCell<CaptureShared>>, msg: CaptureDoneMsg) {
    let (controller, windows, edit_hwnd, brush) = {
        let mut s = shared.borrow_mut();
        if s.finished {
            return;
        }
        s.finished = true;
        let edit = s.text_edit;
        s.text_edit = HWND::default();
        let br = s.edit_bg_brush;
        s.edit_bg_brush = HBRUSH::default();
        s.mosaic_draft = None;
        s.original_crop.release_pixels();
        s.baked_crop.release_pixels();
        for f in &mut s.frames {
            f.buffer.release_pixels();
        }
        s.frames.clear();
        for d in &mut s.dimmed_frames {
            d.release_pixels();
        }
        s.dimmed_frames.clear();
        (s.controller, std::mem::take(&mut s.windows), edit, br)
    };

    unsafe {
        let _ = ReleaseCapture();
        if !edit_hwnd.is_invalid() {
            let _ = DestroyWindow(edit_hwnd);
        }
        if !brush.is_invalid() {
            let _ = DeleteObject(brush.into());
        }
    }
    for w in windows {
        if !w.is_invalid() {
            unsafe {
                let _ = DestroyWindow(w);
            }
        }
    }
    post_boxed(controller, WM_APP_CAPTURE_DONE, 0, msg);
}

fn ensure_bake(s: &mut CaptureShared) {
    if !s.bake_dirty {
        return;
    }
    let d2d = Rc::clone(&s.d2d);
    let baked = bake_annotations(&s.original_crop, &s.ops, |buf, x, y, txt, b, g, r, fsz| {
        d2d.borrow_mut()
            .draw_text_on_buffer(buf, x, y, txt, b, g, r, fsz);
    });
    s.baked_crop = baked;
    s.bake_dirty = false;
}

fn tb_dpi(s: &CaptureShared) -> u32 {
    s.frames
        .get(s.dom_mon_idx)
        .map(|f| f.monitor.dpi_x.max(96))
        .unwrap_or(96)
}

fn toolbar_width_px(dpi: u32) -> i32 {
    let cell = s_px(AnnotateToolbar::CELL as i32, dpi);
    let sep = s_px(AnnotateToolbar::SEPARATOR_WIDTH as i32, dpi);
    let pad = s_px(16, dpi);
    (AnnotateToolbar::ORDER.len() as i32) * cell + 2 * sep + pad
}

#[derive(Clone)]
struct SubbarItem {
    hit: SubHit,
    label: &'static str,
    rect: RECT,
    active: bool,
    swatch_bgr: Option<(u8, u8, u8)>,
}

fn build_subbar_items(s: &CaptureShared, tool: AnnotateTool, sx: i32, sy: i32, dpi: u32) -> (Vec<SubbarItem>, i32) {
    let sh = s_px(AnnotateToolbar::SUB_BAR_HEIGHT as i32, dpi);
    let chip_top = sy + s_px(6, dpi);
    let chip_bot = sy + sh - s_px(6, dpi);
    let swatch_top = sy + s_px(8, dpi);
    let swatch_bot = sy + sh - s_px(8, dpi);
    let gap4 = s_px(4, dpi);
    let gap8 = s_px(8, dpi);

    let mut items = Vec::new();
    let mut x = sx + gap8;

    match tool {
        AnnotateTool::Shape => {
            let w = s_px(42, dpi);
            items.push(SubbarItem {
                hit: SubHit::ShapeRect,
                label: "矩形",
                rect: RECT { left: x, top: chip_top, right: x + w, bottom: chip_bot },
                active: !s.ellipse_mode,
                swatch_bgr: None,
            });
            x += w + gap4;
            items.push(SubbarItem {
                hit: SubHit::ShapeEllipse,
                label: "椭圆",
                rect: RECT { left: x, top: chip_top, right: x + w, bottom: chip_bot },
                active: s.ellipse_mode,
                swatch_bgr: None,
            });
            x += w + gap8;
        }
        AnnotateTool::Curve => {
            let w = s_px(38, dpi);
            for &(style, label) in &[
                (CurveStyle::Solid, "直线"),
                (CurveStyle::Dashed, "虚线"),
                (CurveStyle::Arrow, "箭头"),
            ] {
                items.push(SubbarItem {
                    hit: SubHit::CurveStyle(style),
                    label,
                    rect: RECT { left: x, top: chip_top, right: x + w, bottom: chip_bot },
                    active: s.curve_style == style,
                    swatch_bgr: None,
                });
                x += w + gap4;
            }
            x += gap4;
        }
        AnnotateTool::Mosaic => {
            let w1 = s_px(50, dpi);
            items.push(SubbarItem {
                hit: SubHit::MosaicPixel,
                label: "马赛克",
                rect: RECT { left: x, top: chip_top, right: x + w1, bottom: chip_bot },
                active: !s.blur_mode,
                swatch_bgr: None,
            });
            x += w1 + gap4;
            let w2 = s_px(42, dpi);
            items.push(SubbarItem {
                hit: SubHit::MosaicBlur,
                label: "模糊",
                rect: RECT { left: x, top: chip_top, right: x + w2, bottom: chip_bot },
                active: s.blur_mode,
                swatch_bgr: None,
            });
            x += w2 + gap8;
        }
        _ => {}
    }

    let (chips, current_val): (&[i32], i32) = match tool {
        AnnotateTool::Marker => (&AnnotateToolbar::MARKER_THICKNESS_CHIPS, s.marker_thickness),
        AnnotateTool::Mosaic => (&AnnotateToolbar::MOSAIC_RADIUS_CHIPS, s.mosaic_radius),
        AnnotateTool::Number => (&AnnotateToolbar::NUMBER_RADIUS_CHIPS, s.number_radius),
        AnnotateTool::Eraser => (&AnnotateToolbar::ERASER_THICKNESS_CHIPS, s.eraser_thickness),
        AnnotateTool::Text => (&AnnotateToolbar::TEXT_SIZE_CHIPS, s.text_size),
        _ => (&AnnotateToolbar::THICKNESS_CHIPS, s.stroke_thickness),
    };
    let size_w = s_px(30, dpi);
    for &c in chips {
        items.push(SubbarItem {
            hit: SubHit::SizeChip(c),
            label: "",
            rect: RECT { left: x, top: chip_top, right: x + size_w, bottom: chip_bot },
            active: c == current_val,
            swatch_bgr: None,
        });
        x += size_w + gap4;
    }

    if !matches!(tool, AnnotateTool::Mosaic | AnnotateTool::Eraser) {
        x += gap4;
        let sw_w = s_px(20, dpi);
        for (idx, &(r, g, b)) in AnnotateToolbar::PALETTE.iter().enumerate() {
            items.push(SubbarItem {
                hit: SubHit::Swatch(idx),
                label: "",
                rect: RECT { left: x, top: swatch_top, right: x + sw_w, bottom: swatch_bot },
                active: s.color_bgr == (b, g, r),
                swatch_bgr: Some((b, g, r)),
            });
            x += sw_w + gap4;
        }
    }

    let right_edge = items.last().map(|it| it.rect.right).unwrap_or(x);
    let total_w = (right_edge - sx + gap8).max(s_px(120, dpi));
    (items, total_w)
}

fn toolbar_stack_placement(s: &CaptureShared) -> (StackPlacement, i32, i32) {
    let dpi = tb_dpi(s);
    let mon = &s.frames[s.dom_mon_idx].monitor;
    let work = mon.work_area;
    let main_w = toolbar_width_px(dpi);
    let main_h = s_px(AnnotateToolbar::HEIGHT as i32, dpi) as f64;
    let sub_h = s_px(AnnotateToolbar::SUB_BAR_HEIGHT as i32, dpi) as f64;
    let gap = s_px(AnnotateToolbar::GAP as i32, dpi) as f64;
    let show_sub = s
        .active_tool
        .map(AnnotateToolbar::shows_property_bar)
        .unwrap_or(false);
    let place = AnnotateToolbar::place_stack(
        s.selection.x as f64,
        s.selection.y as f64,
        s.selection.width as f64,
        s.selection.height as f64,
        main_w as f64,
        main_h,
        sub_h,
        show_sub,
        work.x as f64,
        work.y as f64,
        work.right() as f64,
        work.bottom() as f64,
        gap,
    );
    let sub_w = s
        .active_tool
        .filter(|&t| AnnotateToolbar::shows_property_bar(t))
        .map(|t| build_subbar_items(s, t, place.left.round() as i32, place.sub_top.round() as i32, dpi).1)
        .unwrap_or_else(|| s_px(120, dpi));
    (place, main_w, sub_w)
}

fn toolbar_items(s: &CaptureShared) -> Vec<(AnnotateTool, RECT, Option<i32>)> {
    let dpi = tb_dpi(s);
    let (place, _, _) = toolbar_stack_placement(s);
    let mx = place.left.round() as i32;
    let my = place.main_top.round() as i32;
    let mh = s_px(AnnotateToolbar::HEIGHT as i32, dpi);
    let cell = s_px(AnnotateToolbar::CELL as i32, dpi);
    let sep = s_px(AnnotateToolbar::SEPARATOR_WIDTH as i32, dpi);
    let mut x = mx + s_px(8, dpi);
    let mut out = Vec::with_capacity(AnnotateToolbar::ORDER.len());
    for &tool in &AnnotateToolbar::ORDER {
        let sep_x = if matches!(tool, AnnotateTool::Undo | AnnotateTool::Close) {
            let sx = x + sep / 2;
            x += sep;
            Some(sx)
        } else {
            None
        };
        out.push((
            tool,
            RECT {
                left: x,
                top: my,
                right: x + cell,
                bottom: my + mh,
            },
            sep_x,
        ));
        x += cell;
    }
    out
}

fn hit_test_toolbar(s: &CaptureShared, vx: i32, vy: i32) -> Option<AnnotateTool> {
    for (tool, rc, _) in toolbar_items(s) {
        if vx >= rc.left && vx < rc.right && vy >= rc.top && vy < rc.bottom {
            return Some(tool);
        }
    }
    None
}

fn hit_test_subbar(s: &CaptureShared, vx: i32, vy: i32) -> Option<SubHit> {
    let tool = s.active_tool?;
    if !AnnotateToolbar::shows_property_bar(tool) {
        return None;
    }
    let dpi = tb_dpi(s);
    let (place, _, _) = toolbar_stack_placement(s);
    let sx = place.left.round() as i32;
    let sy = place.sub_top.round() as i32;
    let (items, _) = build_subbar_items(s, tool, sx, sy, dpi);
    for it in items {
        if vx >= it.rect.left && vx < it.rect.right && vy >= it.rect.top && vy < it.rect.bottom {
            return Some(it.hit);
        }
    }
    None
}

fn enter_annotate_phase(shared: &Rc<RefCell<CaptureShared>>, rect: PixelRect) {
    let mut windows_to_hide = Vec::new();
    let dom_hwnd;
    {
        let mut s = shared.borrow_mut();
        let bounds: Vec<PixelRect> = s.frames.iter().map(|f| f.monitor.bounds).collect();
        let dom_idx = CaptureUx::index_of_largest_overlap(&bounds, rect).unwrap_or(0);
        let crop = RegionCropper::crop(&s.frames, rect).unwrap_or_else(|_| PixelBuffer::allocate(1, 1));
        s.phase = OverlayPhase::Annotate;
        s.armed_at = Instant::now();
        s.dom_mon_idx = dom_idx;
        s.selection = rect;
        s.baked_crop = crop.clone();
        s.original_crop = crop;
        s.bake_dirty = false;
        s.ops.clear();
        s.redo.clear();
        s.active_tool = None;
        s.hover_tool = None;
        s.hover_sub = None;
        dom_hwnd = s.windows.get(dom_idx).copied().unwrap_or(s.primary_hwnd);
        for (i, &w) in s.windows.iter().enumerate() {
            if i != dom_idx {
                windows_to_hide.push(w);
            }
        }
    }

    unsafe {
        let _ = ReleaseCapture();
        for w in windows_to_hide {
            let _ = ShowWindow(w, SW_HIDE);
        }
        if !dom_hwnd.is_invalid() {
            let _ = SetWindowPos(
                dom_hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
            );
            let _ = SetForegroundWindow(dom_hwnd);
            let _ = SetFocus(Some(dom_hwnd));
        }
    }
    invalidate_all(dom_hwnd);
}

fn commit_and_finish(shared: &Rc<RefCell<CaptureShared>>, pin: bool, save: bool) {
    commit_text_entry_if_open(shared);
    let (final_buf, selection, dpi_x, dpi_y) = {
        let mut s = shared.borrow_mut();
        ensure_bake(&mut s);
        let mon = &s.frames[s.dom_mon_idx].monitor;
        (s.baked_crop.clone(), s.selection, mon.dpi_x, mon.dpi_y)
    };
    finish_capture_session(
        shared,
        CaptureDoneMsg {
            cancelled: false,
            pin_requested: pin,
            save_requested: save,
            image: Some(final_buf),
            selection,
            dpi_x,
            dpi_y,
            error: None,
        },
    );
}

fn commit_text_entry_if_open(shared: &Rc<RefCell<CaptureShared>>) {
    let (edit, anchor, color_bgr, text_size, dom_hwnd) = {
        let mut s = shared.borrow_mut();
        if s.text_edit.is_invalid() {
            return;
        }
        let e = s.text_edit;
        s.text_edit = HWND::default();
        let dom = s.windows.get(s.dom_mon_idx).copied().unwrap_or_default();
        (e, s.text_anchor_px, s.color_bgr, s.text_size, dom)
    };

    let mut buf = vec![0u16; 512];
    let len = unsafe { GetWindowTextW(edit, &mut buf) };
    let text = if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize]).trim().to_string()
    } else {
        String::new()
    };
    unsafe {
        let _ = DestroyWindow(edit);
        if !dom_hwnd.is_invalid() {
            let _ = SetFocus(Some(dom_hwnd));
        }
    }

    if !text.is_empty() {
        let mut s = shared.borrow_mut();
        s.ops.push(AnnotationOp::Text {
            x: anchor.0,
            y: anchor.1,
            text,
            b: color_bgr.0,
            g: color_bgr.1,
            r: color_bgr.2,
            font_size: text_size as f32,
        });
        s.redo.clear();
        s.bake_dirty = true;
        ensure_bake(&mut s);
    }
    invalidate_all(dom_hwnd);
}

fn open_text_entry(shared: &Rc<RefCell<CaptureShared>>, parent: HWND, vx: i32, vy: i32) {
    commit_text_entry_if_open(shared);
    let (lx, ly, px_x, px_y, font_size) = {
        let s = shared.borrow();
        let mon = &s.frames[s.dom_mon_idx].monitor;
        let lx = vx - mon.bounds.x;
        let ly = vy - mon.bounds.y;
        let px_x = (vx - s.selection.x).clamp(0, (s.original_crop.width - 1).max(0));
        let px_y = (vy - s.selection.y).clamp(0, (s.original_crop.height - 1).max(0));
        (lx, ly, px_x, px_y, s.text_size)
    };
    let cls = to_wide_null("EDIT");
    let empty = to_wide_null("");
    let font = create_ui_font(font_size, false, false, "Microsoft YaHei UI");
    let edit = unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(cls.as_ptr()),
            PCWSTR(empty.as_ptr()),
            WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            lx,
            ly,
            180,
            (font_size + 12).max(26),
            Some(parent),
            Some(windows::Win32::UI::WindowsAndMessaging::HMENU(EDIT_CHILD_ID as *mut _)),
            Some(hinstance()),
            None,
        )
        .unwrap_or_default()
    };
    if !edit.is_invalid() {
        set_control_font(edit, font);
        {
            let mut s = shared.borrow_mut();
            s.text_edit = edit;
            s.text_anchor_px = (px_x, px_y);
        }
        unsafe {
            let _ = SetFocus(Some(edit));
        }
    } else {
        unsafe {
            let _ = DeleteObject(font.into());
        }
    }
}

fn trigger_toolbar_action(shared: &Rc<RefCell<CaptureShared>>, hwnd: HWND, tool: AnnotateTool) {
    match tool {
        AnnotateTool::Shape
        | AnnotateTool::Curve
        | AnnotateTool::Pencil
        | AnnotateTool::Marker
        | AnnotateTool::Mosaic
        | AnnotateTool::Number
        | AnnotateTool::Text
        | AnnotateTool::Eraser
        | AnnotateTool::Pick => {
            commit_text_entry_if_open(shared);
            let mut s = shared.borrow_mut();
            s.active_tool = if s.active_tool == Some(tool) {
                None
            } else {
                Some(tool)
            };
            s.hover_sub = None;
        }
        AnnotateTool::Undo => {
            commit_text_entry_if_open(shared);
            let mut s = shared.borrow_mut();
            if let Some(op) = s.ops.pop() {
                s.redo.push(op);
                s.bake_dirty = true;
                ensure_bake(&mut s);
            }
        }
        AnnotateTool::Redo => {
            commit_text_entry_if_open(shared);
            let mut s = shared.borrow_mut();
            if let Some(op) = s.redo.pop() {
                s.ops.push(op);
                s.bake_dirty = true;
                ensure_bake(&mut s);
            }
        }
        AnnotateTool::Close => {
            let armed_ok = shared.borrow().armed_at.elapsed().as_millis() as u64 >= CaptureUx::ANNOTATE_ARM_MS;
            if armed_ok {
                finish_capture_session(
                    shared,
                    CaptureDoneMsg {
                        cancelled: true,
                        pin_requested: false,
                        save_requested: false,
                        image: None,
                        selection: PixelRect::default(),
                        dpi_x: 96,
                        dpi_y: 96,
                        error: None,
                    },
                );
            }
        }
        AnnotateTool::Pin => commit_and_finish(shared, true, false),
        AnnotateTool::Copy => commit_and_finish(shared, false, false),
        AnnotateTool::Save => {
            commit_text_entry_if_open(shared);
            let (buf, save_dir) = {
                let mut s = shared.borrow_mut();
                ensure_bake(&mut s);
                (s.baked_crop.clone(), s.save_dir.clone())
            };
            let default_name = format!(
                "Suite-{}.png",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            );
            if let Some(path) = save_png_dialog(hwnd, &save_dir, &default_name) {
                if save_png_to_path(&buf, &path).is_ok() {
                    commit_and_finish(shared, false, false);
                } else {
                    shared.borrow_mut().status_text = Some("无法保存文件。".to_string());
                }
            }
        }
        AnnotateTool::Ocr => {
            commit_text_entry_if_open(shared);
            let (controller, buf, already_busy) = {
                let mut s = shared.borrow_mut();
                if s.ocr_busy {
                    (s.controller, PixelBuffer::allocate(1, 1), true)
                } else {
                    s.ocr_busy = true;
                    s.status_text = Some("正在识字…".to_string());
                    ensure_bake(&mut s);
                    (s.controller, s.baked_crop.clone(), false)
                }
            };
            if !already_busy {
                let ov = hwnd.0 as isize;
                let ctrl_raw = controller.0 as isize;
                let _ = std::thread::Builder::new()
                    .name("suite-capture-ocr".to_string())
                    .spawn(move || {
                        let res = WindowsOcr::recognize_blocking(buf);
                        post_boxed(
                            HWND(ctrl_raw as *mut _),
                            WM_APP_OCR_DONE,
                            0,
                            OcrDoneMsg {
                                overlay_hwnd: ov,
                                result: res,
                            },
                        );
                    });
            }
        }
    }
    invalidate_all(hwnd);
}

unsafe extern "system" fn capture_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
        if raw != 0 {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let rc = Rc::from_raw(raw as *const RefCell<PerMonitorOverlay>);
                {
                    let st = rc.borrow_mut();
                    if !st.mem_bmp.is_invalid() {
                        let _ = DeleteObject(st.mem_bmp.into());
                    }
                    if !st.mem_dc.is_invalid() {
                        let _ = DeleteDC(st.mem_dc);
                    }
                    if !st.font_ui.is_invalid() {
                        let _ = DeleteObject(st.font_ui.into());
                    }
                    if !st.font_small.is_invalid() {
                        let _ = DeleteObject(st.font_small.into());
                    }
                }
                drop(rc);
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let mon_rc = unsafe {
        let r = Rc::from_raw(ptr as *const RefCell<PerMonitorOverlay>);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_capture_overlay(hwnd, &mut mon_rc.borrow_mut());
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TIMER_STATUS_HIDE => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_STATUS_HIDE);
            }
            mon_rc.borrow().shared.borrow_mut().status_text = None;
            invalidate_all(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as *mut _);
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let s = shared.borrow();
            let (b, g, r) = s.color_bgr;
            unsafe {
                SetTextColor(
                    hdc,
                    COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)),
                );
                SetBkColor(hdc, COLORREF(0x001E_1E1E));
            }
            LRESULT(s.edit_bg_brush.0 as isize)
        }
        WM_COMMAND => LRESULT(0),
        WM_MOUSEMOVE => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let phase = shared.borrow().phase;
            if phase == OverlayPhase::Select {
                let windows = {
                    let mut s = shared.borrow_mut();
                    s.cursor_virtual = (vx, vy);
                    if let Some((sx, sy)) = s.press {
                        if !s.dragging_select && CaptureUx::is_drag(sx, sy, vx, vy) {
                            s.dragging_select = true;
                            s.hover = None;
                        }
                        if s.dragging_select {
                            s.drag = Some(PixelRect::from_corners(sx, sy, vx, vy));
                        }
                        Some(s.windows.clone())
                    } else {
                        let ignore: Vec<isize> = s.windows.iter().map(|h| h.0 as isize).collect();
                        let next = WindowPicker::hit(vx, vy, &ignore, &s.frames.iter().map(|f| f.monitor.clone()).collect::<Vec<_>>());
                        let next_opt = (!next.is_empty()).then_some(next);
                        let changed = next_opt != s.hover || s.show_magnifier;
                        s.hover = next_opt;
                        if changed { Some(s.windows.clone()) } else { None }
                    }
                };
                if let Some(ws) = windows {
                    for w in ws {
                        invalidate_all(w);
                    }
                }
            } else {
                handle_annotate_mousemove(&shared, hwnd, vx, vy);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let phase = shared.borrow().phase;
            if phase == OverlayPhase::Select {
                let mut s = shared.borrow_mut();
                s.press = Some((vx, vy));
                s.dragging_select = false;
                s.drag = None;
            } else {
                handle_annotate_lbuttondown(&shared, hwnd, vx, vy);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let phase = shared.borrow().phase;
            if phase == OverlayPhase::Select {
                let shift_down = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
                let commit_rect = {
                    let mut s = shared.borrow_mut();
                    if s.press.is_none() {
                        None
                    } else {
                        let (sx, sy) = s.press.take().unwrap();
                        if s.dragging_select {
                            s.drag = Some(PixelRect::from_corners(sx, sy, vx, vy));
                        }
                        let hover = s.hover;
                        let act = CaptureUx::decide_release(
                            s.dragging_select,
                            s.drag.map(|d| d.width).unwrap_or(0),
                            s.drag.map(|d| d.height).unwrap_or(0),
                            hover.is_some(),
                        );
                        s.dragging_select = false;
                        match act {
                            ReleaseAction::CommitDrag => s.drag,
                            ReleaseAction::CommitHover => hover,
                            ReleaseAction::Stay => None,
                        }
                    }
                };
                if let Some(rect) = commit_rect {
                    let pin_after = shared.borrow().pin_after_capture;
                    if CaptureUx::open_annotation_after_region(pin_after, shift_down) {
                        enter_annotate_phase(&shared, rect);
                    } else {
                        let crop = RegionCropper::crop(&shared.borrow().frames, rect)
                            .unwrap_or_else(|_| PixelBuffer::allocate(1, 1));
                        let (dpi_x, dpi_y) = {
                            let s = shared.borrow();
                            let b: Vec<PixelRect> = s.frames.iter().map(|f| f.monitor.bounds).collect();
                            let i = CaptureUx::index_of_largest_overlap(&b, rect).unwrap_or(0);
                            (s.frames[i].monitor.dpi_x, s.frames[i].monitor.dpi_y)
                        };
                        finish_capture_session(
                            &shared,
                            CaptureDoneMsg {
                                cancelled: false,
                                pin_requested: true,
                                save_requested: false,
                                image: Some(crop),
                                selection: rect,
                                dpi_x,
                                dpi_y,
                                error: None,
                            },
                        );
                    }
                }
            } else {
                handle_annotate_lbuttonup(&shared, hwnd, vx, vy);
            }
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let should_commit = {
                let s = shared.borrow();
                s.phase == OverlayPhase::Annotate
                    && s.selection.contains(vx, vy)
                    && s.active_tool.is_none()
                    && hit_test_toolbar(&s, vx, vy).is_none()
            };
            if should_commit {
                commit_and_finish(&shared, false, false);
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let should_cancel = {
                let s = shared.borrow();
                s.phase == OverlayPhase::Select
                    || s.armed_at.elapsed().as_millis() as u64 >= CaptureUx::ANNOTATE_ARM_MS
            };
            if should_cancel {
                finish_capture_session(
                    &shared,
                    CaptureDoneMsg {
                        cancelled: true,
                        pin_requested: false,
                        save_requested: false,
                        image: None,
                        selection: PixelRect::default(),
                        dpi_x: 96,
                        dpi_y: 96,
                        error: None,
                    },
                );
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wparam.0 as u16;
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let ctrl = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
            let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
            if vk == VK_ESCAPE.0 {
                let had_text = !shared.borrow().text_edit.is_invalid();
                if had_text {
                    let edit = {
                        let mut s = shared.borrow_mut();
                        let e = s.text_edit;
                        s.text_edit = HWND::default();
                        e
                    };
                    unsafe {
                        let _ = DestroyWindow(edit);
                        let _ = SetFocus(Some(hwnd));
                    }
                } else {
                    finish_capture_session(
                        &shared,
                        CaptureDoneMsg {
                            cancelled: true,
                            pin_requested: false,
                            save_requested: false,
                            image: None,
                            selection: PixelRect::default(),
                            dpi_x: 96,
                            dpi_y: 96,
                            error: None,
                        },
                    );
                }
                return LRESULT(0);
            }
            if vk == VK_RETURN.0 {
                let had_text = !shared.borrow().text_edit.is_invalid();
                if had_text {
                    commit_text_entry_if_open(&shared);
                } else if shared.borrow().phase == OverlayPhase::Annotate {
                    commit_and_finish(&shared, false, false);
                }
                return LRESULT(0);
            }
            let (phase, no_text_edit) = {
                let s = shared.borrow();
                (s.phase, s.text_edit.is_invalid())
            };
            if no_text_edit {
                if phase == OverlayPhase::Select && !ctrl && vk == b'C' as u16 {
                    let (vx, vy) = MonitorEnumerator::cursor_pos();
                    let sampled = {
                        let s = shared.borrow();
                        s.frames.iter().find_map(|f| {
                            let b = f.monitor.bounds;
                            if !b.contains(vx, vy) {
                                return None;
                            }
                            let px = ((vx - b.x) as f64 * f.buffer.width as f64
                                / b.width.max(1) as f64)
                                .round() as i32;
                            let py = ((vy - b.y) as f64 * f.buffer.height as f64
                                / b.height.max(1) as f64)
                                .round() as i32;
                            f.buffer
                                .try_get_pixel(px, py)
                                .map(|(blue, green, red, _)| (red, green, blue))
                        })
                    };
                    if let Some((r, g, b)) = sampled {
                        let val = ColorPick::to_hex(r, g, b);
                        let controller = shared.borrow().controller;
                        let copied = ImageClipboard::copy_text(&val);
                        finish_capture_session(
                            &shared,
                            CaptureDoneMsg {
                                cancelled: true,
                                pin_requested: false,
                                save_requested: false,
                                image: None,
                                selection: PixelRect::default(),
                                dpi_x: 96,
                                dpi_y: 96,
                                error: None,
                            },
                        );
                        post_boxed(
                            controller,
                            WM_APP_COLOR_PICK_DONE,
                            0,
                            ColorPickDoneMsg {
                                copied_value: copied.is_ok().then_some(val),
                                error: copied.err(),
                            },
                        );
                        return LRESULT(0);
                    }
                }
                if phase == OverlayPhase::Annotate {
                    if ctrl && shift && vk == b'Z' as u16 {
                        trigger_toolbar_action(&shared, hwnd, AnnotateTool::Redo);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'Z' as u16 {
                        trigger_toolbar_action(&shared, hwnd, AnnotateTool::Undo);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'Y' as u16 {
                        trigger_toolbar_action(&shared, hwnd, AnnotateTool::Redo);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'C' as u16 {
                        commit_and_finish(&shared, false, false);
                        return LRESULT(0);
                    }
                    if ctrl && shift && vk == b'S' as u16 {
                        commit_and_finish(&shared, false, true);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'S' as u16 {
                        trigger_toolbar_action(&shared, hwnd, AnnotateTool::Save);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'T' as u16 {
                        commit_and_finish(&shared, true, false);
                        return LRESULT(0);
                    }
                    if ctrl && vk == b'O' as u16 {
                        trigger_toolbar_action(&shared, hwnd, AnnotateTool::Ocr);
                        return LRESULT(0);
                    }
                    if !ctrl {
                        let shortcut_tool = match vk {
                            0x31 => Some(AnnotateTool::Shape),
                            0x32 => Some(AnnotateTool::Curve),
                            0x33 => Some(AnnotateTool::Pencil),
                            0x34 => Some(AnnotateTool::Marker),
                            0x35 => Some(AnnotateTool::Mosaic),
                            0x36 => Some(AnnotateTool::Number),
                            0x37 => Some(AnnotateTool::Text),
                            x if x == b'I' as u16 => Some(AnnotateTool::Pick),
                            x if x == b'E' as u16 => Some(AnnotateTool::Eraser),
                            _ => None,
                        };
                        if let Some(t) = shortcut_tool {
                            trigger_toolbar_action(&shared, hwnd, t);
                            return LRESULT(0);
                        }
                        if shared.borrow().active_tool.is_none()
                            && matches!(vk, x if x == VK_LEFT.0 || x == VK_RIGHT.0 || x == VK_UP.0 || x == VK_DOWN.0)
                        {
                            let step = if shift { 10 } else { 1 };
                            let (dx, dy) = if vk == VK_LEFT.0 {
                                (-step, 0)
                            } else if vk == VK_RIGHT.0 {
                                (step, 0)
                            } else if vk == VK_UP.0 {
                                (0, -step)
                            } else {
                                (0, step)
                            };
                            let mut show_warn = false;
                            {
                                let mut s = shared.borrow_mut();
                                if s.ops.is_empty() {
                                    let bounds: Vec<PixelRect> = s.frames.iter().map(|f| f.monitor.bounds).collect();
                                    let clamp = CaptureUx::union_monitor_bounds(&bounds, s.frames[s.dom_mon_idx].monitor.bounds);
                                    let moved = CaptureUx::move_rect(s.selection, dx, dy, clamp);
                                    s.selection = moved;
                                    let crop = RegionCropper::crop(&s.frames, moved)
                                        .unwrap_or_else(|_| PixelBuffer::allocate(1, 1));
                                    s.baked_crop = crop.clone();
                                    s.original_crop = crop;
                                    s.bake_dirty = false;
                                } else {
                                    s.status_text = Some("已有标注，撤销后可用方向键微调选区".to_string());
                                    show_warn = true;
                                }
                            }
                            if show_warn {
                                unsafe {
                                    let _ = SetTimer(Some(hwnd), TIMER_STATUS_HIDE, 1800, None);
                                }
                            }
                            invalidate_all(hwnd);
                            return LRESULT(0);
                        }
                    }
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_MOUSEWHEEL => {
            let shared = Rc::clone(&mon_rc.borrow().shared);
            let can_step = {
                let s = shared.borrow();
                s.phase == OverlayPhase::Annotate
                    && s.text_edit.is_invalid()
                    && !s.drawing_stroke
                    && s.active_tool.map(AnnotateToolbar::shows_property_bar).unwrap_or(false)
            };
            if can_step {
                let delta = ((wparam.0 >> 16) as u16) as i16;
                let dir = if delta > 0 { 1 } else { -1 };
                let msg_text = {
                    let mut s = shared.borrow_mut();
                    let label_val = match s.active_tool {
                        Some(AnnotateTool::Marker) => {
                            s.marker_thickness = (s.marker_thickness + dir * 2).clamp(6, 36);
                            ("马克笔粗细", s.marker_thickness)
                        }
                        Some(AnnotateTool::Mosaic) => {
                            s.mosaic_radius = (s.mosaic_radius + dir * 2).clamp(6, 48);
                            (if s.blur_mode { "模糊半径" } else { "马赛克半径" }, s.mosaic_radius)
                        }
                        Some(AnnotateTool::Number) => {
                            s.number_radius = (s.number_radius + dir * 2).clamp(10, 32);
                            ("序号圆标半径", s.number_radius)
                        }
                        Some(AnnotateTool::Eraser) => {
                            s.eraser_thickness = (s.eraser_thickness + dir * 2).clamp(6, 48);
                            ("橡皮擦粗细", s.eraser_thickness)
                        }
                        Some(AnnotateTool::Text) => {
                            s.text_size = (s.text_size + dir * 2).clamp(10, 48);
                            ("文字字号", s.text_size)
                        }
                        _ => {
                            s.stroke_thickness = (s.stroke_thickness + dir).clamp(1, 16);
                            ("线条粗细", s.stroke_thickness)
                        }
                    };
                    sync_prefs_from_shared(&s);
                    format!("{}: {} px", label_val.0, label_val.1)
                };
                shared.borrow_mut().status_text = Some(msg_text);
                unsafe {
                    let _ = SetTimer(Some(hwnd), TIMER_STATUS_HIDE, 1200, None);
                }
                invalidate_all(hwnd);
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn sync_prefs_from_shared(s: &CaptureShared) {
    let mut p = s.prefs.borrow_mut();
    p.ellipse_mode = s.ellipse_mode;
    p.curve_style = s.curve_style;
    p.blur_mode = s.blur_mode;
    p.color_bgr = s.color_bgr;
    p.stroke_thickness = s.stroke_thickness;
    p.marker_thickness = s.marker_thickness;
    p.mosaic_radius = s.mosaic_radius;
    p.eraser_thickness = s.eraser_thickness;
    p.number_radius = s.number_radius;
    p.text_size = s.text_size;
}

fn next_step_number(ops: &[AnnotationOp]) -> u32 {
    ops.iter()
        .filter(|op| matches!(op, AnnotationOp::StepBadge { .. }))
        .count() as u32
        + 1
}

fn constrain_draw_point(
    tool: Option<AnnotateTool>,
    start: (i32, i32),
    curr: (i32, i32),
    shift: bool,
    max_w: i32,
    max_h: i32,
) -> (i32, i32) {
    if !shift {
        return curr;
    }
    let dx = curr.0 - start.0;
    let dy = curr.1 - start.1;
    let (cx, cy) = match tool {
        Some(AnnotateTool::Shape) => {
            let side = dx.abs().max(dy.abs());
            let sx = if dx < 0 { -side } else { side };
            let sy = if dy < 0 { -side } else { side };
            (start.0 + sx, start.1 + sy)
        }
        Some(AnnotateTool::Curve) => {
            let len = (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)).sqrt();
            if len < 1.0 {
                curr
            } else {
                let angle = f64::from(dy).atan2(f64::from(dx));
                let snap = std::f64::consts::FRAC_PI_4;
                let snapped = (angle / snap).round() * snap;
                (
                    start.0 + (len * snapped.cos()).round() as i32,
                    start.1 + (len * snapped.sin()).round() as i32,
                )
            }
        }
        _ => curr,
    };
    (cx.clamp(0, (max_w - 1).max(0)), cy.clamp(0, (max_h - 1).max(0)))
}

fn handle_annotate_lbuttondown(shared: &Rc<RefCell<CaptureShared>>, hwnd: HWND, vx: i32, vy: i32) {
    // 1. Subbar click?
    let sub_hit = {
        let s = shared.borrow();
        hit_test_subbar(&s, vx, vy)
    };
    if let Some(hit) = sub_hit {
        let mut s = shared.borrow_mut();
        match hit {
            SubHit::ShapeRect => s.ellipse_mode = false,
            SubHit::ShapeEllipse => s.ellipse_mode = true,
            SubHit::CurveStyle(style) => s.curve_style = style,
            SubHit::MosaicPixel => s.blur_mode = false,
            SubHit::MosaicBlur => s.blur_mode = true,
            SubHit::SizeChip(val) => match s.active_tool {
                Some(AnnotateTool::Marker) => s.marker_thickness = val,
                Some(AnnotateTool::Mosaic) => s.mosaic_radius = val,
                Some(AnnotateTool::Number) => s.number_radius = val,
                Some(AnnotateTool::Eraser) => s.eraser_thickness = val,
                Some(AnnotateTool::Text) => s.text_size = val,
                _ => s.stroke_thickness = val,
            },
            SubHit::Swatch(idx) => {
                if let Some(&(r, g, b)) = AnnotateToolbar::PALETTE.get(idx) {
                    s.color_bgr = (b, g, r);
                }
            }
        }
        sync_prefs_from_shared(&s);
        drop(s);
        invalidate_all(hwnd);
        return;
    }

    // 2. Main toolbar click?
    let tb_hit = {
        let s = shared.borrow();
        hit_test_toolbar(&s, vx, vy)
    };
    if let Some(tool) = tb_hit {
        trigger_toolbar_action(shared, hwnd, tool);
        return;
    }

    // Commit any open text box if user clicked outside it
    commit_text_entry_if_open(shared);

    // 3. Resize handle hit on selection?
    let handle = {
        let s = shared.borrow();
        if s.active_tool.is_none() {
            CaptureUx::hit_test_resize_handle(s.selection, vx, vy, CaptureUx::RESIZE_GRIP_PX)
        } else {
            ResizeHandle::None
        }
    };
    if handle != ResizeHandle::None {
        let mut s = shared.borrow_mut();
        s.resizing = true;
        s.resize_handle = handle;
        s.resize_origin = s.selection;
        unsafe {
            let _ = SetCapture(hwnd);
        }
        return;
    }

    // 4. Inside selection?
    let (inside, tool) = {
        let s = shared.borrow();
        (s.selection.contains(vx, vy), s.active_tool)
    };
    if !inside {
        return;
    }

    match tool {
        None => {
            let mut s = shared.borrow_mut();
            s.moving_sel = true;
            s.move_origin = s.selection;
            s.move_start_virt = (vx, vy);
            unsafe {
                let _ = SetCapture(hwnd);
            }
        }
        Some(AnnotateTool::Text) => {
            open_text_entry(shared, hwnd, vx, vy);
        }
        Some(AnnotateTool::Number) => {
            let mut s = shared.borrow_mut();
            let px = (vx - s.selection.x).clamp(0, (s.original_crop.width - 1).max(0));
            let py = (vy - s.selection.y).clamp(0, (s.original_crop.height - 1).max(0));
            let number = next_step_number(&s.ops);
            let (b, g, r) = s.color_bgr;
            let radius = s.number_radius;
            s.ops.push(AnnotationOp::StepBadge {
                cx: px,
                cy: py,
                number,
                radius,
                b,
                g,
                r,
            });
            s.redo.clear();
            s.bake_dirty = true;
            ensure_bake(&mut s);
            drop(s);
            invalidate_all(hwnd);
        }
        Some(AnnotateTool::Pick) => {
            let status = {
                let mut s = shared.borrow_mut();
                ensure_bake(&mut s);
                let px = (vx - s.selection.x).clamp(0, s.baked_crop.width - 1);
                let py = (vy - s.selection.y).clamp(0, s.baked_crop.height - 1);
                if let Some((b, g, r, _)) = s.baked_crop.try_get_pixel(px, py) {
                    let hex = ColorPick::to_hex(r, g, b);
                    match ImageClipboard::copy_text(&hex) {
                        Ok(()) => ColorPick::copied_message(&hex),
                        Err(_) => ColorPick::FAIL.to_string(),
                    }
                } else {
                    ColorPick::FAIL.to_string()
                }
            };
            shared.borrow_mut().status_text = Some(status);
            unsafe {
                let _ = SetTimer(Some(hwnd), TIMER_STATUS_HIDE, 2200, None);
            }
            invalidate_all(hwnd);
        }
        Some(active) => {
            let mut s = shared.borrow_mut();
            let px = (vx - s.selection.x).clamp(0, (s.original_crop.width - 1).max(0));
            let py = (vy - s.selection.y).clamp(0, (s.original_crop.height - 1).max(0));
            s.drawing_stroke = true;
            s.draw_start_px = (px, py);
            s.draw_curr_px = (px, py);
            s.stroke_pts = vec![(px, py)];
            if active == AnnotateTool::Mosaic {
                ensure_bake(&mut s);
                let mut draft = s.baked_crop.clone();
                if s.blur_mode {
                    PixelDraw::blur_stamp(&mut draft, px, py, s.mosaic_radius);
                } else {
                    PixelDraw::mosaic_stamp(&mut draft, px, py, s.mosaic_radius, PixelDraw::DEFAULT_MOSAIC_BLOCK);
                }
                s.mosaic_draft = Some(draft);
                s.mosaic_draft_last = 0;
            }
            unsafe {
                let _ = SetCapture(hwnd);
            }
            invalidate_all(hwnd);
        }
    }
}

fn handle_annotate_mousemove(shared: &Rc<RefCell<CaptureShared>>, hwnd: HWND, vx: i32, vy: i32) {
    let shift_down = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
    let mut need_invalidate = false;
    let cursor_res = {
        let mut s = shared.borrow_mut();
        s.cursor_virtual = (vx, vy);
        if s.resizing {
            s.hover_tool = None;
            s.hover_sub = None;
            let next = CaptureUx::resize_rect(s.resize_origin, s.resize_handle, vx, vy);
            if next.width >= CaptureUx::MIN_COMMIT_PX && next.height >= CaptureUx::MIN_COMMIT_PX {
                s.selection = next;
                need_invalidate = true;
            }
            cursor_for_handle(s.resize_handle)
        } else if s.moving_sel {
            s.hover_tool = None;
            s.hover_sub = None;
            let bounds: Vec<PixelRect> = s.frames.iter().map(|f| f.monitor.bounds).collect();
            let clamp = CaptureUx::union_monitor_bounds(&bounds, s.frames[s.dom_mon_idx].monitor.bounds);
            let dx = vx - s.move_start_virt.0;
            let dy = vy - s.move_start_virt.1;
            s.selection = CaptureUx::move_rect(s.move_origin, dx, dy, clamp);
            need_invalidate = true;
            IDC_SIZEALL
        } else if s.drawing_stroke {
            s.hover_tool = None;
            s.hover_sub = None;
            let raw_px = (vx - s.selection.x).clamp(0, (s.original_crop.width - 1).max(0));
            let raw_py = (vy - s.selection.y).clamp(0, (s.original_crop.height - 1).max(0));
            let (px, py) = constrain_draw_point(
                s.active_tool,
                s.draw_start_px,
                (raw_px, raw_py),
                shift_down,
                s.original_crop.width,
                s.original_crop.height,
            );
            s.draw_curr_px = (px, py);
            s.stroke_pts.push((px, py));
            if s.active_tool == Some(AnnotateTool::Mosaic) {
                let radius = s.mosaic_radius;
                let blur_mode = s.blur_mode;
                let from = s.mosaic_draft_last;
                let seg = if from < s.stroke_pts.len() {
                    s.stroke_pts[from..].to_vec()
                } else {
                    Vec::new()
                };
                if seg.len() >= 2
                    && let Some(ref mut draft) = s.mosaic_draft
                {
                    if blur_mode {
                        PixelDraw::blur_brush(draft, &seg, radius);
                    } else {
                        PixelDraw::mosaic_brush(draft, &seg, radius, PixelDraw::DEFAULT_MOSAIC_BLOCK);
                    }
                    s.mosaic_draft_last = s.stroke_pts.len() - 1;
                }
            }
            need_invalidate = true;
            IDC_CROSS
        } else {
            let hover_tb = hit_test_toolbar(&s, vx, vy);
            let hover_sb = if hover_tb.is_none() {
                hit_test_subbar(&s, vx, vy)
            } else {
                None
            };
            if hover_tb != s.hover_tool || hover_sb != s.hover_sub {
                s.hover_tool = hover_tb;
                s.hover_sub = hover_sb.clone();
                need_invalidate = true;
            }
            if hover_tb.is_some() || hover_sb.is_some() {
                IDC_ARROW
            } else if s.active_tool.is_some() && s.selection.contains(vx, vy) {
                IDC_CROSS
            } else if s.active_tool.is_none() {
                let h = CaptureUx::hit_test_resize_handle(s.selection, vx, vy, CaptureUx::RESIZE_GRIP_PX);
                if h != ResizeHandle::None {
                    cursor_for_handle(h)
                } else if s.selection.contains(vx, vy) {
                    IDC_SIZEALL
                } else {
                    IDC_ARROW
                }
            } else {
                IDC_ARROW
            }
        }
    };

    unsafe {
        if let Ok(c) = LoadCursorW(None, cursor_res) {
            SetCursor(Some(c));
        }
    }
    if need_invalidate {
        invalidate_all(hwnd);
    }
}

fn handle_annotate_lbuttonup(shared: &Rc<RefCell<CaptureShared>>, hwnd: HWND, vx: i32, vy: i32) {
    let shift_down = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
    let mut s = shared.borrow_mut();
    if s.resizing || s.moving_sel {
        s.resizing = false;
        s.moving_sel = false;
        s.resize_handle = ResizeHandle::None;
        unsafe {
            let _ = ReleaseCapture();
        }
        let rect = s.selection;
        let next_crop = RegionCropper::crop(&s.frames, rect).unwrap_or_else(|_| PixelBuffer::allocate(1, 1));
        s.baked_crop = next_crop.clone();
        s.original_crop = next_crop;
        s.ops.clear();
        s.redo.clear();
        s.bake_dirty = false;
        drop(s);
        invalidate_all(hwnd);
        return;
    }

    if !s.drawing_stroke {
        return;
    }
    s.drawing_stroke = false;
    unsafe {
        let _ = ReleaseCapture();
    }
    let raw_px = (vx - s.selection.x).clamp(0, (s.original_crop.width - 1).max(0));
    let raw_py = (vy - s.selection.y).clamp(0, (s.original_crop.height - 1).max(0));
    let (px, py) = constrain_draw_point(
        s.active_tool,
        s.draw_start_px,
        (raw_px, raw_py),
        shift_down,
        s.original_crop.width,
        s.original_crop.height,
    );
    s.draw_curr_px = (px, py);
    let pts = std::mem::take(&mut s.stroke_pts);
    s.mosaic_draft = None;

    let (b, g, r) = s.color_bgr;
    let op = match s.active_tool {
        Some(AnnotateTool::Mosaic) if !pts.is_empty() => {
            if s.blur_mode {
                Some(AnnotationOp::BlurStroke {
                    points: pts,
                    brush_radius: s.mosaic_radius,
                })
            } else {
                Some(AnnotationOp::MosaicStroke {
                    points: pts,
                    brush_radius: s.mosaic_radius,
                })
            }
        }
        Some(AnnotateTool::Eraser) if !pts.is_empty() => Some(AnnotationOp::Erase {
            points: pts,
            thickness: s.eraser_thickness,
        }),
        Some(AnnotateTool::Marker) if !pts.is_empty() => Some(AnnotationOp::Stroke {
            points: pts,
            thickness: s.marker_thickness,
            b,
            g,
            r,
            a: 110,
            dashed: false,
        }),
        Some(AnnotateTool::Pencil) if !pts.is_empty() => Some(AnnotationOp::Stroke {
            points: pts,
            thickness: (s.stroke_thickness - 1).max(1),
            b,
            g,
            r,
            a: 255,
            dashed: false,
        }),
        Some(AnnotateTool::Curve) => {
            let (x0, y0) = s.draw_start_px;
            let thick = s.stroke_thickness.max(1);
            match s.curve_style {
                CurveStyle::Arrow => Some(AnnotationOp::Arrow {
                    x1: x0,
                    y1: y0,
                    x2: px,
                    y2: py,
                    thickness: thick,
                    b,
                    g,
                    r,
                }),
                CurveStyle::Dashed => Some(AnnotationOp::Stroke {
                    points: vec![(x0, y0), (px, py)],
                    thickness: thick,
                    b,
                    g,
                    r,
                    a: 255,
                    dashed: true,
                }),
                CurveStyle::Solid => Some(AnnotationOp::Stroke {
                    points: vec![(x0, y0), (px, py)],
                    thickness: thick,
                    b,
                    g,
                    r,
                    a: 255,
                    dashed: false,
                }),
            }
        }
        Some(AnnotateTool::Shape) => {
            let rect = PixelRect::from_corners(s.draw_start_px.0, s.draw_start_px.1, px, py);
            if rect.width < 2 && rect.height < 2 {
                None
            } else if s.ellipse_mode {
                Some(AnnotationOp::Ellipse {
                    rect,
                    thickness: s.stroke_thickness.max(1),
                    b,
                    g,
                    r,
                })
            } else {
                Some(AnnotationOp::Rect {
                    rect,
                    thickness: s.stroke_thickness.max(1),
                    b,
                    g,
                    r,
                })
            }
        }
        _ => None,
    };

    if let Some(op) = op {
        s.ops.push(op);
        s.redo.clear();
        s.bake_dirty = true;
        ensure_bake(&mut s);
    }
    drop(s);
    invalidate_all(hwnd);
}

fn cursor_for_handle(h: ResizeHandle) -> PCWSTR {
    match h {
        ResizeHandle::N | ResizeHandle::S => IDC_SIZENS,
        ResizeHandle::E | ResizeHandle::W => IDC_SIZEWE,
        ResizeHandle::NW | ResizeHandle::SE => IDC_SIZENWSE,
        ResizeHandle::NE | ResizeHandle::SW => IDC_SIZENESW,
        ResizeHandle::None => IDC_ARROW,
    }
}

fn paint_capture_overlay(hwnd: HWND, mon: &mut PerMonitorOverlay) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc.is_invalid() {
        return;
    }

    let (b, cw, ch) = {
        let s = mon.shared.borrow();
        let bounds = s.frames[mon.mon_idx].monitor.bounds;
        (bounds, bounds.width.max(1), bounds.height.max(1))
    };

    unsafe {
        if mon.mem_dc.is_invalid() {
            mon.mem_dc = CreateCompatibleDC(Some(hdc));
        }
        if mon.mem_bmp.is_invalid() || mon.mem_w != cw || mon.mem_h != ch {
            if !mon.mem_bmp.is_invalid() {
                let _ = DeleteObject(mon.mem_bmp.into());
            }
            mon.mem_bmp = CreateCompatibleBitmap(hdc, cw, ch);
            mon.mem_w = cw;
            mon.mem_h = ch;
        }
        let old_bmp = SelectObject(mon.mem_dc, mon.mem_bmp.into());
        let mem = mon.mem_dc;

        let shared = Rc::clone(&mon.shared);
        let mut s = shared.borrow_mut();

        // 1. Background dimmed frame
        blit_bgra_full(mem, &s.dimmed_frames[mon.mon_idx], cw, ch);

        if s.phase == OverlayPhase::Select {
            let (hl, is_drag_mode) = if s.dragging_select {
                (s.drag, true)
            } else {
                (s.hover, false)
            };

            if let Some(rect) = hl {
                let local = rect.intersect(b);
                if !local.is_empty() {
                    let lx = local.x - b.x;
                    let ly = local.y - b.y;
                    blit_bgra_subrect(
                        mem,
                        &s.frames[mon.mon_idx].buffer,
                        lx,
                        ly,
                        local.width,
                        local.height,
                    );
                    let col = if is_drag_mode {
                        COLORREF(0x00F0_8020)
                    } else {
                        COLORREF(0x0028_28DC)
                    };
                    let pen = CreatePen(PS_SOLID, 2, col);
                    let old_p = SelectObject(mem, pen.into());
                    let old_b = SelectObject(mem, GetStockObject(HOLLOW_BRUSH));
                    let _ = Rectangle(mem, lx, ly, lx + local.width, ly + local.height);
                    SelectObject(mem, old_b);
                    SelectObject(mem, old_p);
                    let _ = DeleteObject(pen.into());

                    draw_size_chip(mem, mon.font_small, (lx, ly), cw, (rect.width, rect.height));
                }
            }

            // Corner hint
            draw_hint_chip(mem, mon.font_ui, CaptureUx::OVERLAY_HINT);

            // Magnifier
            if s.show_magnifier && b.contains(s.cursor_virtual.0, s.cursor_virtual.1) {
                let lx = s.cursor_virtual.0 - b.x;
                let ly = s.cursor_virtual.1 - b.y;
                draw_magnifier_box(
                    mem,
                    &s.frames[mon.mon_idx].buffer,
                    b,
                    lx,
                    ly,
                    cw,
                    ch,
                    mon.font_small,
                );
            }
        } else {
            // Annotate phase on dominant monitor
            ensure_bake(&mut s);
            let sel = s.selection;
            let local = sel.intersect(b);
            if !local.is_empty() {
                let lx = local.x - b.x;
                let ly = local.y - b.y;
                let preview_buf = if s.drawing_stroke && s.active_tool == Some(AnnotateTool::Mosaic) {
                    s.mosaic_draft.as_ref().unwrap_or(&s.baked_crop)
                } else {
                    &s.baked_crop
                };
                if s.resizing || s.moving_sel {
                    blit_bgra_subrect(
                        mem,
                        &s.frames[mon.mon_idx].buffer,
                        lx,
                        ly,
                        local.width,
                        local.height,
                    );
                } else {
                    blit_bgra_at(mem, preview_buf, sel.x - b.x, sel.y - b.y);
                    if s.drawing_stroke {
                        draw_live_draft(mem, &s, b);
                    }
                }

                // Selection border (#2080F0)
                let pen = CreatePen(PS_SOLID, 2, COLORREF(0x00F0_8020));
                let old_p = SelectObject(mem, pen.into());
                let old_b = SelectObject(mem, GetStockObject(HOLLOW_BRUSH));
                let _ = Rectangle(mem, lx, ly, lx + local.width, ly + local.height);
                SelectObject(mem, old_b);
                SelectObject(mem, old_p);
                let _ = DeleteObject(pen.into());

                // 8 white/blue anchor handles
                let anchors = [
                    (lx, ly),
                    (lx + local.width / 2, ly),
                    (lx + local.width, ly),
                    (lx + local.width, ly + local.height / 2),
                    (lx + local.width, ly + local.height),
                    (lx + local.width / 2, ly + local.height),
                    (lx, ly + local.height),
                    (lx, ly + local.height / 2),
                ];
                let a_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00F0_8020));
                let a_br = CreateSolidBrush(COLORREF(0x00FF_FFFF));
                let op = SelectObject(mem, a_pen.into());
                let ob = SelectObject(mem, a_br.into());
                for (ax, ay) in anchors {
                    let _ = Ellipse(mem, ax - 5, ay - 5, ax + 6, ay + 6);
                }
                SelectObject(mem, ob);
                SelectObject(mem, op);
                let _ = DeleteObject(a_br.into());
                let _ = DeleteObject(a_pen.into());

                draw_size_chip(mem, mon.font_small, (lx, ly), cw, (sel.width, sel.height));
            }

            draw_annotate_toolbar(mem, &s, b, mon.font_small);
            draw_annotate_subbar(mem, &s, b, mon.font_small);
            draw_hover_tooltip(mem, &s, b, cw, ch, mon.font_small);

            if let Some(ref st_text) = s.status_text {
                let dpi = tb_dpi(&s);
                let (place, _, _) = toolbar_stack_placement(&s);
                let wt: Vec<u16> = st_text.encode_utf16().collect();
                let old_f = SelectObject(mem, mon.font_ui.into());
                let mut sz = SIZE::default();
                let _ = GetTextExtentPoint32W(mem, &wt, &mut sz);
                let pill_w = sz.cx + s_px(20, dpi);
                let pill_h = (sz.cy + s_px(10, dpi)).max(s_px(26, dpi));
                let span_top = if place.sub_visible {
                    place.main_top.min(place.sub_top)
                } else {
                    place.main_top
                };
                let span_bot = if place.sub_visible {
                    (place.main_top + f64::from(s_px(AnnotateToolbar::HEIGHT as i32, dpi)))
                        .max(place.sub_top + f64::from(s_px(AnnotateToolbar::SUB_BAR_HEIGHT as i32, dpi)))
                } else {
                    place.main_top + f64::from(s_px(AnnotateToolbar::HEIGHT as i32, dpi))
                };
                let sx = ((place.left.round() as i32) - b.x).clamp(s_px(8, dpi), (cw - pill_w - s_px(8, dpi)).max(s_px(8, dpi)));
                let above_y = (span_top.round() as i32) - b.y - pill_h - s_px(6, dpi);
                let sy = if above_y >= s_px(8, dpi) {
                    above_y
                } else {
                    ((span_bot.round() as i32) - b.y + s_px(6, dpi)).min((ch - pill_h - s_px(8, dpi)).max(s_px(8, dpi)))
                };
                let bg = CreateSolidBrush(COLORREF(0x001E_1E1E));
                let bd = CreatePen(PS_SOLID, 1, COLORREF(0x0052_5252));
                let ob = SelectObject(mem, bg.into());
                let op = SelectObject(mem, bd.into());
                let _ = RoundRect(mem, sx, sy, sx + pill_w, sy + pill_h, s_px(6, dpi), s_px(6, dpi));
                SelectObject(mem, op);
                SelectObject(mem, ob);
                let _ = DeleteObject(bd.into());
                let _ = DeleteObject(bg.into());
                SetBkMode(mem, TRANSPARENT);
                SetTextColor(mem, COLORREF(0x00FF_FFFF));
                let mut trc = RECT {
                    left: sx,
                    top: sy,
                    right: sx + pill_w,
                    bottom: sy + pill_h,
                };
                let mut wt_mut = wt;
                DrawTextW(mem, &mut wt_mut, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                SelectObject(mem, old_f);
            }
        }

        drop(s);
        let _ = BitBlt(hdc, 0, 0, cw, ch, Some(mem), 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        let _ = EndPaint(hwnd, &ps);
    }
}

unsafe fn blit_bgra_full(hdc: HDC, buf: &PixelBuffer, dst_w: i32, dst_h: i32) {
    unsafe {
        blit_bgra_at_size(hdc, buf, 0, 0, dst_w, dst_h);
    }
}

unsafe fn blit_bgra_at(hdc: HDC, buf: &PixelBuffer, dx: i32, dy: i32) {
    unsafe {
        blit_bgra_at_size(hdc, buf, dx, dy, buf.width, buf.height);
    }
}

unsafe fn blit_bgra_at_size(hdc: HDC, buf: &PixelBuffer, dx: i32, dy: i32, dw: i32, dh: i32) {
    if buf.width <= 0 || buf.height <= 0 || buf.bgra.is_empty() {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: buf.width,
            biHeight: -buf.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let _ = StretchDIBits(
            hdc,
            dx,
            dy,
            dw,
            dh,
            0,
            0,
            buf.width,
            buf.height,
            Some(buf.bgra.as_ptr().cast()),
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

unsafe fn blit_bgra_subrect(hdc: HDC, buf: &PixelBuffer, lx: i32, ly: i32, w: i32, h: i32) {
    if buf.width <= 0 || buf.height <= 0 || w <= 0 || h <= 0 || buf.bgra.is_empty() {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: buf.width,
            biHeight: -buf.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    // In top-down StretchDIBits source Y is measured from bottom if positive; use StretchDIBits with top-down DIB:
    // srcY in top-down DIB = buf.height - ly - h.
    let src_y = (buf.height - ly - h).max(0);
    unsafe {
        let _ = StretchDIBits(
            hdc,
            lx,
            ly,
            w,
            h,
            lx,
            src_y,
            w,
            h,
            Some(buf.bgra.as_ptr().cast()),
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

unsafe fn draw_size_chip(
    hdc: HDC,
    font: HFONT,
    pos: (i32, i32),
    cw: i32,
    size: (i32, i32),
) {
    let (lx, ly) = pos;
    let (w, h) = size;
    let text = CaptureUx::size_chip(w, h);
    let wt: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let of = SelectObject(hdc, font.into());
        let mut sz = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wt, &mut sz);
        let pad_x = 12;
        let pad_y = 6;
        let bw = (sz.cx + pad_x).max(68);
        let bh = (sz.cy + pad_y).max(20);
        let cy = if ly >= bh + 4 { ly - bh - 4 } else { ly + 4 };
        let cx = lx.clamp(4, (cw - bw - 4).max(4));
        let rc = RECT {
            left: cx,
            top: cy,
            right: cx + bw,
            bottom: cy + bh,
        };
        let br = CreateSolidBrush(COLORREF(0x0010_1010));
        FillRect(hdc, &rc, br);
        let _ = DeleteObject(br.into());
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00FF_FFFF));
        let mut wmut = wt;
        let mut trc = rc;
        DrawTextW(hdc, &mut wmut, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(hdc, of);
    }
}

unsafe fn draw_hint_chip(hdc: HDC, font: HFONT, text: &str) {
    let wt: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let of = SelectObject(hdc, font.into());
        let mut sz = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wt, &mut sz);
        let rc = RECT {
            left: 16,
            top: 16,
            right: 16 + sz.cx + 20,
            bottom: 16 + sz.cy + 12,
        };
        let br = CreateSolidBrush(COLORREF(0x0010_1010));
        FillRect(hdc, &rc, br);
        let _ = DeleteObject(br.into());
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00FF_FFFF));
        let mut wmut = wt;
        let mut trc = rc;
        DrawTextW(hdc, &mut wmut, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(hdc, of);
    }
}

unsafe fn draw_live_draft(hdc: HDC, s: &CaptureShared, mon_bounds: PixelRect) {
    let ox = s.selection.x - mon_bounds.x;
    let oy = s.selection.y - mon_bounds.y;
    let (b, g, r) = s.color_bgr;
    let cr = COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16));
    let thick = match s.active_tool {
        Some(AnnotateTool::Marker) => s.marker_thickness,
        Some(AnnotateTool::Eraser) => s.eraser_thickness,
        _ => s.stroke_thickness,
    };
    unsafe {
        let pen = CreatePen(PS_SOLID, thick.max(1), cr);
        let op = SelectObject(hdc, pen.into());
        let ob = SelectObject(hdc, GetStockObject(HOLLOW_BRUSH));
        match s.active_tool {
            Some(AnnotateTool::Shape) => {
                let rc = PixelRect::from_corners(
                    s.draw_start_px.0,
                    s.draw_start_px.1,
                    s.draw_curr_px.0,
                    s.draw_curr_px.1,
                );
                if s.ellipse_mode {
                    let _ = Ellipse(hdc, ox + rc.x, oy + rc.y, ox + rc.right(), oy + rc.bottom());
                } else {
                    let _ = Rectangle(hdc, ox + rc.x, oy + rc.y, ox + rc.right(), oy + rc.bottom());
                }
            }
            Some(AnnotateTool::Curve) => {
                let x1 = ox + s.draw_start_px.0;
                let y1 = oy + s.draw_start_px.1;
                let x2 = ox + s.draw_curr_px.0;
                let y2 = oy + s.draw_curr_px.1;
                match s.curve_style {
                    CurveStyle::Solid => {
                        let _ = MoveToEx(hdc, x1, y1, None);
                        let _ = LineTo(hdc, x2, y2);
                    }
                    CurveStyle::Dashed => {
                        let dx = f64::from(x2 - x1);
                        let dy = f64::from(y2 - y1);
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 1.0 {
                            let _ = MoveToEx(hdc, x1, y1, None);
                            let _ = LineTo(hdc, x2, y2);
                        } else {
                            let step = 14.0;
                            let on = 8.0;
                            let mut d = 0.0;
                            while d < len {
                                let d_end = (d + on).min(len);
                                let sx = x1 + ((dx * d) / len).round() as i32;
                                let sy = y1 + ((dy * d) / len).round() as i32;
                                let ex = x1 + ((dx * d_end) / len).round() as i32;
                                let ey = y1 + ((dy * d_end) / len).round() as i32;
                                let _ = MoveToEx(hdc, sx, sy, None);
                                let _ = LineTo(hdc, ex, ey);
                                d += step;
                            }
                        }
                    }
                    CurveStyle::Arrow => {
                        let _ = MoveToEx(hdc, x1, y1, None);
                        let _ = LineTo(hdc, x2, y2);
                        let angle = f64::from(y2 - y1).atan2(f64::from(x2 - x1));
                        const HEAD: f64 = 16.0;
                        let hx1 = (f64::from(x2) - HEAD * (angle - 0.45).cos()).round() as i32;
                        let hy1 = (f64::from(y2) - HEAD * (angle - 0.45).sin()).round() as i32;
                        let hx2 = (f64::from(x2) - HEAD * (angle + 0.45).cos()).round() as i32;
                        let hy2 = (f64::from(y2) - HEAD * (angle + 0.45).sin()).round() as i32;
                        let _ = MoveToEx(hdc, x2, y2, None);
                        let _ = LineTo(hdc, hx1, hy1);
                        let _ = MoveToEx(hdc, x2, y2, None);
                        let _ = LineTo(hdc, hx2, hy2);
                    }
                }
            }
            Some(AnnotateTool::Pencil | AnnotateTool::Marker | AnnotateTool::Eraser)
                if s.stroke_pts.len() >= 2 => {
                    let _ = MoveToEx(hdc, ox + s.stroke_pts[0].0, oy + s.stroke_pts[0].1, None);
                    for &(px, py) in &s.stroke_pts[1..] {
                        let _ = LineTo(hdc, ox + px, oy + py);
                    }
                }
            _ => {}
        }
        SelectObject(hdc, ob);
        SelectObject(hdc, op);
        let _ = DeleteObject(pen.into());
    }
}

unsafe fn draw_annotate_toolbar(hdc: HDC, s: &CaptureShared, mon_bounds: PixelRect, font: HFONT) {
    let dpi = tb_dpi(s);
    let (place, main_w, _) = toolbar_stack_placement(s);
    let mx = (place.left.round() as i32) - mon_bounds.x;
    let my = (place.main_top.round() as i32) - mon_bounds.y;
    let mh = s_px(AnnotateToolbar::HEIGHT as i32, dpi);

    unsafe {
        let bg = CreateSolidBrush(COLORREF(0x00F2_F2F2));
        let border = CreatePen(PS_SOLID, 1, COLORREF(0x00C8_C8C8));
        let ob = SelectObject(hdc, bg.into());
        let op = SelectObject(hdc, border.into());
        let r_corner = s_px(6, dpi);
        let _ = RoundRect(hdc, mx, my, mx + main_w, my + mh, r_corner, r_corner);
        SelectObject(hdc, op);
        SelectObject(hdc, ob);
        let _ = DeleteObject(border.into());
        let _ = DeleteObject(bg.into());

        for (tool, rc_virt, sep_x_virt) in toolbar_items(s) {
            if let Some(sx_v) = sep_x_virt {
                let sx = sx_v - mon_bounds.x;
                let sp = CreatePen(PS_SOLID, 1, COLORREF(0x00C8_C8C8));
                let osp = SelectObject(hdc, sp.into());
                let _ = MoveToEx(hdc, sx, my + s_px(8, dpi), None);
                let _ = LineTo(hdc, sx, my + mh - s_px(8, dpi));
                SelectObject(hdc, osp);
                let _ = DeleteObject(sp.into());
            }

            let lx = rc_virt.left - mon_bounds.x;
            let rx = rc_virt.right - mon_bounds.x;
            if s.hover_tool == Some(tool) {
                let hb = CreateSolidBrush(COLORREF(0x00E0_E0E0));
                let hrc = RECT {
                    left: lx + 1,
                    top: my + s_px(3, dpi),
                    right: rx - 1,
                    bottom: my + mh - s_px(3, dpi),
                };
                FillRect(hdc, &hrc, hb);
                let _ = DeleteObject(hb.into());
            }

            let enabled = match tool {
                AnnotateTool::Undo => !s.ops.is_empty(),
                AnnotateTool::Redo => !s.redo.is_empty(),
                AnnotateTool::Ocr => !s.ocr_busy,
                _ => true,
            };
            draw_tool_glyph(hdc, tool, (lx + rx) / 2, my + mh / 2, dpi, enabled, font);

            if s.active_tool == Some(tool) && AnnotateToolbar::shows_selected_dot(tool) {
                let dot_r = s_px(3, dpi).max(2);
                let dot_cx = rx - s_px(7, dpi);
                let dot_cy = my + s_px(7, dpi);
                let dot_b = CreateSolidBrush(COLORREF(0x0000_00FF));
                let dot_p = CreatePen(PS_SOLID, 1, COLORREF(0x0000_00FF));
                let odb = SelectObject(hdc, dot_b.into());
                let odp = SelectObject(hdc, dot_p.into());
                let _ = Ellipse(hdc, dot_cx - dot_r, dot_cy - dot_r, dot_cx + dot_r, dot_cy + dot_r);
                SelectObject(hdc, odp);
                SelectObject(hdc, odb);
                let _ = DeleteObject(dot_p.into());
                let _ = DeleteObject(dot_b.into());
            }
        }
    }
}

unsafe fn draw_tool_glyph(
    hdc: HDC,
    tool: AnnotateTool,
    cx: i32,
    cy: i32,
    dpi: u32,
    enabled: bool,
    font: HFONT,
) {
    let sc = |v: i32| -> i32 { ((v as f64) * (dpi as f64 / 96.0)).round() as i32 };
    let pen_w = s_px(2, dpi).max(2);
    let ink = if enabled { COLORREF(0x0022_2222) } else { COLORREF(0x00AA_AAAA) };
    unsafe {
        let pen = CreatePen(PS_SOLID, pen_w, ink);
        let op = SelectObject(hdc, pen.into());
        let ob = SelectObject(hdc, GetStockObject(HOLLOW_BRUSH));
        match tool {
            AnnotateTool::Shape => {
                let _ = Rectangle(hdc, cx - sc(7), cy - sc(5), cx + sc(7), cy + sc(5));
            }
            AnnotateTool::Curve => {
                let _ = MoveToEx(hdc, cx - sc(7), cy + sc(5), None);
                let _ = LineTo(hdc, cx + sc(7), cy - sc(5));
                let _ = MoveToEx(hdc, cx + sc(2), cy - sc(5), None);
                let _ = LineTo(hdc, cx + sc(7), cy - sc(5));
                let _ = LineTo(hdc, cx + sc(7), cy);
            }
            AnnotateTool::Pencil => {
                let _ = MoveToEx(hdc, cx - sc(6), cy + sc(6), None);
                let _ = LineTo(hdc, cx + sc(6), cy - sc(6));
            }
            AnnotateTool::Marker => {
                let _ = MoveToEx(hdc, cx - sc(6), cy + sc(4), None);
                let _ = LineTo(hdc, cx + sc(6), cy - sc(4));
                let _ = MoveToEx(hdc, cx - sc(7), cy + sc(7), None);
                let _ = LineTo(hdc, cx + sc(7), cy + sc(7));
            }
            AnnotateTool::Mosaic => {
                for dy in [-4, 1] {
                    for dx in [-4, 1] {
                        let _ = Rectangle(hdc, cx + sc(dx), cy + sc(dy), cx + sc(dx + 4), cy + sc(dy + 4));
                    }
                }
            }
            AnnotateTool::Number => {
                let r = sc(8);
                let _ = Ellipse(hdc, cx - r, cy - r, cx + r + 1, cy + r + 1);
                let of = SelectObject(hdc, font.into());
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, ink);
                let mut trc = RECT {
                    left: cx - r,
                    top: cy - r,
                    right: cx + r + 1,
                    bottom: cy + r + 1,
                };
                let mut w: Vec<u16> = "1".encode_utf16().collect();
                DrawTextW(hdc, &mut w, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                SelectObject(hdc, of);
            }
            AnnotateTool::Text => {
                let of = SelectObject(hdc, font.into());
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, ink);
                let mut trc = RECT {
                    left: cx - sc(10),
                    top: cy - sc(10),
                    right: cx + sc(10),
                    bottom: cy + sc(10),
                };
                let mut w: Vec<u16> = "T".encode_utf16().collect();
                DrawTextW(hdc, &mut w, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                SelectObject(hdc, of);
            }
            AnnotateTool::Ocr => {
                let of = SelectObject(hdc, font.into());
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, ink);
                let mut trc = RECT {
                    left: cx - sc(11),
                    top: cy - sc(10),
                    right: cx + sc(11),
                    bottom: cy + sc(10),
                };
                let mut w: Vec<u16> = "文".encode_utf16().collect();
                DrawTextW(hdc, &mut w, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                SelectObject(hdc, of);
            }
            AnnotateTool::Pick => {
                let _ = Ellipse(hdc, cx - sc(6), cy - sc(6), cx + sc(6), cy + sc(6));
                let _ = MoveToEx(hdc, cx - sc(2), cy, None);
                let _ = LineTo(hdc, cx + sc(3), cy);
                let _ = MoveToEx(hdc, cx, cy - sc(2), None);
                let _ = LineTo(hdc, cx, cy + sc(3));
            }
            AnnotateTool::Eraser => {
                let _ = Polygon(
                    hdc,
                    &[
                        POINT { x: cx - sc(6), y: cy + sc(2) },
                        POINT { x: cx, y: cy - sc(6) },
                        POINT { x: cx + sc(7), y: cy - sc(1) },
                        POINT { x: cx + sc(1), y: cy + sc(6) },
                    ],
                );
            }
            AnnotateTool::Undo => {
                let _ = MoveToEx(hdc, cx + sc(6), cy + sc(4), None);
                let _ = LineTo(hdc, cx + sc(6), cy - sc(2));
                let _ = LineTo(hdc, cx - sc(5), cy - sc(2));
                let _ = LineTo(hdc, cx - sc(1), cy - sc(6));
                let _ = MoveToEx(hdc, cx - sc(5), cy - sc(2), None);
                let _ = LineTo(hdc, cx - sc(1), cy + sc(2));
            }
            AnnotateTool::Redo => {
                let _ = MoveToEx(hdc, cx - sc(6), cy + sc(4), None);
                let _ = LineTo(hdc, cx - sc(6), cy - sc(2));
                let _ = LineTo(hdc, cx + sc(5), cy - sc(2));
                let _ = LineTo(hdc, cx + sc(1), cy - sc(6));
                let _ = MoveToEx(hdc, cx + sc(5), cy - sc(2), None);
                let _ = LineTo(hdc, cx + sc(1), cy + sc(2));
            }
            AnnotateTool::Close => {
                let rp = CreatePen(PS_SOLID, pen_w, COLORREF(0x0028_28DC));
                let _ = SelectObject(hdc, rp.into());
                let _ = MoveToEx(hdc, cx - sc(5), cy - sc(5), None);
                let _ = LineTo(hdc, cx + sc(6), cy + sc(6));
                let _ = MoveToEx(hdc, cx + sc(5), cy - sc(5), None);
                let _ = LineTo(hdc, cx - sc(6), cy + sc(6));
                let _ = SelectObject(hdc, pen.into());
                let _ = DeleteObject(rp.into());
            }
            AnnotateTool::Pin => {
                let _ = Rectangle(hdc, cx - sc(5), cy - sc(6), cx + sc(5), cy);
                let _ = MoveToEx(hdc, cx, cy, None);
                let _ = LineTo(hdc, cx, cy + sc(7));
            }
            AnnotateTool::Save => {
                let _ = MoveToEx(hdc, cx, cy - sc(6), None);
                let _ = LineTo(hdc, cx, cy + sc(3));
                let _ = LineTo(hdc, cx - sc(4), cy - sc(1));
                let _ = MoveToEx(hdc, cx, cy + sc(3), None);
                let _ = LineTo(hdc, cx + sc(4), cy - sc(1));
                let _ = MoveToEx(hdc, cx - sc(6), cy + sc(6), None);
                let _ = LineTo(hdc, cx + sc(6), cy + sc(6));
            }
            AnnotateTool::Copy => {
                let gp = CreatePen(PS_SOLID, pen_w, COLORREF(0x00F0_8020));
                let _ = SelectObject(hdc, gp.into());
                let _ = MoveToEx(hdc, cx - sc(6), cy, None);
                let _ = LineTo(hdc, cx - sc(1), cy + sc(5));
                let _ = LineTo(hdc, cx + sc(7), cy - sc(5));
                let _ = SelectObject(hdc, pen.into());
                let _ = DeleteObject(gp.into());
            }
        }
        SelectObject(hdc, ob);
        SelectObject(hdc, op);
        let _ = DeleteObject(pen.into());
    }
}

unsafe fn draw_annotate_subbar(hdc: HDC, s: &CaptureShared, mon_bounds: PixelRect, font: HFONT) {
    let Some(tool) = s.active_tool else {
        return;
    };
    if !AnnotateToolbar::shows_property_bar(tool) {
        return;
    }
    let dpi = tb_dpi(s);
    let (place, _, _) = toolbar_stack_placement(s);
    let sx_v = place.left.round() as i32;
    let sy_v = place.sub_top.round() as i32;
    let (items, sub_w) = build_subbar_items(s, tool, sx_v, sy_v, dpi);
    let sx = sx_v - mon_bounds.x;
    let sy = sy_v - mon_bounds.y;
    let sh = s_px(AnnotateToolbar::SUB_BAR_HEIGHT as i32, dpi);

    unsafe {
        let bg = CreateSolidBrush(COLORREF(0x002B_2B2B));
        let bd = CreatePen(PS_SOLID, 1, COLORREF(0x001A_1A1A));
        let ob = SelectObject(hdc, bg.into());
        let op = SelectObject(hdc, bd.into());
        let r_corner = s_px(6, dpi);
        let _ = RoundRect(hdc, sx, sy, sx + sub_w, sy + sh, r_corner, r_corner);
        SelectObject(hdc, op);
        SelectObject(hdc, ob);
        let _ = DeleteObject(bd.into());
        let _ = DeleteObject(bg.into());

        let of = SelectObject(hdc, font.into());
        SetBkMode(hdc, TRANSPARENT);

        for it in items {
            let lrc = RECT {
                left: it.rect.left - mon_bounds.x,
                top: it.rect.top - mon_bounds.y,
                right: it.rect.right - mon_bounds.x,
                bottom: it.rect.bottom - mon_bounds.y,
            };
            let hovered = s.hover_sub.as_ref() == Some(&it.hit);
            if let Some((b, g, r)) = it.swatch_bgr {
                let cr = COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16));
                let sbr = CreateSolidBrush(cr);
                let spen = CreatePen(
                    PS_SOLID,
                    if it.active || hovered { 2 } else { 1 },
                    if it.active || hovered {
                        COLORREF(0x00FF_FFFF)
                    } else {
                        COLORREF(0x0066_6666)
                    },
                );
                let osb = SelectObject(hdc, sbr.into());
                let osp = SelectObject(hdc, spen.into());
                let _ = Rectangle(hdc, lrc.left, lrc.top, lrc.right, lrc.bottom);
                SelectObject(hdc, osp);
                SelectObject(hdc, osb);
                let _ = DeleteObject(spen.into());
                let _ = DeleteObject(sbr.into());
            } else {
                let fill = if it.active {
                    COLORREF(0x005E_5E5E)
                } else if hovered {
                    COLORREF(0x004A_4A4A)
                } else {
                    COLORREF(0x0038_3838)
                };
                let br = CreateSolidBrush(fill);
                FillRect(hdc, &lrc, br);
                let _ = DeleteObject(br.into());
                SetTextColor(hdc, COLORREF(0x00EE_EEEE));
                let txt = match it.hit {
                    SubHit::SizeChip(v) => v.to_string(),
                    _ => it.label.to_string(),
                };
                let mut wt: Vec<u16> = txt.encode_utf16().collect();
                let mut trc = lrc;
                DrawTextW(hdc, &mut wt, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }
        }
        SelectObject(hdc, of);
    }
}

unsafe fn draw_hover_tooltip(
    hdc: HDC,
    s: &CaptureShared,
    mon_bounds: PixelRect,
    _cw: i32,
    _ch: i32,
    font: HFONT,
) {
    if s.drawing_stroke || s.resizing || s.moving_sel || !s.text_edit.is_invalid() {
        return;
    }
    let dpi = tb_dpi(s);
    let (place, _, _) = toolbar_stack_placement(s);
    let main_h = f64::from(s_px(AnnotateToolbar::HEIGHT as i32, dpi));
    let sub_h = f64::from(s_px(AnnotateToolbar::SUB_BAR_HEIGHT as i32, dpi));

    let tip_spec: Option<(String, RECT, bool, f64, f64, bool)> = if let Some(tool) = s.hover_tool {
        let rc_v = toolbar_items(s)
            .into_iter()
            .find(|(t, _, _)| *t == tool)
            .map(|(_, rc, _)| rc);
        rc_v.map(|rc| {
            let prefer_below = if place.sub_visible {
                place.sub_top < place.main_top
            } else {
                !place.above
            };
            (
                AnnotateToolbar::tooltip_with_shortcut(tool).to_string(),
                rc,
                prefer_below,
                place.sub_top,
                place.sub_top + sub_h,
                place.sub_visible,
            )
        })
    } else if let Some(ref hit) = s.hover_sub
        && let Some(tool) = s.active_tool
        && AnnotateToolbar::shows_property_bar(tool)
    {
        let (items, _) =
            build_subbar_items(s, tool, place.left.round() as i32, place.sub_top.round() as i32, dpi);
        items.into_iter().find(|it| &it.hit == hit).map(|it| {
            let txt = match hit {
                SubHit::ShapeRect => "矩形边框".to_string(),
                SubHit::ShapeEllipse => "椭圆边框".to_string(),
                SubHit::CurveStyle(CurveStyle::Solid) => "直线".to_string(),
                SubHit::CurveStyle(CurveStyle::Dashed) => "虚线".to_string(),
                SubHit::CurveStyle(CurveStyle::Arrow) => "箭头".to_string(),
                SubHit::MosaicPixel => "方块马赛克".to_string(),
                SubHit::MosaicBlur => "柔和模糊".to_string(),
                SubHit::SizeChip(v) => format!("大小 / 粗细: {v} px (滚轮可调)"),
                SubHit::Swatch(idx) => {
                    const NAMES: [&str; 6] = ["青色", "绿色", "黄色", "灰色", "白色", "红色"];
                    NAMES.get(*idx).copied().unwrap_or("颜色").to_string()
                }
            };
            (
                txt,
                it.rect,
                place.sub_top >= place.main_top,
                place.main_top,
                place.main_top + main_h,
                true,
            )
        })
    } else {
        None
    };

    let Some((text, anchor_v, prefer_below, avoid_top, avoid_bot, avoid_active)) = tip_spec else {
        return;
    };

    let wt: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let of = SelectObject(hdc, font.into());
        let mut sz = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wt, &mut sz);
        let tip_w = (sz.cx + s_px(18, dpi)).max(s_px(44, dpi));
        let tip_h = (sz.cy + s_px(10, dpi)).max(s_px(24, dpi));
        let anchor_cx = f64::from(anchor_v.left + anchor_v.right) / 2.0;

        let (vx, vy) = AnnotateToolbar::place_tooltip(
            anchor_cx,
            f64::from(anchor_v.top),
            f64::from(anchor_v.bottom),
            f64::from(tip_w),
            f64::from(tip_h),
            prefer_below,
            avoid_top,
            avoid_bot,
            avoid_active,
            f64::from(mon_bounds.x),
            f64::from(mon_bounds.y),
            f64::from(mon_bounds.right()),
            f64::from(mon_bounds.bottom()),
            f64::from(s_px(6, dpi)),
        );

        let tx = (vx.round() as i32) - mon_bounds.x;
        let ty = (vy.round() as i32) - mon_bounds.y;
        let bg = CreateSolidBrush(COLORREF(0x001E_1E1E));
        let bd = CreatePen(PS_SOLID, 1, COLORREF(0x005A_5A5A));
        let ob = SelectObject(hdc, bg.into());
        let op = SelectObject(hdc, bd.into());
        let rad = s_px(6, dpi);
        let _ = RoundRect(hdc, tx, ty, tx + tip_w, ty + tip_h, rad, rad);
        SelectObject(hdc, op);
        SelectObject(hdc, ob);
        let _ = DeleteObject(bd.into());
        let _ = DeleteObject(bg.into());

        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00FF_FFFF));
        let mut trc = RECT {
            left: tx,
            top: ty,
            right: tx + tip_w,
            bottom: ty + tip_h,
        };
        let mut wt_mut = wt;
        DrawTextW(hdc, &mut wt_mut, &mut trc, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        SelectObject(hdc, of);
    }
}
