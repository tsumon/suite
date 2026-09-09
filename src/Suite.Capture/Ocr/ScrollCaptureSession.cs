using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Shapes;
using System.Windows.Threading;
using Suite.Capture.Grabbers;
using Suite.Capture.Native;

namespace Suite.Capture.Ocr;

/// <summary>
/// Interactive scroll long-shot: Pick (window / drag rect) → Recording (user scrolls; idle finish).
/// </summary>
public sealed class ScrollCaptureSession : IDisposable
{
    private readonly Action<string> _report;
    private readonly IScrollCaptureService _capture;
    private readonly ScrollCaptureOptions _options;
    private readonly Action<ScrollCaptureResult> _completed;
    private readonly Dispatcher _dispatcher;
    private RegionSelectSession? _select;
    private IReadOnlyList<MonitorCapture>? _pickFrames;
    private ScrollRecordChrome? _chrome;
    private CancellationTokenSource? _cts;
    private DispatcherTimer? _escPoll;
    private bool _done;
    private bool _disposed;

    public ScrollCaptureSession(
        Action<string> report,
        IScrollCaptureService capture,
        ScrollCaptureOptions options,
        Action<ScrollCaptureResult> completed,
        Dispatcher? dispatcher = null)
    {
        _report = report;
        _capture = capture;
        _options = options;
        _completed = completed;
        _dispatcher = dispatcher ?? Application.Current.Dispatcher;
    }

    public void Begin()
    {
        if (_disposed || _done)
        {
            return;
        }

        _cts = new CancellationTokenSource();
        Task.Run(() =>
        {
            IReadOnlyList<MonitorCapture> frames;
            try
            {
                frames = ScreenCapturePipeline.CaptureAll();
            }
            catch
            {
                _dispatcher.Invoke(() => Finish(ScrollCaptureResult.Fail(ScrollCaptureService.CaptureFailed)));
                return;
            }

            _dispatcher.Invoke(() => ShowPick(frames));
        });
    }

    public void Cancel()
    {
        if (_done)
        {
            return;
        }

        try
        {
            _cts?.Cancel();
        }
        catch
        {
        }

        _dispatcher.Invoke(() =>
        {
            TearDownPick();
            TearDownChrome();
            Finish(new ScrollCaptureResult { Succeeded = false, Error = null });
        });
    }

    private void ShowPick(IReadOnlyList<MonitorCapture> frames)
    {
        if (_done || _disposed)
        {
            ReleaseFrames(frames);
            return;
        }

        _pickFrames = frames;
        var select = new RegionSelectSession
        {
            ShowMagnifier = false,
            OverlayHintText = ScrollCaptureService.PickHint,
        };
        _select = select;
        select.Cancelled += () =>
        {
            if (!ReferenceEquals(_select, select))
            {
                return;
            }

            TearDownPick();
            Finish(new ScrollCaptureResult { Succeeded = false, Error = null });
        };
        select.Completed += (rect, _) =>
        {
            if (!ReferenceEquals(_select, select))
            {
                return;
            }

            if (rect.IsEmpty || rect.Width < CaptureUx.MinCommitPx || rect.Height < CaptureUx.MinCommitPx)
            {
                TearDownPick();
                Finish(ScrollCaptureResult.Fail(ScrollCaptureService.BadRegion));
                return;
            }

            TearDownPick();
            StartRecording(rect);
        };
        select.Show(frames);
    }

    private void StartRecording(PixelRect rect)
    {
        if (_done || _disposed)
        {
            return;
        }

        _report(ScrollCaptureService.ScrollHint);
        // Outline stays on full rect; BitBlt uses inset so DPI/pad never bleeds red into pixels.
        const int captureInset = 4;
        PixelRect captureRect = InsetRect(rect, captureInset);
        if (captureRect.Width < CaptureUx.MinCommitPx || captureRect.Height < CaptureUx.MinCommitPx)
        {
            captureRect = rect;
        }

        _chrome = new ScrollRecordChrome(rect);
        _chrome.Show();
        StartEscPoll();

        CancellationToken token = _cts?.Token ?? CancellationToken.None;
        _ = Task.Run(async () =>
        {
            ScrollCaptureResult result;
            try
            {
                // Brief pause so pick overlays / frozen frames are gone before first BitBlt.
                await Task.Delay(120, token).ConfigureAwait(false);
                result = await _capture.CaptureAsync(captureRect, _options, token).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                result = new ScrollCaptureResult { Succeeded = false, Error = null };
            }
            catch
            {
                result = ScrollCaptureResult.Fail(ScrollCaptureService.CaptureFailed);
            }

            await _dispatcher.InvokeAsync(() =>
            {
                TearDownChrome();
                if (result.Succeeded)
                {
                    _report(ScrollCaptureService.Busy);
                }

                Finish(result);
            });
        }, token);
    }

