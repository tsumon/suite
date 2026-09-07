using System.Windows;
using System.Windows.Media.Imaging;
using Suite.Contracts;
using Suite.Platform;

namespace Suite.Pinboard;

public sealed class PinInfo
{
    public int Index { get; init; }
    public PinWindow Window { get; init; } = null!;
    public bool IsClickThrough => Window.IsClickThrough;
}

public sealed class PinboardService
{
    private readonly List<PinWindow> _windows = [];
    private ThemeKind _theme = ThemeKind.Dark;

    public event EventHandler? Changed;

    public bool AnyClickThrough => _windows.Any(w => w.IsClickThrough);
    public int Count => _windows.Count;

    public IReadOnlyList<PinInfo> List()
    {
        var list = new List<PinInfo>(_windows.Count);
        for (int i = 0; i < _windows.Count; i++)
        {
            list.Add(new PinInfo { Index = i + 1, Window = _windows[i] });
        }

        return list;
    }

    public void ApplyTheme(ThemeKind theme)
    {
        _theme = theme;
        foreach (PinWindow window in _windows)
        {
            window.ApplyTheme(theme);
        }
    }

    public void Pin(BitmapSource image, double? left = null, double? top = null, uint dpiX = 96, uint dpiY = 96)
    {
        if (image.CanFreeze && !image.IsFrozen)
        {
            image.Freeze();
        }

        var window = new PinWindow(image, dpiX, dpiY);
        window.ApplyTheme(_theme);
        if (left is not null && top is not null)
        {
            window.PlaceContentAt(left.Value, top.Value);
        }
        else if (left is not null)
        {
            window.Left = left.Value - PinWindow.GlowPad;
        }
        else if (top is not null)
        {
            window.Top = top.Value - PinWindow.GlowPad;
        }

        window.ClickThroughChanged += (_, _) => Changed?.Invoke(this, EventArgs.Empty);
        window.Closed += (_, _) =>
        {
            _windows.Remove(window);
            Changed?.Invoke(this, EventArgs.Empty);
        };
        _windows.Add(window);
        window.Show();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public bool TryPinFromClipboard(out string? error)
    {
        error = null;
        try
        {
            if (!Clipboard.ContainsImage())
            {
                error = "剪贴板里没有图片。";
                return false;
            }

            BitmapSource? image = Clipboard.GetImage();
            if (image is null)
            {
                error = "无法读取剪贴板图片。";
                return false;
            }

            if (image.CanFreeze)
            {
                image.Freeze();
            }

            uint dpi = PinNative.DpiNearCursor();
            var (px, py) = PinNative.CursorPos();
            var (left, top) = DipConvert.Origin(px, py, dpi, dpi);
            Pin(image, left, top, dpi, dpi);
            return true;
        }
        catch
        {
            error = "无法读取剪贴板图片。";
            return false;
        }
    }

    public void ClearClickThrough()
    {
        foreach (PinWindow window in _windows.ToArray())
        {
            if (window.IsClickThrough)
            {
                window.SetClickThrough(false);
            }
        }
    }

    /// <summary>Toggle click-through on the topmost (last) pin. INTERACTION-P2 §4.2.</summary>
    public bool TryToggleTopClickThrough(out string? error)
    {
        error = null;
        if (_windows.Count == 0)
        {
            error = "当前没有贴图。";
            return false;
        }

        PinWindow top = _windows[^1];
        top.SetClickThrough(!top.IsClickThrough);
        return true;
    }

    public void Focus(PinWindow window)
    {
        if (!_windows.Contains(window))
        {
            return;
        }

        window.Activate();
        window.Focus();
        window.Topmost = true;
    }

    public void Close(PinWindow window)
    {
        if (_windows.Contains(window))
        {
            window.Close();
        }
    }

    public void CloseAll()
    {
        foreach (PinWindow window in _windows.ToArray())
        {
            window.Close();
        }

        _windows.Clear();
    }
}
