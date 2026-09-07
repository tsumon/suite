namespace Suite.Capture;

public static class PixelDraw
{
    public const byte StrokeB = 40;
    public const byte StrokeG = 40;
    public const byte StrokeR = 220;
    public const byte StrokeA = 255;
    public const int DefaultThickness = 3;
    public const int DefaultMosaicBlock = 12;

    public static void Rectangle(PixelBuffer buffer, PixelRect rect, int thickness = DefaultThickness)
    {
        PixelRect r = rect.ClampTo(buffer.Width, buffer.Height);
        if (r.IsEmpty)
        {
            return;
        }

        int t = Math.Max(1, thickness);
        Fill(buffer, new PixelRect(r.X, r.Y, r.Width, Math.Min(t, r.Height)));
        Fill(buffer, new PixelRect(r.X, r.Bottom - Math.Min(t, r.Height), r.Width, Math.Min(t, r.Height)));
        Fill(buffer, new PixelRect(r.X, r.Y, Math.Min(t, r.Width), r.Height));
        Fill(buffer, new PixelRect(r.Right - Math.Min(t, r.Width), r.Y, Math.Min(t, r.Width), r.Height));
    }

    public static void Arrow(PixelBuffer buffer, int x1, int y1, int x2, int y2, int thickness = DefaultThickness)
    {
        Line(buffer, x1, y1, x2, y2, thickness);
        double angle = Math.Atan2(y2 - y1, x2 - x1);
        const int head = 16;
        int hx1 = (int)Math.Round(x2 - (head * Math.Cos(angle - 0.45)));
        int hy1 = (int)Math.Round(y2 - (head * Math.Sin(angle - 0.45)));
        int hx2 = (int)Math.Round(x2 - (head * Math.Cos(angle + 0.45)));
        int hy2 = (int)Math.Round(y2 - (head * Math.Sin(angle + 0.45)));
        Line(buffer, x2, y2, hx1, hy1, thickness);
        Line(buffer, x2, y2, hx2, hy2, thickness);
    }

    public static void Line(PixelBuffer buffer, int x0, int y0, int x1, int y1, int thickness = DefaultThickness) =>
        LineColor(buffer, x0, y0, x1, y1, thickness, StrokeB, StrokeG, StrokeR, StrokeA);

    public static void LineColor(
        PixelBuffer buffer,
        int x0,
        int y0,
        int x1,
        int y1,
        int thickness,
        byte b,
        byte g,
        byte r,
        byte a)
    {
        int dx = Math.Abs(x1 - x0);
        int dy = Math.Abs(y1 - y0);
        int sx = x0 < x1 ? 1 : -1;
        int sy = y0 < y1 ? 1 : -1;
        int err = dx - dy;
        int x = x0;
        int y = y0;
        int t = Math.Max(1, thickness);
        while (true)
        {
            StampColor(buffer, x, y, t, b, g, r, a);
            if (x == x1 && y == y1)
            {
                break;
            }

            int e2 = 2 * err;
            if (e2 > -dy)
            {
                err -= dy;
                x += sx;
            }

            if (e2 < dx)
            {
                err += dx;
                y += sy;
            }
        }
    }

    public static void Polyline(
        PixelBuffer buffer,
        IReadOnlyList<(int X, int Y)> points,
        int thickness,
        byte b,
        byte g,
        byte r,
        byte a)
    {
        if (points.Count == 0)
        {
            return;
        }

        if (points.Count == 1)
        {
            StampColor(buffer, points[0].X, points[0].Y, thickness, b, g, r, a);
            return;
        }

        for (int i = 1; i < points.Count; i++)
        {
            LineColor(buffer, points[i - 1].X, points[i - 1].Y, points[i].X, points[i].Y, thickness, b, g, r, a);
        }
    }

    public static void Ellipse(PixelBuffer buffer, PixelRect rect, int thickness = DefaultThickness)
    {
        PixelRect r = rect.ClampTo(buffer.Width, buffer.Height);
        if (r.Width < 2 || r.Height < 2)
        {
            Rectangle(buffer, r, thickness);
            return;
        }

        int t = Math.Max(1, thickness);
        double cx = r.X + ((r.Width - 1) / 2.0);
        double cy = r.Y + ((r.Height - 1) / 2.0);
        double rx = Math.Max(0.5, (r.Width - 1) / 2.0);
        double ry = Math.Max(0.5, (r.Height - 1) / 2.0);
        int steps = Math.Max(16, (int)Math.Ceiling(Math.PI * (rx + ry)));
        int prevX = (int)Math.Round(cx + rx);
        int prevY = (int)Math.Round(cy);
        for (int i = 1; i <= steps; i++)
        {
            double theta = (Math.PI * 2 * i) / steps;
            int x = (int)Math.Round(cx + (rx * Math.Cos(theta)));
            int y = (int)Math.Round(cy + (ry * Math.Sin(theta)));
            Line(buffer, prevX, prevY, x, y, t);
            prevX = x;
            prevY = y;
        }
    }

    public static void RestoreStamp(PixelBuffer dest, PixelBuffer original, int cx, int cy, int thickness)
    {
        int t = Math.Max(1, thickness);
        int half = t / 2;
        PixelRect rect = new PixelRect(cx - half, cy - half, t, t).ClampTo(dest.Width, dest.Height);
        if (rect.IsEmpty)
        {
            return;
        }

        dest.BlitFrom(original, rect, rect.X, rect.Y);
    }

