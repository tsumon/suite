using Suite.Capture.Native;

namespace Suite.Capture.Ocr;

/// <summary>
/// User-driven vertical scroll stitch for a screen rect.
/// Suite never injects WM_MOUSEWHEEL / VSCROLL — Joe scrolls; we BitBlt + stitch on change,
/// then idle-stop (IdleStopMs). See OCR-SCROLL-SPEC.
/// </summary>
public sealed class ScrollCaptureService : IScrollCaptureService
{
    public const string BadHwnd = "没找到可滚动的窗口。";
    public const string BadRegion = "没找到可滚动的窗口。";
    public const string CaptureFailed = "这个窗口暂时滚不动长图。";
    public const string Cancelled = "";
    public const string TooShort = "这个窗口暂时滚不动长图。";
    public const string Copied = "已复制长图。";
    public const string CopiedAndSaved = "已复制长图并保存。";
    public const string Busy = "正在拼接长图…";
    public const string Timeout = "滚动超时，没有截到内容。";
    public const string NotEnabled = "滚动长截图尚未在本机启用。";
    public const string PickHint = "点选窗口或拖选区域；Esc 取消。";
    public const string ScrollHint = "在选区内滚动；停滚约 1.5 秒后自动完成";

    public async Task<ScrollCaptureResult> CaptureAsync(
        PixelRect screenRect,
        ScrollCaptureOptions options,
        CancellationToken cancellationToken = default)
    {
        if (screenRect.IsEmpty || screenRect.Width < 2 || screenRect.Height < 2)
        {
            return Fail(BadRegion);
        }

        int idleMs = Math.Max(200, options.IdleStopMs);
        int sampleMs = Math.Clamp(options.SampleIntervalMs, 80, 1000);
        int maxH = Math.Max(screenRect.Height, options.MaxHeightPx);
        int maxDuration = Math.Max(idleMs * 20, options.MaxDurationMs);

        PixelBuffer? previous = CaptureScreen(screenRect);
        if (previous is null || previous.Width < 2 || previous.Height < 2)
        {
            previous?.ReleasePixels();
            return Fail(CaptureFailed);
        }

        int firstHeight = previous.Height;
        PixelBuffer stitch = previous.Clone();
        int stitchedH = firstHeight;
        var started = DateTime.UtcNow;
        var lastChangeUtc = started;
        bool grew = false;
        int consecutiveRejects = 0;
        const int maxConsecutiveRejects = 40;
        PixelBuffer? result = null;

        try
        {
            while (stitchedH < maxH)
            {
                cancellationToken.ThrowIfCancellationRequested();
                if ((DateTime.UtcNow - started).TotalMilliseconds > maxDuration)
                {
                    break;
                }

                await Task.Delay(sampleMs, cancellationToken).ConfigureAwait(false);
                cancellationToken.ThrowIfCancellationRequested();

                PixelBuffer? next = CaptureScreen(screenRect);
                if (next is null)
                {
                    break;
                }

                if (next.Width != previous.Width || next.Height != previous.Height
                    || BuffersVisuallyEqual(previous, next))
                {
                    next.ReleasePixels();
                    double idleFor = (DateTime.UtcNow - lastChangeUtc).TotalMilliseconds;
                    if (grew && idleFor >= idleMs)
                    {
                        break;
                    }

                    if (!grew
                        && (DateTime.UtcNow - started).TotalMilliseconds
                            >= Math.Max(idleMs * 4, options.MaxWaitFirstChangeMs))
                    {
                        break;
                    }

                    continue;
                }

                int overlap = EstimateOverlap(previous, next);
                int append = Math.Max(0, next.Height - overlap);
                if (append <= 0)
                {
                    // Changed but no confident scroll delta — update previous for idle; do not grow.
                    consecutiveRejects++;
                    previous.ReleasePixels();
                    previous = next;
                    lastChangeUtc = DateTime.UtcNow;
                    if (consecutiveRejects >= maxConsecutiveRejects && grew)
                    {
                        // Cap transitional churn; idle path still finishes via equal frames.
                        break;
                    }

                    continue;
                }

                consecutiveRejects = 0;

                int room = maxH - stitchedH;
                if (append > room)
                {
                    append = room;
                }

                if (append <= 0)
                {
                    next.ReleasePixels();
                    break;
                }

                int newH = stitchedH + append;
                PixelBuffer grown = PixelBuffer.Allocate(stitch.Width, newH);
                grown.BlitFrom(stitch, new PixelRect(0, 0, stitch.Width, stitchedH), 0, 0);
                grown.BlitFrom(
                    next,
                    new PixelRect(0, next.Height - append, next.Width, append),
                    0,
                    stitchedH);
                stitch.ReleasePixels();
                stitch = grown;
                stitchedH = newH;
                grew = stitchedH > firstHeight + 2;
                lastChangeUtc = DateTime.UtcNow;

                previous.ReleasePixels();
                previous = next;

                if (stitchedH >= maxH)
                {
                    break;
                }

                // After a growth, keep sampling until idle; do not stop on the growth frame itself.
            }

            // Final idle wait: if we grew and then hit max duration mid-scroll, still OK if taller.
            if (stitchedH <= firstHeight + 2)
            {
                return Fail(
                    (DateTime.UtcNow - started).TotalMilliseconds >= maxDuration
                        ? Timeout
                        : TooShort);
            }

            result = stitch;
            stitch = PixelBuffer.Allocate(1, 1);
            return new ScrollCaptureResult { Succeeded = true, Image = result };
        }
        catch (OperationCanceledException)
        {
            return new ScrollCaptureResult { Succeeded = false, Error = null };
        }
        catch (Exception)
        {
            return Fail(CaptureFailed);
        }
        finally
        {
            previous?.ReleasePixels();
            stitch?.ReleasePixels();
        }
    }

