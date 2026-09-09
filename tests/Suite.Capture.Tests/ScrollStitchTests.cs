using Suite.Capture.Ocr;

namespace Suite.Capture.Tests;

/// <summary>Synthetic stitch overlap — red on bad max/4 fallback, green after fingerprint reject.</summary>
public sealed class ScrollStitchTests
{
    [Fact]
    public void EstimateOverlap_scrolled_by_known_delta_returns_height_minus_delta()
    {
        const int w = 48;
        const int h = 120;
        const int delta = 40;

        PixelBuffer previous = MakeFingerprinted(w, h, seed: 0);
        PixelBuffer next = PixelBuffer.Allocate(w, h);
        // next[0..h-delta) = previous[delta..h); bottom delta rows are new content
        for (int y = 0; y < h - delta; y++)
        {
            CopyRow(previous, y + delta, next, y, w);
        }

        for (int y = h - delta; y < h; y++)
        {
            FillRow(next, y, w, RowColor(10_000 + y));
        }

        int overlap = ScrollCaptureService.EstimateOverlap(previous, next);
        int append = next.Height - overlap;

        Assert.InRange(overlap, h - delta - 2, h - delta + 2);
        Assert.InRange(append, delta - 2, delta + 2);

        previous.ReleasePixels();
        next.ReleasePixels();
    }

    [Fact]
    public void EstimateOverlap_unrelated_frames_rejects_without_large_default_append()
    {
        const int w = 48;
        const int h = 120;

        PixelBuffer previous = MakeFingerprinted(w, h, seed: 0);
        PixelBuffer next = MakeFingerprinted(w, h, seed: 9_001);

        int overlap = ScrollCaptureService.EstimateOverlap(previous, next);
        int append = Math.Max(0, next.Height - overlap);

        // Must not fall back to ~max/4 overlap (append ≈ 0.75*H). Reject ⇒ append 0.
        Assert.True(append <= 4, $"Expected reject/append≈0, got append={append}, overlap={overlap}");

        previous.ReleasePixels();
        next.ReleasePixels();
    }

    [Fact]
    public void EstimateOverlap_tiny_delta_below_min_append_is_rejected()
    {
        const int w = 32;
        const int h = 80;
        const int delta = 2; // noise / sub-min append

        PixelBuffer previous = MakeFingerprinted(w, h, seed: 3);
        PixelBuffer next = PixelBuffer.Allocate(w, h);
        for (int y = 0; y < h - delta; y++)
        {
            CopyRow(previous, y + delta, next, y, w);
        }

        for (int y = h - delta; y < h; y++)
        {
            FillRow(next, y, w, RowColor(20_000 + y));
        }

        int overlap = ScrollCaptureService.EstimateOverlap(previous, next);
        int append = Math.Max(0, next.Height - overlap);
        Assert.True(append <= 4, $"Tiny delta should not append; got append={append}");

        previous.ReleasePixels();
        next.ReleasePixels();
    }

    private static PixelBuffer MakeFingerprinted(int w, int h, int seed)
    {
        PixelBuffer buf = PixelBuffer.Allocate(w, h);
        for (int y = 0; y < h; y++)
        {
            FillRow(buf, y, w, RowColor(seed + (y * 17)));
        }

        return buf;
    }

    private static (byte b, byte g, byte r) RowColor(int key)
    {
        unchecked
        {
            uint x = (uint)(key * 2654435761);
            return ((byte)(x & 0xFF), (byte)((x >> 8) & 0xFF), (byte)((x >> 16) & 0xFF));
        }
    }

    private static void FillRow(PixelBuffer buf, int y, int w, (byte b, byte g, byte r) c)
    {
        int row = y * buf.Stride;
        for (int x = 0; x < w; x++)
        {
            int i = row + (x * 4);
            buf.Bgra[i] = c.b;
            buf.Bgra[i + 1] = c.g;
            buf.Bgra[i + 2] = c.r;
            buf.Bgra[i + 3] = 255;
        }
    }

    private static void CopyRow(PixelBuffer src, int srcY, PixelBuffer dst, int dstY, int w)
    {
        Buffer.BlockCopy(src.Bgra, srcY * src.Stride, dst.Bgra, dstY * dst.Stride, w * 4);
    }
}