    public static void RestoreStroke(PixelBuffer dest, PixelBuffer original, IReadOnlyList<(int X, int Y)> points, int thickness)
    {
        if (points.Count == 0)
        {
            return;
        }

        int t = Math.Max(1, thickness);
        RestoreStamp(dest, original, points[0].X, points[0].Y, t);
        for (int i = 1; i < points.Count; i++)
        {
            int x0 = points[i - 1].X;
            int y0 = points[i - 1].Y;
            int x1 = points[i].X;
            int y1 = points[i].Y;
            int dx = Math.Abs(x1 - x0);
            int dy = Math.Abs(y1 - y0);
            int sx = x0 < x1 ? 1 : -1;
            int sy = y0 < y1 ? 1 : -1;
            int err = dx - dy;
            int x = x0;
            int y = y0;
            while (true)
            {
                RestoreStamp(dest, original, x, y, t);
                if (x == x1 && y == y1)
                {
                    break;
                }

                int e2 = 2 * err;
                if (e2 > -dy)
                {
                    err -= dy;
                    x += sx;
                }

                if (e2 < dx)
                {
                    err += dx;
                    y += sy;
                }
            }
        }
    }

    public static void Mosaic(PixelBuffer buffer, PixelRect rect, int blockSize = DefaultMosaicBlock)
    {
        PixelRect r = rect.ClampTo(buffer.Width, buffer.Height);
        if (r.IsEmpty)
        {
            return;
        }

        int block = Math.Max(2, blockSize);
        for (int by = r.Y; by < r.Bottom; by += block)
        {
            int bh = Math.Min(block, r.Bottom - by);
            for (int bx = r.X; bx < r.Right; bx += block)
            {
                int bw = Math.Min(block, r.Right - bx);
                AverageBlock(buffer, bx, by, bw, bh, out byte b, out byte g, out byte red, out byte a);
                FillColor(buffer, new PixelRect(bx, by, bw, bh), b, g, red, a);
            }
        }
    }

    public static void Fill(PixelBuffer buffer, PixelRect rect) =>
        FillColor(buffer, rect, StrokeB, StrokeG, StrokeR, StrokeA);

    public static void FillColor(PixelBuffer buffer, PixelRect rect, byte b, byte g, byte r, byte a)
    {
        PixelRect clipped = rect.ClampTo(buffer.Width, buffer.Height);
        if (clipped.IsEmpty)
        {
            return;
        }

        for (int y = clipped.Y; y < clipped.Bottom; y++)
        {
            int row = y * buffer.Stride;
            for (int x = clipped.X; x < clipped.Right; x++)
            {
                int i = row + (x * 4);
                buffer.Bgra[i] = b;
                buffer.Bgra[i + 1] = g;
                buffer.Bgra[i + 2] = r;
                buffer.Bgra[i + 3] = a;
            }
        }
    }

    public static void Stamp(PixelBuffer buffer, int cx, int cy, int thickness) =>
        StampColor(buffer, cx, cy, thickness, StrokeB, StrokeG, StrokeR, StrokeA);

    public static void StampColor(
        PixelBuffer buffer,
        int cx,
        int cy,
        int thickness,
        byte b,
        byte g,
        byte r,
        byte a)
    {
        int half = Math.Max(0, thickness / 2);
        PixelRect rect = new PixelRect(cx - half, cy - half, Math.Max(1, thickness), Math.Max(1, thickness));
        if (a == 255)
        {
            FillColor(buffer, rect, b, g, r, a);
            return;
        }

        PixelRect clipped = rect.ClampTo(buffer.Width, buffer.Height);
        if (clipped.IsEmpty)
        {
            return;
        }

        for (int y = clipped.Y; y < clipped.Bottom; y++)
        {
            int row = y * buffer.Stride;
            for (int x = clipped.X; x < clipped.Right; x++)
            {
                int i = row + (x * 4);
                byte db = buffer.Bgra[i];
                byte dg = buffer.Bgra[i + 1];
                byte dr = buffer.Bgra[i + 2];
                byte da = buffer.Bgra[i + 3];
                buffer.Bgra[i] = Blend(db, b, a);
                buffer.Bgra[i + 1] = Blend(dg, g, a);
                buffer.Bgra[i + 2] = Blend(dr, r, a);
                buffer.Bgra[i + 3] = (byte)Math.Min(255, da + a);
            }
        }
    }

    private static byte Blend(byte dst, byte src, byte a) =>
        (byte)(((src * a) + (dst * (255 - a))) / 255);

    private static void AverageBlock(
        PixelBuffer buffer,
        int x,
        int y,
        int w,
        int h,
        out byte b,
        out byte g,
        out byte r,
        out byte a)
    {
        long sb = 0, sg = 0, sr = 0, sa = 0;
        int count = 0;
        for (int py = y; py < y + h; py++)
        {
            int row = py * buffer.Stride;
            for (int px = x; px < x + w; px++)
            {
                int i = row + (px * 4);
                sb += buffer.Bgra[i];
                sg += buffer.Bgra[i + 1];
                sr += buffer.Bgra[i + 2];
                sa += buffer.Bgra[i + 3];
                count++;
            }
        }

        if (count == 0)
        {
            b = g = r = 0;
            a = 255;
            return;
        }

        b = (byte)(sb / count);
        g = (byte)(sg / count);
        r = (byte)(sr / count);
        a = (byte)(sa / count);
    }
}
