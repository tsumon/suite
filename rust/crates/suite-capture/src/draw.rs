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

    /// Draws a filled circular step badge with a crisp 2px white ring.
    #[allow(clippy::too_many_arguments)]
    pub fn filled_circle_with_ring(
        buffer: &mut PixelBuffer,
        cx: i32,
        cy: i32,
        radius: i32,
        b: u8,
        g: u8,
        r: u8,
        a: u8,
    ) {
        if buffer.width <= 0 || buffer.height <= 0 || buffer.bgra.is_empty() {
            return;
        }
        let rad = radius.max(4);
        let outer = rad + 2;
        let inner_r2 = (rad - 1).max(0) * (rad - 1).max(0);
        let mid_r2 = (rad + 1) * (rad + 1);
        let y0 = (cy - outer).clamp(0, buffer.height);
        let y1 = (cy + outer + 1).clamp(0, buffer.height);
        let x0 = (cx - outer).clamp(0, buffer.width);
        let x1 = (cx + outer + 1).clamp(0, buffer.width);

        for y in y0..y1 {
            let dy = y - cy;
            let row = (y * buffer.stride) as usize;
            for x in x0..x1 {
                let dx = x - cx;
                let d2 = dx * dx + dy * dy;
                if d2 > mid_r2 {
                    continue;
                }
                let i = row + (x * 4) as usize;
                let (sb, sg, sr, sa) = if d2 >= inner_r2 {
                    (255u8, 255u8, 255u8, 255u8)
                } else {
                    (b, g, r, a)
                };
                if sa == 255 {
                    buffer.bgra[i] = sb;
                    buffer.bgra[i + 1] = sg;
                    buffer.bgra[i + 2] = sr;
                    buffer.bgra[i + 3] = 255;
                } else {
                    buffer.bgra[i] = Self::blend(buffer.bgra[i], sb, sa);
                    buffer.bgra[i + 1] = Self::blend(buffer.bgra[i + 1], sg, sa);
                    buffer.bgra[i + 2] = Self::blend(buffer.bgra[i + 2], sr, sa);
                    buffer.bgra[i + 3] = buffer.bgra[i + 3].saturating_add(sa);
                }
            }
        }
    }

    pub fn blur_stamp(buffer: &mut PixelBuffer, cx: i32, cy: i32, brush_radius: i32) {
        Self::blur_brush(buffer, &[(cx, cy)], brush_radius);
    }

    /// Applies a smooth separable 2-pass box blur inside the circular brush stroke path.
    pub fn blur_brush(buffer: &mut PixelBuffer, points: &[(i32, i32)], brush_radius: i32) {
        if points.is_empty() || buffer.width <= 0 || buffer.height <= 0 || buffer.bgra.is_empty() {
            return;
        }
        let radius = brush_radius.max(2);
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for &(px, py) in points {
            min_x = min_x.min(px - radius);
            min_y = min_y.min(py - radius);
            max_x = max_x.max(px + radius + 1);
            max_y = max_y.max(py + radius + 1);
        }
        let mask_rect = PixelRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
            .clamp_to(buffer.width, buffer.height);
        if mask_rect.is_empty() {
            return;
        }

        const KERNEL_R: i32 = 5;
        let pad = KERNEL_R * 2;
        let src_rect = PixelRect::new(
            mask_rect.x - pad,
            mask_rect.y - pad,
            mask_rect.width + pad * 2,
            mask_rect.height + pad * 2,
        )
        .clamp_to(buffer.width, buffer.height);
        let sw = src_rect.width as usize;
        let sh = src_rect.height as usize;

        let mut curr = vec![[0u8; 4]; sw * sh];
        for ry in 0..src_rect.height {
            let srow = ((src_rect.y + ry) * buffer.stride + src_rect.x * 4) as usize;
            let dst_base = (ry as usize) * sw;
            for rx in 0..src_rect.width {
                let si = srow + (rx * 4) as usize;
                curr[dst_base + rx as usize] = [
                    buffer.bgra[si],
                    buffer.bgra[si + 1],
                    buffer.bgra[si + 2],
                    buffer.bgra[si + 3],
                ];
            }
        }

        let mut tmp = vec![[0u8; 4]; sw * sh];
        for _pass in 0..2 {
            Self::separable_box_blur_pass(&curr, &mut tmp, sw, sh, KERNEL_R);
            std::mem::swap(&mut curr, &mut tmp);
        }

        // Rasterize capsule brush mask over mask_rect
        let mw = mask_rect.width as usize;
        let mh = mask_rect.height as usize;
        let mut mask = vec![false; mw * mh];
        let r2 = radius * radius;
        let mut stamp_disc = |cx: i32, cy: i32| {
            let x0 = (cx - radius).max(mask_rect.x) - mask_rect.x;
            let x1 = (cx + radius + 1).min(mask_rect.right()) - mask_rect.x;
            let y0 = (cy - radius).max(mask_rect.y) - mask_rect.y;
            let y1 = (cy + radius + 1).min(mask_rect.bottom()) - mask_rect.y;
            for ly in y0..y1 {
                let dy = (mask_rect.y + ly) - cy;
                let row = (ly as usize) * mw;
                for lx in x0..x1 {
                    let dx = (mask_rect.x + lx) - cx;
                    if dx * dx + dy * dy <= r2 {
                        mask[row + lx as usize] = true;
                    }
                }
            }
        };

        stamp_disc(points[0].0, points[0].1);
        let step = (radius / 3).max(1);
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
                    stamp_disc(x, y);
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
            stamp_disc(x1, y1);
        }

        let off_x = mask_rect.x - src_rect.x;
        let off_y = mask_rect.y - src_rect.y;
        for ly in 0..mask_rect.height {
            let mrow = (ly as usize) * mw;
            let brow = ((mask_rect.y + ly) * buffer.stride) as usize;
            let srow = ((ly + off_y) as usize) * sw + off_x as usize;
            for lx in 0..mask_rect.width {
                if mask[mrow + lx as usize] {
                    let px = curr[srow + lx as usize];
                    let bi = brow + ((mask_rect.x + lx) * 4) as usize;
                    buffer.bgra[bi] = px[0];
                    buffer.bgra[bi + 1] = px[1];
                    buffer.bgra[bi + 2] = px[2];
                    buffer.bgra[bi + 3] = px[3];
                }
            }
        }
    }

    fn separable_box_blur_pass(
        src: &[[u8; 4]],
        dst: &mut [[u8; 4]],
        w: usize,
        h: usize,
        k: i32,
    ) {
        if w == 0 || h == 0 {
            return;
        }
        let mut horiz = vec![[0u8; 4]; w * h];
        let mut pb = vec![0u32; w.max(h) + 1];
        let mut pg = vec![0u32; w.max(h) + 1];
        let mut pr = vec![0u32; w.max(h) + 1];
        let mut pa = vec![0u32; w.max(h) + 1];
        let ku = k.max(1) as usize;

        for y in 0..h {
            let row = y * w;
            pb[0] = 0;
            pg[0] = 0;
            pr[0] = 0;
            pa[0] = 0;
            for x in 0..w {
                let p = src[row + x];
                pb[x + 1] = pb[x] + u32::from(p[0]);
                pg[x + 1] = pg[x] + u32::from(p[1]);
                pr[x + 1] = pr[x] + u32::from(p[2]);
                pa[x + 1] = pa[x] + u32::from(p[3]);
            }
            for x in 0..w {
                let x0 = x.saturating_sub(ku);
                let x1 = (x + ku).min(w - 1);
                let cnt = (x1 - x0 + 1) as u32;
                horiz[row + x] = [
                    ((pb[x1 + 1] - pb[x0]) / cnt) as u8,
                    ((pg[x1 + 1] - pg[x0]) / cnt) as u8,
                    ((pr[x1 + 1] - pr[x0]) / cnt) as u8,
                    ((pa[x1 + 1] - pa[x0]) / cnt) as u8,
                ];
            }
        }

        for x in 0..w {
            pb[0] = 0;
            pg[0] = 0;
            pr[0] = 0;
            pa[0] = 0;
            for y in 0..h {
                let p = horiz[y * w + x];
                pb[y + 1] = pb[y] + u32::from(p[0]);
                pg[y + 1] = pg[y] + u32::from(p[1]);
                pr[y + 1] = pr[y] + u32::from(p[2]);
                pa[y + 1] = pa[y] + u32::from(p[3]);
            }
            for y in 0..h {
                let y0 = y.saturating_sub(ku);
                let y1 = (y + ku).min(h - 1);
                let cnt = (y1 - y0 + 1) as u32;
                dst[y * w + x] = [
                    ((pb[y1 + 1] - pb[y0]) / cnt) as u8,
                    ((pg[y1 + 1] - pg[y0]) / cnt) as u8,
                    ((pr[y1 + 1] - pr[y0]) / cnt) as u8,
                    ((pa[y1 + 1] - pa[y0]) / cnt) as u8,
                ];
            }
        }
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
    BlurStroke {
        points: Vec<(i32, i32)>,
        brush_radius: i32,
    },
    StepBadge {
        cx: i32,
        cy: i32,
        number: u32,
        radius: i32,
        b: u8,
        g: u8,
        r: u8,
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
            AnnotationOp::BlurStroke {
                points,
                brush_radius,
            } => {
                PixelDraw::blur_brush(&mut result, points, *brush_radius);
            }
            AnnotationOp::StepBadge {
                cx,
                cy,
                number,
                radius,
                b,
                g,
                r,
            } => {
                PixelDraw::filled_circle_with_ring(&mut result, *cx, *cy, *radius, *b, *g, *r, 255);
                let label = number.to_string();
                let font_size = (*radius as f32 * 1.15).max(11.0);
                let half_w = (label.len() as f32 * font_size * 0.30).round() as i32;
                let half_h = (font_size * 0.64).round() as i32;
                draw_text(
                    &mut result,
                    *cx - half_w,
                    *cy - half_h,
                    &label,
                    255,
                    255,
                    255,
                    font_size,
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

    #[test]
    fn blur_stroke_softens_inside_brush_and_preserves_outside() {
        let mut orig = PixelBuffer::allocate(64, 64);
        PixelDraw::fill_color(&mut orig, PixelRect::new(0, 0, 32, 64), 0, 0, 0, 255);
        PixelDraw::fill_color(&mut orig, PixelRect::new(32, 0, 32, 64), 240, 240, 240, 255);
        let ops = vec![AnnotationOp::BlurStroke {
            points: vec![(32, 32)],
            brush_radius: 10,
        }];
        let baked = bake_annotations(&orig, &ops, |_, _, _, _, _, _, _, _| {});
        let edge_px = baked.try_get_pixel(32, 32).unwrap();
        assert!(edge_px.0 > 20 && edge_px.0 < 220, "expected blurred edge, got {:?}", edge_px);
        assert_eq!(baked.try_get_pixel(2, 2), Some((0, 0, 0, 255)));
        assert_eq!(baked.try_get_pixel(60, 60), Some((240, 240, 240, 255)));
    }

    #[test]
    fn step_badge_paints_fill_ring_and_invokes_text_callback() {
        let mut orig = PixelBuffer::allocate(64, 64);
        PixelDraw::fill_color(&mut orig, PixelRect::new(0, 0, 64, 64), 20, 20, 20, 255);
        let mut captured_label = String::new();
        let ops = vec![AnnotationOp::StepBadge {
            cx: 32,
            cy: 32,
            number: 3,
            radius: 14,
            b: 30,
            g: 60,
            r: 220,
        }];
        let baked = bake_annotations(&orig, &ops, |_, _, _, txt, _, _, _, _| {
            captured_label = txt.to_string();
        });
        assert_eq!(captured_label, "3");
        assert_eq!(baked.try_get_pixel(32, 32), Some((30, 60, 220, 255)));
        assert_eq!(baked.try_get_pixel(32 + 14, 32), Some((255, 255, 255, 255)));
        assert_eq!(baked.try_get_pixel(2, 2), Some((20, 20, 20, 255)));
    }
}
