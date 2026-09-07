using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using MediaBrush = System.Windows.Media.Brushes;
using MediaColor = System.Windows.Media.Color;
using System.Windows.Threading;
using Suite.Contracts;
using Suite.NetSpeed;
using Suite.Platform;

namespace Suite.App;

public partial class NetSpeedWindow : Window
{
    private const double FloatWidth = 168;
    private const double FloatHeight = 56;
    private bool _embedded;
    private bool _embedRequested;
    private bool _embedFailNotified;
    private double _savedLeft = 64;
    private double _savedTop = 64;
    private DispatcherTimer? _heartbeat;
    private ThemeKind _theme = ThemeKind.Light;
    private NetSpeedSettings _appearance = new();
    private double _lastDownBps;
    private double _lastUpBps;
    private bool _adapterMissing;
    /// <summary>True only while WS_EX_LAYERED + LWA_COLORKEY are confirmed active.</summary>
    private bool _chromaActive;
    /// <summary>Transparent requested but chroma failed — solid non-pink fill (tray-matched or #2B2B2B).</summary>
    private bool _safeOpaqueFallback;
    private uint _safeOpaqueArgb = TaskbarEmbed.FallbackOpaqueArgb;
    private DispatcherTimer? _chromaWatchdog;

    public NetSpeedWindow()
    {
        InitializeComponent();
        ApplyTheme(ThemeKind.Light);
        LocationChanged += (_, _) =>
        {
            if (!_embedded && !_embedRequested)
            {
                _savedLeft = Left;
                _savedTop = Top;
            }
        };
        SizeChanged += (_, _) =>
        {
            if (_embedded && _appearance.BackgroundTransparent)
            {
                ApplyEmbedSurface(EnsureHandle());
            }
        };
        Loaded += (_, _) =>
        {
            if (_embedded && _appearance.BackgroundTransparent)
            {
                ApplyEmbedSurface(EnsureHandle());
            }
        };
    }

    public event EventHandler? PositionChangedByUser;
    public event EventHandler<string>? EmbedFailed;
    public event EventHandler? RequestFloat;

    public void ApplyAppearance(NetSpeedSettings settings)
    {
        _appearance = settings.Clone();
        _appearance.Normalize();
        RefreshChrome();
        RefreshTexts();
        if (_embedded)
        {
            ApplyEmbedSurface(EnsureHandle());
            if (_appearance.BackgroundTransparent)
            {
                StartChromaWatchdog();
            }
            else
            {
                StopChromaWatchdog();
            }
        }
    }

    public void ApplyTheme(ThemeKind theme)
    {
        _theme = theme;
        RefreshChrome();
        RefreshTexts();
        if (_embedded && _appearance.BackgroundTransparent)
        {
            ApplyEmbedSurface(EnsureHandle());
        }
    }

    public void ApplySample(RateSample sample)
    {
        _adapterMissing = sample.AdapterMissing;
        if (sample.AdapterMissing)
        {
            _lastDownBps = 0;
            _lastUpBps = 0;
        }
        else
        {
            _lastDownBps = sample.ReceiveBytesPerSecond;
            _lastUpBps = sample.SendBytesPerSecond;
        }

        RefreshTexts();
    }

    public bool TryEmbedInTaskbar(out string? error)
    {
        error = null;
        _embedRequested = true;
        _embedFailNotified = false;
        if (!_embedded)
        {
            _savedLeft = Left;
            _savedTop = Top;
        }

        if (!IsVisible)
        {
            Show();
        }

        IntPtr hwnd = EnsureHandle();
        if (_embedded && TaskbarEmbed.IsAttached(hwnd))
        {
            bool ok = TaskbarEmbed.TryReposition(hwnd, TaskbarEmbed.DefaultWidgetWidthPx, out error);
            if (ok)
            {
                RefreshChrome();
                TaskbarEmbed.ForceVisiblePaint(hwnd);
                ApplyEmbedSurface(hwnd); // MUST be after ForceVisiblePaint (never reverse)
                StartChromaWatchdog();
                if (!IsEmbedVisuallyOk(hwnd))
                {
                    error = "embedded window has zero size or is not visible";
                    return false;
                }
            }

            return ok;
        }

        _embedded = true;
        RefreshChrome();
        if (!TaskbarEmbed.TryAttach(hwnd, TaskbarEmbed.DefaultWidgetWidthPx, out error))
        {
            _embedded = false;
            RefreshChrome();
            return false;
        }

        Topmost = false;
        Opacity = 1;
        Visibility = Visibility.Visible;
        Show();
        StartHeartbeat();
        RefreshChrome();
        UpdateLayout();
        // Order: paint/z-order first, THEN chroma. ForceVisiblePaint must never follow ApplyEmbedSurface.
        TaskbarEmbed.ForceVisiblePaint(hwnd);
        ApplyEmbedSurface(hwnd);
        ScheduleChromaRefresh();
        StartChromaWatchdog();
        if (!IsEmbedVisuallyOk(hwnd))
        {
            error = "embedded window has zero size or is not visible";
            _embedded = false;
            StopHeartbeat();
            TaskbarEmbed.TryDetach(hwnd, out _);
            RefreshChrome();
            return false;
        }

        return true;
    }

