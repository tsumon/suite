namespace Suite.Platform;

/// <summary>
/// Detected by window class tree, not by "is this Windows 11?".
/// Classic Win32: Shell_TrayWnd without a XAML DesktopWindowContentBridge child.
/// Modern XAML: that bridge class is present (Win11 default taskbar, or similar).
/// </summary>
public enum TaskbarKind
{
    NotFound = 0,
    ClassicWin32 = 1,
    ModernXaml = 2,
}
