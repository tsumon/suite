using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Drawing.Text;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using Suite.Contracts;
using Suite.NetSpeed;
using Suite.Platform;
using DrawingColor = System.Drawing.Color;
using DrawingFont = System.Drawing.Font;
using DrawingFontStyle = System.Drawing.FontStyle;
using FormsTimer = System.Windows.Forms.Timer;

namespace Suite.App;

/// <summary>
/// Taskbar net-speed chip as a TopMost layered popup (NOT SetParent child).
/// Every frame: UpdateLayeredWindow with a full transparent 32bpp ARGB bitmap.
/// WS_CHILD of Shell_TrayWnd cannot reliably ULW — that path produced Joe's black+ghost chip.
/// </summary>
public sealed class NetSpeedTrayWidget : Form
{
    private NetSpeedSettings _appearance = new();
    private ThemeKind _theme = ThemeKind.Light;
    private double _lastDownBps;
    private double _lastUpBps;
    private bool _adapterMissing;
    private bool _perPixelActive;
    private bool _embedRequested;
    private bool _embedFailNotified;
    private IntPtr _dockTray = IntPtr.Zero;
    private FormsTimer? _heartbeat;
    private FormsTimer? _surfaceWatchdog;
    private DrawingFont? _rateFont;
    private string? _lastFrameKey;
    /// <summary>Reused ULW pixel scratch — avoid allocating a new BGRA copy every sample tick.</summary>
    private byte[] _frameBuffer = Array.Empty<byte>();

    public NetSpeedTrayWidget()
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        ControlBox = false;
        MaximizeBox = false;
        MinimizeBox = false;
        AutoScaleMode = AutoScaleMode.None;
        TopMost = true;
        Text = "Suite NetSpeed Tray";
        // If LAYERED is briefly off, avoid pure black flash — match common dark tray.
        BackColor = DrawingColor.FromArgb(1, 1, 1); // never Control-gray; ULW must own pixels
        ForeColor = DrawingColor.White;
        Size = new Size(TaskbarEmbed.DefaultWidgetWidthPx, 40);
        SetStyle(
            ControlStyles.AllPaintingInWmPaint
            | ControlStyles.UserPaint
            | ControlStyles.Opaque
            | ControlStyles.ResizeRedraw,
            true);