    private static ScrollCaptureResult Fail(string error) => new()
    {
        Succeeded = false,
        Error = error,
    };

    internal static PixelBuffer? CaptureScreen(PixelRect screenRect)
    {
        if (screenRect.IsEmpty || screenRect.Width < 1 || screenRect.Height < 1)
        {
            return null;
        }

        int width = screenRect.Width;
        int height = screenRect.Height;
        IntPtr hdcScreen = CaptureNative.GetDC(IntPtr.Zero);
        if (hdcScreen == IntPtr.Zero)
        {
            return null;
        }

        IntPtr hdcMem = IntPtr.Zero;
        IntPtr hbmp = IntPtr.Zero;
        IntPtr old = IntPtr.Zero;
        try
        {
            hdcMem = CaptureNative.CreateCompatibleDC(hdcScreen);
            hbmp = CaptureNative.CreateCompatibleBitmap(hdcScreen, width, height);
            if (hdcMem == IntPtr.Zero || hbmp == IntPtr.Zero)
            {
                return null;
            }

            old = CaptureNative.SelectObject(hdcMem, hbmp);
            if (!CaptureNative.BitBlt(
                    hdcMem, 0, 0, width, height, hdcScreen, screenRect.X, screenRect.Y, CaptureNative.Srccopy))
            {
                return null;
            }

            var info = new BitmapInfo
            {
                BmiHeader = new BitmapInfoHeader
                {
                    BiSize = (uint)System.Runtime.InteropServices.Marshal.SizeOf<BitmapInfoHeader>(),
                    BiWidth = width,
                    BiHeight = -height,
                    BiPlanes = 1,
                    BiBitCount = 32,
                    BiCompression = CaptureNative.BiRgb,
                },
            };

            var pixels = new byte[width * 4 * height];
            int lines = CaptureNative.GetDIBits(
                hdcMem, hbmp, 0, (uint)height, pixels, ref info, CaptureNative.DibRgbColors);
            if (lines == 0)
            {
                return null;
            }

            for (int i = 3; i < pixels.Length; i += 4)
            {
                pixels[i] = 255;
            }

            return new PixelBuffer(width, height, pixels, width * 4);
        }
        finally
        {
            if (old != IntPtr.Zero && hdcMem != IntPtr.Zero)
            {
                CaptureNative.SelectObject(hdcMem, old);
            }

            if (hbmp != IntPtr.Zero)
            {
                CaptureNative.DeleteObject(hbmp);
            }

            if (hdcMem != IntPtr.Zero)
            {
                CaptureNative.DeleteDC(hdcMem);
            }

            CaptureNative.ReleaseDC(IntPtr.Zero, hdcScreen);
        }
    }

