namespace Suite.Contracts;

/// <summary>
/// Capture preferences only. Never stores pixels, bitmaps, or file contents.
/// Default save location is computed at save time (My Pictures\Suite).
/// </summary>
public sealed class CaptureSettings
{
    public bool SaveFileAfterCapture { get; set; }
    public bool PinAfterCapture { get; set; } = false;
    public string SaveDirectory { get; set; } = "";

    /// <summary>Show Snipaste-like magnifier during region select. INTERACTION-P2 §5. Default on.</summary>
    public bool ShowMagnifier { get; set; } = true;

    /// <summary>Keep recent captures under History\. Default on.</summary>
    public bool HistoryEnabled { get; set; } = true;

    /// <summary>Max history entries (also capped by 7 days). Default 30.</summary>
    public int HistoryMax { get; set; } = 30;

    /// <summary>Color-pick hotkey. Default F2. VirtualKey 0 = disabled.</summary>
    public HotkeyBinding ColorPickHotkey { get; set; } = HotkeyBinding.DefaultColorPick.Clone();

    /// <summary>Toggle click-through on topmost pin. Default Ctrl+Shift+Q.</summary>
    public HotkeyBinding PinClickThroughHotkey { get; set; } = HotkeyBinding.DefaultPinClickThrough.Clone();

    public CaptureSettings Clone() => new()
    {
        SaveFileAfterCapture = SaveFileAfterCapture,
        PinAfterCapture = PinAfterCapture,
        SaveDirectory = SaveDirectory,
        ShowMagnifier = ShowMagnifier,
        HistoryEnabled = HistoryEnabled,
        HistoryMax = HistoryMax,
        ColorPickHotkey = ColorPickHotkey.Clone(),
        PinClickThroughHotkey = PinClickThroughHotkey.Clone(),
    };

    public void Normalize()
    {
        if (HistoryMax < 1)
        {
            HistoryMax = 30;
        }

        if (HistoryMax > 200)
        {
            HistoryMax = 200;
        }

        ColorPickHotkey ??= HotkeyBinding.DefaultColorPick.Clone();
        PinClickThroughHotkey ??= HotkeyBinding.DefaultPinClickThrough.Clone();
        if (ColorPickHotkey.VirtualKey is < 0 or > 0xFE)
        {
            ColorPickHotkey = HotkeyBinding.DefaultColorPick.Clone();
        }

        if (PinClickThroughHotkey.VirtualKey is < 0 or > 0xFE)
        {
            PinClickThroughHotkey = HotkeyBinding.DefaultPinClickThrough.Clone();
        }
    }
}
