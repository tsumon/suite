using System.Windows.Interop;
using System.Windows.Threading;
using Suite.Capture.Native;

namespace Suite.Capture;

internal sealed class RegionSelectSession
{
    private readonly List<RegionOverlayWindow> _windows = [];
    private readonly List<IntPtr> _overlayHwnds = [];
    private IReadOnlyList<MonitorInfo> _monitors = [];
    private (int X, int Y)? _press;
    private PixelRect? _hoverAtPress;
    private bool _dragging;
    // A modified capture hotkey can leave Shift down; that must not force the toolbar.
    private bool _suppressAnnotateUntilShiftUp;
    private DispatcherTimer? _hoverPoll;
    public bool ShowMagnifier { get; set; } = true;

    /// <summary>Overlay corner hint; null uses CaptureUx.OverlayHint.</summary>
    public string? OverlayHintText { get; set; }

    public PixelRect? Hover { get; private set; }
    public PixelRect? Drag { get; private set; }
    public PixelRect? Committed { get; private set; }
    public bool IsDragging => _dragging;
    public bool IsAnnotating => Committed is PixelRect committed && !committed.IsEmpty;

    public PixelRect? Highlight => Committed ?? (_dragging ? Drag : Hover);

    public event Action? Changed;
    public event Action<PixelRect, bool>? Completed;
    public event Action? Cancelled;

    public void Show(IReadOnlyList<MonitorCapture> frames)
    {
        _monitors = frames.Select(frame => frame.Monitor).ToArray();
        foreach (MonitorCapture frame in frames)
        {
            var window = new RegionOverlayWindow(frame, this, ShowMagnifier);
            _windows.Add(window);
            window.SourceInitialized += (_, _) =>
            {
                IntPtr hwnd = new WindowInteropHelper(window).Handle;
                if (hwnd != IntPtr.Zero && !_overlayHwnds.Contains(hwnd))
                {
                    _overlayHwnds.Add(hwnd);
                }
            };
            window.Show();
            IntPtr shown = new WindowInteropHelper(window).Handle;
            if (shown != IntPtr.Zero && !_overlayHwnds.Contains(shown))
            {
                _overlayHwnds.Add(shown);
            }
        }

        if (_windows.Count > 0)
        {
            _windows[0].Activate();
            _windows[0].Focus();
        }

        // Arm after overlays exist. Cleared on first Shift key-up.
        _suppressAnnotateUntilShiftUp = System.Windows.Input.Keyboard.Modifiers.HasFlag(
            System.Windows.Input.ModifierKeys.Shift);
        StartHoverPoll();
    }

    private void StartHoverPoll()
    {
        StopHoverPoll();
        _hoverPoll = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(50) };
        _hoverPoll.Tick += (_, _) =>
        {
            if (_press is not null || IsAnnotating)
            {
                return;
            }

            if (!CaptureNative.GetCursorPos(out POINT pt))
            {
                return;
            }

            HoverAt((pt.X, pt.Y));
        };
        _hoverPoll.Start();
    }

    private void StopHoverPoll()
    {
        if (_hoverPoll is null)
        {
            return;
        }

        _hoverPoll.Stop();
        _hoverPoll = null;
    }

    public bool WantsAnnotate(bool shiftDownNow)
    {
        if (!shiftDownNow)
        {
            return false;
        }

        return !_suppressAnnotateUntilShiftUp;
    }

    public void NotifyShiftUp()
    {
        _suppressAnnotateUntilShiftUp = false;
    }

    public void HoverAt((int X, int Y) point)
    {
        if (_press is not null)
        {
            return;
        }

        PixelRect next = WindowPicker.Hit(point.X, point.Y, _overlayHwnds, _monitors);
        PixelRect? hover = next.IsEmpty ? null : next;
        if (Hover.HasValue == hover.HasValue
            && (!Hover.HasValue || Hover.Value.Equals(hover!.Value)))
        {
            return;
        }

        Hover = hover;
        Changed?.Invoke();
    }

    public void Press((int X, int Y) point)
    {
        _press = point;
        _hoverAtPress = Hover;
        _dragging = false;
        Drag = null;
        Changed?.Invoke();
    }

    public void Move((int X, int Y) point)
    {
        if (_press is null)
        {
            HoverAt(point);
            return;
        }

        if (!_dragging && CaptureUx.IsDrag(_press.Value.X, _press.Value.Y, point.X, point.Y))
        {
            _dragging = true;
            Hover = null;
        }

        if (_dragging)
        {
            Drag = PixelRect.FromCorners(_press.Value.X, _press.Value.Y, point.X, point.Y);
            Changed?.Invoke();
        }
    }

    public void Release((int X, int Y) point, bool annotate = false)
    {
        if (_press is null)
        {
            return;
        }

        if (_dragging)
        {
            Drag = PixelRect.FromCorners(_press.Value.X, _press.Value.Y, point.X, point.Y);
        }

        PixelRect? hover = Hover ?? _hoverAtPress;
        CaptureUx.ReleaseAction action = CaptureUx.DecideRelease(
            _dragging,
            Drag?.Width ?? 0,
            Drag?.Height ?? 0,
            hover is PixelRect h && !h.IsEmpty);

        _press = null;
        _dragging = false;

        switch (action)
        {
            case CaptureUx.ReleaseAction.CommitDrag when Drag is PixelRect drag && !drag.IsEmpty:
                Completed?.Invoke(drag, annotate);
                return;
            case CaptureUx.ReleaseAction.CommitHover when hover is PixelRect window && !window.IsEmpty:
                Completed?.Invoke(window, annotate);
                return;
            default:
                Drag = null;
                Hover = hover;
                Changed?.Invoke();
                return;
        }
    }

    public void EnterAnnotate(PixelRect rect)
    {
        StopHoverPoll();
        Committed = rect;
        Drag = null;
        Hover = null;
        _press = null;
        _dragging = false;
        Changed?.Invoke();
    }

    public void UpdateCommitted(PixelRect rect)
    {
        Committed = rect;
        // Overlays are closed during annotate; skip Changed so drag cannot rebake masks.
        // Callers should only invoke this on CommitMove/CommitResize, not every pixel.
        if (_windows.Count == 0)
        {
            return;
        }

        Changed?.Invoke();
    }

    public void Cancel()
    {
        StopHoverPoll();
        _press = null;
        _dragging = false;
        Drag = null;
        Hover = null;
        Committed = null;
        Cancelled?.Invoke();
    }

    public void CloseOverlays()
    {
        StopHoverPoll();
        foreach (RegionOverlayWindow window in _windows.ToArray())
        {
            try
            {
                window.Close();
            }
            catch
            {
            }
        }

        _windows.Clear();
        _overlayHwnds.Clear();
    }
}
