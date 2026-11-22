use crate::pixel::{PixelBuffer, PixelRect};

pub struct PixelDraw;

impl PixelDraw {
    pub const STROKE_B: u8 = 40;
    pub const STROKE_G: u8 = 40;
    pub const STROKE_R: u8 = 220;
    pub const STROKE_A: u8 = 255;
    pub const DEFAULT_THICKNESS: i32 = 3;
    pub const DEFAULT_MOSAIC_BLOCK: i32 = 12;

    pub fn rectangle_color(
        buffer: &mut PixelBuffer,
        rect: PixelRect,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        let clipped = rect.clamp_to(buffer.width, buffer.height);
        if clipped.is_empty() {
            return;
        }
        let t = thickness.max(1);
        let h_bar = t.min(clipped.height);
        let w_bar = t.min(clipped.width);
        Self::fill_color(buffer, PixelRect::new(clipped.x, clipped.y, clipped.width, h_bar), b, g, r, a);
        Self::fill_color(
            buffer,
            PixelRect::new(clipped.x, clipped.bottom() - h_bar, clipped.width, h_bar),
            b,
            g,
            r,
            a,
        );
        Self::fill_color(buffer, PixelRect::new(clipped.x, clipped.y, w_bar, clipped.height), b, g, r, a);
        Self::fill_color(
            buffer,
            PixelRect::new(clipped.right() - w_bar, clipped.y, w_bar, clipped.height),
            b,
            g,
            r,
            a,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn arrow_color(
        buffer: &mut PixelBuffer,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        Self::line_color(buffer, x1, y1, x2, y2, thickness, b, g, r, a);
        let angle = f64::from(y2 - y1).atan2(f64::from(x2 - x1));
        const HEAD: f64 = 16.0;
        let hx1 = (f64::from(x2) - HEAD * (angle - 0.45).cos()).round() as i32;
        let hy1 = (f64::from(y2) - HEAD * (angle - 0.45).sin()).round() as i32;
        let hx2 = (f64::from(x2) - HEAD * (angle + 0.45).cos()).round() as i32;
        let hy2 = (f64::from(y2) - HEAD * (angle + 0.45).sin()).round() as i32;
        Self::line_color(buffer, x2, y2, hx1, hy1, thickness, b, g, r, a);
        Self::line_color(buffer, x2, y2, hx2, hy2, thickness, b, g, r, a);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn dashed_line_color(
        buffer: &mut PixelBuffer,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
        dash: i32,
        gap: i32,
    ) {
        let on = dash.max(1);
        let off = gap.max(1);
        let period = on + off;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;
        let t = thickness.max(1);
        let mut step = 0;
        loop {
            if (step % period) < on {
                Self::stamp_color(buffer, x, y, t, b, g, r, a);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
            step += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn line_color(
        buffer: &mut PixelBuffer,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;
        let t = thickness.max(1);
        loop {
            Self::stamp_color(buffer, x, y, t, b, g, r, a);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn polyline(
        buffer: &mut PixelBuffer,
        points: &[(i32, i32)],
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        if points.is_empty() {
            return;
        }
        if points.len() == 1 {
            Self::stamp_color(buffer, points[0].0, points[0].1, thickness, b, g, r, a);
            return;
        }
        for pair in points.windows(2) {
            Self::line_color(
                buffer,
                pair[0].0,
                pair[0].1,
                pair[1].0,
                pair[1].1,
                thickness,
                b,
                g,
                r,
                a,
            );
        }
    }

    pub fn ellipse_color(
        buffer: &mut PixelBuffer,
        rect: PixelRect,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        let clipped = rect.clamp_to(buffer.width, buffer.height);
        if clipped.width < 2 || clipped.height < 2 {
            Self::rectangle_color(buffer, clipped, thickness, b, g, r, a);
            return;
        }
        let t = thickness.max(1);
        let cx = f64::from(clipped.x) + f64::from(clipped.width - 1) / 2.0;
        let cy = f64::from(clipped.y) + f64::from(clipped.height - 1) / 2.0;
        let rx = (f64::from(clipped.width - 1) / 2.0).max(0.5);
        let ry = (f64::from(clipped.height - 1) / 2.0).max(0.5);
        let steps = (std::f64::consts::PI * (rx + ry)).ceil().max(16.0) as i32;
        let mut prev_x = (cx + rx).round() as i32;
        let mut prev_y = cy.round() as i32;
        for i in 1..=steps {
            let theta = (std::f64::consts::PI * 2.0 * f64::from(i)) / f64::from(steps);
            let x = (cx + rx * theta.cos()).round() as i32;
            let y = (cy + ry * theta.sin()).round() as i32;
            Self::line_color(buffer, prev_x, prev_y, x, y, t, b, g, r, a);
            prev_x = x;
            prev_y = y;
        }
    }

    pub fn restore_stamp(
        dest: &mut PixelBuffer,
        original: &PixelBuffer,
        cx: i32,
        cy: i32,
        thickness: i32,
    ) {
        let t = thickness.max(1);
        let half = t / 2;
        let rect = PixelRect::new(cx - half, cy - half, t, t).clamp_to(dest.width, dest.height);
        if rect.is_empty() {
            return;
        }
        dest.blit_from(original, rect, rect.x, rect.y);
    }

    pub fn restore_stroke(
        dest: &mut PixelBuffer,
        original: &PixelBuffer,
        points: &[(i32, i32)],
        thickness: i32,
    ) {
        if points.is_empty() {
            return;
        }
        let t = thickness.max(1);
        Self::restore_stamp(dest, original, points[0].0, points[0].1, t);
        for pair in points.windows(2) {
            let (mut x, mut y) = pair[0];
            let (x1, y1) = pair[1];
            let dx = (x1 - x).abs();
            let dy = (y1 - y).abs();
            let sx = if x < x1 { 1 } else { -1 };
            let sy = if y < y1 { 1 } else { -1 };
            let mut err = dx - dy;
            loop {
                Self::restore_stamp(dest, original, x, y, t);
                if x == x1 && y == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    x += sx;
                }
                if e2 < dx {
                    err += dx;
                    y += sy;
                }
            }
        }
    }

    pub fn mosaic(buffer: &mut PixelBuffer, rect: PixelRect, block_size: i32) {
        let r = rect.clamp_to(buffer.width, buffer.height);
        if r.is_empty() {
            return;
        }
        let block = block_size.max(2);
        let mut by = r.y;
        while by < r.bottom() {
            let bh = block.min(r.bottom() - by);
            let mut bx = r.x;
            while bx < r.right() {
                let bw = block.min(r.right() - bx);
                let (b, g, red, a) = Self::average_block(buffer, bx, by, bw, bh);
                Self::fill_color(buffer, PixelRect::new(bx, by, bw, bh), b, g, red, a);
                bx += block;
            }
            by += block;
        }
    }

    pub fn mosaic_brush(
        buffer: &mut PixelBuffer,
        points: &[(i32, i32)],
        brush_radius: i32,
        block_size: i32,
    ) {
        if points.is_empty() {
            return;
        }
        let radius = brush_radius.max(2);
        let step = (radius / 2).max(1);
        Self::mosaic_stamp(buffer, points[0].0, points[0].1, radius, block_size);
        for pair in points.windows(2) {
            let (mut x, mut y) = pair[0];
            let (x1, y1) = pair[1];
            let dx = (x1 - x).abs();
            let dy = (y1 - y).abs();
            let sx = if x < x1 { 1 } else { -1 };
            let sy = if y < y1 { 1 } else { -1 };
            let mut err = dx - dy;
            let mut walked = 0;
            loop {
                if walked % step == 0 {
                    Self::mosaic_stamp(buffer, x, y, radius, block_size);
                }
                if x == x1 && y == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    x += sx;
                }
                if e2 < dx {
                    err += dx;
                    y += sy;
                }
                walked += 1;
            }
        }
    }

    pub fn mosaic_stamp(
        buffer: &mut PixelBuffer,
        cx: i32,
        cy: i32,
        brush_radius: i32,
        block_size: i32,
    ) {
        let radius = brush_radius.max(2);
        Self::mosaic(
            buffer,
            PixelRect::new(cx - radius, cy - radius, radius * 2, radius * 2),
            block_size,
        );
    }

    pub fn fill_color(buffer: &mut PixelBuffer, rect: PixelRect, b: u8, g: u8, r: u8, a: u8) {
        let clipped = rect.clamp_to(buffer.width, buffer.height);
        if clipped.is_empty() {
            return;
        }
        for y in clipped.y..clipped.bottom() {
            let row = (y * buffer.stride) as usize;
            for x in clipped.x..clipped.right() {
                let i = row + (x * 4) as usize;
                buffer.bgra[i] = b;
                buffer.bgra[i + 1] = g;
                buffer.bgra[i + 2] = r;
                buffer.bgra[i + 3] = a;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stamp_color(
        buffer: &mut PixelBuffer,
        cx: i32,
        cy: i32,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        let half = (thickness / 2).max(0);
        let t = thickness.max(1);
        let rect = PixelRect::new(cx - half, cy - half, t, t);
        if a == 255 {
            Self::fill_color(buffer, rect, b, g, r, a);
            return;
        }
        let clipped = rect.clamp_to(buffer.width, buffer.height);
        if clipped.is_empty() {
            return;
        }
        for y in clipped.y..clipped.bottom() {
            let row = (y * buffer.stride) as usize;
            for x in clipped.x..clipped.right() {
                let i = row + (x * 4) as usize;
                let db = buffer.bgra[i];
                let dg = buffer.bgra[i + 1];
                let dr = buffer.bgra[i + 2];
                let da = buffer.bgra[i + 3];
                buffer.bgra[i] = Self::blend(db, b, a);
                buffer.bgra[i + 1] = Self::blend(dg, g, a);
                buffer.bgra[i + 2] = Self::blend(dr, r, a);
                buffer.bgra[i + 3] = da.saturating_add(a);
            }
        }
    }

    fn blend(dst: u8, src: u8, a: u8) -> u8 {
        ((u16::from(src) * u16::from(a) + u16::from(dst) * u16::from(255 - a)) / 255) as u8
    }

    fn average_block(buffer: &PixelBuffer, x: i32, y: i32, w: i32, h: i32) -> (u8, u8, u8, u8) {
        let mut sb = 0u64;
        let mut sg = 0u64;
        let mut sr = 0u64;
        let mut sa = 0u64;
        let mut count = 0u64;
        for py in y..(y + h) {
            let row = (py * buffer.stride) as usize;
            for px in x..(x + w) {
                let i = row + (px * 4) as usize;
                sb += u64::from(buffer.bgra[i]);
                sg += u64::from(buffer.bgra[i + 1]);
                sr += u64::from(buffer.bgra[i + 2]);
                sa += u64::from(buffer.bgra[i + 3]);
                count += 1;
            }
        }
        match count {
            0 => (0, 0, 0, 255),
            n => (
                (sb / n) as u8,
                (sg / n) as u8,
                (sr / n) as u8,
                (sa / n) as u8,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnnotationOp {
    Rect {
        rect: PixelRect,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
    },
    Ellipse {
        rect: PixelRect,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
    },
    Stroke {
        points: Vec<(i32, i32)>,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
        dashed: bool,
    },
    Arrow {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        thickness: i32,
        b: u8,
        g: u8,
        r: u8,
    },
    MosaicStroke {
        points: Vec<(i32, i32)>,
        brush_radius: i32,
    },
    Erase {
        points: Vec<(i32, i32)>,
        thickness: i32,
    },
    Text {
        x: i32,
        y: i32,
        text: String,
        b: u8,
        g: u8,
        r: u8,
        font_size: f32,
    },
}

pub fn bake_annotations<F>(
    original: &PixelBuffer,
    ops: &[AnnotationOp],
    mut draw_text: F,
) -> PixelBuffer
where
    F: FnMut(&mut PixelBuffer, i32, i32, &str, u8, u8, u8, f32),
{
    let mut result = original.clone();
    for op in ops {
        match op {
            AnnotationOp::Rect {
                rect,
                thickness,
                b,
                g,
                r,
            } => {
                PixelDraw::rectangle_color(&mut result, *rect, *thickness, *b, *g, *r, 255);
            }
            AnnotationOp::Ellipse {
                rect,
                thickness,
                b,
                g,
                r,
            } => {
                PixelDraw::ellipse_color(&mut result, *rect, *thickness, *b, *g, *r, 255);
            }
            AnnotationOp::Stroke {
                points,
                thickness,
                b,
                g,
                r,
                a,
                dashed,
            } => {
                if *dashed && points.len() >= 2 {
                    for pair in points.windows(2) {
                        PixelDraw::dashed_line_color(
                            &mut result,
                            pair[0].0,
                            pair[0].1,
                            pair[1].0,
                            pair[1].1,
                            *thickness,
                            *b,
                            *g,
                            *r,
                            *a,
                            8,
                            6,
                        );
                    }
                } else {
                    PixelDraw::polyline(&mut result, points, *thickness, *b, *g, *r, *a);
                }
            }
            AnnotationOp::Arrow {
                x1,
                y1,
                x2,
                y2,
                thickness,
                b,
                g,
                r,
            } => {
                PixelDraw::arrow_color(&mut result, *x1, *y1, *x2, *y2, *thickness, *b, *g, *r, 255);
            }
            AnnotationOp::MosaicStroke {
                points,
                brush_radius,
            } => {
                PixelDraw::mosaic_brush(
                    &mut result,
                    points,
                    *brush_radius,
                    PixelDraw::DEFAULT_MOSAIC_BLOCK,
                );
            }
            AnnotationOp::Erase { points, thickness } => {
                PixelDraw::restore_stroke(&mut result, original, points, *thickness);
            }
            AnnotationOp::Text {
                x,
                y,
                text,
                b,
                g,
                r,
                font_size,
            } => {
                draw_text(&mut result, *x, *y, text, *b, *g, *r, *font_size);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_rectangle_mosaic_and_erase_restore() {
        let mut orig = PixelBuffer::allocate(32, 32);
        PixelDraw::fill_color(&mut orig, PixelRect::new(0, 0, 32, 32), 10, 20, 30, 255);
        let ops = vec![
            AnnotationOp::Rect {
                rect: PixelRect::new(4, 4, 16, 16),
                thickness: 2,
                b: 0,
                g: 0,
                r: 255,
            },
            AnnotationOp::Erase {
                points: vec![(4, 4), (5, 4)],
                thickness: 4,
            },
        ];
        let baked = bake_annotations(&orig, &ops, |_, _, _, _, _, _, _, _| {});
        assert_eq!(baked.try_get_pixel(4, 4), Some((10, 20, 30, 255)));
        assert_eq!(baked.try_get_pixel(15, 4), Some((0, 0, 255, 255)));
    }
}
