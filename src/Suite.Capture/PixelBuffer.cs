using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Suite.Capture;

public sealed class PixelBuffer
{
    public PixelBuffer(int width, int height, byte[] bgra, int stride)
    {
        if (width <= 0 || height <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(width), "Buffer must be at least 1x1.");
        }

        if (stride < width * 4)
        {
            throw new ArgumentOutOfRangeException(nameof(stride));
        }

        if (bgra.Length < stride * height)
        {
            throw new ArgumentException("Buffer is smaller than stride * height.", nameof(bgra));
        }

        Width = width;
        Height = height;
        Stride = stride;
        Bgra = bgra;
    }

    public int Width { get; private set; }
    public int Height { get; private set; }
    public int Stride { get; private set; }
    public byte[] Bgra { get; private set; }

    /// <summary>Drop large pixel storage early (capture frames / scroll stitch / bake temps).</summary>
    public void ReleasePixels()
    {
        if (Bgra.Length == 0)
        {
            return;
        }

        Array.Clear(Bgra, 0, Bgra.Length);
        Bgra = Array.Empty<byte>();
        Width = 0;
        Height = 0;
        Stride = 0;
    }

    public static PixelBuffer Allocate(int width, int height)
    {
        int stride = width * 4;
        return new PixelBuffer(width, height, new byte[stride * height], stride);
    }

    public PixelBuffer Clone()
    {
        var copy = new byte[Bgra.Length];
        Buffer.BlockCopy(Bgra, 0, copy, 0, Bgra.Length);
        return new PixelBuffer(Width, Height, copy, Stride);
    }

    public PixelBuffer Crop(PixelRect rect)
    {
        PixelRect clipped = rect.ClampTo(Width, Height);
        if (clipped.IsEmpty)
        {
            throw new ArgumentOutOfRangeException(nameof(rect), "Crop rectangle is empty.");
        }

        PixelBuffer dest = Allocate(clipped.Width, clipped.Height);
        for (int y = 0; y < clipped.Height; y++)
        {
            int src = ((clipped.Y + y) * Stride) + (clipped.X * 4);
            int dst = y * dest.Stride;
            Buffer.BlockCopy(Bgra, src, dest.Bgra, dst, clipped.Width * 4);
        }

        return dest;
    }

    public void BlitFrom(PixelBuffer src, PixelRect srcRect, int destX, int destY)
    {
        PixelRect clippedSrc = srcRect.ClampTo(src.Width, src.Height);
        if (clippedSrc.IsEmpty)
        {
            return;
        }

        int copyW = clippedSrc.Width;
        int copyH = clippedSrc.Height;
        if (destX < 0)
        {
            copyW += destX;
            clippedSrc = new PixelRect(clippedSrc.X - destX, clippedSrc.Y, copyW, copyH);
            destX = 0;
        }

        if (destY < 0)
        {
            copyH += destY;
            clippedSrc = new PixelRect(clippedSrc.X, clippedSrc.Y - destY, copyW, copyH);
            destY = 0;
        }

        if (destX + copyW > Width)
        {
            copyW = Width - destX;
        }

        if (destY + copyH > Height)
        {
            copyH = Height - destY;
        }

        if (copyW <= 0 || copyH <= 0)
        {
            return;
        }

        for (int y = 0; y < copyH; y++)
        {
            int srcIndex = ((clippedSrc.Y + y) * src.Stride) + (clippedSrc.X * 4);
            int dstIndex = ((destY + y) * Stride) + (destX * 4);
            Buffer.BlockCopy(src.Bgra, srcIndex, Bgra, dstIndex, copyW * 4);
        }
    }

    public bool TryGetPixel(int x, int y, out byte b, out byte g, out byte r, out byte a)
    {
        b = g = r = a = 0;
        if (x < 0 || y < 0 || x >= Width || y >= Height || Bgra.Length == 0)
        {
            return false;
        }

        int i = (y * Stride) + (x * 4);
        b = Bgra[i];
        g = Bgra[i + 1];
        r = Bgra[i + 2];
        a = Bgra[i + 3];
        return true;
    }

    public BitmapSource? TryCropZoom(int centerX, int centerY, int srcHalf, int scale)
    {
        if (Bgra.Length == 0 || Width <= 0 || Height <= 0 || scale < 1)
        {
            return null;
        }

        int left = Math.Clamp(centerX - srcHalf, 0, Math.Max(0, Width - 1));
        int top = Math.Clamp(centerY - srcHalf, 0, Math.Max(0, Height - 1));
        int size = srcHalf * 2;
        if (left + size > Width)
        {
            left = Math.Max(0, Width - size);
        }

        if (top + size > Height)
        {
            top = Math.Max(0, Height - size);
        }

        size = Math.Min(size, Math.Min(Width - left, Height - top));
        if (size <= 0)
        {
            return null;
        }

        try
        {
            PixelBuffer crop = Crop(new PixelRect(left, top, size, size));
            BitmapSource src = crop.ToBitmapSource();
            crop.ReleasePixels();
            int outSize = size * scale;
            var scaled = new TransformedBitmap(src, new System.Windows.Media.ScaleTransform(scale, scale));
            scaled.Freeze();
            return scaled;
        }
        catch
        {
            return null;
        }
    }

    public BitmapSource ToBitmapSource()
    {
        var source = BitmapSource.Create(
            Width,
            Height,
            96,
            96,
            PixelFormats.Bgra32,
            null,
            Bgra,
            Stride);
        source.Freeze();
        return source;
    }

    public static PixelBuffer FromBitmapSource(BitmapSource source)
    {
        BitmapSource converted = source;
        if (source.Format != PixelFormats.Bgra32)
        {
            converted = new FormatConvertedBitmap(source, PixelFormats.Bgra32, null, 0);
            converted.Freeze();
        }

        int width = converted.PixelWidth;
        int height = converted.PixelHeight;
        int stride = width * 4;
        var pixels = new byte[stride * height];
        converted.CopyPixels(new Int32Rect(0, 0, width, height), pixels, stride, 0);
        return new PixelBuffer(width, height, pixels, stride);
    }
}