    private void StartEscPoll()
    {
        StopEscPoll();
        _escPoll = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(50) };
        _escPoll.Tick += (_, _) =>
        {
            if (_done)
            {
                return;
            }

            short state = CaptureNative.GetAsyncKeyState(CaptureNative.VkEscape);
            if ((state & 0x8000) != 0)
            {
                try
                {
                    _cts?.Cancel();
                }
                catch
                {
                }

                TearDownChrome();
                Finish(new ScrollCaptureResult { Succeeded = false, Error = null });
            }
        };
        _escPoll.Start();
    }

    private void StopEscPoll()
    {
        if (_escPoll is null)
        {
            return;
        }

        _escPoll.Stop();
        _escPoll = null;
    }

    private void TearDownPick()
    {
        _select?.CloseOverlays();
        _select = null;
        if (_pickFrames is not null)
        {
            ReleaseFrames(_pickFrames);
            _pickFrames = null;
        }
    }

    private void TearDownChrome()
    {
        StopEscPoll();
        try
        {
            _chrome?.Close();
        }
        catch
        {
        }

        _chrome = null;
    }

    private void Finish(ScrollCaptureResult result)
    {
        if (_done)
        {
            result.Image?.ReleasePixels();
            return;
        }

        _done = true;
        TearDownPick();
        TearDownChrome();
        _completed(result);
    }


    private static PixelRect InsetRect(PixelRect rect, int inset)
    {
        if (inset <= 0)
        {
            return rect;
        }

        int w = rect.Width - (inset * 2);
        int h = rect.Height - (inset * 2);
        if (w < 2 || h < 2)
        {
            return rect;
        }

        return new PixelRect(rect.X + inset, rect.Y + inset, w, h);
    }

    private static void ReleaseFrames(IReadOnlyList<MonitorCapture> frames)
    {
        foreach (MonitorCapture frame in frames)
        {
            try
            {
                frame.Buffer.ReleasePixels();
            }
            catch
            {
            }
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Cancel();
        _cts?.Dispose();
        _cts = null;
    }
}

/// <summary>
/// Click-through outline around the capture rect (border outside content so BitBlt stays clean).
/// </summary>
internal sealed class ScrollRecordChrome : Window
{
    private static readonly SolidColorBrush Accent = Freeze(Color.FromRgb(0xDC, 0x28, 0x28));

    public ScrollRecordChrome(PixelRect screenRect)
    {
        const int pad = 3;
        int x = screenRect.X - pad;
        int y = screenRect.Y - pad;
        int w = screenRect.Width + (pad * 2);
        int h = screenRect.Height + (pad * 2);

        double dpiX = 96;
        double dpiY = 96;
        try
        {
            foreach (MonitorInfo mon in MonitorEnumerator.GetAll())
            {
                if (mon.Bounds.Contains(screenRect.X + (screenRect.Width / 2), screenRect.Y + (screenRect.Height / 2)))
                {
                    dpiX = Math.Max(1, mon.DpiX);
                    dpiY = Math.Max(1, mon.DpiY);
                    break;
                }
            }
        }
        catch
        {
        }

        double dipX = 96.0 / dpiX;
        double dipY = 96.0 / dpiY;

        WindowStyle = WindowStyle.None;
        ResizeMode = ResizeMode.NoResize;
        ShowInTaskbar = false;
        Topmost = true;
        AllowsTransparency = true;
        Background = Brushes.Transparent;
        Focusable = false;
        IsHitTestVisible = false;
        Title = "Suite 滚动长截图";

        Left = x * dipX;
        Top = y * dipY;
        Width = Math.Max(1, w * dipX);
        Height = Math.Max(1, h * dipY);

        Content = new Rectangle
        {
            Stroke = Accent,
            StrokeThickness = 3,
            Fill = Brushes.Transparent,
            IsHitTestVisible = false,
        };

        SourceInitialized += (_, _) =>
        {
            IntPtr hwnd = new WindowInteropHelper(this).Handle;
            if (hwnd == IntPtr.Zero)
            {
                return;
            }

            long style = CaptureNative.GetWindowLongPtr(hwnd, CaptureNative.GwlpExstyle).ToInt64();
            CaptureNative.SetWindowLongPtr(
                hwnd,
                CaptureNative.GwlpExstyle,
                new IntPtr(style | CaptureNative.WsExTransparent | CaptureNative.WsExLayered));
        };
    }

    private static SolidColorBrush Freeze(Color c)
    {
        var b = new SolidColorBrush(c);
        b.Freeze();
        return b;
    }
}
