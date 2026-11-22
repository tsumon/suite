pub struct RateFormatter;

impl RateFormatter {
    pub fn format_bytes_per_second(bytes_per_second: f64) -> String {
        let abs = bytes_per_second.abs();
        if abs < 1024.0 {
            format!("{bytes_per_second:.0} B/s")
        } else if abs < (1024.0 * 1024.0) {
            format!("{:.1} KB/s", bytes_per_second / 1024.0)
        } else if abs < (1024.0 * 1024.0 * 1024.0) {
            format!("{:.2} MB/s", bytes_per_second / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB/s", bytes_per_second / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

pub struct TaskbarSlotChrome;

impl TaskbarSlotChrome {
    pub const DEFAULT_FONT_SIZE_DIP: f64 = 13.0;
    pub const LINE_HEIGHT: f64 = 1.05;
    pub const OPTICAL_NUDGE_DIP: f64 = 1.0;
    pub const PAD_HORIZONTAL_DIP: f64 = 8.0;
    pub const FLOAT_FONT_SIZE_DIP: f64 = 13.0;
    pub const WIDGET_WIDTH_PX: i32 = 148;

    pub fn clamp_font_size(font_size_dip: f64) -> f64 {
        font_size_dip.clamp(10.0, 18.0)
    }

    pub fn content_height_dip(font_size_dip: f64, visible_line_count: usize) -> f64 {
        let lines = visible_line_count.max(1) as f64;
        Self::clamp_font_size(font_size_dip) * Self::LINE_HEIGHT * lines
    }

    pub fn offset_y_dip(window_height_dip: f64, font_size_dip: f64, visible_line_count: usize) -> f64 {
        let leftover = window_height_dip - Self::content_height_dip(font_size_dip, visible_line_count);
        if leftover <= 0.0 {
            0.0
        } else {
            (leftover / 2.0) + Self::OPTICAL_NUDGE_DIP
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveStyle {
    Solid,
    Dashed,
    Arrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnotateTool {
    Shape,
    Curve,
    Pencil,
    Marker,
    Mosaic,
    Text,
    Ocr,
    Pick,
    Eraser,
    Undo,
    Redo,
    Close,
    Pin,
    Save,
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub left: f64,
    pub top: f64,
    pub above: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackPlacement {
    pub left: f64,
    pub main_top: f64,
    pub sub_top: f64,
    pub above: bool,
    pub sub_visible: bool,
}

pub struct AnnotateToolbar;

impl AnnotateToolbar {
    pub const HEIGHT: f64 = 40.0;
    pub const SUB_BAR_HEIGHT: f64 = 36.0;
    pub const SUB_BAR_GAP: f64 = 4.0;
    pub const CELL: f64 = 36.0;
    pub const GAP: f64 = 8.0;
    pub const SEPARATOR_WIDTH: f64 = 8.0;
    pub const ICON_SIZE: f64 = 16.0;
    pub const SELECTED_DOT: f64 = 5.0;
    pub const STROKE_PX: i32 = 3;
    pub const ANCHOR_DIAMETER: i32 = 12;

    pub const PALETTE: [(u8, u8, u8); 6] = [
        (0x00, 0xD4, 0xFF),
        (0x00, 0xE6, 0x76),
        (0xFF, 0xEB, 0x3B),
        (0x9E, 0x9E, 0x9E),
        (0xFF, 0xFF, 0xFF),
        (0xDC, 0x28, 0x28),
    ];

    pub const THICKNESS_CHIPS: [i32; 3] = [2, 4, 6];
    pub const MARKER_THICKNESS_CHIPS: [i32; 3] = [10, 14, 20];
    pub const MOSAIC_RADIUS_CHIPS: [i32; 3] = [10, 16, 24];
    pub const ERASER_THICKNESS_CHIPS: [i32; 3] = [12, 18, 28];
    pub const TEXT_SIZE_CHIPS: [i32; 3] = [14, 18, 24];

    pub const ORDER: [AnnotateTool; 15] = [
        AnnotateTool::Shape,
        AnnotateTool::Curve,
        AnnotateTool::Pencil,
        AnnotateTool::Marker,
        AnnotateTool::Mosaic,
        AnnotateTool::Text,
        AnnotateTool::Ocr,
        AnnotateTool::Pick,
        AnnotateTool::Eraser,
        AnnotateTool::Undo,
        AnnotateTool::Redo,
        AnnotateTool::Close,
        AnnotateTool::Pin,
        AnnotateTool::Save,
        AnnotateTool::Copy,
    ];

    pub fn tooltip(tool: AnnotateTool) -> &'static str {
        match tool {
            AnnotateTool::Shape => "矩形",
            AnnotateTool::Curve => "曲线",
            AnnotateTool::Pencil => "铅笔",
            AnnotateTool::Marker => "马克笔",
            AnnotateTool::Mosaic => "马赛克",
            AnnotateTool::Text => "文字",
            AnnotateTool::Eraser => "橡皮",
            AnnotateTool::Undo => "撤销",
            AnnotateTool::Redo => "重做",
            AnnotateTool::Close => "关闭",
            AnnotateTool::Pin => "钉图",
            AnnotateTool::Save => "保存",
            AnnotateTool::Ocr => "识字",
            AnnotateTool::Pick => "取色",
            AnnotateTool::Copy => "复制",
        }
    }

    pub fn shows_selected_dot(tool: AnnotateTool) -> bool {
        matches!(
            tool,
            AnnotateTool::Shape
                | AnnotateTool::Curve
                | AnnotateTool::Pencil
                | AnnotateTool::Marker
                | AnnotateTool::Mosaic
                | AnnotateTool::Text
                | AnnotateTool::Eraser
        )
    }

    pub fn shows_property_bar(tool: AnnotateTool) -> bool {
        Self::shows_selected_dot(tool)
    }

    pub fn is_history(tool: AnnotateTool) -> bool {
        matches!(tool, AnnotateTool::Undo | AnnotateTool::Redo)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn place(
        sel_left: f64,
        sel_top: f64,
        sel_width: f64,
        sel_height: f64,
        bar_width: f64,
        bar_height: f64,
        work_left: f64,
        work_top: f64,
        work_right: f64,
        work_bottom: f64,
        gap: f64,
    ) -> Placement {
        let (top, above) = if sel_top + sel_height + gap + bar_height <= work_bottom {
            (sel_top + sel_height + gap, false)
        } else if sel_top - gap - bar_height >= work_top {
            (sel_top - gap - bar_height, true)
        } else {
            let max_top = work_top.max(work_bottom - bar_height);
            ((sel_top + sel_height + gap).clamp(work_top, max_top), false)
        };

        let mut left = if bar_width > sel_width {
            let candidate = sel_left + sel_width - bar_width;
            if candidate < work_left {
                sel_left
            } else {
                candidate
            }
        } else {
            sel_left
        };

        if left + bar_width > work_right {
            left = work_right - bar_width;
        }
        if left < work_left {
            left = work_left;
        }

        Placement { left, top, above }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn place_stack(
        sel_left: f64,
        sel_top: f64,
        sel_width: f64,
        sel_height: f64,
        bar_width: f64,
        main_height: f64,
        sub_height: f64,
        show_sub: bool,
        work_left: f64,
        work_top: f64,
        work_right: f64,
        work_bottom: f64,
        gap: f64,
    ) -> StackPlacement {
        let stack_h = if show_sub {
            main_height + Self::SUB_BAR_GAP + sub_height
        } else {
            main_height
        };
        let place = Self::place(
            sel_left,
            sel_top,
            sel_width,
            sel_height,
            bar_width,
            stack_h,
            work_left,
            work_top,
            work_right,
            work_bottom,
            gap,
        );

        if !show_sub {
            return StackPlacement {
                left: place.left,
                main_top: place.top,
                sub_top: place.top,
                above: place.above,
                sub_visible: false,
            };
        }

        if !place.above {
            StackPlacement {
                left: place.left,
                main_top: place.top,
                sub_top: place.top + main_height + Self::SUB_BAR_GAP,
                above: false,
                sub_visible: true,
            }
        } else {
            StackPlacement {
                left: place.left,
                main_top: place.top + sub_height + Self::SUB_BAR_GAP,
                sub_top: place.top,
                above: true,
                sub_visible: true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_formatter_thresholds() {
        assert_eq!(RateFormatter::format_bytes_per_second(0.0), "0 B/s");
        assert_eq!(RateFormatter::format_bytes_per_second(512.0), "512 B/s");
        assert_eq!(RateFormatter::format_bytes_per_second(1536.0), "1.5 KB/s");
        assert_eq!(RateFormatter::format_bytes_per_second(2.5 * 1024.0 * 1024.0), "2.50 MB/s");
        assert_eq!(
            RateFormatter::format_bytes_per_second(1.25 * 1024.0 * 1024.0 * 1024.0),
            "1.25 GB/s"
        );
    }

    #[test]
    fn taskbar_slot_chrome_math() {
        assert_eq!(TaskbarSlotChrome::clamp_font_size(6.0), 10.0);
        assert_eq!(TaskbarSlotChrome::clamp_font_size(24.0), 18.0);
        let offset = TaskbarSlotChrome::offset_y_dip(48.0, 14.0, 2);
        assert!((offset - ((48.0 - 29.4) / 2.0 + 1.0)).abs() < 1e-9);
        assert_eq!(TaskbarSlotChrome::offset_y_dip(20.0, 14.0, 2), 0.0);
    }

    #[test]
    fn annotate_toolbar_stack_flips_above_near_bottom() {
        let p = AnnotateToolbar::place_stack(
            100.0, 950.0, 400.0, 80.0, 500.0, 40.0, 36.0, true, 0.0, 0.0, 1920.0, 1040.0, 8.0,
        );
        assert!(p.above);
        assert!(p.sub_visible);
        assert!(p.sub_top < p.main_top);
        assert!(p.main_top + 40.0 <= 950.0);
    }
}
