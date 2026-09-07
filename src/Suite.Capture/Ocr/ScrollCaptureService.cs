using Suite.Capture.Native;

namespace Suite.Capture.Ocr;

/// <summary>
/// Best-effort vertical scroll stitch for one HWND.
/// Wheel / VSCROLL + client BitBlt; stop on no-change, idle timeout, or max height.
/// Win11 Chromium / smooth-scroll / composition often fail — see OCR-SCROLL-SPEC.
/// </summary>
public sealed class ScrollCaptureService : IScrollCaptureService
{
    public const string BadHwnd = "没找到可滚动的窗口。";
    public const string CaptureFailed = "这个窗口暂时滚不动长图。";
    public const string Cancelled = "";
    public const string TooShort = "这个窗口暂时滚不动长图。";
    public const string Copied = "已复制长图。";
    public const string CopiedAndSaved = "已复制长图并保存。";
    public const string Busy = "正在拼接长图…";
    public const string Timeout = "滚动超时，没有截到内容。";
    public const string NotEnabled = "滚动长截图尚未在本机启用。";

    public async Task<ScrollCaptureResult> CaptureAsync(
        IntPtr hwnd,
        ScrollCaptureOptions options,
        CancellationToken cancellationToken = default)
    {
        if (hwnd == IntPtr.Zero || !CaptureNative.IsWindow(hwnd) || !CaptureNative.IsWindowVisible(hwnd))
        {
            return Fail(BadHwnd);
        }

        IntPtr target = CaptureNative.GetAncestor(hwnd, CaptureNative.GaRoot);
        if (target == IntPtr.Zero)
        {
            target = hwnd;
        }

        PixelBuffer? previous = CaptureClient(target);
        if (previous is null || previous.Width < 2 || previous.Height < 2)
        {
            previous?.ReleasePixels();
            return Fail(CaptureFailed);
        }

        int firstHeight = previous.Height;
        int maxH = Math.Max(firstHeight, options.MaxHeightPx);
        PixelBuffer stitch = previous.Clone();
        int stitchedH = firstHeight;
        var started = DateTime.UtcNow;
        int unchangedRounds = 0;
        const int UnchangedStop = 2;
        PixelBuffer? result = null;

        try
        {
            while (stitchedH < maxH)
            {
                cancellationToken.ThrowIfCancellationRequested();
                if ((DateTime.UtcNow - started).TotalMilliseconds > Math.Max(options.IdleStopMs * 40, 12_000))
                {
                    break;
                }

                ScrollStep(target);
                await Task.Delay(Math.Max(40, options.StepDelayMs), cancellationToken).ConfigureAwait(false);

                PixelBuffer? next = CaptureClient(target);
                if (next is null)
                {
                    break;
                }

                if (next.Width != previous.Width || BuffersVisuallyEqual(previous, next))
                {
                    next.ReleasePixels();
                    unchangedRounds++;
                    if (unchangedRounds >= UnchangedStop)
                    {
                        break;
                    }

                    continue;
                }

                unchangedRounds = 0;
                int overlap = EstimateOverlap(previous, next);
                int append = Math.Max(0, next.Height - overlap);
                if (append <= 0)
                {
                    next.ReleasePixels();
                    unchangedRounds++;
                    if (unchangedRounds >= UnchangedStop)
                    {
                        break;
                    }

                    continue;
                }

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

                previous.ReleasePixels();
                previous = next;

                if (stitchedH >= maxH)
                {
                    break;
                }
            }

            if (stitchedH <= firstHeight + 2)
            {
                return Fail(TooShort);
            }

            // Hand ownership of stitch to caller; replace local with a tiny buffer for finally cleanup.
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

    public static IntPtr ResolveForegroundHwnd()
    {
        IntPtr hwnd = CaptureNative.GetForegroundWindow();
        if (hwnd == IntPtr.Zero)
        {
            return IntPtr.Zero;
        }

        IntPtr root = CaptureNative.GetAncestor(hwnd, CaptureNative.GaRoot);
        return root == IntPtr.Zero ? hwnd : root;
    }

    private static ScrollCaptureResult Fail(string error) => new()
    {
        Succeeded = false,
        Error = error,
    };

    private static void ScrollStep(IntPtr hwnd)
    {
        int delta = -120 * 3;
        IntPtr wParam = new IntPtr((delta << 16) & unchecked((int)0xFFFF0000));
        CaptureNative.SendMessage(hwnd, CaptureNative.WmMouseWheel, wParam, IntPtr.Zero);
        CaptureNative.SendMessage(hwnd, CaptureNative.WmVscroll, new IntPtr(CaptureNative.SbPagedown), IntPtr.Zero);
    }

    private static PixelBuffer? CaptureClient(IntPtr hwnd)
    {
        if (!CaptureNative.GetClientRect(hwnd, out Rect client)
            || client.Right - client.Left < 2
            || client.Bottom - client.Top < 2)
        {
            return null;
        }

        int width = client.Right - client.Left;
        int height = client.Bottom - client.Top;
        var pt = new POINT(0, 0);
        if (!CaptureNative.ClientToScreen(hwnd, ref pt))
        {
            return null;
        }

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
                    hdcMem, 0, 0, width, height, hdcScreen, pt.X, pt.Y, CaptureNative.Srccopy))
            {
                CaptureNative.PrintWindow(
                    hwnd, hdcMem, CaptureNative.PwClientOnly | CaptureNative.PwRenderFullContent);
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

    private static int EstimateOverlap(PixelBuffer previous, PixelBuffer next)
    {
        int w = Math.Min(previous.Width, next.Width);
        int max = Math.Min(previous.Height, next.Height);
        for (int overlap = max; overlap >= Math.Max(8, max / 8); overlap -= Math.Max(1, max / 32))
        {
            if (RowsMatch(previous, previous.Height - overlap, next, 0, overlap, w))
            {
                return overlap;
            }
        }

        return Math.Min(max / 4, next.Height / 4);
    }

    private static bool RowsMatch(
        PixelBuffer a, int aY, PixelBuffer b, int bY, int rows, int width)
    {
        int bytes = width * 4;
        int yStep = Math.Max(1, rows / 16);
        int xStep = Math.Max(4, bytes / 64);
        for (int y = 0; y < rows; y += yStep)
        {
            int ai = (aY + y) * a.Stride;
            int bi = (bY + y) * b.Stride;
            for (int x = 0; x < bytes; x += xStep)
            {
                if (a.Bgra[ai + x] != b.Bgra[bi + x])
                {
                    return false;
                }
            }
        }

        return true;
    }
}
