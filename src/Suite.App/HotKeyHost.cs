using System.Windows.Interop;
using Suite.Contracts;
using Suite.Platform;

namespace Suite.App;

public sealed class HotKeyHost : IDisposable
{
    private const int ProbeHotKeyId = 9100;

    private HwndSource? _source;
    private bool _captureRegistered;
    private bool _pinRegistered;
    private bool _colorPickRegistered;
    private bool _clickThroughRegistered;
    private bool _scrollCaptureRegistered;
    private HotkeyBinding? _captureBinding;
    private int _lastCaptureWin32;
    private int _lastPinWin32;
    private int _lastColorPickWin32;
    private int _lastClickThroughWin32;
    private int _lastScrollCaptureWin32;

    public event EventHandler? Pressed;
    public event EventHandler? PinClipboardPressed;
    public event EventHandler? ColorPickPressed;
    public event EventHandler? PinClickThroughPressed;
    public event EventHandler? ScrollCapturePressed;

    public bool IsCaptureRegistered => _captureRegistered;
    public bool IsPinRegistered => _pinRegistered;
    public bool IsColorPickRegistered => _colorPickRegistered;
    public bool IsClickThroughRegistered => _clickThroughRegistered;
    public bool IsScrollCaptureRegistered => _scrollCaptureRegistered;
    public int LastCaptureWin32Error => _lastCaptureWin32;
    public int LastPinWin32Error => _lastPinWin32;
    public int LastColorPickWin32Error => _lastColorPickWin32;
    public int LastClickThroughWin32Error => _lastClickThroughWin32;
    public int LastScrollCaptureWin32Error => _lastScrollCaptureWin32;

    public bool TryStart(
        HotkeyBinding capture,
        HotkeyBinding? colorPick,
        HotkeyBinding? pinClickThrough,
        HotkeyBinding? scrollCapture,
        out string? error,
        out string? pinError,
        out string? colorPickError,
        out string? clickThroughError,
        out string? scrollCaptureError)
    {
        error = null;
        pinError = null;
        colorPickError = null;
        clickThroughError = null;
        scrollCaptureError = null;
        _lastCaptureWin32 = 0;
        _lastPinWin32 = 0;
        _lastColorPickWin32 = 0;
        _lastClickThroughWin32 = 0;
        _lastScrollCaptureWin32 = 0;
        Stop();
        var parameters = new HwndSourceParameters("SuiteHotKey")
        {
            Width = 1,
            Height = 1,
            WindowStyle = unchecked((int)0x80000000),
            ExtendedWindowStyle = 0x00000080 | 0x08000000,
        };
        _source = new HwndSource(parameters);
        _source.AddHook(Hook);
        if (NativeHotKey.TryRegister(
                _source.Handle,
                NativeConstants.PlaceholderHotKeyId,
                capture,
                out error,
                out _lastCaptureWin32))
        {
            _captureRegistered = true;
            _captureBinding = capture.Clone();
        }

        bool sameAsCapture = !capture.Control && !capture.Shift && !capture.Alt && !capture.Win
            && capture.VirtualKey == HotkeyBinding.PinClipboardVirtualKey;
        if (!sameAsCapture)
        {
            var pin = new HotkeyBinding { VirtualKey = HotkeyBinding.PinClipboardVirtualKey };
            if (NativeHotKey.TryRegister(
                    _source.Handle,
                    NativeConstants.PinClipboardHotKeyId,
                    pin,
                    out pinError,
                    out _lastPinWin32))
            {
                _pinRegistered = true;
                pinError = null;
            }
        }

        if (colorPick is not null && !colorPick.IsDisabled && !SameBinding(colorPick, capture))
        {
            if (NativeHotKey.TryRegister(
                    _source.Handle,
                    NativeConstants.ColorPickHotKeyId,
                    colorPick,
                    out colorPickError,
                    out _lastColorPickWin32))
            {
                _colorPickRegistered = true;
                colorPickError = null;
            }
        }

        if (pinClickThrough is not null && !pinClickThrough.IsDisabled
            && !SameBinding(pinClickThrough, capture)
            && (colorPick is null || colorPick.IsDisabled || !SameBinding(pinClickThrough, colorPick)))
        {
            if (NativeHotKey.TryRegister(
                    _source.Handle,
                    NativeConstants.PinClickThroughHotKeyId,
                    pinClickThrough,
                    out clickThroughError,
                    out _lastClickThroughWin32))
            {
                _clickThroughRegistered = true;
                clickThroughError = null;
            }
        }

        if (scrollCapture is not null && !scrollCapture.IsDisabled
            && !SameBinding(scrollCapture, capture)
            && (colorPick is null || colorPick.IsDisabled || !SameBinding(scrollCapture, colorPick))
            && (pinClickThrough is null || pinClickThrough.IsDisabled || !SameBinding(scrollCapture, pinClickThrough)))
        {
            if (NativeHotKey.TryRegister(
                    _source.Handle,
                    NativeConstants.ScrollCaptureHotKeyId,
                    scrollCapture,
                    out scrollCaptureError,
                    out _lastScrollCaptureWin32))
            {
                _scrollCaptureRegistered = true;
                scrollCaptureError = null;
            }
        }

        return _captureRegistered;
    }

    public bool TryStart(
        HotkeyBinding capture,
        HotkeyBinding? colorPick,
        HotkeyBinding? pinClickThrough,
        out string? error,
        out string? pinError,
        out string? colorPickError,
        out string? clickThroughError) =>
        TryStart(capture, colorPick, pinClickThrough, null, out error, out pinError, out colorPickError, out clickThroughError, out _);

