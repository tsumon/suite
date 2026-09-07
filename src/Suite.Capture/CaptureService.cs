using System.IO;
using System.Windows;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Suite.Capture.Grabbers;
using Suite.Capture.Ocr;
using Suite.Contracts;

namespace Suite.Capture;

public sealed class CaptureService : IDisposable
{
    private readonly Dispatcher _dispatcher;
    private readonly IOcrService _ocr;
    private readonly Action<string>? _reportStatus;
    private RegionSelectSession? _select;
    private AnnotationWindow? _annotation;
    private IReadOnlyList<MonitorCapture>? _activeFrames;
    private bool _busy;
    private bool _disposed;

    public CaptureService(Dispatcher dispatcher, IOcrService? ocr = null, Action<string>? reportStatus = null)
    {
        _dispatcher = dispatcher;
        _ocr = ocr ?? new NullOcrService();
        _reportStatus = reportStatus;
    }

    public bool IsBusy => _busy;

    public void Begin(CaptureRequest request, Action<CaptureResult> completed)
    {
        if (_disposed)
        {
            completed(new CaptureResult { Error = "Capture service disposed." });
            return;
        }

        if (_busy)
        {
            return;
        }

        _busy = true;
        Task.Run(() =>
        {
            try
            {
                IReadOnlyList<MonitorCapture> frames = ScreenCapturePipeline.CaptureAll();
                _dispatcher.Invoke(() =>
                {
                    if (_disposed)
                    {
                        ReleaseFrames(frames);
                        return;
                    }

                    _activeFrames = frames;
                    ShowSelect(frames, request, completed);
                });
            }
            catch (Exception ex)
            {
                _dispatcher.Invoke(() =>
                {
                    if (_disposed)
                    {
                        return;
                    }

                    _busy = false;
                    completed(new CaptureResult { Error = "无法截屏。" + ex.Message });
                    AppendLog("fail " + ex.Message);
                });
            }
        });
    }

    public void Cancel()
    {
        if (_annotation is not null)
        {
            _annotation.Close();
            return;
        }

        _select?.Cancel();
        _select?.CloseOverlays();
        _select = null;
        ReleaseActiveFrames();
        _busy = false;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Cancel();
    }

    private void ShowSelect(IReadOnlyList<MonitorCapture> frames, CaptureRequest request, Action<CaptureResult> completed)
    {
        var select = new RegionSelectSession { ShowMagnifier = request.ShowMagnifier };
        _select = select;
        select.Cancelled += () => _ = _dispatcher.InvokeAsync(() =>
        {
            if (!ReferenceEquals(_select, select))
            {
                return;
            }

            select.CloseOverlays();
            _select = null;
            ReleaseActiveFrames();
            _busy = false;
            completed(new CaptureResult { Cancelled = true });
            AppendLog("cancel");
            GC.Collect(2, GCCollectionMode.Optimized);
        });
        select.Completed += (rect, annotate) =>
        {
            void Handle()
            {
                if (!ReferenceEquals(_select, select))
                {
                    return;
                }

                if (CaptureUx.OpenAnnotationAfterRegion(request.PinAfterCapture, annotate))
                {
                    select.EnterAnnotate(rect);
                    // Close select overlays BEFORE annotation so they cannot steal
                    // Topmost / mouse and cover the toolbar (transparent click-through).
                    select.CloseOverlays();
                    ShowAnnotation(frames, rect, request, completed, select);
                    return;
                }

                select.CloseOverlays();
                _select = null;
                CommitRegion(frames, rect, request, pin: true, completed);
            }

            if (_dispatcher.CheckAccess())
            {
                Handle();
            }
            else
            {
                _dispatcher.Invoke(Handle);
            }
        };
        select.Show(frames);
    }

