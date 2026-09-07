namespace Suite.Contracts;

/// <summary>
/// Net-speed visibility, taskbar embed, and TrafficMonitor-style appearance.
/// Spec: design/NETSPEED-TASKBAR-SPEC.md §3. Missing JSON fields use these defaults (no schema bump required).
/// </summary>
public sealed class NetSpeedSettings
{
    public const double DefaultFontSizeDip = 13;
    public const double MinFontSizeDip = 10;
    public const double MaxFontSizeDip = 18;

    public bool Visible { get; set; } = true;
    public bool EmbedInTaskbar { get; set; } = true;

    /// <summary>Also embed on Shell_SecondaryTrayWnd. Default off. INTERACTION-P2 §7.</summary>
    public bool EmbedSecondary { get; set; }
    public uint? IfIndex { get; set; }
    public string? AdapterAlias { get; set; }
    public double Left { get; set; } = 64;
    public double Top { get; set; } = 64;

    /// <summary>Taskbar slot font size only. Floating window stays 13.</summary>
    public double FontSizeDip { get; set; } = DefaultFontSizeDip;

    public bool ShowDownload { get; set; } = true;
    public bool ShowUpload { get; set; } = true;

    /// <summary>Null = follow system ink (and zero-rate muted color).</summary>
    public uint? DownColorArgb { get; set; }

    /// <summary>Null = follow system ink (and zero-rate muted color).</summary>
    public uint? UpColorArgb { get; set; }

    public bool BackgroundTransparent { get; set; } = true;

    /// <summary>Used when BackgroundTransparent is false.</summary>
    public uint? BackgroundArgb { get; set; }

    public bool ShowBorder { get; set; }
    public bool TwoLine { get; set; } = true;
    public bool Bold { get; set; }

    public NetSpeedSettings Clone() => new()
    {
        Visible = Visible,
        EmbedInTaskbar = EmbedInTaskbar,
        EmbedSecondary = EmbedSecondary,
        IfIndex = IfIndex,
        AdapterAlias = AdapterAlias,
        Left = Left,
        Top = Top,
        FontSizeDip = FontSizeDip,
        ShowDownload = ShowDownload,
        ShowUpload = ShowUpload,
        DownColorArgb = DownColorArgb,
        UpColorArgb = UpColorArgb,
        BackgroundTransparent = BackgroundTransparent,
        BackgroundArgb = BackgroundArgb,
        ShowBorder = ShowBorder,
        TwoLine = TwoLine,
        Bold = Bold,
    };

    /// <summary>Clamp font; if both lines off, restore both on (never persist a blank widget).</summary>
    public void Normalize()
    {
        FontSizeDip = Math.Clamp(FontSizeDip, MinFontSizeDip, MaxFontSizeDip);
        if (!ShowDownload && !ShowUpload)
        {
            ShowDownload = true;
            ShowUpload = true;
        }
    }

    public int VisibleLineCount()
    {
        if (!TwoLine)
        {
            return (ShowDownload || ShowUpload) ? 1 : 0;
        }

        int n = 0;
        if (ShowDownload)
        {
            n++;
        }

        if (ShowUpload)
        {
            n++;
        }

        return n;
    }
}