    /// <summary>
    /// Prefer real chroma-key transparency. If LAYERED+COLORKEY cannot stick on a WPF child of
    /// Shell_TrayWnd, immediately fall back to a tray-matched or dark solid — NEVER leave magenta visible.
    /// </summary>
    private void ApplyEmbedSurface(IntPtr hwnd)
    {
        if (!_appearance.BackgroundTransparent || !_embedded)
        {
            _chromaActive = false;
            _safeOpaqueFallback = false;
            TaskbarEmbed.ClearChromaKeyTransparency(hwnd);
            ApplyEmbeddedBackground(_theme == ThemeKind.Dark);
            return;
        }

        bool ok = TaskbarEmbed.ApplyChromaKeyTransparency(hwnd);
        if (ok)
        {
            _chromaActive = true;
            _safeOpaqueFallback = false;
            ApplyEmbeddedBackground(_theme == ThemeKind.Dark);
            return;
        }

        // Color-key failed — do not paint magenta. Match tray or use intentional dark chip.
        _chromaActive = false;
        _safeOpaqueFallback = true;
        TaskbarEmbed.ClearChromaKeyTransparency(hwnd);
        if (TaskbarEmbed.TrySampleTrayArgb(out uint trayArgb))
        {
            _safeOpaqueArgb = trayArgb;
        }
        else
        {
            _safeOpaqueArgb = TaskbarEmbed.FallbackOpaqueArgb;
        }

        ApplyEmbeddedBackground(_theme == ThemeKind.Dark);
    }