    private void ShowAnnotation(
        IReadOnlyList<MonitorCapture> frames,
        PixelRect rect,
        CaptureRequest request,
        Action<CaptureResult> completed,
        RegionSelectSession select)
    {
        CaptureMethod method = RegionCropper.DominantMethod(frames, rect);
        MonitorInfo monitor = ResolveMonitor(frames, rect);

        var window = new AnnotationWindow(frames, rect, monitor, _ocr, _reportStatus);
        _annotation = window;
        // Overlays already closed; keep handler no-op-safe for resize bookkeeping.
        window.RegionChanged += updated => select.UpdateCommitted(updated);
        window.SaveClicked += () =>
        {
            try
            {
                PngFileSaver.Save(window.Peek(), request.SaveDirectory);
            }
            catch (Exception ex)
            {
                completed(new CaptureResult
                {
                    Error = "无法保存文件：" + ex.Message,
                    Method = method,
                    Selection = rect,
                    DpiX = monitor.DpiX,
                    DpiY = monitor.DpiY,
                });
            }
        };
        window.Closed += (_, _) =>
        {
            _annotation = null;
            select.CloseOverlays();
            if (ReferenceEquals(_select, select))
            {
                _select = null;
            }

            PixelBuffer? committed = window.Committed;
            bool accepted = window.Accepted;
            bool pin = window.PinRequested;
            PixelRect selection = window.Selection;
            // Full-screen frames no longer needed after annotate ends.
            window.ReleaseSessionBuffers();
            if (ReferenceEquals(_activeFrames, frames))
            {
                _activeFrames = null;
            }

            if (!accepted || committed is null)
            {
                committed?.ReleasePixels();
                _busy = false;
                completed(new CaptureResult { Cancelled = true, Method = method, Selection = selection, DpiX = monitor.DpiX, DpiY = monitor.DpiY });
                AppendLog("cancel-annotate " + method);
                GC.Collect(2, GCCollectionMode.Optimized);
                return;
            }

            Finish(committed, method, request, pin, selection, monitor.DpiX, monitor.DpiY, completed);
        };
        window.Show();
        window.Activate();
        window.Focus();
        System.Windows.Input.Keyboard.Focus(window);
    }

    private void CommitRegion(
        IReadOnlyList<MonitorCapture> frames,
        PixelRect rect,
        CaptureRequest request,
        bool pin,
        Action<CaptureResult> completed)
    {
        PixelBuffer crop = RegionCropper.Crop(frames, rect);
        CaptureMethod method = RegionCropper.DominantMethod(frames, rect);
        MonitorInfo monitor = ResolveMonitor(frames, rect);
        if (ReferenceEquals(_activeFrames, frames))
        {
            ReleaseActiveFrames();
        }
        else
        {
            ReleaseFrames(frames);
        }

        Finish(crop, method, request, pin, rect, monitor.DpiX, monitor.DpiY, completed);
    }

    private void Finish(
        PixelBuffer image,
        CaptureMethod method,
        CaptureRequest request,
        bool pin,
        PixelRect selection,
        uint dpiX,
        uint dpiY,
        Action<CaptureResult> completed)
    {
        string? saved = null;
        string? error = null;
        try
        {
            ImageClipboard.Copy(image);
        }
        catch (Exception)
        {
            error = "无法写入剪贴板。";
        }

        if (request.SaveFile)
        {
            try
            {
                saved = PngFileSaver.Save(image, request.SaveDirectory);
            }
            catch (Exception ex)
            {
                error = (error is null ? "" : error + " ") + "无法保存文件：" + ex.Message;
            }
        }

        BitmapSource? bitmap = null;
        try
        {
            bitmap = image.ToBitmapSource();
        }
        finally
        {
            image.ReleasePixels();
        }

        _busy = false;
        completed(new CaptureResult
        {
            Succeeded = error is null,
            PinRequested = pin,
            Error = error,
            Image = bitmap,
            SavedFilePath = saved,
            Method = method,
            Selection = selection,
            DpiX = dpiX == 0 ? 96u : dpiX,
            DpiY = dpiY == 0 ? 96u : dpiY,
        });
        AppendLog((error is null ? "ok " : "partial ") + method + (saved is null ? "" : " " + saved));
        // Deterministic ReleasePixels already dropped BGRA; one gen-2 collect helps return WorkingSet after large monitor frames.
        // Prefer ReleasePixels; this is a documented nudge only after a capture session ends.
        GC.Collect(2, GCCollectionMode.Optimized);
    }

    private void ReleaseActiveFrames()
    {
        if (_activeFrames is null)
        {
            return;
        }

        ReleaseFrames(_activeFrames);
        _activeFrames = null;
    }

    private static void ReleaseFrames(IReadOnlyList<MonitorCapture> frames)
    {
        foreach (MonitorCapture frame in frames)
        {
            frame.Buffer.ReleasePixels();
        }
    }

    private static MonitorInfo ResolveMonitor(IReadOnlyList<MonitorCapture> frames, PixelRect rect)
    {
        if (frames.Count == 0)
        {
            throw new ArgumentException("frames must not be empty.", nameof(frames));
        }

        var bounds = new PixelRect[frames.Count];
        for (int i = 0; i < frames.Count; i++)
        {
            bounds[i] = frames[i].Monitor.Bounds;
        }

        int index = CaptureUx.IndexOfLargestOverlap(bounds, rect);
        return index >= 0 ? frames[index].Monitor : frames[0].Monitor;
    }

    private static void AppendLog(string line)
    {
        try
        {
            Directory.CreateDirectory(SettingsPaths.DirectoryPath);
            File.AppendAllText(
                SettingsPaths.CaptureLogPath,
                DateTime.Now.ToString("s") + " " + line + Environment.NewLine);
        }
        catch
        {
        }
    }
}
