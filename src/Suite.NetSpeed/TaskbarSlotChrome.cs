namespace Suite.NetSpeed;

/// <summary>
/// Pure layout math for the taskbar net-speed slot.
/// Spec: design/NETSPEED-TASKBAR-SPEC.md §2.2.1 — runnable on Debian without WPF.
/// </summary>
public static class TaskbarSlotChrome
{
    public const double DefaultFontSizeDip = 13;
    public const double LineHeight = 1.05;
    public const double OpticalNudgeDip = 1;
    public const double PadHorizontalDip = 8;
    public const double FloatFontSizeDip = 13;
    public const int WidgetWidthPx = 148;

    public static double ClampFontSize(double fontSizeDip) =>
        Math.Clamp(fontSizeDip, 10, 18);

    public static double ContentHeightDip(double fontSizeDip, int visibleLineCount)
    {
        int lines = Math.Max(1, visibleLineCount);
        return ClampFontSize(fontSizeDip) * LineHeight * lines;
    }

    /// <summary>
    /// Top offset for the rate block so it is optically centered in the slot.
    /// +1 DIP nudge (Consolas visual center sits slightly high). When content is taller than the slot, 0.
    /// </summary>
    public static double OffsetYDip(double windowHeightDip, double fontSizeDip, int visibleLineCount)
    {
        double leftover = windowHeightDip - ContentHeightDip(fontSizeDip, visibleLineCount);
        if (leftover <= 0)
        {
            return 0;
        }

        return (leftover / 2.0) + OpticalNudgeDip;
    }
}
