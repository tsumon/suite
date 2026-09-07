using Suite.Capture;

namespace Suite.Capture.Tests;

public sealed class PixelDrawTests
{
    [Fact]
    public void Mosaic_fills_block_with_average()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(4, 2);
        Set(buffer, 0, 0, 10, 0, 0, 255);
        Set(buffer, 1, 0, 30, 0, 0, 255);
        Set(buffer, 0, 1, 10, 0, 0, 255);
        Set(buffer, 1, 1, 30, 0, 0, 255);
        Set(buffer, 2, 0, 0, 0, 100, 255);
        Set(buffer, 3, 0, 0, 0, 100, 255);
        Set(buffer, 2, 1, 0, 0, 100, 255);
        Set(buffer, 3, 1, 0, 0, 100, 255);

        PixelDraw.Mosaic(buffer, new PixelRect(0, 0, 4, 2), 2);

        AssertPixel(buffer, 0, 0, 20, 0, 0, 255);
        AssertPixel(buffer, 1, 1, 20, 0, 0, 255);
        AssertPixel(buffer, 2, 0, 0, 0, 100, 255);
        AssertPixel(buffer, 3, 1, 0, 0, 100, 255);
    }

    [Fact]
    public void Rectangle_writes_border_not_interior()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(6, 6);
        PixelDraw.Rectangle(buffer, new PixelRect(1, 1, 4, 4), 1);
        AssertPixel(buffer, 1, 1, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
        AssertPixel(buffer, 3, 3, 0, 0, 0, 0);
        AssertPixel(buffer, 4, 4, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
    }


    [Fact]
    public void DashedLine_skips_gap_pixels()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(40, 3);
        PixelDraw.DashedLine(buffer, 0, 1, 39, 1, 1, dash: 4, gap: 4);
        AssertPixel(buffer, 0, 1, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
        AssertPixel(buffer, 3, 1, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
        AssertPixel(buffer, 4, 1, 0, 0, 0, 0);
        AssertPixel(buffer, 7, 1, 0, 0, 0, 0);
        AssertPixel(buffer, 8, 1, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
    }

    [Fact]
    public void Arrow_marks_endpoints()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(20, 20);
        PixelDraw.Arrow(buffer, 2, 2, 15, 2, 1);
        AssertPixel(buffer, 2, 2, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
        AssertPixel(buffer, 15, 2, PixelDraw.StrokeB, PixelDraw.StrokeG, PixelDraw.StrokeR, PixelDraw.StrokeA);
    }

    [Fact]
    public void Ellipse_strokes_rim_not_center()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(20, 16);
        PixelDraw.Ellipse(buffer, new PixelRect(1, 1, 18, 14), 1);
        Assert.True(RowHasStroke(buffer, 1));
        Assert.True(ColumnHasStroke(buffer, 1));
        AssertPixel(buffer, 10, 8, 0, 0, 0, 0);
    }

    [Fact]
    public void LineColor_uses_supplied_ink()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(8, 3);
        PixelDraw.LineColor(buffer, 0, 1, 7, 1, 1, 9, 8, 7, 255);
        AssertPixel(buffer, 0, 1, 9, 8, 7, 255);
        AssertPixel(buffer, 7, 1, 9, 8, 7, 255);
    }

    [Fact]
    public void Crop_copies_sub_rectangle()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(4, 4);
        Set(buffer, 2, 1, 1, 2, 3, 255);
        PixelBuffer crop = buffer.Crop(new PixelRect(2, 1, 1, 1));
        Assert.Equal(1, crop.Width);
        Assert.Equal(1, crop.Height);
        AssertPixel(crop, 0, 0, 1, 2, 3, 255);
    }


    [Fact]
    public void MosaicBrush_stamps_along_stroke_not_full_bbox()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(40, 40);
        for (int y = 0; y < 40; y++)
        {
            for (int x = 0; x < 40; x++)
            {
                int i = (y * buffer.Stride) + (x * 4);
                buffer.Bgra[i] = 0;
                buffer.Bgra[i + 1] = 0;
                // Checker so block average differs from a single pixel.
                buffer.Bgra[i + 2] = (byte)(((x + y) % 2 == 0) ? 200 : 0);
                buffer.Bgra[i + 3] = 255;
            }
        }

        PixelDraw.MosaicBrush(buffer, [(8, 20), (18, 20)], brushRadius: 4, blockSize: 4);

        // Far corner checker pattern untouched (200 or 0 depending on parity).
        AssertPixel(buffer, 38, 38, 0, 0, (byte)(((38 + 38) % 2 == 0) ? 200 : 0), 255);
        // Pixel under brush path should be block-averaged (~100), not pure checker.
        int i8 = (20 * buffer.Stride) + (8 * 4);
        Assert.Equal(100, buffer.Bgra[i8 + 2]);
    }

    [Fact]
    public void ArrowColor_uses_supplied_ink()
    {
        PixelBuffer buffer = PixelBuffer.Allocate(20, 20);
        PixelDraw.ArrowColor(buffer, 2, 2, 15, 2, 1, 1, 2, 3, 255);
        AssertPixel(buffer, 2, 2, 1, 2, 3, 255);
        AssertPixel(buffer, 15, 2, 1, 2, 3, 255);
    }

    private static void Set(PixelBuffer buffer, int x, int y, byte b, byte g, byte r, byte a)
    {
        int i = (y * buffer.Stride) + (x * 4);
        buffer.Bgra[i] = b;
        buffer.Bgra[i + 1] = g;
        buffer.Bgra[i + 2] = r;
        buffer.Bgra[i + 3] = a;
    }

    private static void AssertPixel(PixelBuffer buffer, int x, int y, byte b, byte g, byte r, byte a)
    {
        int i = (y * buffer.Stride) + (x * 4);
        Assert.Equal(b, buffer.Bgra[i]);
        Assert.Equal(g, buffer.Bgra[i + 1]);
        Assert.Equal(r, buffer.Bgra[i + 2]);
        Assert.Equal(a, buffer.Bgra[i + 3]);
    }

    private static bool RowHasStroke(PixelBuffer buffer, int y)
    {
        for (int x = 0; x < buffer.Width; x++)
        {
            int i = (y * buffer.Stride) + (x * 4);
            if (buffer.Bgra[i + 2] == PixelDraw.StrokeR)
            {
                return true;
            }
        }

        return false;
    }

    private static bool ColumnHasStroke(PixelBuffer buffer, int x)
    {
        for (int y = 0; y < buffer.Height; y++)
        {
            int i = (y * buffer.Stride) + (x * 4);
            if (buffer.Bgra[i + 2] == PixelDraw.StrokeR)
            {
                return true;
            }
        }

        return false;
    }
}