        var menu = new ContextMenuStrip();
        menu.Items.Add("改回桌面悬浮窗", null, (_, _) => RequestFloat?.Invoke(this, EventArgs.Empty));
        ContextMenuStrip = menu;
    }

    public event EventHandler<string>? EmbedFailed;
    public event EventHandler? RequestFloat;

    protected override CreateParams CreateParams
    {
        get
        {
            CreateParams cp = base.CreateParams;
            cp.ExStyle |= unchecked((int)TaskbarEmbedNativePublic.WsExLayered);
            cp.ExStyle |= unchecked((int)TaskbarEmbedNativePublic.WsExToolwindow);
            cp.ExStyle |= unchecked((int)TaskbarEmbedNativePublic.WsExTopmost);
            return cp;
        }
    }

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        if (_appearance.BackgroundTransparent)
        {
            TaskbarEmbed.PreparePerPixelLayered(Handle);
            PushPerPixelFrame();
        }
    }

    protected override void OnPaintBackground(PaintEventArgs e)
    {
        // Suppress WinForms erase — would paint opaque BackColor (black card).
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        if (_appearance.BackgroundTransparent)
        {
            // Only ULW — never GDI fill into HWND (ghosts + black).
            PushPerPixelFrame();
            return;
        }

        Graphics g = e.Graphics;
        g.TextRenderingHint = TextRenderingHint.SingleBitPerPixelGridFit;
        Rectangle bounds = ClientRectangle;
        if (bounds.Width < 8 || bounds.Height < 8)
        {
            SyncBoundsFromHwnd();
            bounds = new Rectangle(0, 0, Math.Max(1, Width), Math.Max(1, Height));
        }

        DrawingColor bg = _appearance.BackgroundArgb is uint tint
            ? ArgbToColor(tint)
            : DrawingColor.FromArgb(0x1E, 0x1E, 0x1E);
        using (var brush = new SolidBrush(bg))
        {
            g.FillRectangle(brush, bounds);
        }

        PaintRates(g, bounds);
    }

    /// <summary>
    /// Full-frame transparent ARGB → UpdateLayeredWindow. Never paints an opaque black card.
    /// On failure: leave previous ULW surface (or empty); retry next tick — no COLORKEY/GDI fallback.
    /// </summary>
    private bool PushPerPixelFrame()
    {
        if (!IsHandleCreated)
        {
            return false;
        }

        IntPtr hwnd = Handle;
        // Read HWND size for the bitmap — do NOT assign Form.Size here.
        // Changing Size on a ULW window clears the layered buffer → gray acrylic flash every sample.
        int w = Math.Max(1, Width);
        int h = Math.Max(1, Height);
        if (TaskbarEmbed.TryGetWindowRect(hwnd, out int left, out int top, out int right, out int bottom))
        {
            w = Math.Max(1, right - left);
            h = Math.Max(1, bottom - top);
        }

        string frameKey = w + "x" + h + "|" + _adapterMissing + "|"
            + _lastDownBps.ToString("F0") + "|" + _lastUpBps.ToString("F0") + "|"
            + _appearance.ShowDownload + "|" + _appearance.ShowUpload + "|"
            + _appearance.TwoLine + "|" + _theme;
        if (_perPixelActive && frameKey == _lastFrameKey)
        {
            return true; // identical frame — skip ULW (avoids acrylic flash)
        }

        using var bmp = new Bitmap(w, h, PixelFormat.Format32bppPArgb);
        using (var g = Graphics.FromImage(bmp))
        {
            g.CompositingMode = CompositingMode.SourceCopy;
            g.Clear(DrawingColor.Transparent);
            g.CompositingMode = CompositingMode.SourceOver;
            g.CompositingQuality = CompositingQuality.HighQuality;
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
            g.PixelOffsetMode = PixelOffsetMode.HighQuality;
            PaintRates(g, new Rectangle(0, 0, w, h));
        }

        EnsureFrameBuffer(w * h * 4);
        ExtractPremultipliedBgra(bmp, _frameBuffer);
        bool ok = TaskbarEmbed.TryPushPerPixelLayered(hwnd, w, h, _frameBuffer);
        _perPixelActive = ok;
        if (ok)
        {
            _lastFrameKey = frameKey;
        }

        return ok;
    }

    private void PaintRates(Graphics g, Rectangle bounds)
    {
        EnsureRateFont();
        if (_rateFont is null)
        {
            return;
        }

        bool trayDark = IsTrayLikelyDark();
        bool dark = trayDark || _theme == ThemeKind.Dark;
        DrawingColor ink = dark
            ? DrawingColor.FromArgb(0xF0, 0xF0, 0xF0)
            : DrawingColor.FromArgb(0x1A, 0x1A, 0x1A);
        DrawingColor zero = DrawingColor.FromArgb(0x88, 0x88, 0x88);

        string downText;
        string upText;
        string singleText;

        if (_adapterMissing)
        {
            downText = "↓ —";
            upText = "↑ 选网卡";
            singleText = BuildSingleMissing();
        }
        else
        {
            downText = "↓ " + RateFormatter.FormatBytesPerSecond(_lastDownBps);
            upText = "↑ " + RateFormatter.FormatBytesPerSecond(_lastUpBps);
            singleText = BuildSingleLine(downText, upText);
        }

        DrawingColor downColor = ResolveLineColor(_appearance.DownColorArgb, ink, zero, _lastDownBps, _adapterMissing);
        DrawingColor upColor = ResolveLineColor(_appearance.UpColorArgb, ink, zero, _lastUpBps, _adapterMissing);

        float fontDip = (float)TaskbarSlotChrome.ClampFontSize(_appearance.FontSizeDip);
        uint dpi = TaskbarEmbed.DpiFor(Handle);
        float lineHeightPx = fontDip * (float)TaskbarSlotChrome.LineHeight * dpi / 96f;
        int visibleLines = Math.Max(1, _appearance.VisibleLineCount());
        float contentH = lineHeightPx * visibleLines;
        float leftover = bounds.Height - contentH;
        float offsetY = leftover > 0 ? leftover / 2f + (float)TaskbarSlotChrome.OpticalNudgeDip * dpi / 96f : 0f;
        float padX = (float)TaskbarSlotChrome.PadHorizontalDip * dpi / 96f;

        bool twoLine = _appearance.TwoLine;
        bool showDown = _appearance.ShowDownload;
        bool showUp = _appearance.ShowUpload;

        if (!twoLine && (showDown || showUp))
        {
            DrawingColor singleColor = showDown ? downColor : upColor;
            using var brush = new SolidBrush(singleColor);
            g.DrawString(singleText ?? "", _rateFont, brush, padX, offsetY);
            DrawOptionalBorder(g, dark, bounds);
            return;
        }

        float y = offsetY;
        if (showDown)
        {
            using var brush = new SolidBrush(downColor);
            g.DrawString(downText ?? "", _rateFont, brush, padX, y);
            y += lineHeightPx;
        }

        if (showUp)
        {
            using var brush = new SolidBrush(upColor);
            g.DrawString(upText ?? "", _rateFont, brush, padX, y);
        }

        DrawOptionalBorder(g, dark, bounds);
    }

    private void DrawOptionalBorder(Graphics g, bool dark, Rectangle bounds)
    {
        if (!_appearance.ShowBorder)
        {
            return;
        }

        DrawingColor edge = dark
            ? DrawingColor.FromArgb(0x28, 0xFF, 0xFF, 0xFF)
            : DrawingColor.FromArgb(0x28, 0x00, 0x00, 0x00);
        using var pen = new Pen(edge);
        g.DrawRectangle(pen, 0, 0, Math.Max(0, bounds.Width - 1), Math.Max(0, bounds.Height - 1));
    }

    private void EnsureFrameBuffer(int bytes)
    {
        if (_frameBuffer.Length < bytes)
        {
            _frameBuffer = new byte[bytes];
        }
    }

    private static void ExtractPremultipliedBgra(Bitmap bmp, byte[] buf)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        int need = w * h * 4;
        if (buf.Length < need)
        {
            throw new ArgumentException("Frame buffer too small.", nameof(buf));
        }

        var rect = new Rectangle(0, 0, w, h);
        BitmapData data = bmp.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format32bppPArgb);
        try
        {
            int stride = data.Stride;
            for (int y = 0; y < h; y++)
            {
                Marshal.Copy(data.Scan0 + y * stride, buf, y * w * 4, w * 4);
            }
        }
        finally
        {
            bmp.UnlockBits(data);
        }
    }

    public void ApplyAppearance(NetSpeedSettings settings)
    {
        _appearance = settings.Clone();
        _appearance.Normalize();
        RebuildFont();
        if (IsHandleCreated && _embedRequested)
        {
            ApplySurface();
            if (_appearance.BackgroundTransparent)
            {
                StartSurfaceWatchdog();
            }
            else
            {
                StopSurfaceWatchdog();
            }
        }

        RequestFrame();
    }

    public void ApplyTheme(ThemeKind theme)
    {
        _theme = theme;
        RequestFrame();
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

        RequestFrame();
    }

    private void RequestFrame()
    {
        if (!IsHandleCreated)
        {
            return;
        }

        if (_appearance.BackgroundTransparent)
        {
            PushPerPixelFrame();
            return;
        }

        Invalidate();
    }

    public bool TryEmbed(out string? error)
    {
        error = null;
        _embedRequested = true;
        _embedFailNotified = false;
        _ = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: Environment.ProcessId);

        if (!IsHandleCreated)
        {
            CreateControl();
        }

        if (!Visible)
        {
            Visible = true;
        }

        IntPtr tray = TaskbarEmbed.FindPrimaryTray();
        _dockTray = tray;

        // Prefer top-level IN-slot dock — ULW works. Never SetParent for transparent chip.
        if (!TaskbarEmbed.TryDockInPrimaryTraySlot(Handle, TaskbarEmbed.DefaultWidgetWidthPx, out error))
        {
            return false;
        }

        SyncBoundsFromHwnd();
        RebuildFont();
        ApplySurface();
        ScheduleSurfaceRefresh();
        StartHeartbeat();
        StartSurfaceWatchdog();
        RequestFrame();

        if (!IsVisuallyPresent())
        {
            error = "embedded window has zero size or is not visible";
            return false;
        }

        return true;
    }

    public void HideEmbed()
    {
        _embedRequested = false;
        _dockTray = IntPtr.Zero;
        StopHeartbeat();
        StopSurfaceWatchdog();
        if (IsHandleCreated)
        {
            TaskbarEmbed.DestroyHwnd(Handle);
        }

        _perPixelActive = false;
        if (IsHandleCreated)
        {
            try { DestroyHandle(); } catch { }
        }

        if (Visible)
        {
            try { Hide(); } catch { }
        }
    }

    public bool IsVisuallyPresent()
    {
        if (!IsHandleCreated || !Visible)
        {
            return false;
        }

        // Docked top-level over/in tray OR legacy child attach.
        return TaskbarEmbed.IsProvenVisibleOnPrimaryTray(Handle)
            || TaskbarEmbed.IsDockedOverPrimaryTray(Handle);
    }

    public IntPtr PublicHandle
    {
        get
        {
            if (!IsHandleCreated)
            {
                CreateControl();
            }

            return Handle;
        }
    }

    public bool TryEmbedOnTray(IntPtr tray, out string? error)
    {
        error = null;
        _embedRequested = true;
        _dockTray = tray;
        _ = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: Environment.ProcessId);
        if (!IsHandleCreated)
        {
            CreateControl();
        }

        if (!Visible)
        {
            Visible = true;
        }

        if (!TaskbarEmbed.TryDockInTraySlot(Handle, tray, TaskbarEmbed.DefaultWidgetWidthPx, out error))
        {
            return false;
        }

        SyncBoundsFromHwnd();
        RebuildFont();
        ApplySurface();
        StartSurfaceWatchdog();
        RequestFrame();
        return TaskbarEmbed.IsWindowVisible(Handle);
    }

    private void SyncBoundsFromHwnd()
    {
        if (!IsHandleCreated)
        {
            return;
        }

        if (!TaskbarEmbed.TryGetWindowRect(Handle, out int left, out int top, out int right, out int bottom))
        {
            return;
        }

        int w = Math.Max(1, right - left);
        int h = Math.Max(1, bottom - top);
        // Ignore 1–2px jitter — Form.Size changes destroy the ULW surface.
        if (Math.Abs(Width - w) > 2 || Math.Abs(Height - h) > 2)
        {
            Size = new Size(w, h);
        }
    }

    private void ApplySurface()
    {
        if (!IsHandleCreated)
        {
            return;
        }

        IntPtr hwnd = Handle;
        if (!_appearance.BackgroundTransparent)
        {
            _perPixelActive = false;
            TaskbarEmbed.ClearChromaKeyTransparency(hwnd);
            BackColor = _appearance.BackgroundArgb is uint tint
                ? ArgbToColor(tint)
                : DrawingColor.FromArgb(0x1E, 0x1E, 0x1E);
            Invalidate();
            return;
        }

        // Transparent: ULW only. Never COLORKEY / gray acrylic card.
        BackColor = DrawingColor.FromArgb(1, 1, 1);
        if (!_perPixelActive)
        {
            TaskbarEmbed.PreparePerPixelLayered(hwnd);
        }

        PushPerPixelFrame();
    }

    private void ScheduleSurfaceRefresh()
    {
        if (!_appearance.BackgroundTransparent)
        {
            return;
        }

        // Early ULW pushes only — no ApplySurface/Prepare, no 800/1600 re-dock flash.
        foreach (int ms in new[] { 50, 150 })
        {
            var timer = new FormsTimer { Interval = ms };
            timer.Tick += (_, _) =>
            {
                timer.Stop();
                timer.Dispose();
                if (_embedRequested && _appearance.BackgroundTransparent && IsHandleCreated)
                {
                    RequestFrame();
                }
            };
            timer.Start();
        }
    }

    private void StartSurfaceWatchdog()
    {
        // Disabled: periodic ULW/SetWindowPos caused gray acrylic flashes. ApplySample owns frames.
        return;
    }

    private void StopSurfaceWatchdog()
    {
        if (_surfaceWatchdog is null)
        {
            return;
        }

        _surfaceWatchdog.Stop();
        _surfaceWatchdog.Dispose();
        _surfaceWatchdog = null;
    }

    private void StartHeartbeat()
    {
        if (_heartbeat is not null)
        {
            return;
        }

        _heartbeat = new FormsTimer { Interval = 2000 };
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
        _heartbeat.Dispose();
        _heartbeat = null;
    }

    private void OnHeartbeat(object? sender, EventArgs e)
    {
        if (!_embedRequested || !IsHandleCreated)
        {
            return;
        }

        _ = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: Environment.ProcessId);

        IntPtr tray = _dockTray != IntPtr.Zero ? _dockTray : TaskbarEmbed.FindPrimaryTray();
        // Already on slot: do NOTHING (no RequestFrame/TryDock/SetWindowPos).
        if (TaskbarEmbed.IsInTraySlot(Handle, tray, TaskbarEmbed.DefaultWidgetWidthPx))
        {
            _embedFailNotified = false;
            return;
        }

        if (!TaskbarEmbed.TryDockInTraySlot(Handle, tray, TaskbarEmbed.DefaultWidgetWidthPx, out string? error))
        {
            if (!_embedFailNotified)
            {
                _embedFailNotified = true;
                EmbedFailed?.Invoke(
                    this,
                    "网速嵌入失败，仍保留任务栏槽并重试。" + (string.IsNullOrWhiteSpace(error) ? "" : " " + error));
            }

            return;
        }

        SyncBoundsFromHwnd();
        RequestFrame();
        if (IsVisuallyPresent())
        {
            _embedFailNotified = false;
            return;
        }

        if (!_embedFailNotified)
        {
            _embedFailNotified = true;
            EmbedFailed?.Invoke(this, "网速已嵌入但未在任务栏上可见，正在重试显示。");
        }
    }

    private void EnsureRateFont()
    {
        if (_rateFont is not null)
        {
            return;
        }

        RebuildFont();
    }

    private void RebuildFont()
    {
        _rateFont?.Dispose();
        float fontDip = (float)TaskbarSlotChrome.ClampFontSize(_appearance.FontSizeDip);
        uint dpi = IsHandleCreated ? TaskbarEmbed.DpiFor(Handle) : 96;
        float fontPx = fontDip * dpi / 96f;
        DrawingFontStyle style = _appearance.Bold ? DrawingFontStyle.Bold : DrawingFontStyle.Regular;
        try
        {
            _rateFont = new DrawingFont("Consolas", fontPx, style, GraphicsUnit.Pixel);
        }
        catch
        {
            _rateFont = new DrawingFont(FontFamily.GenericMonospace, fontPx, style, GraphicsUnit.Pixel);
        }
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

    private static DrawingColor ResolveLineColor(uint? customArgb, DrawingColor ink, DrawingColor zero, double bps, bool missing)
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

    private bool IsTrayLikelyDark()
    {
        if (TaskbarEmbed.TrySampleTrayArgb(out uint argb))
        {
            byte r = (byte)((argb >> 16) & 0xFF);
            byte g = (byte)((argb >> 8) & 0xFF);
            byte b = (byte)(argb & 0xFF);
            int y = (r * 30 + g * 59 + b * 11) / 100;
            return y < 140;
        }

        return _theme == ThemeKind.Dark;
    }

    private static DrawingColor ArgbToColor(uint argb) =>
        DrawingColor.FromArgb(
            (byte)((argb >> 24) & 0xFF),
            (byte)((argb >> 16) & 0xFF),
            (byte)((argb >> 8) & 0xFF),
            (byte)(argb & 0xFF));

    protected override void Dispose(bool disposing)
    {
        if (disposing)
        {
            HideEmbed();
            _rateFont?.Dispose();
            _rateFont = null;
            ContextMenuStrip?.Dispose();
            _frameBuffer = Array.Empty<byte>();
        }

        if (IsHandleCreated)
        {
            TaskbarEmbed.DestroyHwnd(Handle);
            try { DestroyHandle(); } catch { }
        }

        base.Dispose(disposing);
    }
}

internal static class TaskbarEmbedNativePublic
{
    internal const long WsExLayered = 0x00080000L;
    internal const long WsExToolwindow = 0x00000080L;
    internal const long WsExTopmost = 0x00000008L;
}