    private void ScheduleChromaRefresh()
    {
        if (!_embedded || !_appearance.BackgroundTransparent)
        {
            return;
        }

        foreach (int ms in new[] { 50, 150, 400, 800, 1600 })
        {
            var timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(ms) };
            timer.Tick += (_, _) =>
            {
                timer.Stop();
                if (_embedded && _appearance.BackgroundTransparent)
                {
                    ApplyEmbedSurface(EnsureHandle());
                }
            };
            timer.Start();
        }
    }

    private void StartChromaWatchdog()
    {
        if (_chromaWatchdog is not null)
        {
            return;
        }

        _chromaWatchdog = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(750) };
        _chromaWatchdog.Tick += (_, _) =>
        {
            if (!_embedded || !_appearance.BackgroundTransparent)
            {
                return;
            }

            IntPtr hwnd = EnsureHandle();
            if (_chromaActive && TaskbarEmbed.IsChromaKeyActive(hwnd))
            {
                return;
            }

            // WPF stripped LAYERED, or we are on fallback — re-attempt chroma; fall back if needed.
            ApplyEmbedSurface(hwnd);
        };
        _chromaWatchdog.Start();
    }

    private void StopChromaWatchdog()
    {
        if (_chromaWatchdog is null)
        {
            return;
        }

        _chromaWatchdog.Stop();
        _chromaWatchdog = null;
    }

    public void ShowAsFloating()
    {
        _embedRequested = false;
        DetachFromTaskbar();
        _chromaActive = false;
        _safeOpaqueFallback = false;
        TaskbarEmbed.ClearChromaKeyTransparency(EnsureHandle());
        Topmost = true;
        ShowInTaskbar = false;
        // Guarantee on-screen presence: never leave a blank transparent float.
        if (double.IsNaN(_savedLeft) || double.IsNaN(_savedTop)
            || _savedLeft < -200 || _savedTop < -200
            || _savedLeft > SystemParameters.VirtualScreenWidth + 200
            || _savedTop > SystemParameters.VirtualScreenHeight + 200)
        {
            _savedLeft = 64;
            _savedTop = 64;
        }

        Left = _savedLeft;
        Top = _savedTop;
        Width = FloatWidth;
        Height = FloatHeight;
        if (!IsVisible)
        {
            Show();
        }

        Opacity = 1;
        Visibility = Visibility.Visible;
        WindowState = WindowState.Normal;
        RefreshChrome();
        // Force a layout pass so text is painted before Activate.
        UpdateLayout();
        Activate();
    }

    /// <summary>True when HWND exists, is visible, and has a usable on-screen rect.</summary>
    public bool IsVisuallyPresent()
    {
        IntPtr hwnd = EnsureHandle();
        if (hwnd == IntPtr.Zero || !IsVisible || Opacity < 0.05)
        {
            return false;
        }

        if (_embedded)
        {
            return IsEmbedVisuallyOk(hwnd);
        }

        if (!TaskbarEmbed.TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            return ActualWidth >= 8 && ActualHeight >= 8;
        }

        int w = right - left;
        int h = bottom - top;
        return w >= 8 && h >= 8;
    }

    public void DetachFromTaskbar()
    {
        StopHeartbeat();
        StopChromaWatchdog();
        IntPtr hwnd = EnsureHandle();
        if (hwnd != IntPtr.Zero)
        {
            TaskbarEmbed.ClearChromaKeyTransparency(hwnd);
            TaskbarEmbed.TryDetach(hwnd, out _);
        }

        _embedded = false;
        _chromaActive = false;
        _safeOpaqueFallback = false;
        RefreshChrome();
        Topmost = true;
    }

    private void OnDrag(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton != MouseButton.Left || _embedded)
        {
            return;
        }

        DragMove();
        _savedLeft = Left;
        _savedTop = Top;
        PositionChangedByUser?.Invoke(this, EventArgs.Empty);
    }

    private void RefreshChrome()
    {
        bool dark = _theme == ThemeKind.Dark;
        bool embed = _embedded;
        double fontSize = embed
            ? TaskbarSlotChrome.ClampFontSize(_appearance.FontSizeDip)
            : TaskbarSlotChrome.FloatFontSizeDip;
        FontWeight weight = _appearance.Bold ? FontWeights.SemiBold : FontWeights.Normal;

        DownText.FontSize = fontSize;
        UpText.FontSize = fontSize;
        SingleLineText.FontSize = fontSize;
        DownText.FontWeight = weight;
        UpText.FontWeight = weight;
        SingleLineText.FontWeight = weight;
        DownText.LineHeight = fontSize * TaskbarSlotChrome.LineHeight;
        UpText.LineHeight = fontSize * TaskbarSlotChrome.LineHeight;
        SingleLineText.LineHeight = fontSize * TaskbarSlotChrome.LineHeight;

        RootBorder.CornerRadius = new CornerRadius(embed ? 0 : 6);
        bool showBorder = embed ? _appearance.ShowBorder : true;
        RootBorder.BorderThickness = new Thickness(showBorder ? 1 : 0);
        if (showBorder)
        {
            RootBorder.BorderBrush = embed
                ? new SolidColorBrush(dark ? MediaColor.FromArgb(0x28, 0xFF, 0xFF, 0xFF) : MediaColor.FromArgb(0x28, 0x00, 0x00, 0x00))
                : new SolidColorBrush(dark ? MediaColor.FromRgb(0x55, 0x55, 0x55) : MediaColor.FromRgb(0xC8, 0xC8, 0xC8));
        }
        else
        {
            RootBorder.BorderBrush = MediaBrush.Transparent;
        }

        if (embed)
        {
            RootBorder.Padding = new Thickness(0);
            ApplyEmbeddedBackground(dark);
            uint dpi = TaskbarEmbed.DpiFor(EnsureHandle());
            Width = DipConvert.PixelsToDip(TaskbarEmbed.DefaultWidgetWidthPx, dpi);
            double heightDip = DipConvert.PixelsToDip(40, dpi);
            if (TaskbarEmbed.TryGetTrayClientSize(out _, out int trayH))
            {
                heightDip = DipConvert.PixelsToDip(trayH, dpi);
            }

            Height = heightDip;
            int visibleLines = Math.Max(1, _appearance.VisibleLineCount());
            double offsetY = TaskbarSlotChrome.OffsetYDip(heightDip, fontSize, visibleLines);
            RateStack.Margin = new Thickness(TaskbarSlotChrome.PadHorizontalDip, offsetY, TaskbarSlotChrome.PadHorizontalDip, 0);
        }
        else
        {
            RootBorder.Padding = new Thickness(10, 8, 10, 8);
            ApplyFloatingBackground(dark);
            Width = FloatWidth;
            Height = FloatHeight;
            RateStack.Margin = new Thickness(0);
        }

        ApplyLineVisibility();
        ApplyLineColors(dark);
    }

    private void ApplyEmbeddedBackground(bool dark)
    {
        if (_appearance.BackgroundTransparent && _chromaActive && !_safeOpaqueFallback)
        {
            // Re-verify before painting magenta — WPF may have stripped LAYERED since last apply.
            if (TaskbarEmbed.IsChromaKeyActive(EnsureHandle()))
            {
                var key = new SolidColorBrush(MediaColor.FromRgb(0xFF, 0x00, 0xFF));
                RootBorder.Background = key;
                Background = key;
                return;
            }

            _chromaActive = false;
            // Fall through to safe hold / opaque path below.
        }

        if (_appearance.BackgroundTransparent && _safeOpaqueFallback)
        {
            // Chroma failed: tray-matched or intentional dark — never pink.
            var safe = new SolidColorBrush(ArgbToColor(_safeOpaqueArgb));
            RootBorder.Background = safe;
            Background = safe;
            return;
        }

        if (_appearance.BackgroundTransparent)
        {
            // Transparent requested but chroma not yet confirmed — hold dark, not magenta.
            var hold = new SolidColorBrush(MediaColor.FromRgb(0x2B, 0x2B, 0x2B));
            RootBorder.Background = hold;
            Background = hold;
            return;
        }

        MediaColor fill = _appearance.BackgroundArgb is uint tint
            ? ArgbToColor(tint)
            : MediaColor.FromRgb(0x1E, 0x1E, 0x1E);
        var brush = new SolidColorBrush(fill);
        RootBorder.Background = brush;
        Background = brush;
    }

    private void ApplyFloatingBackground(bool dark)
    {
        // Never leave a blank/transparent float — Joe priority: netspeed MUST be visible.
        if (!_appearance.BackgroundTransparent && _appearance.BackgroundArgb is uint tint)
        {
            RootBorder.Background = new SolidColorBrush(ArgbToColor(tint));
            return;
        }

        RootBorder.Background = new SolidColorBrush(
            dark ? MediaColor.FromRgb(0x1E, 0x1E, 0x1E) : MediaColor.FromRgb(0xF2, 0xF2, 0xF2));
        RootBorder.BorderThickness = new Thickness(1);
        RootBorder.BorderBrush = new SolidColorBrush(
            dark ? MediaColor.FromRgb(0x66, 0x66, 0x66) : MediaColor.FromRgb(0xC8, 0xC8, 0xC8));
    }

    private void ApplyLineVisibility()
    {
        bool twoLine = _appearance.TwoLine;
        bool showDown = _appearance.ShowDownload;
        bool showUp = _appearance.ShowUpload;

        if (!twoLine && (showDown || showUp))
        {
            DownText.Visibility = Visibility.Collapsed;
            UpText.Visibility = Visibility.Collapsed;
            SingleLineText.Visibility = Visibility.Visible;
            return;
        }

        SingleLineText.Visibility = Visibility.Collapsed;
        DownText.Visibility = showDown ? Visibility.Visible : Visibility.Collapsed;
        UpText.Visibility = showUp ? Visibility.Visible : Visibility.Collapsed;
    }

    private void ApplyLineColors(bool dark)
    {
        // Floating: high-contrast. Embed: honor TrafficMonitor-style custom colors (or system ink).
        bool forceHiContrast = !_embedded;
        MediaColor ink = forceHiContrast
            ? (dark ? MediaColor.FromRgb(0xF2, 0xF2, 0xF2) : MediaColor.FromRgb(0x1A, 0x1A, 0x1A))
            : (dark ? MediaColor.FromRgb(0xF0, 0xF0, 0xF0) : MediaColor.FromRgb(0x1A, 0x1A, 0x1A));
        MediaColor zero = MediaColor.FromRgb(0x88, 0x88, 0x88);

        MediaColor downColor = forceHiContrast
            ? ink
            : ResolveLineColor(_appearance.DownColorArgb, ink, zero, _lastDownBps, _adapterMissing);
        MediaColor upColor = forceHiContrast
            ? (dark ? MediaColor.FromRgb(0xD0, 0xD0, 0xD0) : MediaColor.FromRgb(0x33, 0x33, 0x33))
            : ResolveLineColor(_appearance.UpColorArgb, ink, zero, _lastUpBps, _adapterMissing);
        DownText.Foreground = new SolidColorBrush(downColor);
        UpText.Foreground = new SolidColorBrush(upColor);
        // Single line: prefer down color when both shown; else the visible one.
        MediaColor single = _appearance.ShowDownload ? downColor : upColor;
        SingleLineText.Foreground = new SolidColorBrush(single);
    }

    private static MediaColor ResolveLineColor(uint? customArgb, MediaColor ink, MediaColor zero, double bps, bool missing)
    {
        if (customArgb is uint argb)
        {
            return ArgbToColor(argb);
        }

        if (missing)
        {
            return ink;
        }

        return Math.Abs(bps) < 0.5 ? zero : ink;
    }

    private void RefreshTexts()
    {
        if (_adapterMissing)
        {
            DownText.Text = "↓ —";
            UpText.Text = "↑ 选网卡";
            SingleLineText.Text = BuildSingleMissing();
            ApplyLineColors(_theme == ThemeKind.Dark);
            if (_embedded)
            {
                RefreshChrome();
            }

            return;
        }

        string down = "↓ " + RateFormatter.FormatBytesPerSecond(_lastDownBps);
        string up = "↑ " + RateFormatter.FormatBytesPerSecond(_lastUpBps);
        DownText.Text = down;
        UpText.Text = up;
        SingleLineText.Text = BuildSingleLine(down, up);
        ApplyLineColors(_theme == ThemeKind.Dark);
    }

    private string BuildSingleLine(string down, string up)
    {
        if (_appearance.ShowDownload && _appearance.ShowUpload)
        {
            return down + "  " + up;
        }

        return _appearance.ShowDownload ? down : up;
    }

    private string BuildSingleMissing()
    {
        if (_appearance.ShowDownload && _appearance.ShowUpload)
        {
            return "↓ —  ↑ 选网卡";
        }

        return _appearance.ShowDownload ? "↓ —" : "↑ 选网卡";
    }

    private static MediaColor ArgbToColor(uint argb) =>
        MediaColor.FromArgb(
            (byte)((argb >> 24) & 0xFF),
            (byte)((argb >> 16) & 0xFF),
            (byte)((argb >> 8) & 0xFF),
            (byte)(argb & 0xFF));

    private void StartHeartbeat()
    {
        if (_heartbeat is not null)
        {
            return;
        }

        _heartbeat = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2) };
        _heartbeat.Tick += OnHeartbeat;
        _heartbeat.Start();
    }

    private void StopHeartbeat()
    {
        if (_heartbeat is null)
        {
            return;
        }

        _heartbeat.Tick -= OnHeartbeat;
        _heartbeat.Stop();
        _heartbeat = null;
    }

    private void OnHeartbeat(object? sender, EventArgs e)
    {
        if (!_embedRequested)
        {
            return;
        }

        IntPtr hwnd = EnsureHandle();
        if (TaskbarEmbed.IsAttached(hwnd))
        {
            _embedded = true;
            Opacity = 1;
            Visibility = Visibility.Visible;
            TaskbarEmbed.TryReposition(hwnd, TaskbarEmbed.DefaultWidgetWidthPx, out _);
            RefreshChrome();
            TaskbarEmbed.ForceVisiblePaint(hwnd);
            ApplyEmbedSurface(hwnd);
            StartChromaWatchdog();
            if (IsEmbedVisuallyOk(hwnd))
            {
                return;
            }

            if (!_embedFailNotified)
            {
                _embedFailNotified = true;
                EmbedFailed?.Invoke(this, "网速已嵌入但未在任务栏上可见，正在重试显示。");
            }

            return;
        }

        if (TaskbarEmbed.TryAttach(hwnd, TaskbarEmbed.DefaultWidgetWidthPx, out string? error))
        {
            _embedded = true;
            Topmost = false;
            Opacity = 1;
            RefreshChrome();
            TaskbarEmbed.ForceVisiblePaint(hwnd);
            ApplyEmbedSurface(hwnd);
            StartChromaWatchdog();
            return;
        }

        if (_embedFailNotified)
        {
            return;
        }

        _embedFailNotified = true;
        // Keep window shown near tray as last-ditch visibility while still requesting embed retries next tick.
        EmbedFailed?.Invoke(this, NetSpeedCopy.EmbedFailed(error));
    }


    private static bool IsEmbedVisuallyOk(IntPtr hwnd) =>
        TaskbarEmbed.IsProvenVisibleOnPrimaryTray(hwnd);

    public IntPtr EnsurePublicHandle() => EnsureHandle();

    private void OnFloatFromMenu(object sender, RoutedEventArgs e)
    {
        if (_embedded || _embedRequested)
        {
            RequestFloat?.Invoke(this, EventArgs.Empty);
        }
    }

    private IntPtr EnsureHandle()
    {

        var helper = new WindowInteropHelper(this);
        if (helper.Handle == IntPtr.Zero)
        {
            helper.EnsureHandle();
        }

        return helper.Handle;
    }
}