    public bool TryStart(HotkeyBinding binding, out string? error, out string? pinError) =>
        TryStart(binding, null, null, null, out error, out pinError, out _, out _, out _);

    public bool TryStart(HotkeyBinding binding, out string? error) =>
        TryStart(binding, out error, out _);

    public void Stop()
    {
        if (_source is not null && _captureRegistered)
        {
            NativeHotKey.TryUnregister(_source.Handle, NativeConstants.PlaceholderHotKeyId, out _);
            _captureRegistered = false;
        }

        if (_source is not null && _pinRegistered)
        {
            NativeHotKey.TryUnregister(_source.Handle, NativeConstants.PinClipboardHotKeyId, out _);
            _pinRegistered = false;
        }

        if (_source is not null && _colorPickRegistered)
        {
            NativeHotKey.TryUnregister(_source.Handle, NativeConstants.ColorPickHotKeyId, out _);
            _colorPickRegistered = false;
        }

        if (_source is not null && _clickThroughRegistered)
        {
            NativeHotKey.TryUnregister(_source.Handle, NativeConstants.PinClickThroughHotKeyId, out _);
            _clickThroughRegistered = false;
        }

        if (_source is not null && _scrollCaptureRegistered)
        {
            NativeHotKey.TryUnregister(_source.Handle, NativeConstants.ScrollCaptureHotKeyId, out _);
            _scrollCaptureRegistered = false;
        }

        _captureBinding = null;
        _source?.Dispose();
        _source = null;
    }

    public void Dispose() => Stop();

    public IReadOnlyList<HotkeyBinding> SuggestAlternates(HotkeyBinding seed, int max = 3)
    {
        var list = new List<HotkeyBinding>(max);
        HotkeyBinding cursor = seed;
        for (int i = 0; i < max; i++)
        {
            HotkeyBinding? next = FindNextAvailableAfter(cursor);
            if (next is null)
            {
                break;
            }

            if (list.Any(x => SameBinding(x, next)))
            {
                break;
            }

            list.Add(next);
            cursor = next;
        }

        return list;
    }

    public HotkeyBinding? FindNextAvailableAfter(HotkeyBinding seed)
    {
        var ordered = CandidateVirtualKeys().ToList();
        int start = ordered.IndexOf(seed.VirtualKey);
        if (start < 0)
        {
            start = -1;
        }

        for (int offset = 1; offset <= ordered.Count; offset++)
        {
            int vk = ordered[(start + offset) % ordered.Count];
            var candidate = new HotkeyBinding
            {
                Control = seed.Control,
                Shift = seed.Shift,
                Alt = seed.Alt,
                Win = seed.Win,
                VirtualKey = vk,
            };
            if (IsReservedForPin(candidate))
            {
                continue;
            }

            if (OwnsCapture(candidate) || ProbeOnce(candidate))
            {
                return candidate;
            }
        }

        return null;
    }

    private bool OwnsCapture(HotkeyBinding binding) =>
        _captureRegistered
        && _captureBinding is not null
        && SameBinding(_captureBinding, binding);

    private static bool IsReservedForPin(HotkeyBinding binding) =>
        !binding.Control && !binding.Shift && !binding.Alt && !binding.Win
        && binding.VirtualKey == HotkeyBinding.PinClipboardVirtualKey;

    private static bool ProbeOnce(HotkeyBinding binding)
    {
        var parameters = new HwndSourceParameters("SuiteHotKeyProbe")
        {
            Width = 1,
            Height = 1,
            WindowStyle = unchecked((int)0x80000000),
            ExtendedWindowStyle = 0x00000080 | 0x08000000,
        };
        using var source = new HwndSource(parameters);
        if (!NativeHotKey.TryRegister(source.Handle, ProbeHotKeyId, binding, out _))
        {
            return false;
        }

        NativeHotKey.TryUnregister(source.Handle, ProbeHotKeyId, out _);
        return true;
    }

    private static IEnumerable<int> CandidateVirtualKeys()
    {
        for (int vk = 0x70; vk <= 0x7B; vk++)
        {
            yield return vk;
        }

        for (int vk = 0x41; vk <= 0x5A; vk++)
        {
            yield return vk;
        }
    }

    private static bool SameBinding(HotkeyBinding a, HotkeyBinding b) =>
        a.Control == b.Control
        && a.Shift == b.Shift
        && a.Alt == b.Alt
        && a.Win == b.Win
        && a.VirtualKey == b.VirtualKey;

    private IntPtr Hook(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg != unchecked((int)NativeConstants.WmHotKey))
        {
            return IntPtr.Zero;
        }

        int id = wParam.ToInt32();
        if (id == NativeConstants.PlaceholderHotKeyId)
        {
            Pressed?.Invoke(this, EventArgs.Empty);
            handled = true;
        }
        else if (id == NativeConstants.PinClipboardHotKeyId)
        {
            PinClipboardPressed?.Invoke(this, EventArgs.Empty);
            handled = true;
        }
        else if (id == NativeConstants.ColorPickHotKeyId)
        {
            ColorPickPressed?.Invoke(this, EventArgs.Empty);
            handled = true;
        }
        else if (id == NativeConstants.PinClickThroughHotKeyId)
        {
            PinClickThroughPressed?.Invoke(this, EventArgs.Empty);
            handled = true;
        }
        else if (id == NativeConstants.ScrollCaptureHotKeyId)
        {
            ScrollCapturePressed?.Invoke(this, EventArgs.Empty);
            handled = true;
        }

        return IntPtr.Zero;
    }
}
