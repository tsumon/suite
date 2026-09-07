namespace Suite.Contracts;

/// <summary>
/// Region-capture hotkey. Default F1 (Joe habit). Registration failure must be visible.
/// </summary>
public sealed class HotkeyBinding
{
    public const int DefaultVirtualKey = 0x70; // VK_F1
    public const int PinClipboardVirtualKey = 0x72; // VK_F3
    public const int DefaultColorPickVirtualKey = 0x71; // VK_F2

    public bool Control { get; set; }
    public bool Shift { get; set; }
    public bool Alt { get; set; }
    public bool Win { get; set; }
    public int VirtualKey { get; set; } = DefaultVirtualKey;

    public static HotkeyBinding Default { get; } = new();

    /// <summary>F2 — color pick (INTERACTION-P2 §6).</summary>
    public static HotkeyBinding DefaultColorPick { get; } = new() { VirtualKey = DefaultColorPickVirtualKey };

    /// <summary>Ctrl+Shift+Q — toggle top pin click-through (INTERACTION-P2 §4.2).</summary>
    public static HotkeyBinding DefaultPinClickThrough { get; } = new()
    {
        Control = true,
        Shift = true,
        VirtualKey = 0x51, // Q
    };

    /// <summary>VirtualKey 0 means hotkey disabled / not registered.</summary>
    public bool IsDisabled => VirtualKey <= 0;

    public uint ToNativeModifiers(bool noRepeat = true)
    {
        uint mods = 0;
        if (Alt)
        {
            mods |= 0x0001; // MOD_ALT
        }

        if (Control)
        {
            mods |= 0x0002; // MOD_CONTROL
        }

        if (Shift)
        {
            mods |= 0x0004; // MOD_SHIFT
        }

        if (Win)
        {
            mods |= 0x0008; // MOD_WIN
        }

        if (noRepeat)
        {
            mods |= 0x4000; // MOD_NOREPEAT
        }

        return mods;
    }

    public string ToDisplayString()
    {
        var parts = new List<string>(5);
        if (Control)
        {
            parts.Add("Ctrl");
        }

        if (Shift)
        {
            parts.Add("Shift");
        }

        if (Alt)
        {
            parts.Add("Alt");
        }

        if (Win)
        {
            parts.Add("Win");
        }

        parts.Add(VirtualKeyName(VirtualKey));
        return string.Join("+", parts);
    }

    public HotkeyBinding Clone() => new()
    {
        Control = Control,
        Shift = Shift,
        Alt = Alt,
        Win = Win,
        VirtualKey = VirtualKey,
    };

    public static string VirtualKeyName(int vk) => vk switch
    {
        >= 0x70 and <= 0x87 => "F" + (vk - 0x6F).ToString(),
        >= 0x30 and <= 0x39 => ((char)vk).ToString(),
        >= 0x41 and <= 0x5A => ((char)vk).ToString(),
        0x20 => "Space",
        0x2D => "Insert",
        0x2E => "Delete",
        0x24 => "Home",
        0x23 => "End",
        0x21 => "PageUp",
        0x22 => "PageDown",
        _ => "VK_0x" + vk.ToString("X2"),
    };
}