    private static bool BuffersVisuallyEqual(PixelBuffer a, PixelBuffer b)
    {
        if (a.Width != b.Width || a.Height != b.Height)
        {
            return false;
        }

        int len = Math.Min(a.Bgra.Length, b.Bgra.Length);
        int step = Math.Max(4, (len / 4096) & ~3);
        if (step < 4)
        {
            step = 4;
        }

        for (int i = 0; i < len; i += step)
        {
            if (a.Bgra[i] != b.Bgra[i])
            {
                return false;
            }

            if (i + 2 < len && (a.Bgra[i + 1] != b.Bgra[i + 1] || a.Bgra[i + 2] != b.Bgra[i + 2]))
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Overlap rows between bottom of <paramref name="previous"/> and top of <paramref name="next"/>.
    /// On no confident match returns <c>next.Height</c> so append = 0 — never guesses max/4.
    /// </summary>
    internal static int EstimateOverlap(PixelBuffer previous, PixelBuffer next)
    {
        int w = Math.Min(previous.Width, next.Width);
        int h = Math.Min(previous.Height, next.Height);
        if (w < 2 || h < 12)
        {
            return next.Height;
        }

        const int minAppend = 4;
        // Mean abs BGR sample must beat this to accept a candidate.
        const double passMeanAbs = 12.0;

        int fingerprintH = Math.Clamp(h / 4, 24, Math.Min(80, h / 3));
        if (fingerprintH > h - minAppend)
        {
            fingerprintH = Math.Max(8, h - minAppend);
        }

        int maxMatchY = h - fingerprintH; // y in next where strip may start
        if (maxMatchY < 0)
        {
            return next.Height;
        }

        // Strip = last fingerprintH rows of previous. Content scrolled down by D matches at y = h - fingerprintH - D.
        int stripY = previous.Height - fingerprintH;
        int bestY = -1;
        double bestScore = double.MaxValue;

        // Step-1 Y search (strip SAD is already subsampled). Coarse steps can skip the true D.
        for (int y = 0; y <= maxMatchY; y++)
        {
            double score = MeanAbsDiffStrip(previous, stripY, next, y, fingerprintH, w);
            if (score < bestScore)
            {
                bestScore = score;
                bestY = y;
            }
        }

        if (bestY < 0)
        {
            return next.Height;
        }

        // D = how many new rows at bottom of next.
        int delta = (h - fingerprintH) - bestY;
        if (delta < minAppend || bestScore > passMeanAbs)
        {
            return next.Height; // reject → append 0
        }

        // Optional: verify a taller overlap band at this delta (reduces false positives).
        int overlap = h - delta;
        double verify = MeanAbsDiffStrip(previous, previous.Height - overlap, next, 0, overlap, w);
        if (verify > passMeanAbs * 1.25)
        {
            return next.Height;
        }

        return overlap;
    }

    /// <summary>Subsampled mean abs diff on B,G,R (skip alpha). Lower is better.</summary>
    private static double MeanAbsDiffStrip(
        PixelBuffer a, int aY, PixelBuffer b, int bY, int rows, int width)
    {
        if (rows <= 0 || width <= 0)
        {
            return double.MaxValue;
        }

        int yStep = Math.Max(1, rows / 24);
        int xStep = Math.Max(1, width / 48);
        long sum = 0;
        int samples = 0;

        for (int y = 0; y < rows; y += yStep)
        {
            int ai = (aY + y) * a.Stride;
            int bi = (bY + y) * b.Stride;
            for (int x = 0; x < width; x += xStep)
            {
                int ao = ai + (x * 4);
                int bo = bi + (x * 4);
                sum += Math.Abs(a.Bgra[ao] - b.Bgra[bo]);
                sum += Math.Abs(a.Bgra[ao + 1] - b.Bgra[bo + 1]);
                sum += Math.Abs(a.Bgra[ao + 2] - b.Bgra[bo + 2]);
                samples += 3;
            }
        }

        return samples == 0 ? double.MaxValue : (double)sum / samples;
    }
}
